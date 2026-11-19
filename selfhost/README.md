# Host your own Hollow Hours server

Run Hollow Hours online on your own machine (a VPS, a home server, a Raspberry Pi) and list it on the website's server
list. Same rooms as the official server: the first player in is the host, up to 8 players.

## 1. Register it
Log in on the website, **Make a server**, and fill in **Hosted somewhere else?** with your server's public address,
e.g. `wss://hh.example.com`. The website shows a **code** and a **secret** once: keep the secret private.

## 2. Run it
Needs Node.js 20+.
```
npm install
HH_CODE=ABC123 HH_SECRET=... HH_PUBLIC_URL=wss://hh.example.com PORT=8080 node server.mjs
```
Put it behind TLS so players reach it as `wss://` (e.g. Caddy: `hh.example.com { reverse_proxy localhost:8080 }`).
`HH_PUBLIC_URL` must be exactly the address you registered. `HH_MAX` (2-8) limits players.

## How logins stay safe
Players' logins never reach your server. The game asks the website for a **ticket**: signed with Ed25519, valid
**60 seconds**, made only for **your server's address**, and usable **once**. Your server checks it with the website's
public key (`/api/ticket-key`) and gets the player's verified name from it. Your server reports its player count to
the website every minute with its secret, which keeps it in the server list.

## Console (kick, ban, mute)

While the server runs, type commands in the same terminal:

```
help                   this list
list                   who's online
kick <name> [reason]   disconnect a player (they can come back)
ban <name> [reason]    disconnect and keep them out
unban <name>           let them back in
bans                   who's banned
mute <name>            their voice and radio stop reaching anyone
unmute <name>          give them their voice back
mutes                  who's muted
say <message>          a message from the server to everyone
stop [reason]          disconnect everyone
```

Bans and mutes are saved in `hh-lists.json` next to `server.mjs`, so they survive restarts. Running it as a service
without a terminal (systemd, Docker)? Set `HH_CONSOLE=1` to read commands from standard input anyway.
