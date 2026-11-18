// Hollow Hours online: the website (accounts, server list, .hh mods) and the multiplayer rooms.
//
// Website:  /  /register  /login  /logout  /servers/new  /mods  /mods/upload  /mods/:id  /mods/:id/download
// Game API: POST /api/register  POST /api/login  GET /api/me  GET /api/servers  POST /api/servers
//           POST /api/ticket  GET /api/ticket-key  POST /api/servers/heartbeat   (servers hosted somewhere else)
//           GET /api/mods  GET /api/mods/:id
// Rooms:    GET /room/<CODE> (WebSocket, needs a login: Authorization: Bearer <token>). A room keeps no game data: it hands out ids, says who the host is (the
//           player there longest; the host's game runs the AIs), passes messages on, and only lets the host send
//           the world. Registered servers also give their name, mods and player limit.

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
  async fetch(req) {
    if (req.headers.get("Upgrade") !== "websocket") return new Response("expected a WebSocket", { status: 426 });
    const code = new URL(req.url).pathname.split("/").pop().toUpperCase();
    const server = await this.env.DB.prepare("SELECT name, max, mods FROM servers WHERE code = ?").bind(code).first().catch(() => null);
    const max = Math.min(server?.max ?? 20, MAX_PLAYERS);
    if (this.sockets().length >= max) return new Response("room is full", { status: 429 });
    // The Worker checked the login before sending the player here (see the router); these headers come only from it.
    const uid = parseInt(req.headers.get("X-HH-User-Id") || "0");
    const uname = req.headers.get("X-HH-User-Name") || "";
    if (!uid || !uname) return new Response("log in first", { status: 401 });
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
    this.broadcast(msg, ws);
    if (msg.t === "world" && Math.random() < 0.05) await this.seen(me.code); // keep the server "alive" in the list
  }
  async webSocketClose(ws) {
    const me = ws.deserializeAttachment();
    ws.serializeAttachment(null);
    try { ws.close(1000, "bye"); } catch {}
    if (me) {
      this.broadcast({ t: "left", from: me.id, host: this.host(), players: this.sockets().length });
      await this.seen(me.code);
    }
  }
  async webSocketError(ws) {
    await this.webSocketClose(ws);
  }
}

// ---------------- Data ----------------

async function servers(env) {
  const { results } = await env.DB.prepare(
    "SELECT s.code, s.name, s.players, s.max, s.mods, s.address, CASE WHEN u.hide_host = 1 THEN 'a player' ELSE u.name END AS host, CASE WHEN u.hide_host = 1 THEN NULL ELSE u.gamertag END AS gamertag FROM servers s JOIN users u ON u.id = s.owner " +
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
    return { code: r.code, name: r.name, host: r.host, players: r.players, max: r.max, mods: mods.map((id) => ({ id, name: names[id] ?? id })),
      address: r.address || null, external: !!r.address };
  });
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
  // Hosted somewhere else? Then it gets a secret for its heartbeat (shown once, only its hash is kept).
  let address = null, secret = null;
  if (body.address) {
    address = cleanAddress(body.address);
    if (!address) return { error: "Server address: wss://your-host (ws:// only for localhost while testing)." };
    secret = hex(crypto.getRandomValues(new Uint8Array(24)));
  }
  await env.DB.prepare("INSERT INTO servers (code, name, owner, public, max, mods, created, seen, address, secret_hash) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
    .bind(code, name, user.id, pub, max, JSON.stringify(mods), now(), now(), address, secret ? await sha256(secret) : null).run();
  return { code, name, public: !!pub, max, mods, address, secret };
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

const STYLE = `<style>
:root{--bg:#0d0b10;--card:#17141c;--line:#2a2531;--fg:#ece8f0;--dim:#a39cad;--acc:#d9475a;--ok:#5fd17a;--bad:#ff6b6b}
@media(prefers-color-scheme:light){:root{--bg:#f6f4f8;--card:#fff;--line:#e3dfe8;--fg:#17141c;--dim:#635c6d}}
*{box-sizing:border-box}body{margin:0;background:var(--bg);color:var(--fg);font:15px/1.5 -apple-system,"Segoe UI",system-ui,sans-serif}
header{display:flex;align-items:center;gap:14px;padding:14px 24px;border-bottom:1px solid var(--line);flex-wrap:wrap}
header a.logo{font-weight:800;font-size:19px;color:var(--acc);text-decoration:none}nav{margin-left:auto;display:flex;gap:10px;align-items:center}
a{color:var(--acc)}.btn{background:var(--acc);color:#fff;padding:8px 14px;border-radius:9px;font-weight:600;border:0;cursor:pointer;font:inherit;text-decoration:none;display:inline-block}
.btn.ghost{background:var(--card);color:var(--fg);border:1px solid var(--line)}
main{max-width:960px;margin:0 auto;padding:26px 20px}.card{background:var(--card);border:1px solid var(--line);border-radius:14px;padding:18px;margin-bottom:16px}
.hero{background:linear-gradient(135deg,#3a1220,#120b16);color:#fff;border-radius:18px;padding:26px;margin-bottom:20px}.hero p{color:#e3c9d0;margin:6px 0 0}
table{width:100%;border-collapse:collapse}td,th{text-align:left;padding:8px;border-top:1px solid var(--line)}th{color:var(--dim);font-weight:600;border-top:0}
code{background:var(--bg);border:1px solid var(--line);border-radius:6px;padding:1px 6px}
input,textarea,select{width:100%;background:var(--bg);color:var(--fg);border:1px solid var(--line);border-radius:9px;padding:10px;font:inherit}
label{display:block;margin:12px 0 5px;font-weight:600}.dim{color:var(--dim)}.small{font-size:13px}.err{color:var(--bad)}.ok{color:var(--ok)}
.pill{display:inline-block;padding:1px 8px;border-radius:999px;font-size:12px;background:var(--bg);border:1px solid var(--line);margin:1px}
@media(max-width:600px){main{padding:18px 16px}td,th{padding:6px 4px}}
</style>`;

function page(title, user, body, extraHeaders = {}) {
  const nav = user
    ? `<a class="dim small" href="/account">Hi, ${esc(user.name)}</a><a class="btn ghost" href="/account">Account</a><a class="btn ghost" href="/mods">Mods</a><a class="btn" href="/servers/new">Make a server</a><a class="btn ghost" href="/logout">Log out</a>`
    : `<a class="btn ghost" href="/mods">Mods</a><a class="btn ghost" href="/login">Log in</a><a class="btn" href="/register">Register</a>`;
  return new Response(`<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>${esc(title)} · Hollow Hours</title>${STYLE}</head><body><header><a class="logo" href="/">👻 Hollow Hours</a><nav>${nav}</nav></header><main>${body}</main></body></html>`,
    { headers: { "content-type": "text/html; charset=utf-8", ...extraHeaders } });
}

const redirect = (to, headers = {}) => new Response(null, { status: 303, headers: { Location: to, ...headers } });

async function home(env, user) {
  const list = await servers(env);
  const rows = list.map((s) => `<tr><td><b>${esc(s.name)}</b><div class="dim small">host ${esc(s.host)}${s.gamertag ? ` (🎮 ${esc(s.gamertag)})` : ""} · code <code>${esc(s.code)}</code></div></td>
<td>${s.players}/${s.max}</td><td>${s.mods.map((m) => `<a class="pill" href="/mods/${m.id}">${esc(m.name)}</a>`).join("") || '<span class="dim small">no mods</span>'}</td></tr>`).join("");
  const { results: mods } = await env.DB.prepare("SELECT m.id, m.name, m.version, m.description, CASE WHEN u.hide_author = 1 THEN 'anonymous' ELSE u.name END AS author FROM mods m JOIN users u ON u.id = m.author ORDER BY m.created DESC LIMIT 6").all();
  return page("Servers", user, `<div class="hero"><h1 style="margin:0">Hollow Hours</h1>
<p>A small horror game where the characters are real AIs. Infection mode: every AI outside the Claude family is infected, and you're the last human. Play online with friends.</p></div>
<div class="card"><h2 style="margin-top:0">Servers</h2>${list.length ? `<table><tr><th>Server</th><th>Players</th><th>Mods</th></tr>${rows}</table>`
    : `<p class="dim">No public servers right now. ${user ? '<a href="/servers/new">Make one.</a>' : '<a href="/register">Register</a> to make one.'}</p>`}
<p class="dim small">In the game: <b>S</b> on the title screen shows this list (mods download by themselves), or <b>J</b> and type a code.</p></div>
<div class="card"><h2 style="margin-top:0">Newest mods</h2>${mods.length ? mods.map((m) => `<p><a href="/mods/${m.id}"><b>${esc(m.name)}</b></a> ${esc(m.version)} <span class="dim small">by ${esc(m.author)}</span><br><span class="small">${esc(m.description)}</span></p>`).join("") : '<p class="dim">No mods yet.</p>'}
<a href="/mods">All mods and how to make one</a></div>`);
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
  const { results: srv } = await env.DB.prepare("SELECT code, name, players, max, address FROM servers WHERE owner = ? ORDER BY created DESC").bind(user.id).all();
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

<div class="card"><h3 style="margin-top:0">Your servers</h3>${srv.length ? `<table>${srv.map((x) => `<tr><td><b>${esc(x.name)}</b> <code>${x.code}</code>${x.address ? ` <span class="pill">own server: ${esc(x.address)}</span>` : ""}<div class="dim small">${x.players}/${x.max} playing</div></td><td style="text-align:right">${form("delete_server", `<input type="hidden" name="code" value="${x.code}"><button class="btn ghost" type="submit">Delete</button>`, "Delete this server? Its code stops working.")}</td></tr>`).join("")}</table>` : '<p class="dim">None yet. <a href="/servers/new">Make one.</a></p>'}</div>

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
        headers.set("X-HH-User-Id", String(user.id));
        headers.set("X-HH-User-Name", user.name);
        return env.ROOMS.get(env.ROOMS.idFromName(room[1].toUpperCase())).fetch(new Request(req, { headers }));
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
        const r = await makeServer(env, user, { name: f.get("name"), max: f.get("max"), public: f.get("public") === "on", mods: f.getAll("mods"), address: f.get("address") });
        if (r.error) return await newServerPage(env, user, r.error);
        return page("Server made", user, `<div class="card"><h2 style="margin-top:0">${esc(r.name)} is ready</h2><p>Room code: <code style="font-size:22px">${r.code}</code></p>
<p>In the game: <b>S</b> on the title screen and pick it, or <b>J</b> and type <code>${r.code}</code>. Share the code with friends.${r.public ? "" : " It's private: only people with the code can join."}</p>
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
