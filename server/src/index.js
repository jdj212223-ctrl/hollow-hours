// Hollow Hours multiplayer relay.
//   GET /room/<CODE>  (WebSocket upgrade) joins a room. Codes are 4-8 letters/digits.
// The room keeps no game logic: it hands each player an id, tells everyone who the host is (the player who has been
// there longest; the host's game runs the AIs), and passes every message on to the others in the room.
// Messages are JSON and small; anything over 16 KB, or not JSON, is dropped. At most 8 players per room.

const MAX_PLAYERS = 8;

export class Room {
  constructor(ctx) {
    this.ctx = ctx;
  }

  sockets() {
    return this.ctx.getWebSockets().filter((ws) => ws.readyState === 1 && ws.deserializeAttachment());
  }

  host() {
    const all = this.sockets().map((ws) => ws.deserializeAttachment()).sort((a, b) => a.joined - b.joined);
    return all[0]?.id ?? null;
  }

  broadcast(msg, except) {
    const text = typeof msg === "string" ? msg : JSON.stringify(msg);
    for (const ws of this.sockets()) if (ws !== except) try { ws.send(text); } catch {}
  }

  async fetch(req) {
    if (req.headers.get("Upgrade") !== "websocket") return new Response("expected a WebSocket", { status: 426 });
    if (this.sockets().length >= MAX_PLAYERS) return new Response("room is full", { status: 429 });
    const [client, server] = Object.values(new WebSocketPair());
    const id = crypto.randomUUID().slice(0, 8);
    this.ctx.acceptWebSocket(server);
    server.serializeAttachment({ id, joined: Date.now() + Math.random() });
    server.send(JSON.stringify({ t: "hello", you: id, host: this.host(), players: this.sockets().length }));
    this.broadcast({ t: "host", host: this.host(), players: this.sockets().length }, server);
    return new Response(null, { status: 101, webSocket: client });
  }

  async webSocketMessage(ws, data) {
    if (typeof data !== "string" || data.length > 16384) return;
    let msg;
    try { msg = JSON.parse(data); } catch { return; }
    const me = ws.deserializeAttachment();
    if (!me) return;
    msg.from = me.id; // nobody can pretend to be someone else
    // Only the host may send the world (AIs, supplies, tornado).
    if (msg.t === "world" && me.id !== this.host()) return;
    this.broadcast(msg, ws);
  }

  async webSocketClose(ws) {
    const me = ws.deserializeAttachment();
    ws.serializeAttachment(null);
    try { ws.close(1000, "bye"); } catch {}
    if (me) this.broadcast({ t: "left", from: me.id, host: this.host(), players: this.sockets().length });
  }

  async webSocketError(ws) {
    await this.webSocketClose(ws);
  }
}

export default {
  async fetch(req, env) {
    const url = new URL(req.url);
    const m = url.pathname.match(/^\/room\/([A-Za-z0-9]{4,8})$/);
    if (m) return env.ROOMS.get(env.ROOMS.idFromName(m[1].toUpperCase())).fetch(req);
    return new Response("Hollow Hours multiplayer. Join a room from the game (J on the title screen).\n", { headers: { "content-type": "text/plain" } });
  },
};
