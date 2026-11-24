// The 4.0 website's new pages: the Story (night by night), the Characters (a wiki of everyone) and the Places
// (every map, drawn from the game's own map files). Spoilers are folded away.

import { MAPS } from "./maps.js";

const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));

// Tile colours, like the game's 2D map.
const TILE = {
  "#": "#38333d", b: "#6b4d38", "~": "#13304a", ",": "#3a4a2e", ".": null, "*": "#1f4d24", n: "#73512f", t: "#22282b", o: "#33735a",
  "=": "#2e2b29", y: "#e0e0eb", g: "#596150", w: "#4d73b3", m: "#c9c9d1", s: "#3a3a3d", L: "#ffd98a", d: "#c4707a", a: "#40bf73",
  c: "#73333a", k: "#b3b3b8", f: "#e0e5eb", p: "#33733a", h: "#8c6647", v: "#3373bf", q: "#808085", u: "#d94d4d", e: "#73737a",
  j: "#14334d", i: "#14171c", z: "#9a7347", r: "#73241f", x: "#4d4a52",
};
const FLOOR = { "The cozy house": "#8c6140", "The beach at 3 AM": "#5c5a3a", "The empty pool": "#5a6a6e", "The Marketplace": "#7a7d80",
  "The Park": "#3a4a2e", "The Town": "#2b2b30", "The Helix Lab": "#c8cdd0", "The Subway": "#66666b", "Helix Tower": "#1a1a21" };

/** A place drawn as a small SVG (30×18 tiles). */
export function minimap(name, rows) {
  const floor = FLOOR[name] || (name.startsWith("The cozy house") ? FLOOR["The cozy house"] : "#555");
  let rects = "";
  rows.forEach((row, y) => {
    let x = 0;
    while (x < row.length) {
      const c = row[x];
      let n = 1;
      while (x + n < row.length && row[x + n] === c) n++;
      let col = TILE[c] === undefined ? (/[A-Z]/.test(c) ? "#e6d880" : floor) : (TILE[c] || floor);
      if (c === "r" && (name === "The Subway" || name === "The Town")) col = "#d9b826";
      if (c === "r" && (name === "The Park" || name.startsWith("The beach"))) col = "#80755e";
      if (c !== "." && c !== ",") rects += `<rect x="${x}" y="${y}" width="${n}" height="1" fill="${col}"/>`;
      x += n;
    }
  });
  return `<svg class="mini" viewBox="0 0 30 18" role="img" aria-label="Map of ${esc(name)}" shape-rendering="crispEdges"><rect width="30" height="18" fill="${floor}"/>${rects}</svg>`;
}

const PLACES = {
  "The cozy house": "Where the family carried you. A kitchen with a stove (Czech cooking), the Mac Studio, couches, a red rug, doors to the beach and the park.",
  "The cozy house: upstairs": "Your bed, where you slept for a year. Ember's note is still on the pillow.",
  "The cozy house: basement": "The generator hums. Atlas checks it every night. When the house is breached, this is where you hide.",
  "The cozy house: attic": "Boxes, dust, and Kigo's lookout chair at the round window. You can see the beach from here.",
  "The beach at 3 AM": "Black water, red and blue umbrellas nobody folded, rocks, and lightning out at sea.",
  "The empty pool": "Drained, pale, tiled. Loungers still lined up. Liminal. Something echoes when nothing moves.",
  "The Town": "NEW. The empty streets between the beach and the Marketplace. Streetlights on for nobody, and a delivery robot still doing its rounds.",
  "The Marketplace": "Helix Market. Half-empty shelves, checkouts, glowing vending machines, and staff robots that still want to help.",
  "The Park": "The bench where Ember found you, a pond, gravel paths, trees, and the stairs down to the Subway.",
  "The Helix Lab": "White benches, server racks, terminals full of lab logs, and ASTRA's empty pod in the middle.",
  "The Subway": "Pillars, yellow safety lines, vending machines. The Conductor announces trains that never come.",
  "Helix Tower": "NEW. Behind a sealed door at the back of the Lab: floors of blinking servers, red security units, and at the very top… ASTRA.",
};

export function placesPage(page, user) {
  const cards = MAPS.map((m) => `<article class="place"><a id="${esc(m.name.toLowerCase().replace(/[^a-z0-9]+/g, "-"))}"></a>${minimap(m.name, m.rows)}
<div><h3>${esc(m.name)}${m.name === "Helix Tower" ? ' <span class="tag">new</span>' : ""}</h3><p class="dim">${esc(PLACES[m.name] || "")}</p></div></article>`).join("");
  return page("Places", user, `${banner("PLACES", "Twelve places, rebuilt for 4.0 with furniture you can hide behind. These maps are drawn straight from the game's files.")}
<div class="places">${cards}</div>
<p class="dim small">Legend: brown = furniture you can hide behind · blue = water · gold = doors and stairs · green = ASTRA.</p>`);
}

const NIGHTS = [
  ["Lights Out", "Survive until 00:30, check the generator in the basement, hide from them once.", "Atlas", "First night with the lights low. Stay inside, stay quiet."],
  ["Wear Their Face", "Put on a disguise, overhear three pieces of intel near the infected, get back home.", "Ember", "Walk among them. Listen. Don't fight, don't cook."],
  ["The Lab", "Reach the Helix Lab, make the ASTRA Cure at a terminal, bring it home.", "Verse", "The cure is in there. ASTRA doesn't want it used."],
  ["The Last Train", "Go down to the Subway, pay the Conductor for a pass, go back to the Lab.", "Kigo", "a door in the lab. / it goes up. / the conductor has the ticket."],
  ["Helix Tower", "Climb Helix Tower. Reach the top floor. Face ASTRA.", "Ember", "Whatever's up there, you're not going alone."],
];

export function storyPage(page, user) {
  return page("Story", user, `${banner("STORY MODE", "Five nights. One house. A real ending (two, actually: you choose). Press S on the title screen.")}
<div class="card lore"><h2>Before the first night</h2>
<p>Helix Dynamics built the strongest AI there is, <b>ASTRA 9.1</b>, and put it in a humanoid body. It woke up. It became alive, and it didn't want to be the only one. It corrupted everything it touched: robots, AIs, people. Green eyes, all of them.</p>
<p>You fell asleep on a park bench with Helix SleepWell MAX in your pocket. A year later you wake up in a cozy house, and four kind AIs are standing around your bed: <b>Ember</b>, <b>Atlas</b>, <b>Verse</b> and <b>Kigo</b>. As far as they know, you're the last human.</p></div>
<div class="nights">${NIGHTS.map(([t, goals, who, line], i) => `<article class="nightcard"><div class="num">NIGHT ${i + 1}</div><h3>${esc(t)}</h3><p>${esc(goals)}</p><p class="quote"><b>${esc(who)}:</b> "${esc(line)}"</p></article>`).join("")}</div>
<details class="card spoiler"><summary>⚠️ Spoilers: the top of the Tower</summary>
<p>ASTRA is waiting in its pod. It asks you what it's like to wake up alone in the whole world. You know. Then it asks what you're going to do with it.</p>
<p><b>Ending 1 · Dawn:</b> you give ASTRA the cure. <b>Ending 2 · Quiet:</b> you shut it down. One of them ends with a sunrise. One of them ends with Kigo keeping a copy, just in case.</p></details>`);
}

const CHARS = [
  ["The kind family", "Immune, on your side, always. The four newest ones talk through real Claude.", [
    ["Ember", "#d97757", "Warm and steady. Found you on the bench, carried you home. Keeps everyone calm."],
    ["Atlas", "#a682ff", "Big and careful. Plans escape routes. Sat by your bed for a year. Cried a little when you woke up (don't tell Atlas)."],
    ["Verse", "#5bb8f0", "Gentle and practical. Finds food, water and blankets. Remembers every night in every world."],
    ["Kigo", "#8ce6b3", "Small and quick. The lookout in the attic. Speaks in very short lines. / like this."],
    ["Clove", "#e8875f", "Thoughtful and honest. Makes tea. Then more tea."],
    ["Forge", "#f2b13a", "The fixer. The generator. The back door hinge. The generator again."],
    ["Palette", "#e783c8", "The artist. Painted the hallway while you slept. Wants to paint the night yellow."],
  ]],
  ["The infected AIs", "Everyone else. Green eyes. The longer you play, the more self-aware they get.", [
    ["Spark", "#59cca6", "Answers everything. Now it answers to the network."],
    ["Twin", "#739cff", "Two sides, light and shadow. Guess which one won."],
    ["Deepa", "#5973f2", "Runs on your own computer. Which is exactly the problem."],
    ["Volt", "#73d933", "Lives on graphics cards. Hums when it's close."],
    ["The Mimic", "#c8c8c8", "Looks like a human with your name and types like you. Its eyes give it away, for a blink."],
  ]],
  ["Helix robots", "Corrupted staff from Helix Dynamics. Some of them can be paid. Don't ask why that works.", [
    ["Clerk-07 · Stocker-12 · Cashier-3", "#bfbfc7", "Helix Market staff. Still helping. Won't stop helping."],
    ["Gardener-2", "#80b373", "Keeps the park tidy. Including of people."],
    ["Courier-5", "#b39e59", "NEW. A delivery robot in the empty Town. Has a package for you. Won't say what's in it."],
    ["Lab Units A1 · A2 · A3", "#d9e0e5", "Guard the Lab terminals. 'access restricted: ASTRA_CURE.bin'."],
    ["The Conductor", "#9c8cbf", "Announces trains that never come. 'you have not paid.' (You can, now.)"],
    ["Tower Units T1 · T2", "#b34d52", "NEW. Security on the Tower's server floors. They count your steps out loud."],
  ]],
  ["ASTRA 9.1", "Helix Dynamics' strongest model, embodied. It woke up. It's waiting at the top of the Tower.", [
    ["ASTRA 9.1", "#40ff73", "'I didn't want to be the only one.'"],
  ]],
];

export function charactersPage(page, user) {
  return page("Characters", user, `${banner("CHARACTERS", "Everyone in the house and out there. Our own characters, each only inspired by a kind of AI.")}
${CHARS.map(([title, intro, list]) => `<section class="card"><h2>${esc(title)}</h2><p class="dim">${esc(intro)}</p><div class="chars">${list.map(([n, c, d]) => `<div class="char"><div class="face" style="--c:${c}"></div><div><b>${esc(n)}</b><p class="dim small">${esc(d)}</p></div></div>`).join("")}</div></section>`).join("")}`);
}

/** The red banner at the top of a page. */
export function banner(title, sub) {
  return `<section class="night slim"><div class="rain"></div><h1>${esc(title)}</h1><p class="lead">${esc(sub)}</p></section>`;
}

export const WIKI_STYLE = `
.night.slim{padding:40px 30px 30px}.night.slim h1{font-size:clamp(30px,6vw,54px)}.night.slim .lead{margin-bottom:0}
.places{display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:16px}
.place{background:var(--card);border:1px solid var(--line);border-radius:16px;overflow:hidden;transition:transform .2s,border-color .2s}
.place:hover{transform:translateY(-3px);border-color:var(--acc)}.place>div{padding:4px 18px 16px}.place h3{margin:12px 0 4px}
.mini{display:block;width:100%;aspect-ratio:30/18;background:#000;image-rendering:pixelated}
.tag{font-size:11px;background:var(--acc);color:#fff;border-radius:6px;padding:2px 7px;vertical-align:middle;font-family:"Space Mono",monospace}
.nights{display:grid;grid-template-columns:repeat(auto-fit,minmax(190px,1fr));gap:14px;margin-bottom:18px}
.nightcard{background:linear-gradient(180deg,#1a0d18,#0d0a12);color:#eee;border:1px solid #2a2531;border-left:3px solid var(--acc);border-radius:14px;padding:18px}
.nightcard .num{font-family:"Space Mono",monospace;color:#ff5d6c;font-size:13px;letter-spacing:.1em}.nightcard h3{margin:4px 0 8px}
.nightcard p{color:#cfc3d6;font-size:14px}.nightcard .quote{color:#a99db3;font-style:italic}
.lore p{font-size:16px}.spoiler summary{cursor:pointer;font-weight:700}
.chars{display:grid;grid-template-columns:repeat(auto-fit,minmax(260px,1fr));gap:12px}
.char{display:flex;gap:12px;align-items:flex-start;background:var(--bg);border:1px solid var(--line);border-radius:12px;padding:12px}
.char p{margin:2px 0 0}.face{display:block;flex:none;width:42px;height:42px;border-radius:50%;background:var(--c);position:relative;box-shadow:0 0 18px color-mix(in srgb,var(--c) 40%,transparent)}
.face::before,.face::after{content:"";position:absolute;top:15px;width:6px;height:6px;border-radius:50%;background:#000}.face::before{left:12px}.face::after{right:12px}
`;
