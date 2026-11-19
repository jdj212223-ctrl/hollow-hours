// Server console commands, shared by the website's Console page (rooms on the official server) and the terminal of
// a self-hosted server (selfhost/console.mjs is a copy of this file). Plain text in, lines of text out.
//
//   help                    this list
//   list                    who's online (and who's muted)
//   kick <name> [reason]    disconnect a player (they can come back)
//   ban <name> [reason]     disconnect and keep them out
//   unban <name>            let them back in
//   bans                    who's banned
//   mute <name>             their voice chat and radio stop reaching anyone
//   unmute <name>
//   mutes                   who's muted
//   say <message>           a message from the server to everyone in the game
//   stop [reason]           disconnect everyone

export const HELP = [
  "help                   this list",
  "list                   who's online",
  "kick <name> [reason]   disconnect a player (they can come back)",
  "ban <name> [reason]    disconnect and keep them out",
  "unban <name>           let them back in",
  "bans                   who's banned",
  "mute <name>            their voice and radio stop reaching anyone",
  "unmute <name>          give them their voice back",
  "mutes                  who's muted",
  "say <message>          a message from the server to everyone",
  "stop [reason]          disconnect everyone",
];

/**
 * Runs one console line. `s` is the server:
 *   players(): [{ uid, name }]   bans / mutes: { [uid]: { name, reason?, at } }
 *   kick(uid, reason), say(text), stop(reason), mutedChanged(uid, muted)
 * Returns the lines to print. Changes `bans` / `mutes` in place (the caller saves them).
 */
export function runCommand(line, s) {
  const text = String(line || "").trim().slice(0, 300);
  if (!text) return [];
  const [cmd0, ...rest] = text.split(/\s+/);
  const cmd = cmd0.toLowerCase();
  const arg = rest.join(" ");
  const players = s.players();
  // A player by name: exact (any case), or a unique start of a name.
  const find = (name) => {
    const n = String(name || "").toLowerCase();
    if (!n) return { error: "Which player? (try: list)" };
    const exact = players.filter((p) => p.name.toLowerCase() === n);
    if (exact.length) return { p: exact[0] };
    const starts = players.filter((p) => p.name.toLowerCase().startsWith(n));
    if (starts.length === 1) return { p: starts[0] };
    return { error: starts.length ? `"${name}" matches ${starts.map((p) => p.name).join(", ")}: type more of the name.` : `Nobody called "${name}" is online.` };
  };
  const inList = (list, name) => Object.entries(list).find(([, v]) => v.name.toLowerCase() === String(name || "").toLowerCase());
  switch (cmd) {
    case "help": case "?": return HELP;
    case "list": case "players": case "who":
      return players.length
        ? [`${players.length} online:`, ...players.map((p) => `  ${p.name}${s.mutes[p.uid] ? "  (muted)" : ""}`)]
        : ["Nobody's online."];
    case "kick": case "ban": {
      const [name, ...why] = rest;
      const f = find(name);
      if (f.error) return [f.error];
      const reason = why.join(" ").slice(0, 120) || (cmd === "ban" ? "banned by the host" : "kicked by the host");
      if (cmd === "ban") s.bans[f.p.uid] = { name: f.p.name, reason, at: Date.now() };
      s.kick(f.p.uid, cmd === "ban" ? `banned: ${reason}` : `kicked: ${reason}`);
      return [`${cmd === "ban" ? "Banned" : "Kicked"} ${f.p.name} (${reason}).`];
    }
    case "unban": {
      const e = inList(s.bans, arg);
      if (!e) return [`"${arg}" isn't banned. (bans shows the list)`];
      delete s.bans[e[0]];
      return [`Unbanned ${e[1].name}.`];
    }
    case "bans": {
      const b = Object.values(s.bans);
      return b.length ? [`${b.length} banned:`, ...b.map((x) => `  ${x.name}: ${x.reason}`)] : ["Nobody's banned."];
    }
    case "mute": {
      const f = find(arg);
      if (f.error) return [f.error];
      s.mutes[f.p.uid] = { name: f.p.name, at: Date.now() };
      s.mutedChanged(f.p.uid, true);
      return [`Muted ${f.p.name}: their voice and radio no longer reach anyone.`];
    }
    case "unmute": {
      const e = inList(s.mutes, arg);
      if (!e) return [`"${arg}" isn't muted.`];
      delete s.mutes[e[0]];
      s.mutedChanged(Number(e[0]), false);
      return [`Unmuted ${e[1].name}.`];
    }
    case "mutes": {
      const m = Object.values(s.mutes);
      return m.length ? [`${m.length} muted:`, ...m.map((x) => `  ${x.name}`)] : ["Nobody's muted."];
    }
    case "say": case "broadcast":
      if (!arg) return ["say what?"];
      s.say(arg.slice(0, 200));
      return [`[server] ${arg.slice(0, 200)}`];
    case "stop": {
      const n = players.length;
      s.stop(arg.slice(0, 120) || "the server was stopped");
      return [`Disconnected everyone (${n}).`];
    }
    default:
      return [`Unknown command "${cmd0}". Type help.`];
  }
}
