// Hollow Hours server you host yourself (a VPS, a home server, anything with Node.js 20+).
// Make the server on the website first ("Hosted somewhere else?"): it gives you HH_CODE and HH_SECRET.
//
//   HH_CODE=ABC123 HH_SECRET=... HH_PUBLIC_URL=wss://hh.example.com PORT=8080 node server.mjs
//
// HH_PUBLIC_URL must be exactly the address you registered. Put it behind TLS (wss://), e.g. Caddy or nginx in
// front of PORT. Same room protocol as the official server. Players' logins never reach this server: each player
// brings a 60-second ticket signed by the website, and this server checks it with the website's public key.

import { WebSocketServer } from "ws";
import { createPublicKey, verify } from "node:crypto";

const SITE = (process.env.HH_SITE || "https://hollow-hours-mp.jdj212223.workers.dev").replace(/\/$/, "");
const CODE = String(process.env.HH_CODE || "").toUpperCase();
const SECRET = process.env.HH_SECRET || "";
const PUBLIC_URL = String(process.env.HH_PUBLIC_URL || "").replace(/\/+$/, "");
const PORT = parseInt(process.env.PORT || "8080");
const MAX = Math.min(40, parseInt(process.env.HH_MAX || "20"));
if (!CODE || !SECRET || !PUBLIC_URL) {
  console.error("Set HH_CODE, HH_SECRET and HH_PUBLIC_URL (from the website, when you made the server).");
  process.exit(1);
}

// The website's ticket key (Ed25519, raw 32 bytes as hex) → a Node public key.
const keyHex = (await (await fetch(`${SITE}/api/ticket-key`)).json()).public_key;
const publicKey = createPublicKey({ key: Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), Buffer.from(keyHex, "hex")]), format: "der", type: "spki" });
const seenTickets = new Map(); // ticket → expiry: each ticket works once

/** Checks a ticket: signed by the website, not expired, made for THIS server, not used before. Returns the player. */
function checkTicket(t) {
  const [payload, sig] = String(t || "").split(".");
  if (!payload || !sig) return null;
  const ok = verify(null, Buffer.from(payload), publicKey, Buffer.from(sig, "base64url"));
  if (!ok) return null;
  let p;
  try { p = JSON.parse(Buffer.from(payload, "base64url").toString()); } catch { return null; }
  const now = Math.floor(Date.now() / 1000);
  if (p.v !== 1 || p.aud !== PUBLIC_URL || !(p.exp > now) || seenTickets.has(t)) return null;
  seenTickets.set(t, p.exp);
  return { uid: p.uid, name: String(p.name).slice(0, 16) };
}

let order = 0;
const players = new Map(); // ws → { id, uid, name, joined }
const hostId = () => [...players.values()].sort((a, b) => a.joined - b.joined)[0]?.id ?? null;
const broadcast = (msg, except) => { const t = JSON.stringify(msg); for (const ws of players.keys()) if (ws !== except && ws.readyState === 1) ws.send(t); };

const wss = new WebSocketServer({ port: PORT, maxPayload: 16384 });
wss.on("connection", (ws, req) => {
  const path = (req.url || "").split("?")[0];
  const auth = req.headers["authorization"] || "";
  const who = path.toUpperCase() === `/ROOM/${CODE}` && auth.startsWith("Ticket ") ? checkTicket(auth.slice(7).trim()) : null;
  if (!who) { ws.close(4001, "log in first"); return; }
  for (const [old, p] of players) if (p.uid === who.uid) { old.send(JSON.stringify({ t: "error", msg: "you joined from somewhere else" })); old.close(4000, "replaced"); players.delete(old); }
  if (players.size >= MAX) { ws.close(4029, "room is full"); return; }
  const me = { id: Math.random().toString(36).slice(2, 10), uid: who.uid, name: who.name, joined: ++order };
  players.set(ws, me);
  ws.send(JSON.stringify({ t: "hello", you: me.id, host: hostId(), players: players.size, name: null, mods: [] }));
  broadcast({ t: "host", host: hostId(), players: players.size }, ws);
  ws.on("message", (data) => {
    let msg;
    try { msg = JSON.parse(String(data)); } catch { return; }
    msg.from = me.id;
    if (msg.t === "state") msg.name = me.name; // verified name only
    if (msg.t === "world" && me.id !== hostId()) return;
    broadcast(msg, ws);
  });
  ws.on("close", () => {
    if (players.delete(ws)) broadcast({ t: "left", from: me.id, host: hostId(), players: players.size });
  });
});

// Tell the website we're alive (and how many are playing) every minute, so the server list shows us.
async function heartbeat() {
  try {
    const r = await fetch(`${SITE}/api/servers/heartbeat`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ code: CODE, secret: SECRET, players: players.size }) });
    if (!r.ok) console.error("heartbeat refused:", (await r.json()).error);
  } catch (e) { console.error("heartbeat failed:", e.message); }
  const now = Math.floor(Date.now() / 1000);
  for (const [t, exp] of seenTickets) if (exp < now) seenTickets.delete(t);
}
heartbeat();
setInterval(heartbeat, 60_000);
console.log(`Hollow Hours server ${CODE} on port ${PORT} (public address ${PUBLIC_URL})`);
