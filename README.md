# Hollow Hours 👻

A small horror game where the characters are **real AIs**. Inspired by the *kind* of game The Mita is (AI characters,
cozy house, a crazy mode), with our own characters, maps and look.

## The characters (our own, each only *inspired by* a kind of AI, no real names or logos)
| | Inspired by | Brain |
|---|---|---|
| Clove | a thoughtful, honest assistant AI | Claude API if `ANTHROPIC_API_KEY` is set, else local |
| Forge | a coding agent AI (fixes the house) | Claude API / local |
| Palette | a design AI (keeps redecorating) | Claude API / local |
| Spark | a chat AI that answers everything | OpenAI API if `OPENAI_API_KEY`, else local |
| Twin | a two-sided AI (light and shadow) | Gemini API if `GEMINI_API_KEY`, else local |
| Deepa | an open AI on your own computer | local DeepSeek-R1 (offline) |
| Volt | an AI that lives on graphics cards | local |

**The Claude family is always friendly** (Clove, Forge, Palette, and in Infection mode Claude, Opus 5.5, Sonnet 4.6 and
Haiku 4.5): never crazy, never infected. If the **Claude desktop app** is installed, they talk through it (`claude -p`,
your own Claude account), so you're chatting with real Claude. An `ANTHROPIC_API_KEY` is used first if set.
Each Claude character keeps **one session per game**, named **"Hollow Hours: <character>"**, and remembers the whole
chat. To find them: `cd /Volumes/important/hollow-hours && claude --resume` (they're saved in
`~/.claude/projects/-Volumes-important-hollow-hours/`).

"Local" = [Ollama](https://ollama.com) on this computer (qwen3:4b-instruct by default): free and offline. If no AI
answers at all, characters use scripted lines. They also talk to *each other* now and then (local AI only).

## Maps
The cozy house · the beach at 3 AM · the empty pool (very liminal).

## Meters
- 😨 **Fear**: darkness, a crazy character near you, deep water. High fear shrinks your light and shakes the screen.
- 😣 **Discomfort**: someone standing too close, dolls whose eyes follow you. Tints the screen, glitches the HUD.
- 🌫️ **Liminality**: how empty and wrong a place feels. Greys everything out.

## Choose your courage
1. **Brave**: no chasing, no crazy mode, friendly AIs, fear capped low
2. **Normal**
3. **Many phobias**: turn each one ON (the world uses it) or OFF (it never happens): darkness, water, dolls, being
   chased, crowds, empty places

## Infection mode (4 on the title screen)
Every non-Claude AI is infected (green eyes). You're the last human.
- **Sneak** with **Shift**: slow and silent. Walking makes **noise** (the ring around you): infected inside it hear you
  even without seeing you. Grabbing food or water while sneaking is quieter too.
- **H** hides you next to furniture. Food 🍎 and water 💧 run down. **Tornadoes** sweep across the map after a warning.
- Being touched raises your **Infection %**. At **100%** you can still see, but you **can't move**: your body walks off
  to infect the Claudes.
- The **Claudes can be infected too** (a little green bar over them). When **every Claude is infected, it's game over.**

## Outside the window (asked once, on first launch)
If you allow it, Infection mode reaches outside the game:
- **Files** in a `Hollow Hours` folder on your Desktop (at 10%, 75% and 100% infection, when a Claude turns, and at the
  end). They **stay after you quit**. It only creates its own new files, never touches yours, and **C** on the title
  screen removes exactly the files it made.
- **Terminal** opens to show a message (it only prints text).
- **Your window switches** to TextEdit (75%) or Finder (100%).
- **M** on the title screen changes your answer. **P** (panic) stops all of it for the rest of the session.

## Crazy mode
At night a character on your map can snap: red eyes, too-calm voice, and it walks toward you. If it catches you: a
jumpscare (one quick fade, **no flashing**) and you wake up in the cozy house. Nobody dies.

## Safety
- **P = panic**, any time: every scare stops, meters reset, vest effects stop, a calm "You're safe" screen.
- **Haptic vest** (bHaptics, through the bHaptics Player app): found automatically, but **OFF** until you press V on
  the title screen. Mild patterns (a chill down the back, a heartbeat, a tense pulse), capped at 60%, `[` `]` to make
  them weaker or stronger. Don't use it with a heart condition.

## Run
```
cargo run --release              # needs Rust; start Ollama first for real AI characters
HOLLOW_DEMO=1 cargo run --release   # jumps straight in next to Clove and asks her something
```
WASD move · E talk (type, Enter) · P panic · Esc menu
