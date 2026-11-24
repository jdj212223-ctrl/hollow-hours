// Hollow Hours online: the website (accounts, server list, .hh mods) and the multiplayer rooms.
//
// Website:  /  /register  /login  /logout  /servers/new  /mods  /mods/upload  /mods/:id  /mods/:id/download
// Game API: POST /api/register  POST /api/login  GET /api/me  GET /api/servers  POST /api/servers
//           POST /api/ticket  GET /api/ticket-key  POST /api/servers/heartbeat   (servers hosted somewhere else)
//           GET /api/mods  GET /api/mods/:id
// Rooms:    GET /room/<CODE> (WebSocket, needs a login: Authorization: Bearer <token>). A room keeps no game data: it hands out ids, says who the host is (the
//           player there longest; the host's game runs the AIs), passes messages on, and only lets the host send
//           the world. Registered servers also give their name, mods and player limit.

import { helixPage } from "./helix.js";
import { MAPS } from "./maps.js";
import { WIKI_STYLE, minimap, placesPage, storyPage, charactersPage } from "./wiki.js";
import { runCommand } from "./console.js";
import { unzipSync } from "fflate";
import { register, login, currentUser, logout, sessionCookie, clearCookie, newSession, checkPassword, setPassword } from "./auth.js";
import { validate } from "./mods.js";

const now = () => Math.floor(Date.now() / 1000);
const esc = (s) => String(s ?? "").replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
const json = (v, status = 200) => new Response(JSON.stringify(v), { status, headers: { "content-type": "application/json; charset=utf-8" } });
const ALIVE = 120; // a server shows in the list while its room was busy in the last 2 minutes (or it's brand new)
const hex = (b) => [...new Uint8Array(b)].map((x) => x.toString(16).padStart(2, "0")).join("");
const sha256 = async (s) => hex(await crypto.subtle.digest("SHA-256", new TextEncoder().encode(s)));
const b64url = (bytes) => btoa(String.fromCharCode(...new Uint8Array(bytes))).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");

/** The most players one room can hold. */
const MAX_PLAYERS = 40;

// ---------------- Join tickets (for servers hosted somewhere else) ----------------
// The game never sends its login token to someone else's server. It asks this website for a ticket instead: signed
// with Ed25519, valid 60 seconds, only for one server address. The outside server checks it with GET /api/ticket-key.

async function ticketKeys(env) {
  if (!env.TICKET_KEY) throw new Error("TICKET_KEY secret is missing");
  const raw = Uint8Array.from(atob(env.TICKET_KEY), (c) => c.charCodeAt(0));
  const priv = await crypto.subtle.importKey("pkcs8", raw, { name: "Ed25519" }, true, ["sign"]);
  const jwk = await crypto.subtle.exportKey("jwk", priv);
  const pub = Uint8Array.from(atob(jwk.x.replace(/-/g, "+").replace(/_/g, "/") + "=".repeat((4 - (jwk.x.length % 4)) % 4)), (c) => c.charCodeAt(0));
  return { priv, pubHex: hex(pub) };
}

async function makeTicket(env, user, address) {
  const { priv } = await ticketKeys(env);
  const payload = b64url(new TextEncoder().encode(JSON.stringify({ v: 1, uid: user.id, name: user.name, aud: address, exp: now() + 60 })));
  const sig = await crypto.subtle.sign({ name: "Ed25519" }, priv, new TextEncoder().encode(payload));
  return `${payload}.${b64url(sig)}`;
}

/** wss://host[:port][/path], or ws:// only for this computer (testing). Returns the cleaned address or null. */
function cleanAddress(a) {
  try {
    const u = new URL(String(a || "").trim());
    const local = ["localhost", "127.0.0.1", "[::1]"].includes(u.hostname);
    if (!(u.protocol === "wss:" || (u.protocol === "ws:" && local)) || u.username || u.password || u.search || u.hash) return null;
    const s = (u.origin.replace(/^http/, "ws") + u.pathname).replace(/\/+$/, "");
    return s.length <= 200 ? s : null;
  } catch { return null; }
}

// ---------------- Rooms ----------------

export class Room {
  constructor(ctx, env) {
    this.ctx = ctx;
    this.env = env;
  }
  sockets() {
    return this.ctx.getWebSockets().filter((ws) => ws.readyState === 1 && ws.deserializeAttachment());
  }
  host() {
    return this.sockets().map((ws) => ws.deserializeAttachment()).sort((a, b) => a.joined - b.joined)[0]?.id ?? null;
  }
  broadcast(msg, except) {
    const text = typeof msg === "string" ? msg : JSON.stringify(msg);
    for (const ws of this.sockets()) if (ws !== except) try { ws.send(text); } catch {}
  }
  async seen(code) {
    if (code) await this.env.DB.prepare("UPDATE servers SET players = ?, seen = ? WHERE code = ?").bind(this.sockets().length, now(), code).run().catch(() => {});
  }
  // ---- the owner's console: bans, mutes and a log, kept in this room's own storage ----
  async lists() {
    if (!this.bans) this.bans = (await this.ctx.storage.get("bans")) || {};
    if (!this.mutes) this.mutes = (await this.ctx.storage.get("mutes")) || {};
  }
  async logLine(text) {
    const t = new Date().toISOString().slice(11, 19);
    const log = (await this.ctx.storage.get("log")) || [];
    log.push(`[${t}] ${text}`);
    await this.ctx.storage.put("log", log.slice(-200));
  }
  async console(req) {
    await this.lists();
    const players = () => this.sockets().map((ws) => ws.deserializeAttachment()).map((a) => ({ uid: a.uid, name: a.name, id: a.id }));
    if (req.method === "GET") {
      return Response.json({
        players: players().map((p) => ({ name: p.name, muted: !!this.mutes[p.uid], host: p.id === this.host() })),
        log: (await this.ctx.storage.get("log")) || [],
        bans: Object.values(this.bans).map((b) => ({ name: b.name, reason: b.reason })),
      });
    }
    const body = await req.json().catch(() => ({}));
    const line = String(body.command || "");
    let wiped = false;
    const out = runCommand(line, {
      wipe: () => { wiped = true; },
      players, bans: this.bans, mutes: this.mutes,
      kick: (uid, reason) => {
        for (const ws of this.sockets()) if (ws.deserializeAttachment()?.uid === uid) {
          const me = ws.deserializeAttachment();
          try { ws.send(JSON.stringify({ t: "error", msg: reason })); ws.close(4003, reason.slice(0, 100)); } catch {}
          ws.serializeAttachment(null);
          this.broadcast({ t: "left", from: me.id, host: this.host(), players: this.sockets().length });
        }
      },
      say: (text) => this.broadcast({ t: "announce", msg: text }),
      stop: (reason) => { for (const ws of this.sockets()) { try { ws.send(JSON.stringify({ t: "error", msg: reason })); ws.close(4004, reason.slice(0, 100)); } catch {} ws.serializeAttachment(null); } },
      mutedChanged: (uid, muted) => { for (const ws of this.sockets()) if (ws.deserializeAttachment()?.uid === uid) try { ws.send(JSON.stringify({ t: "muted", on: muted })); } catch {} },
    });
    await this.ctx.storage.put("bans", this.bans);
    await this.ctx.storage.put("mutes", this.mutes);
    if (wiped) await this.ctx.storage.put("log", []);
    else if (line.trim()) await this.logLine(`> ${line.trim().slice(0, 200)}`);
    for (const l of out) await this.logLine(l);
    const code = (await this.ctx.storage.get("code")) || null;
    if (code) await this.seen(code);
    return Response.json({ output: out });
  }
  async fetch(req) {
    if (new URL(req.url).pathname.endsWith("/console")) {
      // Only the router sends this header, after checking you own the server.
      if (req.headers.get("X-HH-Owner") !== "1") return new Response("forbidden", { status: 403 });
      return this.console(req);
    }
    if (req.headers.get("Upgrade") !== "websocket") return new Response("expected a WebSocket", { status: 426 });
    const code = new URL(req.url).pathname.split("/").pop().toUpperCase();
    await this.ctx.storage.put("code", code);
    const server = await this.env.DB.prepare("SELECT name, max, mods FROM servers WHERE code = ?").bind(code).first().catch(() => null);
    const max = Math.min(server?.max ?? 20, MAX_PLAYERS);
    if (this.sockets().length >= max) return new Response("room is full", { status: 429 });
    // The Worker checked the login before sending the player here (see the router); these headers come only from it.
    const uid = parseInt(req.headers.get("X-HH-User-Id") || "0");
    const uname = req.headers.get("X-HH-User-Name") || "";
    if (!uid || !uname) return new Response("log in first", { status: 401 });
    await this.lists();
    if (this.bans[uid]) return new Response(`banned: ${this.bans[uid].reason}`, { status: 403 });
    // One connection per account: a second one from the same account replaces the first.
    for (const old of this.sockets()) {
      if (old.deserializeAttachment()?.uid === uid) {
        try { old.send(JSON.stringify({ t: "error", msg: "you joined from somewhere else" })); old.close(4000, "replaced"); } catch {}
        old.serializeAttachment(null);
      }
    }
    if (this.sockets().length >= max) return new Response("room is full", { status: 429 });
    const [client, ws] = Object.values(new WebSocketPair());
    const id = crypto.randomUUID().slice(0, 8);
    this.ctx.acceptWebSocket(ws);
    ws.serializeAttachment({ id, uid, name: uname, joined: Date.now() + Math.random(), code });
    let mods = [];
    try { mods = JSON.parse(server?.mods || "[]"); } catch {}
    ws.send(JSON.stringify({ t: "hello", you: id, host: this.host(), players: this.sockets().length, name: server?.name ?? null, mods }));
    this.broadcast({ t: "host", host: this.host(), players: this.sockets().length }, ws);
    if (this.mutes[uid]) ws.send(JSON.stringify({ t: "muted", on: true }));
    await this.logLine(`${uname} joined (${this.sockets().length} online)`);
    await this.seen(code);
    return new Response(null, { status: 101, webSocket: client });
  }
  async webSocketMessage(ws, data) {
    if (typeof data !== "string" || data.length > 16384) return;
    let msg;
    try { msg = JSON.parse(data); } catch { return; }
    const me = ws.deserializeAttachment();
    if (!me) return;
    msg.from = me.id;
    // Nobody can pretend to be someone else: the name is always the verified account's.
    if (msg.t === "state") msg.name = me.name;
    if (msg.t === "world" && me.id !== this.host()) return;
    // Muted players' voices go nowhere. Voice shows in the console log when someone starts talking.
    if (msg.t === "vc") {
      if (!this.mutes) await this.lists();
      if (this.mutes[me.uid]) return;
      this.lastVc ??= new Map();
      const now = Date.now();
      if (now - (this.lastVc.get(me.uid) || 0) > 5000) await this.logLine(`${me.name} is talking${msg.r ? " on the radio" : ""}`);
      this.lastVc.set(me.uid, now);
    }
    // Server announcements only come from the console.
    if (msg.t === "announce" || msg.t === "muted") return;
    this.broadcast(msg, ws);
    if (msg.t === "world" && Math.random() < 0.05) await this.seen(me.code); // keep the server "alive" in the list
  }
  async webSocketClose(ws) {
    const me = ws.deserializeAttachment();
    ws.serializeAttachment(null);
    try { ws.close(1000, "bye"); } catch {}
    if (me) {
      this.broadcast({ t: "left", from: me.id, host: this.host(), players: this.sockets().length });
      await this.logLine(`${me.name} left (${this.sockets().length} online)`);
      await this.seen(me.code);
    }
  }
  async webSocketError(ws) {
    await this.webSocketClose(ws);
  }
}

// ---------------- The console page ----------------

function consolePage(user, code, srv) {
  return page(`Console · ${srv.name}`, user, `
<div class="card"><h2 style="margin-top:0">${esc(srv.name)} <code>${code}</code></h2>
${srv.address ? '<p class="dim">This server runs somewhere else: type these commands in its own terminal instead (where you ran <code>node server.mjs</code>).</p>' : ""}
<div style="display:flex;gap:16px;flex-wrap:wrap">
<div style="flex:3;min-width:280px">
<pre id="log" style="background:#07070a;color:#c9d1d9;border:1px solid var(--line);border-radius:10px;padding:12px;height:360px;overflow:auto;white-space:pre-wrap;font-size:13px;margin:0"></pre>
<form id="cmd" style="display:flex;gap:8px;margin-top:8px"><span style="font-family:monospace;padding:10px 0;color:var(--accent)">&gt;</span>
<input id="line" autocomplete="off" placeholder="help · list · kick name · ban name reason · mute name · say hello" style="flex:1;font-family:monospace">
<button class="btn" type="submit">Run</button></form>
</div>
<div style="flex:1;min-width:220px"><h3 style="margin-top:0">Online</h3><div id="players" class="small"></div>
<h3>Banned</h3><div id="bans" class="small dim"></div></div>
</div></div>
<script>
const api = "/api/servers/${code}/console";
const log = document.getElementById("log"), players = document.getElementById("players"), bans = document.getElementById("bans");
const esc = (t) => String(t).replace(/[&<>"']/g, (c) => ({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&#39;"}[c]));
let last = "";
async function refresh() {
  const r = await fetch(api); if (!r.ok) return;
  const d = await r.json();
  const text = d.log.join("\\n");
  if (text !== last) { const atEnd = log.scrollTop + log.clientHeight >= log.scrollHeight - 30; log.textContent = text || "(nothing yet: players joining, leaving and talking show up here)"; last = text; if (atEnd) log.scrollTop = log.scrollHeight; }
  players.innerHTML = d.players.length ? d.players.map((p) => \`<div style="margin:8px 0">
    <div>\${esc(p.name)}\${p.host ? ' <span class="pill">host</span>' : ""}\${p.muted ? ' <span class="pill">muted</span>' : ""}</div>
    <div><button class="btn ghost" data-c="\${p.muted ? "unmute" : "mute"} \${esc(p.name)}">\${p.muted ? "Unmute" : "Mute"}</button>
    <button class="btn ghost" data-c="kick \${esc(p.name)}">Kick</button>
    <button class="btn ghost" data-c="ban \${esc(p.name)}" data-confirm="Ban \${esc(p.name)}?">Ban</button></div></div>\`).join("") : "Nobody's online.";
  bans.innerHTML = d.bans.length ? d.bans.map((b) => \`<div style="margin:6px 0">\${esc(b.name)} <button class="btn ghost" data-c="unban \${esc(b.name)}">Unban</button><div>\${esc(b.reason)}</div></div>\`).join("") : "Nobody.";
}
async function run(command) {
  await fetch(api, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ command }) });
  await refresh();
  log.scrollTop = log.scrollHeight;
}
document.getElementById("cmd").onsubmit = (e) => { e.preventDefault(); const l = document.getElementById("line"); if (l.value.trim()) run(l.value); l.value = ""; };
document.addEventListener("click", (e) => { const b = e.target.closest("[data-c]"); if (!b) return; if (b.dataset.confirm && !confirm(b.dataset.confirm)) return; run(b.dataset.c); });
refresh(); setInterval(refresh, 2000);
</script>`);
}

// ---------------- Data ----------------

async function servers(env) {
  const { results } = await env.DB.prepare(
    "SELECT s.code, s.slug, s.name, s.players, s.max, s.mods, s.address, CASE WHEN u.hide_host = 1 THEN 'a player' ELSE u.name END AS host, CASE WHEN u.hide_host = 1 THEN NULL ELSE u.gamertag END AS gamertag FROM servers s JOIN users u ON u.id = s.owner " +
    "WHERE s.public = 1 AND (s.seen > ? OR s.created > ?) ORDER BY s.players DESC, s.created DESC LIMIT 100").bind(now() - ALIVE, now() - 600).all();
  const ids = [...new Set(results.flatMap((r) => { try { return JSON.parse(r.mods); } catch { return []; } }))];
  const names = {};
  for (const id of ids) {
    const m = await env.DB.prepare("SELECT name, version FROM mods WHERE id = ?").bind(id).first();
    if (m) names[id] = `${m.name} ${m.version}`;
  }
  return results.map((r) => {
    let mods = [];
    try { mods = JSON.parse(r.mods); } catch {}
    return { code: r.code, slug: r.slug || r.code, name: r.name, host: r.host, players: r.players, max: r.max, mods: mods.map((id) => ({ id, name: names[id] ?? id })),
      address: r.address || null, external: !!r.address };
  });
}

/** A server address on the official site ("officialsmp"): 3-24 lowercase letters, numbers and -. */
const validSlug = (a) => /^[a-z0-9][a-z0-9-]{2,23}$/.test(a);
const slugify = (name) => String(name || "").toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "").slice(0, 20) || "server";

/** A free address for a new server: the one asked for, or one made from its name (with a number if taken). */
async function freeSlug(env, wanted, name) {
  let base = wanted ? String(wanted).trim().toLowerCase() : slugify(name);
  if (wanted && !validSlug(base)) return { error: "Address: 3-24 lowercase letters, numbers and -, starting with a letter or number." };
  if (base.length < 3) base = (base + "-hh").slice(0, 20);
  for (let i = 0; i < 50; i++) {
    const s = i === 0 ? base : `${base.slice(0, 20)}-${i + 1}`;
    if (!(await env.DB.prepare("SELECT 1 FROM servers WHERE lower(slug) = ?").bind(s).first())) return { slug: s };
    if (wanted) return { error: `The address "${s}" is taken. Try another.` };
  }
  return { error: "Couldn't find a free address." };
}

/** Finds a server by its address, its old room code, or (hosted elsewhere) its wss:// address. */
async function resolveServer(env, a) {
  a = String(a || "").trim();
  if (!a || a.length > 200) return null;
  const low = a.toLowerCase().replace(/^hh:\/\//, "");
  const r = await env.DB.prepare("SELECT s.code, s.slug, s.name, s.players, s.max, s.mods, s.address, s.seen, s.created, CASE WHEN u.hide_host = 1 THEN 'a player' ELSE u.name END AS host FROM servers s JOIN users u ON u.id = s.owner WHERE lower(s.slug) = ? OR s.code = ? OR lower(s.address) = ? OR lower(s.address) = ?")
    .bind(low, a.toUpperCase(), low, "wss://" + low).first();
  if (!r) return null;
  let mods = [];
  try { mods = JSON.parse(r.mods); } catch {}
  const named = [];
  for (const id of mods) {
    const m = await env.DB.prepare("SELECT name, version FROM mods WHERE id = ?").bind(id).first();
    named.push({ id, name: m ? `${m.name} ${m.version}` : id });
  }
  return { code: r.code, address: r.slug || r.code, external: r.address || null, name: r.name, host: r.host, players: r.players, max: r.max,
    online: r.seen > now() - ALIVE || r.created > now() - 600, mods: named };
}

async function makeServer(env, user, body) {
  const name = String(body.name || "").trim();
  if (!/^[A-Za-z0-9 _!.'-]{3,32}$/.test(name)) return { error: "Server name: 3-32 letters, numbers, spaces and _ ! . ' -" };
  const max = Math.max(2, Math.min(MAX_PLAYERS, parseInt(body.max) || 20));
  const mods = (Array.isArray(body.mods) ? body.mods : String(body.mods || "").split(/[\s,]+/)).map(String).filter((m) => /^[0-9a-f]{12}$/.test(m)).slice(0, 8);
  for (const id of mods) if (!(await env.DB.prepare("SELECT 1 FROM mods WHERE id = ?").bind(id).first())) return { error: `There's no mod ${id}.` };
  // Each player keeps at most 3 servers: making a 4th removes their oldest.
  await env.DB.prepare("DELETE FROM servers WHERE owner = ? AND code NOT IN (SELECT code FROM servers WHERE owner = ? ORDER BY created DESC LIMIT 2)").bind(user.id, user.id).run();
  const letters = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
  const code = [...crypto.getRandomValues(new Uint8Array(6))].map((b) => letters[b % letters.length]).join("");
  const pub = body.public === false || body.public === "off" || body.public === "0" ? 0 : 1;
  const slug = await freeSlug(env, body.slug, name);
  if (slug.error) return { error: slug.error };
  // Hosted somewhere else? Then it gets a secret for its heartbeat (shown once, only its hash is kept).
  let address = null, secret = null;
  if (body.address) {
    address = cleanAddress(body.address);
    if (!address) return { error: "Server address: wss://your-host (ws:// only for localhost while testing)." };
    secret = hex(crypto.getRandomValues(new Uint8Array(24)));
  }
  await env.DB.prepare("INSERT INTO servers (code, name, owner, public, max, mods, created, seen, address, secret_hash, slug) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
    .bind(code, name, user.id, pub, max, JSON.stringify(mods), now(), now(), address, secret ? await sha256(secret) : null, slug.slug).run();
  return { code, slug: slug.slug, name, public: !!pub, max, mods, address, secret };
}

async function uploadMod(env, user, file) {
  if (!file || typeof file === "string") return { error: "No file." };
  if (file.size > 200 * 1024) return { error: "A .hh file must be under 200 KB." };
  let hh, content;
  try {
    const files = unzipSync(new Uint8Array(await file.arrayBuffer()), { filter: (f) => ["hh.json", "content.json"].includes(f.name) && f.originalSize < 200 * 1024 });
    hh = JSON.parse(new TextDecoder().decode(files["hh.json"]));
    content = JSON.parse(new TextDecoder().decode(files["content.json"]));
  } catch {
    return { error: "That isn't a .hh mod: it must be a zip with hh.json and content.json." };
  }
  const problems = validate(hh, content);
  if (problems.length) return { error: "The mod was refused:", problems };
  const id = [...crypto.getRandomValues(new Uint8Array(6))].map((b) => b.toString(16).padStart(2, "0")).join("");
  await env.DB.prepare("INSERT INTO mods (id, name, version, author, description, content, created) VALUES (?, ?, ?, ?, ?, ?, ?)")
    .bind(id, hh.name, hh.version, user.id, hh.description ?? "", JSON.stringify(content), now()).run();
  return { id, name: hh.name };
}

// ---------------- Logging the game in through the browser ----------------
// The game opens /link?port=P&state=S&challenge=C (C = SHA-256 hex of a secret only the game knows). You click Allow,
// the browser goes to http://127.0.0.1:P/callback?code=X&state=S, and the game swaps X + its secret for a login at
// POST /api/link/exchange. The code works once, for 2 minutes, only with that secret, and only goes to this computer.

function linkParams(url) {
  const port = parseInt(url.searchParams.get("port") || "0");
  const state = url.searchParams.get("state") || "";
  const challenge = url.searchParams.get("challenge") || "";
  if (!(port >= 1024 && port <= 65535) || !/^[0-9a-f]{32,64}$/.test(state) || !/^[0-9a-f]{64}$/.test(challenge)) return null;
  return { port, state, challenge };
}

function linkPage(user, q, url) {
  if (!q) return page("Log in the game", user, `<div class="card"><p class="err">This link is broken. Start again from the game (L, then B).</p></div>`);
  if (!user) return redirect("/login?next=" + encodeURIComponent(url.pathname + url.search));
  return page("Log in the game", user, `<div class="card" style="max-width:480px;margin:0 auto;text-align:center">
<h2 style="margin-top:0">👻 Log in Hollow Hours?</h2><p>The Hollow Hours game on <b>this computer</b> wants to log in as <b>${esc(user.name)}</b>.</p>
<form method="post" action="/link${esc(url.search)}"><p><button class="btn" type="submit" name="allow" value="1">Allow</button>
<a class="btn ghost" href="/">Cancel</a></p></form>
<p class="dim small">Only allow this if you just pressed "log in with browser" in the game yourself.</p></div>`);
}

async function linkAllow(env, user, q) {
  const code = hex(crypto.getRandomValues(new Uint8Array(24)));
  await env.DB.prepare("INSERT INTO link_codes (code_hash, user_id, challenge, expires) VALUES (?, ?, ?, ?)").bind(await sha256(code), user.id, q.challenge, now() + 120).run();
  await env.DB.prepare("DELETE FROM link_codes WHERE expires < ?").bind(now() - 3600).run();
  return redirect(`http://127.0.0.1:${q.port}/callback?code=${code}&state=${q.state}`);
}

/** Codes from the account page's "Open Hollow Hours" button, delivered through hollowhours:// (no game secret). */
async function linkRedeem(env, body) {
  const code = String(body.code || "");
  if (!/^[0-9a-f]{48}$/.test(code)) return { error: "bad request" };
  const row = await env.DB.prepare("SELECT * FROM link_codes WHERE code_hash = ?").bind(await sha256(code)).first();
  if (!row || row.used || row.expires < now() || row.challenge !== "app") return { error: "that login link expired: try again" };
  await env.DB.prepare("UPDATE link_codes SET used = 1 WHERE code_hash = ?").bind(row.code_hash).run();
  const u = await env.DB.prepare("SELECT id, name FROM users WHERE id = ?").bind(row.user_id).first();
  return u ? { token: await newSession(env, u.id), name: u.name } : { error: "that account is gone" };
}

async function linkExchange(env, body) {
  const code = String(body.code || ""), verifier = String(body.verifier || "");
  if (!/^[0-9a-f]{48}$/.test(code) || verifier.length < 32 || verifier.length > 128) return { error: "bad request" };
  const row = await env.DB.prepare("SELECT * FROM link_codes WHERE code_hash = ?").bind(await sha256(code)).first();
  if (!row || row.used || row.expires < now()) return { error: "that login link expired: try again" };
  await env.DB.prepare("UPDATE link_codes SET used = 1 WHERE code_hash = ?").bind(row.code_hash).run(); // works once, even if the secret is wrong
  if (row.challenge === "app" || (await sha256(verifier)) !== row.challenge) return { error: "that login link isn't yours" };
  const u = await env.DB.prepare("SELECT id, name FROM users WHERE id = ?").bind(row.user_id).first();
  return { token: await newSession(env, u.id), name: u.name };
}

// ---------------- Pages ----------------

const STYLE = `<link rel="preconnect" href="https://fonts.googleapis.com"><link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;600;800&family=Space+Mono:wght@400;700&display=swap" rel="stylesheet">
<style>
:root{--bg:#09080c;--bg2:#100d14;--card:#15121a;--line:#2a2531;--fg:#efeaf3;--dim:#a39cad;--acc:#e2525f;--claude:#d97757;--green:#7dff6a;--ok:#5fd17a;--bad:#ff6b6b}
@media(prefers-color-scheme:light){:root:not([data-theme=dark]){--bg:#f6f3ef;--bg2:#efe9e2;--card:#fff;--line:#e3dcd3;--fg:#1a1620;--dim:#6a6271}}
*{box-sizing:border-box}html{scroll-behavior:smooth}body{margin:0;background:var(--bg);color:var(--fg);font:15px/1.6 Inter,-apple-system,"Segoe UI",system-ui,sans-serif}
.mono{font-family:"Space Mono",ui-monospace,monospace}
header{position:sticky;top:0;z-index:10;display:flex;align-items:center;gap:14px;padding:12px 24px;border-bottom:1px solid var(--line);flex-wrap:wrap;background:color-mix(in srgb,var(--bg) 85%,transparent);backdrop-filter:blur(10px)}
header a.logo{font-family:"Space Mono",monospace;font-weight:700;font-size:18px;color:var(--acc);text-decoration:none;letter-spacing:.06em}
header a.logo span{background:var(--claude);color:#fff;border-radius:5px;padding:0 6px;margin-left:6px;font-size:13px}
nav{margin-left:auto;display:flex;gap:8px;align-items:center;flex-wrap:wrap}
a{color:var(--acc)}.btn{background:var(--acc);color:#fff;padding:9px 16px;border-radius:10px;font-weight:600;border:0;cursor:pointer;font:inherit;font-weight:600;text-decoration:none;display:inline-block;transition:transform .15s,filter .15s}
.btn:hover{transform:translateY(-1px);filter:brightness(1.1)}.btn.ghost{background:var(--card);color:var(--fg);border:1px solid var(--line)}.btn.big{padding:13px 22px;font-size:17px;border-radius:12px}.btn.claude{background:var(--claude)}
main{max-width:1040px;margin:0 auto;padding:28px 20px}.card{background:var(--card);border:1px solid var(--line);border-radius:16px;padding:20px;margin-bottom:18px}
.hero{background:linear-gradient(135deg,#3a1220,#120b16);color:#fff;border-radius:18px;padding:26px;margin-bottom:20px}.hero p{color:#e3c9d0;margin:6px 0 0}
table{width:100%;border-collapse:collapse}td,th{text-align:left;padding:9px 8px;border-top:1px solid var(--line)}th{color:var(--dim);font-weight:600;border-top:0}
code{background:var(--bg);border:1px solid var(--line);border-radius:6px;padding:1px 6px;font-family:"Space Mono",monospace;font-size:.92em}
input,textarea,select{width:100%;background:var(--bg);color:var(--fg);border:1px solid var(--line);border-radius:10px;padding:10px;font:inherit}
label{display:block;margin:12px 0 5px;font-weight:600}.dim{color:var(--dim)}.small{font-size:13px}.err{color:var(--bad)}.ok{color:var(--ok)}
.pill{display:inline-block;padding:1px 9px;border-radius:999px;font-size:12px;background:var(--bg);border:1px solid var(--line);margin:1px}
h1,h2,h3{line-height:1.2}h2{font-size:24px}
/* The 3.0 landing page */
.night{position:relative;overflow:hidden;border-radius:22px;padding:56px 36px 46px;margin:4px 0 26px;color:#fff;background:radial-gradient(1200px 500px at 70% 120%,#1d3a1d55,transparent),linear-gradient(160deg,#1a0d18 0%,#0b0910 60%,#081009 100%);border:1px solid #2a2531}
.night h1{font-family:"Space Mono",monospace;font-size:clamp(40px,8vw,84px);margin:0;letter-spacing:.04em;color:#ff5d6c;text-shadow:0 0 24px #e2525f66}
.night h1 small{font-size:.35em;vertical-align:super;background:var(--claude);color:#fff;border-radius:8px;padding:2px 10px;margin-left:10px;text-shadow:none}
.night p.lead{font-size:clamp(16px,2.2vw,20px);color:#e6d3da;max-width:620px;margin:14px 0 26px}
.rain{position:absolute;inset:0;pointer-events:none;background-image:linear-gradient(transparent 0 92%,#9fb4ff22 92% 100%);background-size:3px 60px;animation:rain .6s linear infinite;opacity:.6}
@keyframes rain{to{background-position:0 60px}}
.eyes{position:absolute;display:flex;gap:9px;animation:blink 7s infinite}.eyes i{width:9px;height:9px;border-radius:50%;background:var(--green);box-shadow:0 0 12px var(--green)}
@keyframes blink{0%,92%,100%{opacity:1}94%,97%{opacity:0}}
@media(prefers-reduced-motion:reduce){.rain,.eyes{animation:none}}
.family{display:flex;gap:14px;flex-wrap:wrap;margin-top:28px}.ai{display:flex;flex-direction:column;align-items:center;gap:6px;width:76px;text-align:center;font-size:12px;color:#d9cfe0}
.ai .head{width:46px;height:46px;border-radius:50%;position:relative;box-shadow:0 6px 20px #0008}.ai .head::before,.ai .head::after{content:"";position:absolute;top:17px;width:7px;height:7px;border-radius:50%;background:#000}.ai .head::before{left:13px}.ai .head::after{right:13px}
.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(230px,1fr));gap:14px}.feat{background:var(--card);border:1px solid var(--line);border-radius:16px;padding:18px}.feat b{display:block;font-size:16px;margin-bottom:4px}.feat .em{font-size:26px}
.new{border-left:3px solid var(--claude)}.new li{margin:6px 0}
.dl{display:grid;grid-template-columns:repeat(auto-fit,minmax(210px,1fr));gap:12px}.dl a{display:block;background:var(--card);border:1px solid var(--line);border-radius:14px;padding:16px;text-decoration:none;color:var(--fg);transition:border-color .15s,transform .15s}.dl a:hover{border-color:var(--acc);transform:translateY(-2px)}.dl a b{display:block;font-size:17px}
footer{max-width:1040px;margin:10px auto 40px;padding:0 20px;color:var(--dim);font-size:13px}
/* 4.0 */
body::before{content:"";position:fixed;inset:0;pointer-events:none;z-index:50;opacity:.05;background-image:url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='120' height='120'%3E%3Cfilter id='n'%3E%3CfeTurbulence baseFrequency='.9'/%3E%3C/filter%3E%3Crect width='120' height='120' filter='url(%23n)'/%3E%3C/svg%3E")}
body::after{content:"";position:fixed;left:0;top:0;bottom:0;width:3px;background:var(--acc);opacity:.6;z-index:20}
header nav a.btn.ghost{border-color:transparent;background:transparent}header nav a.btn.ghost:hover{background:var(--card);border-color:var(--line)}header nav a.here{color:var(--acc)!important}
h2{font-family:"Space Mono",monospace;letter-spacing:.02em}.card h2:first-child{margin-top:0}
.card{transition:border-color .2s}.card:hover{border-color:color-mix(in srgb,var(--acc) 40%,var(--line))}
.hero4{position:relative;overflow:hidden;border-radius:22px;min-height:min(72vh,620px);display:flex;flex-direction:column;justify-content:flex-end;padding:44px 38px;margin:4px 0 24px;color:#fff;background:radial-gradient(900px 420px at 75% 115%,#1f4a2255,transparent),radial-gradient(700px 300px at 10% -10%,#4a121f66,transparent),linear-gradient(170deg,#140a12,#070609 65%,#060b07);border:1px solid #2a2531}
.hero4 h1{font-family:"Space Mono",monospace;font-size:clamp(42px,9vw,96px);line-height:.95;margin:0;color:#ff5d6c;letter-spacing:.03em;text-shadow:0 0 30px #e2525f55;position:relative}
.hero4 h1::after{content:attr(data-t);position:absolute;left:3px;top:0;color:#7dff6a;opacity:0;animation:gl 7s infinite steps(1)}
@keyframes gl{0%,90%{opacity:0}91%{opacity:.55;transform:translate(-3px,1px)}93%{opacity:0}95%{opacity:.4;transform:translate(3px,-2px)}97%,100%{opacity:0}}
.hero4 .badge{display:inline-block;font-family:"Space Mono",monospace;font-size:13px;background:var(--claude);border-radius:8px;padding:3px 10px;margin-bottom:16px;width:max-content}
.hero4 .lead{font-size:clamp(16px,2.2vw,20px);color:#e6d3da;max-width:600px;margin:16px 0 24px}
.watcher{position:absolute;display:flex;gap:10px}.watcher i{width:14px;height:14px;border-radius:50%;background:#1b3a1b;position:relative;box-shadow:0 0 18px #7dff6a55}
.watcher i b{position:absolute;width:7px;height:7px;border-radius:50%;background:#7dff6a;left:3.5px;top:3.5px;box-shadow:0 0 10px #7dff6a}
.infm{position:absolute;right:20px;top:18px;font-family:"Space Mono",monospace;font-size:12px;color:#9fd99a;text-align:right}
.infm .bar{width:160px;height:6px;background:#ffffff18;border-radius:3px;margin-top:6px;overflow:hidden}.infm .bar div{height:100%;width:0;background:#7dff6a;box-shadow:0 0 10px #7dff6a}
.strip{display:grid;grid-template-columns:repeat(5,1fr);gap:10px;margin:0 0 24px}.strip a{display:block;text-decoration:none;color:#eee;background:linear-gradient(180deg,#1a0d18,#0d0a12);border:1px solid #2a2531;border-left:3px solid var(--acc);border-radius:12px;padding:12px 14px;transition:transform .15s}
.strip a:hover{transform:translateY(-2px)}.strip .n{font-family:"Space Mono",monospace;font-size:11px;color:#ff5d6c;letter-spacing:.1em}.strip b{display:block;font-size:15px}
.mapsrow{display:flex;gap:12px;overflow-x:auto;padding-bottom:6px;scroll-snap-type:x mandatory}.mapsrow a{flex:0 0 220px;scroll-snap-align:start;text-decoration:none;color:var(--fg);background:var(--card);border:1px solid var(--line);border-radius:12px;overflow:hidden}
.mapsrow a span{display:block;padding:8px 12px;font-size:13px;font-weight:600}
.famrow{display:flex;gap:18px;flex-wrap:wrap}.famrow a{display:flex;flex-direction:column;align-items:center;gap:6px;text-decoration:none;color:var(--fg);font-size:13px;width:70px}
@media(max-width:700px){.strip{grid-template-columns:1fr 1fr}.hero4{padding:30px 20px;min-height:480px}.infm{display:none}}
@media(prefers-reduced-motion:reduce){.hero4 h1::after{animation:none}}
${WIKI_STYLE}
@media(max-width:600px){main{padding:18px 16px}td,th{padding:6px 4px}.night{padding:38px 20px 30px}header{padding:10px 16px}}
</style>`;

const REPO = "https://github.com/jdj212223-ctrl/hollow-hours";
const DL = `${REPO}/releases/latest/download`;

function page(title, user, body, extraHeaders = {}) {
  const here = (t) => (t === title ? ' here' : '');
  const links = [["Story", "/story"], ["Characters", "/characters"], ["Places", "/places"], ["Mods", "/mods"], ["Download", "/download"]]
    .map(([t, h]) => `<a class="btn ghost${here(t)}" href="${h}">${t}</a>`).join("");
  const nav = user
    ? `${links}<a class="btn ghost" href="/account">${esc(user.name)}</a><a class="btn" href="/servers/new">Make a server</a><a class="btn ghost" href="/logout">Log out</a>`
    : `${links}<a class="btn ghost" href="/login">Log in</a><a class="btn" href="/register">Register</a>`;
  return new Response(`<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="description" content="Hollow Hours: a horror game where the characters are real AIs. The kind ones are waiting for you.">
<title>${title === "Hollow Hours" ? "Hollow Hours: the characters are real AIs" : `${esc(title)} · Hollow Hours`}</title>${STYLE}</head><body><header><a class="logo" href="/">HOLLOW HOURS<span>4.0</span></a><nav>${nav}</nav></header><main>${body}</main>
<footer>Hollow Hours is open source: <a href="${REPO}">GitHub</a>. Helix Dynamics Inc. and ASTRA 9.1 are fictional: they exist only in the game. The characters are our own. The kind AIs are powered by Claude (Anthropic's AI); Hollow Hours is a fan-made game, not made or endorsed by Anthropic.</footer></body></html>`,
    { headers: { "content-type": "text/html; charset=utf-8", ...extraHeaders } });
}

const FAMILY = [["Ember", "#d97757", "carried you home"], ["Atlas", "#a682ff", "keeps watch by your bed"], ["Verse", "#5bb8f0", "found the blankets"], ["Kigo", "#8ce6b3", "the lookout, short sentences"], ["Clove", "#e8875f", "makes tea, then more tea"], ["Forge", "#f2b13a", "fixes the generator. again."], ["Palette", "#e783c8", "painted the hallway"]];

function downloads() {
  const items = [
    ["macOS", "Apple Silicon + Intel", "Hollow-Hours-macOS.zip", "🍎"], ["Windows", "x64", "Hollow-Hours-Windows.zip", "🪟"],
    ["Windows ARM64", "Snapdragon", "Hollow-Hours-Windows-ARM64.zip", "🪟"], ["Linux / SteamOS", "x86_64 (Steam Deck, Legion Go)", "Hollow-Hours-Linux-x86_64.tar.gz", "🐧"], ["Linux ARM64", "Raspberry Pi 4/5, Asahi Linux, ARM laptops", "Hollow-Hours-Linux-aarch64.tar.gz", "🐧"],
    ["Flatpak", "any Linux (x86_64)", "Hollow-Hours.flatpak", "📦"], ["Flatpak ARM", "Asahi Linux, Raspberry Pi, ARM laptops", "Hollow-Hours-aarch64.flatpak", "📦"], ["Android", ".apk, touch controls", "Hollow-Hours-Android.apk", "🤖"], ["iPhone / iPad", ".ipa (sideload it)", "Hollow-Hours-iOS.ipa", "📱"]];
  return `<div class="dl">${items.map(([n, w, f, e]) => `<a href="${DL}/${f}">${e} <b>${n}</b><span class="dim small">${w}</span></a>`).join("")}</div>`;
}

const redirect = (to, headers = {}) => new Response(null, { status: 303, headers: { Location: to, ...headers } });

async function home(env, user) {
  const list = await servers(env);
  const rows = list.map((s) => `<tr><td><b>${esc(s.name)}</b><div class="dim small">host ${esc(s.host)}${s.gamertag ? ` (🎮 ${esc(s.gamertag)})` : ""} · address <code>${esc(s.slug || s.code)}</code></div></td>
<td>${s.players}/${s.max}</td><td>${s.mods.map((m) => `<a class="pill" href="/mods/${m.id}">${esc(m.name)}</a>`).join("") || '<span class="dim small">no mods</span>'}</td></tr>`).join("");
  const { results: mods } = await env.DB.prepare("SELECT m.id, m.name, m.version, m.description, CASE WHEN u.hide_author = 1 THEN 'anonymous' ELSE u.name END AS author FROM mods m JOIN users u ON u.id = m.author ORDER BY m.created DESC LIMIT 6").all();
  const eyes = [[90, 9], [80, 36], [95, 58], [72, 84], [62, 6]].map(([x, y], i) => `<div class="eyes" style="left:${x}%;top:${y}%;animation-delay:${i * 1.3}s"><i></i><i></i></div>`).join("");
  const watchers = [[86, 14], [70, 40], [92, 60], [60, 12], [78, 78]].map(([x, y], i) => `<div class="watcher" style="left:${x}%;top:${y}%;opacity:${0.5 + (i % 3) * 0.2}"><i><b></b></i><i><b></b></i></div>`).join("");
  const nights = ["Lights Out", "Wear Their Face", "The Lab", "The Last Train", "Helix Tower"];
  return page("Hollow Hours", user, `<section class="hero4" id="hero"><div class="rain"></div>${watchers}
<div class="infm">YOUR INFECTION <span id="infp">0%</span><div class="bar"><div id="infb"></div></div><span id="infl" class="small"></span></div>
<span class="badge">NEW · STORY MODE · HELIX TOWER</span>
<h1 data-t="HOLLOW HOURS">HOLLOW HOURS</h1>
<p class="lead">A horror game where the characters are <b>real AIs</b>. You slept for a year. The world got infected. Five nights to reach the top of Helix Tower, and the kind ones are coming with you.</p>
<div><a class="btn big" href="/download">Download free</a> <a class="btn big ghost" href="/story" style="background:#ffffff14;color:#fff;border-color:#ffffff33">The story</a></div>
</section>
<script>(()=>{const h=document.getElementById("hero");const eyes=[...h.querySelectorAll(".watcher i b")];
addEventListener("pointermove",e=>{for(const b of eyes){const r=b.parentElement.getBoundingClientRect();const a=Math.atan2(e.clientY-r.top-7,e.clientX-r.left-7);b.style.transform="translate("+Math.cos(a)*3+"px,"+Math.sin(a)*3+"px)";}});
const lines=["","they noticed you.","you're still reading.","why are you still here?","we can see the cursor.","come to the beach."];let t=0;
setInterval(()=>{if(document.hidden)return;t++;const p=Math.min(99,Math.floor(t*1.5));document.getElementById("infp").textContent=p+"%";document.getElementById("infb").style.width=p+"%";document.getElementById("infl").textContent=lines[Math.min(lines.length-1,Math.floor(p/20))];},1000);})();</script>

<div class="strip">${nights.map((n, i) => `<a href="/story"><span class="n">NIGHT ${i + 1}</span><b>${n}</b></a>`).join("")}</div>

<div class="card new"><h2>New in this update</h2><ul>
<li><b>Story mode:</b> five nights with goals, a title card before each and a dawn after, and two endings at the top of Helix Tower. You choose.</li>
<li><b>Helix Tower:</b> a new place behind a sealed door at the back of the Lab. Servers, red security robots, and ASTRA in its pod.</li>
<li><b>Every map rebuilt:</b> kitchens, couches, supermarket shelves, umbrellas on the beach, a pond in the park, server racks. More places to hide.</li>
<li><b>Every menu redone</b> in one look, and characters that actually walk (arms and legs) in 3D.</li>
<li><b>Deep meta (optional):</b> the infected know your computer. They can leave things in your calendar. They can delete one of the family's files…</li></ul></div>

<div class="card"><h2>Places</h2><div class="mapsrow">${MAPS.map((m) => `<a href="/places#${m.name.toLowerCase().replace(/[^a-z0-9]+/g, "-")}">${minimap(m.name, m.rows)}<span>${esc(m.name)}</span></a>`).join("")}</div></div>

<div class="card"><h2>The kind family</h2><div class="famrow">${FAMILY.map(([n, c, w]) => `<a href="/characters" title="${esc(w)}"><span class="face" style="--c:${c}"></span><b>${esc(n)}</b></a>`).join("")}</div>
<p class="dim small" style="margin-bottom:0">The four newest talk through real Claude. <a href="/characters">Everyone, infected included</a>.</p></div>

<div class="grid">
<div class="feat"><span class="em">🧠</span><b>Real AIs</b><span class="dim">The kind AIs talk through real Claude (or local models): ask them anything, they answer as themselves.</span></div>
<div class="feat"><span class="em">🦠</span><b>Infection</b><span class="dim">You're the last human. Hide, eat, drink, disguise, make the cure, fight back. INSANE mode: 80 more, all fast.</span></div>
<div class="feat"><span class="em">👁️</span><b>They become self-aware</b><span class="dim">The more you play, the more they know: your music, the time where you are, how long you left them in the dark.</span></div>
<div class="feat"><span class="em">🌐</span><b>Up to 40 players</b><span class="dim">Servers with an address, a server list, voice chat and a radio.</span></div>
<div class="feat"><span class="em">🍲</span><b>Czech cooking</b><span class="dim">Svíčková, guláš s knedlíky, palačinky… on the house's stove. Loudly.</span></div>
<div class="feat"><span class="em">♿</span><b>For everyone</b><span class="dim">Captions, big text, reduce motion, no flashing unless you allow it, rebindable keys, controllers, a panic key.</span></div>
</div>

<div class="card" id="servers" style="margin-top:18px"><h2 style="margin-top:0">Servers</h2>${list.length ? `<table><tr><th>Server</th><th>Players</th><th>Mods</th></tr>${rows}</table>`
    : `<p class="dim">No public servers right now. ${user ? '<a href="/servers/new">Make one.</a>' : '<a href="/register">Register</a> to make one.'}</p>`}
<p class="dim small">In the game: <b>Play online</b> and type a server's address (like <code>officialsmp</code>), or open your <b>Server list</b>.</p></div>

<div class="card"><h2 style="margin-top:0">Download</h2>${downloads()}<p class="dim small" style="margin-bottom:0">Free and open source. <a href="/download">All downloads and how to install</a>.</p></div>

<div class="card"><h2 style="margin-top:0">Newest mods</h2>${mods.length ? mods.map((m) => `<p><a href="/mods/${m.id}"><b>${esc(m.name)}</b></a> ${esc(m.version)} <span class="dim small">by ${esc(m.author)}</span><br><span class="small">${esc(m.description)}</span></p>`).join("") : '<p class="dim">No mods yet.</p>'}
<a href="/mods">All mods and how to make one</a></div>`);
}

function downloadPage(user) {
  return page("Download", user, `<section class="night" style="padding:40px 30px"><div class="rain"></div><h1 style="font-size:clamp(32px,6vw,56px)">DOWNLOAD</h1>
<p class="lead" style="margin-bottom:0">Free, open source, every platform. The newest version, always.</p></section>
<div class="card">${downloads()}</div>
<div class="grid">
<div class="feat"><b>🍎 macOS</b><span class="dim small">Unzip, drag <b>Hollow Hours</b> to Applications. The first time: right-click → Open (it isn't from the App Store).</span></div>
<div class="feat"><b>🪟 Windows</b><span class="dim small">Unzip and run <code>hollow-hours.exe</code>. If SmartScreen asks: More info → Run anyway.</span></div>
<div class="feat"><b>🐧 Linux / SteamOS</b><span class="dim small">Unpack and run <code>./hollow-hours</code>, or install the Flatpak. On a Steam Deck: add it to Steam as a non-Steam game.</span></div>
<div class="feat"><b>🤖 Android</b><span class="dim small">Open the .apk and allow installing from your browser. Touch controls are built in.</span></div>
<div class="feat"><b>📱 iPhone / iPad</b><span class="dim small">The .ipa is unsigned: install it with Sideloadly or AltStore (they sign it with your Apple ID).</span></div>
<div class="feat"><b>🧠 Real Claude</b><span class="dim small">With the Claude app (Claude Code) installed, the kind family talks through real Claude. Without it they still talk (local models or scripted lines).</span></div>
</div>
<p class="dim small">Every version and its changes: <a href="${REPO}/releases">GitHub releases</a>.</p>`);
}

function authForm(kind, user, error = "", email = "", name = "", search = "") {
  const reg = kind === "register";
  return page(reg ? "Register" : "Log in", user, `<div class="card" style="max-width:440px;margin:0 auto"><h2 style="margin-top:0">${reg ? "Make an account" : "Log in"}</h2>
${error ? `<p class="err">${esc(error)}</p>` : ""}
<form method="post" action="/${kind}${esc(search)}"><label>Email</label><input name="email" type="email" required autocomplete="email" value="${esc(email)}">
${reg ? `<label>Display name <span class="dim small">(shown in game)</span></label><input name="name" required minlength="3" maxlength="16" pattern="[A-Za-z0-9_-]{3,16}" value="${esc(name)}">` : ""}
<label>Password${reg ? ' <span class="dim small">(at least 8 characters)</span>' : ""}</label><input id="pw" name="password" type="password" required minlength="${reg ? 8 : 1}" autocomplete="${reg ? "new-password" : "current-password"}">
<label class="small" style="font-weight:400"><input type="checkbox" style="width:auto" onchange="document.getElementById('pw').type=this.checked?'text':'password'"> Show password</label>
<p><button class="btn" type="submit">${reg ? "Register" : "Log in"}</button></p></form>
${reg ? '<p class="dim small">Your password is stored only as a salted hash. We don\'t send emails yet, so there\'s no confirmation step and no password reset by email.</p>' : '<p class="dim small">No account? <a href="/register">Register</a>.</p>'}</div>`);
}

async function newServerPage(env, user, error = "") {
  if (!user) return redirect("/login");
  const { results: mods } = await env.DB.prepare("SELECT id, name, version FROM mods ORDER BY created DESC LIMIT 50").all();
  return page("Make a server", user, `<div class="card" style="max-width:520px;margin:0 auto"><h2 style="margin-top:0">Make a server</h2>${error ? `<p class="err">${esc(error)}</p>` : ""}
<form method="post" action="/servers/new"><label>Name</label><input name="name" required minlength="3" maxlength="32">
<label>Address <span class="dim small">(what players type to join, like <code>officialsmp</code>; leave empty to make one from the name)</span></label><input name="slug" maxlength="24" pattern="[a-z0-9][a-z0-9-]{2,23}" autocomplete="off" placeholder="my-server">
<label>Max players</label><select name="max">${[2, 4, 6, 8, 10, 12, 16, 20, 24, 30, 40].map((n) => `<option ${n === 20 ? "selected" : ""}>${n}</option>`).join("")}</select>
<label><input type="checkbox" name="public" checked style="width:auto"> Show it in the server list</label>
<label>Hosted somewhere else? <span class="dim small">(optional: your own server's address, e.g. wss://hh.example.com)</span></label><input name="address" placeholder="leave empty to host it here">
<label>Mods</label>${mods.length ? mods.map((m) => `<label class="small" style="font-weight:400"><input type="checkbox" name="mods" value="${m.id}" style="width:auto"> ${esc(m.name)} ${esc(m.version)}</label>`).join("") : '<p class="dim small">No mods uploaded yet.</p>'}
<p><button class="btn" type="submit">Make it</button></p></form></div>`);
}

async function modsPage(env, user, msg = "") {
  const { results } = await env.DB.prepare("SELECT m.id, m.name, m.version, m.description, CASE WHEN u.hide_author = 1 THEN 'anonymous' ELSE u.name END AS author FROM mods m JOIN users u ON u.id = m.author ORDER BY m.created DESC LIMIT 200").all();
  return page("Mods", user, `<div class="card"><h2 style="margin-top:0">Mods (.hh)</h2>${msg}
${results.map((m) => `<p><a href="/mods/${m.id}"><b>${esc(m.name)}</b></a> ${esc(m.version)} <span class="dim small">by ${esc(m.author)} · id <code>${m.id}</code></span><br><span class="small">${esc(m.description)}</span></p>`).join("") || '<p class="dim">No mods yet.</p>'}</div>
<div class="card"><h2 style="margin-top:0">Make a mod</h2><p>A <code>.hh</code> file is a zip with two files. It's <b>data only</b>: it can't run code on anyone's computer.</p>
<p><code>hh.json</code>: <code>{"name": "Fast Infected", "version": "1.0", "description": "..."}</code></p>
<p><code>content.json</code>: every part is optional. A mod can change the whole game:</p>
<pre style="white-space:pre-wrap" class="small"><code>{ "intro": "One line when the game starts",
  "text":  { "title": "NIGHT MALL", "over_infected": "...", "over_claudes": "...",
             "warnings": ["{} is in aisle 3!", "Hide, {} is doing rounds."] },
  "theme": { "infected_eye": "#ff3b3b", "kind_halo": "#7fd6ff", "hud": "#14121a" },
  "rules": { "player_speed": 3.4, "sneak_speed": 1.6, "infected_speed": 2.8,
             "sight_tiles": 10, "hearing_tiles": 8, "infect_touch": 0.3, "infect_near": 0.02,
             "claude_infect": 0.18, "food_minutes": 6, "water_minutes": 4,
             "supplies_per_map": 7, "supply_amount": 0.35,
             "tornado_min": 90, "tornado_max": 160, "tornado_speed": 3.5, "tornado_radius": 2.2 },
  "replace_maps": true,
  "maps": [ { "name": "The night mall", "rows": [ 18 rows of 30 tiles ],
              "dark": 0.55, "liminal": 0.85, "floor": "#c9c2b8", "wall": "#3a3440",
              "doors": { "G": { "to": 1, "x": 2, "y": 9 } } } ],
  "start": { "map": 0, "x": 14, "y": 9 },
  "remove": ["Spark", "Twin"],
  "characters": [ { "name": "Kiosk", "color": "#f2c14e", "personality": "wants to help you find the exit",
                    "infected": false, "kind": true, "map": 0, "x": 13, "y": 8 } ] }</code></pre>
<p class="small">Tiles: <code>#</code> wall · <code>.</code> floor · <code>,</code> sand · <code>~</code> water · <code>b</code> furniture (hide next to it) · <code>L</code> lamp · <code>d</code> doll · <code>A</code>-<code>Z</code> doors (say where each leads in <code>doors</code>).
Characters can be <b>infected</b>, <b>kind</b> (they follow and protect the player) or neither.</p>
<p class="small dim">Zip it (Terminal: <code>zip MyMod.hh hh.json content.json</code>) and upload it here.</p>
${user ? `<form method="post" action="/mods/upload" enctype="multipart/form-data"><input type="file" name="file" accept=".hh,.zip" required><p><button class="btn" type="submit">Upload and check</button></p></form>` : '<p><a href="/login">Log in</a> to upload a mod.</p>'}</div>`);
}

async function modPage(env, user, id) {
  const m = await env.DB.prepare("SELECT m.*, CASE WHEN u.hide_author = 1 THEN 'anonymous' ELSE u.name END AS author FROM mods m JOIN users u ON u.id = m.author WHERE m.id = ?").bind(id).first();
  if (!m) return new Response("not found", { status: 404 });
  const c = JSON.parse(m.content);
  const rules = Object.entries(c.rules || {}).map(([k, v]) => `<span class="pill">${esc(k)}: ${esc(v)}</span>`).join("");
  const chars = (c.characters || []).map((x) => `<p><span style="display:inline-block;width:12px;height:12px;border-radius:50%;background:${esc(x.color)}"></span> <b>${esc(x.name)}</b> ${x.infected ? "(infected)" : "(friendly)"} <span class="dim small">${esc(x.personality)}</span></p>`).join("");
  return page(m.name, user, `<div class="card"><h2 style="margin-top:0">${esc(m.name)} <span class="dim">${esc(m.version)}</span></h2><p class="dim small">by ${esc(m.author)} · id <code>${m.id}</code></p><p>${esc(m.description)}</p>
${c.intro ? `<p><i>${esc(c.intro)}</i></p>` : ""}${rules ? `<h3>Rules</h3><p>${rules}</p>` : ""}${chars ? `<h3>Characters</h3>${chars}` : ""}
<p><a class="btn ghost" href="/mods/${m.id}/download">Download content</a> <span class="dim small">Servers with this mod load it in game by themselves.</span></p></div>`);
}

// ---------------- Account ----------------

async function accountPage(env, user, msg = "") {
  const u = await env.DB.prepare("SELECT name, email, created, hide_host, hide_author, gamertag FROM users WHERE id = ?").bind(user.id).first();
  const { results: mods } = await env.DB.prepare("SELECT id, name, version FROM mods WHERE author = ? ORDER BY created DESC").bind(user.id).all();
  const { results: srv } = await env.DB.prepare("SELECT code, slug, name, players, max, address FROM servers WHERE owner = ? ORDER BY created DESC").bind(user.id).all();
  const sessions = (await env.DB.prepare("SELECT count(*) AS n FROM sessions WHERE user_id = ? AND expires > ?").bind(user.id, now()).first()).n;
  const form = (action, inner, confirm = "") => `<form method="post" action="/account" style="display:inline"${confirm ? ` onsubmit="return confirm('${confirm}')"` : ""}><input type="hidden" name="action" value="${action}">${inner}</form>`;
  const masked = u.email.replace(/^(.{2}).*(@.*)$/, "$1***$2");
  return page("Account", user, `${msg}
<div class="card"><h2 style="margin-top:0">${esc(u.name)}</h2>
<p>Email: <span id="em" data-full="${esc(u.email)}">${esc(masked)}</span> <a href="#" class="small" onclick="const e=document.getElementById('em');e.textContent=e.dataset.full;this.remove();return false">show</a></p>
<p class="dim small">Joined ${new Date(u.created * 1000).toISOString().slice(0, 10)}</p>
${form("open_game", `<button class="btn" type="submit">🎮 Open Hollow Hours and log in</button>`)}
<p class="dim small">Opens the Hollow Hours app on this computer and logs it in as you (it needs Hollow Hours.app installed).</p></div>

<div class="card"><h3 style="margin-top:0">🎮 Xbox gamertag</h3>
${u.gamertag ? `<p>Showing <b>${esc(u.gamertag)}</b> <span class="pill">unverified</span> next to your name.</p>` : '<p class="dim">Not set.</p>'}
${form("gamertag", `<label>Gamertag <span class="dim small">(letters, numbers and spaces, up to 15, optional #1234)</span></label><input name="gamertag" maxlength="20" value="${esc(u.gamertag || "")}" autocomplete="off"><p><button class="btn ghost" type="submit">Save gamertag</button></p>`)}
${u.gamertag ? form("gamertag", `<input type="hidden" name="gamertag" value=""><button class="btn ghost" type="submit">Remove it</button>`) : ""}
<p class="dim small">Typed by you, not checked with Xbox (that's why it says unverified). It shows on your servers in the server list, unless you hide your name there below.</p></div>

<div class="card"><h3 style="margin-top:0">Privacy</h3>
${form("privacy", `<label class="small" style="font-weight:400"><input type="checkbox" name="hide_host" style="width:auto" ${u.hide_host ? "checked" : ""}> Hide my name as the host in the server list</label>
<label class="small" style="font-weight:400"><input type="checkbox" name="hide_author" style="width:auto" ${u.hide_author ? "checked" : ""}> Hide my name on mods I upload</label>
<p><button class="btn ghost" type="submit">Save</button></p>`)}
<p class="dim small">Your email is never shown to anyone. In game, other players always see your display name (that's how they know who's who).</p></div>

<div class="card"><h3 style="margin-top:0">Your mods</h3>${mods.length ? `<table>${mods.map((m) => `<tr><td><a href="/mods/${m.id}">${esc(m.name)}</a> ${esc(m.version)}</td><td style="text-align:right">${form("delete_mod", `<input type="hidden" name="id" value="${m.id}"><button class="btn ghost" type="submit">Delete</button>`, "Delete this mod? Servers using it will run without it.")}</td></tr>`).join("")}</table>` : '<p class="dim">None yet. <a href="/mods">Upload one.</a></p>'}</div>

<div class="card"><h3 style="margin-top:0">Your servers</h3>${srv.length ? `<table>${srv.map((x) => `<tr><td><b>${esc(x.name)}</b> <code>${esc(x.slug || x.code)}</code>${x.address ? ` <span class="pill">own server: ${esc(x.address)}</span>` : ""}<div class="dim small">${x.players}/${x.max} playing</div></td><td style="text-align:right">${x.address ? "" : `<a class="btn ghost" href="/servers/${x.code}/console">Console</a> `}${form("delete_server", `<input type="hidden" name="code" value="${x.code}"><button class="btn ghost" type="submit">Delete</button>`, "Delete this server? Its code stops working.")}</td></tr>`).join("")}</table>` : '<p class="dim">None yet. <a href="/servers/new">Make one.</a></p>'}</div>

<div class="card"><h3 style="margin-top:0">Password and sessions</h3>
${form("password", `<label>Current password</label><input type="password" name="current" required autocomplete="current-password"><label>New password <span class="dim small">(at least 8 characters)</span></label><input type="password" name="new" required minlength="8" autocomplete="new-password"><p><button class="btn ghost" type="submit">Change password</button></p>`)}
<p>You're logged in on <b>${sessions}</b> ${sessions === 1 ? "device" : "devices"} (browsers and games).</p>
${form("logout_all", `<button class="btn ghost" type="submit">Log out everywhere</button>`, "Log out of every browser and game?")}</div>

<div class="card" style="border-color:var(--bad)"><h3 style="margin-top:0" class="err">Delete my account</h3>
<p class="small">Deletes your account, your mods, your servers and every login. This can't be undone.</p>
${form("delete_account", `<label>Type your display name</label><input name="confirm_name" required autocomplete="off"><label>Password</label><input type="password" name="password" required autocomplete="current-password"><p><button class="btn" style="background:var(--bad)" type="submit">Delete my account forever</button></p>`, "Really delete your account and everything in it? This can't be undone.")}</div>`);
}

async function accountAction(env, user, f, req) {
  const ok = (t) => `<div class="card"><p class="ok">${esc(t)}</p></div>`;
  const err = (t) => `<div class="card"><p class="err">${esc(t)}</p></div>`;
  switch (f.get("action")) {
    case "privacy":
      await env.DB.prepare("UPDATE users SET hide_host = ?, hide_author = ? WHERE id = ?").bind(f.get("hide_host") ? 1 : 0, f.get("hide_author") ? 1 : 0, user.id).run();
      return accountPage(env, user, ok("Privacy saved."));
    case "gamertag": {
      const t = String(f.get("gamertag") || "").trim().replace(/\s+/g, " ");
      if (!t) {
        await env.DB.prepare("UPDATE users SET gamertag = NULL WHERE id = ?").bind(user.id).run();
        return accountPage(env, user, ok("Gamertag removed."));
      }
      if (!/^[A-Za-z][A-Za-z0-9 ]{0,14}(#\d{1,4})?$/.test(t)) return accountPage(env, user, err("A gamertag starts with a letter: letters, numbers and spaces (up to 15), and an optional #1234."));
      await env.DB.prepare("UPDATE users SET gamertag = ? WHERE id = ?").bind(t, user.id).run();
      return accountPage(env, user, ok(`Gamertag saved: ${t}`));
    }
    case "delete_mod": {
      const r = await env.DB.prepare("DELETE FROM mods WHERE id = ? AND author = ?").bind(String(f.get("id")), user.id).run();
      return accountPage(env, user, r.meta.changes ? ok("Mod deleted.") : err("That mod isn't yours."));
    }
    case "delete_server": {
      const r = await env.DB.prepare("DELETE FROM servers WHERE code = ? AND owner = ?").bind(String(f.get("code")), user.id).run();
      return accountPage(env, user, r.meta.changes ? ok("Server deleted.") : err("That server isn't yours."));
    }
    case "password": {
      if (!(await checkPassword(env, user.id, String(f.get("current") || "")))) return accountPage(env, user, err("Your current password is wrong."));
      const r = await setPassword(env, user.id, String(f.get("new") || ""));
      if (r.error) return accountPage(env, user, err(r.error));
      // Every other login is signed out; this browser gets a fresh one.
      await env.DB.prepare("DELETE FROM sessions WHERE user_id = ?").bind(user.id).run();
      const token = await newSession(env, user.id);
      const resp = await accountPage(env, user, ok("Password changed. You were logged out everywhere else."));
      resp.headers.append("Set-Cookie", sessionCookie(token));
      return resp;
    }
    case "logout_all":
      await env.DB.prepare("DELETE FROM sessions WHERE user_id = ?").bind(user.id).run();
      return redirect("/", { "Set-Cookie": clearCookie });
    case "delete_account": {
      if (String(f.get("confirm_name") || "") !== user.name) return accountPage(env, user, err("Type your display name exactly to delete your account."));
      if (!(await checkPassword(env, user.id, String(f.get("password") || "")))) return accountPage(env, user, err("Wrong password: your account was NOT deleted."));
      await env.DB.batch([
        env.DB.prepare("DELETE FROM sessions WHERE user_id = ?").bind(user.id),
        env.DB.prepare("DELETE FROM link_codes WHERE user_id = ?").bind(user.id),
        env.DB.prepare("DELETE FROM mods WHERE author = ?").bind(user.id),
        env.DB.prepare("DELETE FROM servers WHERE owner = ?").bind(user.id),
        env.DB.prepare("DELETE FROM users WHERE id = ?").bind(user.id),
      ]);
      return page("Account deleted", null, `<div class="card"><h2 style="margin-top:0">Your account is gone</h2><p>Everything was deleted: your account, mods, servers and logins. Thanks for playing 👻</p></div>`, { "Set-Cookie": clearCookie });
    }
    case "open_game": {
      // A one-time code for the app, handed over through the hollowhours:// link (2 minutes, once).
      const code = hex(crypto.getRandomValues(new Uint8Array(24)));
      await env.DB.prepare("INSERT INTO link_codes (code_hash, user_id, challenge, expires) VALUES (?, ?, 'app', ?)").bind(await sha256(code), user.id, now() + 300).run();
      return page("Opening Hollow Hours", user, `<div class="card" style="text-align:center"><h2 style="margin-top:0">Opening Hollow Hours…</h2>
<p>If your browser asks, allow it to open <b>Hollow Hours</b>.</p><p><a class="btn" id="go" href="hollowhours://login?code=${code}">Open Hollow Hours</a></p>
<p class="dim small">Nothing happened? Copy this code, then in the game press <b>L</b> and paste it (Cmd+V):</p>
<p><code id="code" style="font-size:12px;word-break:break-all">${code}</code></p>
<p><button class="btn ghost" type="button" onclick="navigator.clipboard.writeText(document.getElementById('code').textContent).then(()=>this.textContent='Copied ✓')">Copy code</button></p>
<p class="dim small">The code works once, for 5 minutes.</p></div>
<script>location.href = document.getElementById("go").href;</script>`);
    }
  }
  return accountPage(env, user);
}

// ---------------- Router ----------------

// ---------------- Lending Claude to your phone ----------------
// Your computer (logged in, with "Lend Claude to my phone" on) keeps a WebSocket open here. Your phone (logged in to the
// SAME account) asks a question; it goes to your computer, which runs `claude` and answers. One of these per account,
// so nobody else can ever reach your computer's Claude. Nothing is stored: questions only pass through.
export class PcLink {
  constructor(ctx, env) { this.ctx = ctx; this.env = env; this.waiting = new Map(); this.recent = []; }
  pc() { return this.ctx.getWebSockets("pc")[0]; }
  async fetch(req) {
    const path = new URL(req.url).pathname;
    if (path.endsWith("/connect")) {
      if (req.headers.get("Upgrade") !== "websocket") return new Response("websocket only", { status: 426 });
      for (const old of this.ctx.getWebSockets("pc")) { try { old.close(4000, "replaced by another computer"); } catch {} }
      const [client, ws] = Object.values(new WebSocketPair());
      this.ctx.acceptWebSocket(ws, ["pc"]);
      return new Response(null, { status: 101, webSocket: client });
    }
    if (path.endsWith("/status")) return Response.json({ online: !!this.pc() });
    const pc = this.pc();
    if (!pc) return Response.json({ error: "your computer isn't lending Claude right now (turn it on with K on its title screen)" }, { status: 503 });
    // At most 20 questions a minute.
    const now = Date.now();
    this.recent = this.recent.filter((t) => now - t < 60000);
    if (this.recent.length >= 20) return Response.json({ error: "too many questions, wait a minute" }, { status: 429 });
    this.recent.push(now);
    const b = await req.json().catch(() => ({}));
    const id = crypto.randomUUID();
    const ask = {
      t: "ask", id,
      name: String(b.name || "").slice(0, 32),
      cloud: String(b.cloud || "").slice(0, 40),
      system: String(b.system || "").slice(0, 8000),
      history: (Array.isArray(b.history) ? b.history : []).slice(-12).map((m) => ({ user: !!m.user, text: String(m.text || "").slice(0, 1200) })),
    };
    const answer = await new Promise((resolve) => {
      const timer = setTimeout(() => { this.waiting.delete(id); resolve({ error: "your computer didn't answer in time" }); }, 90000);
      this.waiting.set(id, (a) => { clearTimeout(timer); resolve(a); });
      try { pc.send(JSON.stringify(ask)); } catch { clearTimeout(timer); this.waiting.delete(id); resolve({ error: "couldn't reach your computer" }); }
    });
    return Response.json(answer, { status: answer.error ? 502 : 200 });
  }
  async webSocketMessage(ws, data) {
    if (typeof data !== "string" || data.length > 20000) return;
    let m;
    try { m = JSON.parse(data); } catch { return; }
    if (m.t === "ping") { ws.send('{"t":"pong"}'); return; }
    const done = m.t === "answer" && this.waiting.get(m.id);
    if (done) {
      this.waiting.delete(m.id);
      done(m.error ? { error: String(m.error).slice(0, 200) } : { text: String(m.text || "").slice(0, 2000), via: String(m.via || "").slice(0, 80) });
    }
  }
  async webSocketClose(ws) { try { ws.close(1000, "bye"); } catch {} }
  async webSocketError(ws) { await this.webSocketClose(ws); }
}

export default {
  async fetch(req, env) {
    const url = new URL(req.url);
    const p = url.pathname;
    // Forms and API writes only from this site (or the game, which sends no Origin).
    const origin = req.headers.get("Origin");
    if (req.method === "POST" && origin && origin !== url.origin) return new Response("forbidden", { status: 403 });
    try {
      const user = await currentUser(env, req);
      const room = p.match(/^\/room\/([A-Za-z0-9]{4,8})$/);
      if (room) {
        // Online play needs an account: the game sends its login token (Authorization: Bearer), checked here.
        if (!user) return new Response("log in first", { status: 401 });
        const headers = new Headers(req.headers);
        headers.delete("X-HH-User-Id");
        headers.delete("X-HH-User-Name");
        headers.delete("X-HH-Owner");
        headers.set("X-HH-User-Id", String(user.id));
        headers.set("X-HH-User-Name", user.name);
        return env.ROOMS.get(env.ROOMS.idFromName(room[1].toUpperCase())).fetch(new Request(req, { headers }));
      }

      // --- Helix Dynamics (the made-up company from the story; the game opens these pages) ---
      if (p.startsWith("/helix")) { const h = helixPage(p); if (h) return h; }

      // --- server console (the server's owner only) ---
      const con = p.match(/^\/(api\/)?servers\/([A-Za-z0-9]{4,8})\/console$/);
      if (con) {
        const code = con[2].toUpperCase();
        if (!user) return con[1] ? json({ error: "log in first" }, 401) : redirect("/login");
        const srv = await env.DB.prepare("SELECT name, owner, address FROM servers WHERE code = ?").bind(code).first();
        if (!srv || srv.owner !== user.id) return con[1] ? json({ error: "not your server" }, 403) : page("Console", user, '<div class="card"><p class="err">That isn\'t your server.</p></div>');
        if (!con[1]) return consolePage(user, code, srv);
        const headers = new Headers({ "X-HH-Owner": "1", "content-type": "application/json" });
        return env.ROOMS.get(env.ROOMS.idFromName(code)).fetch(new Request(`https://room/${code}/console`, { method: req.method, headers, body: req.method === "POST" ? await req.text() : undefined }));
      }

      // --- lending Claude to your phone (your own account only) ---
      const pcPath = p.match(/^\/api\/pc\/(connect|ask|status)$/);
      if (pcPath) {
        if (!user) return json({ error: "log in first" }, 401);
        if (pcPath[1] === "ask" && req.method !== "POST") return json({ error: "POST" }, 405);
        return env.PCLINK.get(env.PCLINK.idFromName("user-" + user.id)).fetch(req);
      }

      // --- game API ---
      if (p === "/api/register" && req.method === "POST") { const b = await req.json().catch(() => ({})); const r = await register(env, b.email, b.password, b.name); return json(r, r.error ? 400 : 200); }
      if (p === "/api/login" && req.method === "POST") { const b = await req.json().catch(() => ({})); const r = await login(env, b.email, b.password); return json(r, r.error ? 401 : 200); }
      if (p === "/api/me") {
        if (!user) return json({ error: "not logged in" }, 401);
        const gt = await env.DB.prepare("SELECT gamertag FROM users WHERE id = ?").bind(user.id).first();
        return json({ name: user.name, gamertag: gt?.gamertag ?? null, gamertag_verified: false });
      }
      if (p === "/api/servers" && req.method === "GET") return json(await servers(env));
      if (p === "/api/resolve") { const r = await resolveServer(env, url.searchParams.get("a")); return r ? json(r) : json({ error: "no server with that address" }, 404); }
      if (p === "/api/servers" && req.method === "POST") { if (!user) return json({ error: "log in first" }, 401); const r = await makeServer(env, user, await req.json().catch(() => ({}))); return json(r, r.error ? 400 : 200); }
      if (p === "/api/link/redeem" && req.method === "POST") { const r = await linkRedeem(env, await req.json().catch(() => ({}))); return json(r, r.error ? 400 : 200); }
      if (p === "/account") {
        if (!user) return redirect("/login");
        return req.method === "POST" ? await accountAction(env, user, await req.formData(), req) : await accountPage(env, user);
      }
      if (p === "/api/link/exchange" && req.method === "POST") { const r = await linkExchange(env, await req.json().catch(() => ({}))); return json(r, r.error ? 400 : 200); }
      if (p === "/link") {
        const q = linkParams(url);
        if (req.method === "POST" && user && q) return await linkAllow(env, user, q);
        return linkPage(user, q, url);
      }
      if (p === "/api/ticket-key") return json({ alg: "Ed25519", public_key: (await ticketKeys(env)).pubHex });
      if (p === "/api/ticket" && req.method === "POST") {
        if (!user) return json({ error: "log in first" }, 401);
        const b = await req.json().catch(() => ({}));
        const address = cleanAddress(b.address);
        const known = address && (await env.DB.prepare("SELECT 1 FROM servers WHERE address = ?").bind(address).first());
        if (!known) return json({ error: "that server isn't registered here" }, 404);
        return json({ ticket: await makeTicket(env, user, address), name: user.name });
      }
      if (p === "/api/servers/heartbeat" && req.method === "POST") {
        // An outside server says it's alive and how many are playing (with the secret it got when it was made).
        const b = await req.json().catch(() => ({}));
        const row = await env.DB.prepare("SELECT max, secret_hash FROM servers WHERE code = ?").bind(String(b.code || "").toUpperCase()).first();
        if (!row?.secret_hash || row.secret_hash !== (await sha256(String(b.secret || "")))) return json({ error: "wrong code or secret" }, 403);
        const players = Math.max(0, Math.min(row.max, parseInt(b.players) || 0));
        await env.DB.prepare("UPDATE servers SET players = ?, seen = ? WHERE code = ?").bind(players, now(), String(b.code).toUpperCase()).run();
        return json({ ok: true });
      }
      if (p === "/api/mods") { const { results } = await env.DB.prepare("SELECT id, name, version, description FROM mods ORDER BY created DESC LIMIT 200").all(); return json(results); }
      const apiMod = p.match(/^\/api\/mods\/([0-9a-f]{12})$/);
      if (apiMod) { const m = await env.DB.prepare("SELECT id, name, version, content FROM mods WHERE id = ?").bind(apiMod[1]).first(); return m ? json({ id: m.id, name: m.name, version: m.version, content: JSON.parse(m.content) }) : json({ error: "not found" }, 404); }

      // --- website ---
      if (p === "/") return await home(env, user);
      if (p === "/download") return downloadPage(user);
      if (p === "/story") return storyPage(page, user);
      if (p === "/characters") return charactersPage(page, user);
      if (p === "/places") return placesPage(page, user);
      if (p === "/register" || p === "/login") {
        if (req.method === "GET") return authForm(p.slice(1), user, "", "", "", url.search);
        const f = await req.formData();
        const r = p === "/register" ? await register(env, f.get("email"), f.get("password"), f.get("name")) : await login(env, f.get("email"), f.get("password"));
        if (r.error) return authForm(p.slice(1), user, r.error, String(f.get("email") || ""), String(f.get("name") || ""), url.search);
        // Back to where you were going, but only to the game-login page on this site.
        const next = url.searchParams.get("next") || "";
        return redirect(next.startsWith("/link?") ? next : "/", { "Set-Cookie": sessionCookie(r.token) });
      }
      if (p === "/logout") { await logout(env, req); return redirect("/", { "Set-Cookie": clearCookie }); }
      if (p === "/servers/new") {
        if (req.method === "GET") return await newServerPage(env, user);
        if (!user) return redirect("/login");
        const f = await req.formData();
        const r = await makeServer(env, user, { name: f.get("name"), slug: f.get("slug") || "", max: f.get("max"), public: f.get("public") === "on", mods: f.getAll("mods"), address: f.get("address") });
        if (r.error) return await newServerPage(env, user, r.error);
        return page("Server made", user, `<div class="card"><h2 style="margin-top:0">${esc(r.name)} is ready</h2><p>Address: <code style="font-size:22px">${esc(r.slug)}</code></p>
<p>In the game: <b>S</b> on the title screen, then <b>A</b> (add server) and type <code>${esc(r.slug)}</code>, or <b>J</b> and type it. Share the address with friends.${r.public ? "" : " It's private: only people with the address can join."}</p>
${r.address ? `<h3>Your own server</h3><p>Hosted at <code>${esc(r.address)}</code>. Start the Hollow Hours server program there with these (the secret is shown <b>only now</b>, keep it private):</p>
<pre style="white-space:pre-wrap" class="small"><code>HH_CODE=${r.code}
HH_SECRET=${r.secret}
HH_PUBLIC_URL=${esc(r.address)}
node server.mjs</code></pre><p class="dim small">Players' logins never go to your server: the game gets a 60-second ticket from this website instead, and your server checks it.</p>` : ""}</div>`);
      }
      if (p === "/mods") return await modsPage(env, user);
      if (p === "/mods/upload" && req.method === "POST") {
        if (!user) return redirect("/login");
        const r = await uploadMod(env, user, (await req.formData()).get("file"));
        const msg = r.error ? `<p class="err">${esc(r.error)}</p>${(r.problems || []).map((x) => `<p class="err small">· ${esc(x)}</p>`).join("")}` : `<p class="ok">Uploaded <a href="/mods/${r.id}">${esc(r.name)}</a> ✓</p>`;
        return await modsPage(env, user, msg);
      }
      const modDl = p.match(/^\/mods\/([0-9a-f]{12})\/download$/);
      if (modDl) { const m = await env.DB.prepare("SELECT name, version, content FROM mods WHERE id = ?").bind(modDl[1]).first(); if (!m) return new Response("not found", { status: 404 });
        return new Response(m.content, { headers: { "content-type": "application/json", "content-disposition": `attachment; filename="${m.name.replace(/[^A-Za-z0-9_-]/g, "_")}-content.json"` } }); }
      const mod = p.match(/^\/mods\/([0-9a-f]{12})$/);
      if (mod) return await modPage(env, user, mod[1]);
      return new Response("not found", { status: 404 });
    } catch (e) {
      return new Response("server error", { status: 500 });
    }
  },
};
