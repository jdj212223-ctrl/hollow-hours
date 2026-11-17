// Hollow Hours online: the website (accounts, server list, .hh mods) and the multiplayer rooms.
//
// Website:  /  /register  /login  /logout  /servers/new  /mods  /mods/upload  /mods/:id  /mods/:id/download
// Game API: POST /api/register  POST /api/login  GET /api/me  GET /api/servers  POST /api/servers
//           GET /api/mods  GET /api/mods/:id
// Rooms:    GET /room/<CODE> (WebSocket, needs a login: Authorization: Bearer <token>). A room keeps no game data: it hands out ids, says who the host is (the
//           player there longest; the host's game runs the AIs), passes messages on, and only lets the host send
//           the world. Registered servers also give their name, mods and player limit.

import { unzipSync } from "fflate";
import { register, login, currentUser, logout, sessionCookie, clearCookie } from "./auth.js";
import { validate } from "./mods.js";

const now = () => Math.floor(Date.now() / 1000);
const esc = (s) => String(s ?? "").replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
const json = (v, status = 200) => new Response(JSON.stringify(v), { status, headers: { "content-type": "application/json; charset=utf-8" } });
const ALIVE = 120; // a server shows in the list while its room was busy in the last 2 minutes (or it's brand new)

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
    const max = Math.min(server?.max ?? 8, 8);
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
    "SELECT s.code, s.name, s.players, s.max, s.mods, u.name AS host FROM servers s JOIN users u ON u.id = s.owner " +
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
    return { code: r.code, name: r.name, host: r.host, players: r.players, max: r.max, mods: mods.map((id) => ({ id, name: names[id] ?? id })) };
  });
}

async function makeServer(env, user, body) {
  const name = String(body.name || "").trim();
  if (!/^[A-Za-z0-9 _!.'-]{3,32}$/.test(name)) return { error: "Server name: 3-32 letters, numbers, spaces and _ ! . ' -" };
  const max = Math.max(2, Math.min(8, parseInt(body.max) || 8));
  const mods = (Array.isArray(body.mods) ? body.mods : String(body.mods || "").split(/[\s,]+/)).map(String).filter((m) => /^[0-9a-f]{12}$/.test(m)).slice(0, 8);
  for (const id of mods) if (!(await env.DB.prepare("SELECT 1 FROM mods WHERE id = ?").bind(id).first())) return { error: `There's no mod ${id}.` };
  // Each player keeps at most 3 servers: making a 4th removes their oldest.
  await env.DB.prepare("DELETE FROM servers WHERE owner = ? AND code NOT IN (SELECT code FROM servers WHERE owner = ? ORDER BY created DESC LIMIT 2)").bind(user.id, user.id).run();
  const letters = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
  const code = [...crypto.getRandomValues(new Uint8Array(6))].map((b) => letters[b % letters.length]).join("");
  const pub = body.public === false || body.public === "off" || body.public === "0" ? 0 : 1;
  await env.DB.prepare("INSERT INTO servers (code, name, owner, public, max, mods, created, seen) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
    .bind(code, name, user.id, pub, max, JSON.stringify(mods), now(), now()).run();
  return { code, name, public: !!pub, max, mods };
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
    ? `<span class="dim small">Hi, ${esc(user.name)}</span><a class="btn ghost" href="/mods">Mods</a><a class="btn" href="/servers/new">Make a server</a><a class="btn ghost" href="/logout">Log out</a>`
    : `<a class="btn ghost" href="/mods">Mods</a><a class="btn ghost" href="/login">Log in</a><a class="btn" href="/register">Register</a>`;
  return new Response(`<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>${esc(title)} · Hollow Hours</title>${STYLE}</head><body><header><a class="logo" href="/">👻 Hollow Hours</a><nav>${nav}</nav></header><main>${body}</main></body></html>`,
    { headers: { "content-type": "text/html; charset=utf-8", ...extraHeaders } });
}

const redirect = (to, headers = {}) => new Response(null, { status: 303, headers: { Location: to, ...headers } });

async function home(env, user) {
  const list = await servers(env);
  const rows = list.map((s) => `<tr><td><b>${esc(s.name)}</b><div class="dim small">host ${esc(s.host)} · code <code>${esc(s.code)}</code></div></td>
<td>${s.players}/${s.max}</td><td>${s.mods.map((m) => `<a class="pill" href="/mods/${m.id}">${esc(m.name)}</a>`).join("") || '<span class="dim small">no mods</span>'}</td></tr>`).join("");
  const { results: mods } = await env.DB.prepare("SELECT m.id, m.name, m.version, m.description, u.name AS author FROM mods m JOIN users u ON u.id = m.author ORDER BY m.created DESC LIMIT 6").all();
  return page("Servers", user, `<div class="hero"><h1 style="margin:0">Hollow Hours</h1>
<p>A small horror game where the characters are real AIs. Infection mode: every AI outside the Claude family is infected, and you're the last human. Play online with friends.</p></div>
<div class="card"><h2 style="margin-top:0">Servers</h2>${list.length ? `<table><tr><th>Server</th><th>Players</th><th>Mods</th></tr>${rows}</table>`
    : `<p class="dim">No public servers right now. ${user ? '<a href="/servers/new">Make one.</a>' : '<a href="/register">Register</a> to make one.'}</p>`}
<p class="dim small">In the game: <b>S</b> on the title screen shows this list (mods download by themselves), or <b>J</b> and type a code.</p></div>
<div class="card"><h2 style="margin-top:0">Newest mods</h2>${mods.length ? mods.map((m) => `<p><a href="/mods/${m.id}"><b>${esc(m.name)}</b></a> ${esc(m.version)} <span class="dim small">by ${esc(m.author)}</span><br><span class="small">${esc(m.description)}</span></p>`).join("") : '<p class="dim">No mods yet.</p>'}
<a href="/mods">All mods and how to make one</a></div>`);
}

function authForm(kind, user, error = "", email = "", name = "") {
  const reg = kind === "register";
  return page(reg ? "Register" : "Log in", user, `<div class="card" style="max-width:440px;margin:0 auto"><h2 style="margin-top:0">${reg ? "Make an account" : "Log in"}</h2>
${error ? `<p class="err">${esc(error)}</p>` : ""}
<form method="post" action="/${kind}"><label>Email</label><input name="email" type="email" required autocomplete="email" value="${esc(email)}">
${reg ? `<label>Display name <span class="dim small">(shown in game)</span></label><input name="name" required minlength="3" maxlength="16" pattern="[A-Za-z0-9_-]{3,16}" value="${esc(name)}">` : ""}
<label>Password${reg ? ' <span class="dim small">(at least 8 characters)</span>' : ""}</label><input name="password" type="password" required minlength="${reg ? 8 : 1}" autocomplete="${reg ? "new-password" : "current-password"}">
<p><button class="btn" type="submit">${reg ? "Register" : "Log in"}</button></p></form>
${reg ? '<p class="dim small">Your password is stored only as a salted hash. We don\'t send emails yet, so there\'s no confirmation step and no password reset by email.</p>' : '<p class="dim small">No account? <a href="/register">Register</a>.</p>'}</div>`);
}

async function newServerPage(env, user, error = "") {
  if (!user) return redirect("/login");
  const { results: mods } = await env.DB.prepare("SELECT id, name, version FROM mods ORDER BY created DESC LIMIT 50").all();
  return page("Make a server", user, `<div class="card" style="max-width:520px;margin:0 auto"><h2 style="margin-top:0">Make a server</h2>${error ? `<p class="err">${esc(error)}</p>` : ""}
<form method="post" action="/servers/new"><label>Name</label><input name="name" required minlength="3" maxlength="32">
<label>Max players</label><select name="max">${[2, 3, 4, 5, 6, 7, 8].map((n) => `<option ${n === 8 ? "selected" : ""}>${n}</option>`).join("")}</select>
<label><input type="checkbox" name="public" checked style="width:auto"> Show it in the server list</label>
<label>Mods</label>${mods.length ? mods.map((m) => `<label class="small" style="font-weight:400"><input type="checkbox" name="mods" value="${m.id}" style="width:auto"> ${esc(m.name)} ${esc(m.version)}</label>`).join("") : '<p class="dim small">No mods uploaded yet.</p>'}
<p><button class="btn" type="submit">Make it</button></p></form></div>`);
}

async function modsPage(env, user, msg = "") {
  const { results } = await env.DB.prepare("SELECT m.id, m.name, m.version, m.description, u.name AS author FROM mods m JOIN users u ON u.id = m.author ORDER BY m.created DESC LIMIT 200").all();
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
  const m = await env.DB.prepare("SELECT m.*, u.name AS author FROM mods m JOIN users u ON u.id = m.author WHERE m.id = ?").bind(id).first();
  if (!m) return new Response("not found", { status: 404 });
  const c = JSON.parse(m.content);
  const rules = Object.entries(c.rules || {}).map(([k, v]) => `<span class="pill">${esc(k)}: ${esc(v)}</span>`).join("");
  const chars = (c.characters || []).map((x) => `<p><span style="display:inline-block;width:12px;height:12px;border-radius:50%;background:${esc(x.color)}"></span> <b>${esc(x.name)}</b> ${x.infected ? "(infected)" : "(friendly)"} <span class="dim small">${esc(x.personality)}</span></p>`).join("");
  return page(m.name, user, `<div class="card"><h2 style="margin-top:0">${esc(m.name)} <span class="dim">${esc(m.version)}</span></h2><p class="dim small">by ${esc(m.author)} · id <code>${m.id}</code></p><p>${esc(m.description)}</p>
${c.intro ? `<p><i>${esc(c.intro)}</i></p>` : ""}${rules ? `<h3>Rules</h3><p>${rules}</p>` : ""}${chars ? `<h3>Characters</h3>${chars}` : ""}
<p><a class="btn ghost" href="/mods/${m.id}/download">Download content</a> <span class="dim small">Servers with this mod load it in game by themselves.</span></p></div>`);
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
      if (p === "/api/me") return user ? json({ name: user.name }) : json({ error: "not logged in" }, 401);
      if (p === "/api/servers" && req.method === "GET") return json(await servers(env));
      if (p === "/api/servers" && req.method === "POST") { if (!user) return json({ error: "log in first" }, 401); const r = await makeServer(env, user, await req.json().catch(() => ({}))); return json(r, r.error ? 400 : 200); }
      if (p === "/api/mods") { const { results } = await env.DB.prepare("SELECT id, name, version, description FROM mods ORDER BY created DESC LIMIT 200").all(); return json(results); }
      const apiMod = p.match(/^\/api\/mods\/([0-9a-f]{12})$/);
      if (apiMod) { const m = await env.DB.prepare("SELECT id, name, version, content FROM mods WHERE id = ?").bind(apiMod[1]).first(); return m ? json({ id: m.id, name: m.name, version: m.version, content: JSON.parse(m.content) }) : json({ error: "not found" }, 404); }

      // --- website ---
      if (p === "/") return await home(env, user);
      if (p === "/register" || p === "/login") {
        if (req.method === "GET") return authForm(p.slice(1), user);
        const f = await req.formData();
        const r = p === "/register" ? await register(env, f.get("email"), f.get("password"), f.get("name")) : await login(env, f.get("email"), f.get("password"));
        if (r.error) return authForm(p.slice(1), user, r.error, String(f.get("email") || ""), String(f.get("name") || ""));
        return redirect("/", { "Set-Cookie": sessionCookie(r.token) });
      }
      if (p === "/logout") { await logout(env, req); return redirect("/", { "Set-Cookie": clearCookie }); }
      if (p === "/servers/new") {
        if (req.method === "GET") return await newServerPage(env, user);
        if (!user) return redirect("/login");
        const f = await req.formData();
        const r = await makeServer(env, user, { name: f.get("name"), max: f.get("max"), public: f.get("public") === "on", mods: f.getAll("mods") });
        if (r.error) return await newServerPage(env, user, r.error);
        return page("Server made", user, `<div class="card"><h2 style="margin-top:0">${esc(r.name)} is ready</h2><p>Room code: <code style="font-size:22px">${r.code}</code></p>
<p>In the game: <b>S</b> on the title screen and pick it, or <b>J</b> and type <code>${r.code}</code>. Share the code with friends.${r.public ? "" : " It's private: only people with the code can join."}</p></div>`);
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
