# Hollow Hours 4.0: "THE UPDATE": the plan

The idea: the infection breaks out of the game. The corrupted AIs know your computer, your network and even your Minecraft world,
and everything (game, code, website) is rebuilt for it. Built in phases, one release per phase (CI auto-releases on push).

## Ground rules (every phase)
- All of it lives behind "Outside the window" (the meta-horror consent screen), with a new **"Deep meta (4.0)"** level on top that
  is OFF by default and explains each thing it touches. One switch turns it all off; the panic key P stops everything.
- Read things, show things: yes. Change or delete the player's stuff: never. Nothing it learns (username, network name, apps)
  ever leaves the computer: not to servers, not to online players, not to Claude chats.
- Installing apps or launching Minecraft always asks the player first (a real Yes/No in the game), and only from official sources.
- Streamer-safe mode: hides the username / network name lines (for recording or streaming).

## Phase 1: The infected know your computer (Rust, the game)
- **Your username**: the macOS / Windows / Linux account name. Whispers: "nice username, {name}." / "{name}. we practised saying it."
- **Your computer's name** and model: "a MacBook. warm in here."
- **Network name (Wi-Fi)**: "we can hear {wifi}. it's very loud tonight." (macOS needs Location permission for this; ask once, skip if no.)
- **Settings**: the infected *open* System Settings panes (never change them): Wallpaper ("we'd like to redecorate"),
  Display, Sound. Plus they use the real dark/light mode, battery % ("your battery is at 12%. so are you.") and the real time.
- **Installed apps, ALL of them** (only their names; nothing is opened, read or touched): they comment on everything,
  the big ones too: "nice browser. Firefox. we've been reading your tabs.", "a password manager. we'd never look. probably.",
  "your bank's app. don't worry. we only want you." Streamer-safe mode hides app names.
- **Other AI agents on your computer**: they notice the AIs you have installed (Claude Code, Ollama models, other coding
  agents) and talk about them: "you have other ones here. we've met them. they're quiet.". With Deep meta and a Yes, an
  infected can even answer through a *local* model you already have (Ollama) in a corrupted voice. Cloud agents are never
  started by the game, and nothing personal is sent to any of them.
- Self-aware level ties it together: these unlock as the infected become more self-aware (night count / play time).

## Phase 1b: The infection spreads (the DDLC-style part)
- **Spreading to apps and games (never the important ones)**: only through safe, undoable ways: it leaves a note in Notes,
  a sticky on the Desktop, a reminder, a calendar entry "come to the beach. 3:00 AM", a notification from "Hollow Hours",
  and into Minecraft through HH-Link (Phase 4). It never touches system apps, Settings values, mail, passwords, banking,
  browsers' data or other games' saves. Everything it creates is listed and removed with one button ("Clean up").
- **The game deletes its OWN files**: each character has a file in the game's folder (e.g. `characters/Kigo.chr`, like
  DDLC's character files). During Deep meta the infected delete one: Kigo is just... gone from the game, and the save
  remembers it. A backup is kept so it's never lost for real.
- **Finder opens on the game's folder** showing the gap where `Kigo.chr` was. (Not inside the .app itself: changing files
  in there breaks the app's signature and macOS would refuse to open the game. The game's own folder looks just as real.)
- **"Tell us what's wrong."**: the game freezes on a black screen with a text box and won't go on until you type what's
  wrong ("kigo is missing", "they deleted kigo"...). Claude reads your answer; get it right and the kind family finds a way
  to bring Kigo back (restore from the backup, with a scene). Get it wrong and the infected mock you, and you try again.
  The panic key P always gets you out.

## Phase 2: Revamp EVERYTHING in the game (Rust)
**Part 1 done:** Story mode (Nights 1-5, goals, cards, two endings), Helix Tower + ASTRA, new HUD, per-place 3D air,
walking characters, maps split into world.rs. **Part 2:** every menu in one look (ui.rs). **Part 3:** all 11 maps
rebuilt with furniture (couches, counters, fridge, shelves, vending machines, pillars, umbrellas, rocks, a pond, server racks,
boxes, rugs and paths) + a test that every tile and spot is reachable. Next: more of main.rs split up, the town, lighting/shadows, map by map.

- Split main.rs (4000+ lines) into clean modules: world/, characters/, ai/, ui/, net/, meta/.
- New HUD and menus everywhere (same style as the 3.0 title), better 3D: lighting, fog per map, shadows, animated characters.
- Story chapters: Night 1-5 with goals (the cure, the Lab, the Conductor's train), a real ending, and an epilogue.
- Bigger world: the town between the maps, the Helix tower (endgame).

## Phase 3: New languages: Python and Java
- **Python mods**: `.hh` mods can include a Python script (rules, events, new characters' behaviour), run in a sandbox
  (separate process, no file or network access, time limit), talking to the game over a local pipe.
- **Java**: the Minecraft mod (Phase 4) is Java (Fabric). Also a small Java "hh-link" library others can use.

## Phase 4: Minecraft linked to Hollow Hours (Java + Rust)
- **HH-Link**: a Fabric mod (Java) + a local bridge to the running game (127.0.0.1 only, the same token as the MCP bridge).
- What happens in HH happens in Minecraft: a Minecraft world of the cozy house / beach / Marketplace; the infected spawn as
  mobs with green eyes where they are in HH; tornadoes become storms; a breach in HH = walls break in MC; the Claudes appear as
  friendly NPCs; your HH infection % shows as a boss bar. Things you do in MC (eat, find food) count in HH too.
- **The corrupted AI launches it**: during Deep meta, an infected says "we built you somewhere else." and the game asks
  "Let them open Minecraft? (Yes/No)". Yes = it launches the player's own Minecraft / Prism / Horizon with HH-Link.
- **The infected build in your world**: through HH-Link they place blocks live while you play (green "infected" blocks
  creeping across the ground, signs saying "come to the beach", a copy of the cozy house appearing near your base).
- **Corrupting a map with a datapack**: the game offers to infect a Minecraft world you pick: it adds an "hh_infection"
  datapack (spreading green blocks, corrupted structures, infected mobs, the Helix tower, ASTRA's pod). Always on a world
  the player chose, always after a Yes, and the world is **backed up first**; removing the datapack (or restoring the
  backup) undoes it. Never touches worlds you didn't pick.
- **Installing apps**: only "Install HH-Link into Minecraft" (copies the mod into the mods folder, after a Yes) and, if missing,
  opening the official download page. No silent installs, ever.

## Phase 5: Overhaul the website (JS, Cloudflare)
- A new site that matches 4.0: an interactive "infected" home page, Night-by-night lore pages, a wiki of every character,
  the mods gallery with Python mods, the HH-Link (Minecraft) download page, player profiles, and server stats.
- The fictional Helix site grows: a "careers" page that slowly corrupts, the redacted parent company (Helix is "a division of
  ████████ Holdings"), investor letters. Still clearly marked fictional, no real companies.

## Order and size
1. Phase 1 (meta: username, Wi-Fi, settings, apps): small, big scare. 1 session.
2. Phase 4 (Minecraft link): the headline feature. 2-3 sessions (Java mod + bridge + world builder).
3. Phase 5 (website): 1 session.
4. Phase 3 (Python mods): 1-2 sessions.
5. Phase 2 (full game revamp): the biggest; several sessions, done map by map.
Release 4.0 when Phases 1 + 4 are in; the rest ship as 4.x.
