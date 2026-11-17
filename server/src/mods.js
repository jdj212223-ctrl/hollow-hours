// The .hh mod format (Hollow Hours), version 2: a mod can customise the whole game. A zip with
//   hh.json       { "name", "version", "description" }
//   content.json  every part optional:
//     "intro":      "one line when the game starts"                (kept from v1)
//     "rules":      { see RULES below }
//     "text":       { "title", "over_infected", "over_claudes", "warnings": ["{} is coming!", ...] }
//     "theme":      { "infected_eye": "#hex", "kind_halo": "#hex", "hud": "#hex" }
//     "start":      { "map": 0, "x": 4, "y": 5 }
//     "maps":       [ { "name", "rows": [18 strings of 30 tiles], "dark": 0-1, "liminal": 0-1, "floor": "#hex", "wall": "#hex",
//                       "doors": { "A": { "to": 1, "x": 2, "y": 9 } } } ]
//     "replace_maps": true   (use only the mod's maps; otherwise they're added after the built-in three)
//     "characters": [ { "name", "color", "personality", "infected": bool, "kind": bool, "map", "x", "y" } ]
//     "remove":     ["Spark", ...]   built-in characters to leave out
// Tiles: # wall  . floor  , sand  ~ water  b furniture (you can hide next to it)  L lamp  d doll  A-Z doors (see "doors").
// Data only, never code. Every field is checked here and again in the game; unknown keys are refused.

export const RULES = {
  infected_speed: [1.0, 4.0],
  player_speed: [2.0, 5.0],
  sneak_speed: [0.8, 2.5],
  sight_tiles: [3, 14],
  hearing_tiles: [2, 12],
  infect_touch: [0.05, 1.0],
  infect_near: [0, 0.2],
  claude_infect: [0.05, 1.0],
  food_minutes: [1, 30],
  water_minutes: [1, 30],
  supplies_per_map: [0, 12],
  supply_amount: [0.1, 1.0],
  tornado_min: [20, 600],
  tornado_max: [20, 900],
  tornado_speed: [1, 6],
  tornado_radius: [0.5, 4],
};
const TOP = ["intro", "rules", "text", "theme", "start", "maps", "replace_maps", "characters", "remove"];
const TILES = /^[#.,~bLdA-Z]{30}$/;
const hex = (v) => /^#[0-9a-fA-F]{6}$/.test(String(v));
const str = (v, max) => typeof v === "string" && v.length <= max;
const num = (v, lo, hi) => typeof v === "number" && v >= lo && v <= hi;

export function validate(hh, content) {
  const bad = [];
  if (!hh || typeof hh !== "object") return ["hh.json is missing or not JSON"];
  if (!str(hh.name, 24) || !/^[A-Za-z0-9 _!.-]{1,24}$/.test(hh.name)) bad.push("hh.json: name must be 1-24 letters, numbers, spaces, _ ! . -");
  if (!str(hh.version, 16) || !hh.version) bad.push("hh.json: version is missing");
  if (hh.description !== undefined && !str(hh.description, 300)) bad.push("hh.json: description is over 300 characters");
  if (!content || typeof content !== "object" || Array.isArray(content)) return [...bad, "content.json is missing or not an object"];
  for (const k of Object.keys(content)) if (!TOP.includes(k)) bad.push(`content.json: unknown key "${k}"`);
  if (content.intro !== undefined && !str(content.intro, 200)) bad.push("intro: over 200 characters");

  if (content.rules !== undefined) {
    if (typeof content.rules !== "object" || Array.isArray(content.rules)) bad.push("rules must be an object");
    else for (const [k, v] of Object.entries(content.rules)) {
      if (!RULES[k]) bad.push(`rules: unknown rule "${k}"`);
      else if (!num(v, ...RULES[k])) bad.push(`rules.${k} must be a number from ${RULES[k][0]} to ${RULES[k][1]}`);
    }
    const r = content.rules || {};
    if (r.tornado_min && r.tornado_max && r.tornado_max < r.tornado_min) bad.push("rules: tornado_max is smaller than tornado_min");
  }
  if (content.text !== undefined) {
    const t = content.text;
    if (typeof t !== "object" || Array.isArray(t)) bad.push("text must be an object");
    else for (const [k, v] of Object.entries(t)) {
      if (k === "warnings") {
        if (!Array.isArray(v) || v.length > 8 || !v.every((w) => str(w, 60) && w.length > 0)) bad.push("text.warnings: up to 8 lines of 1-60 characters ({} = the infected's name)");
      } else if (["title", "over_infected", "over_claudes"].includes(k)) {
        if (!str(v, k === "title" ? 24 : 120) || !v) bad.push(`text.${k}: 1-${k === "title" ? 24 : 120} characters`);
      } else bad.push(`text: unknown key "${k}"`);
    }
  }
  if (content.theme !== undefined) {
    if (typeof content.theme !== "object" || Array.isArray(content.theme)) bad.push("theme must be an object");
    else for (const [k, v] of Object.entries(content.theme)) {
      if (!["infected_eye", "kind_halo", "hud"].includes(k)) bad.push(`theme: unknown key "${k}"`);
      else if (!hex(v)) bad.push(`theme.${k}: a colour like "#33ff66"`);
    }
  }
  if (content.replace_maps !== undefined && typeof content.replace_maps !== "boolean") bad.push("replace_maps: true or false");
  const mapCount = (content.replace_maps ? 0 : 3) + (Array.isArray(content.maps) ? content.maps.length : 0);
  if (content.maps !== undefined) {
    if (!Array.isArray(content.maps) || content.maps.length > 8) bad.push("maps: a list of at most 8");
    else content.maps.forEach((m, i) => {
      const at = `maps[${i}]`;
      if (!m || typeof m !== "object") return bad.push(`${at} is not an object`);
      for (const k of Object.keys(m)) if (!["name", "rows", "dark", "liminal", "floor", "wall", "doors"].includes(k)) bad.push(`${at}: unknown key "${k}"`);
      if (!str(m.name, 32) || !m.name) bad.push(`${at}.name: 1-32 characters`);
      if (!Array.isArray(m.rows) || m.rows.length !== 18 || !m.rows.every((r) => typeof r === "string" && TILES.test(r))) bad.push(`${at}.rows: exactly 18 rows of 30 tiles from # . , ~ b L d A-Z`);
      if (m.dark !== undefined && !num(m.dark, 0, 1)) bad.push(`${at}.dark: 0 to 1`);
      if (m.liminal !== undefined && !num(m.liminal, 0, 1)) bad.push(`${at}.liminal: 0 to 1`);
      for (const c of ["floor", "wall"]) if (m[c] !== undefined && !hex(m[c])) bad.push(`${at}.${c}: a colour like "#8a6a50"`);
      if (m.doors !== undefined) {
        if (typeof m.doors !== "object" || Array.isArray(m.doors)) bad.push(`${at}.doors must be an object`);
        else for (const [letter, d] of Object.entries(m.doors)) {
          if (!/^[A-Z]$/.test(letter)) bad.push(`${at}.doors: "${letter}" must be one capital letter`);
          if (!d || !Number.isInteger(d.to) || d.to < 0 || d.to >= mapCount || !num(d.x, 1, 28) || !num(d.y, 1, 16)) bad.push(`${at}.doors.${letter}: { "to": a map number 0-${mapCount - 1}, "x": 1-28, "y": 1-16 }`);
        }
      }
    });
  }
  if (content.start !== undefined) {
    const s = content.start;
    if (!s || !Number.isInteger(s.map) || s.map < 0 || s.map >= Math.max(mapCount, 1) || !num(s.x, 1, 28) || !num(s.y, 1, 16)) bad.push(`start: { "map": 0-${Math.max(mapCount, 1) - 1}, "x": 1-28, "y": 1-16 }`);
  }
  if (content.characters !== undefined) {
    if (!Array.isArray(content.characters) || content.characters.length > 12) bad.push("characters: a list of at most 12");
    else content.characters.forEach((c, i) => {
      const at = `characters[${i}]`;
      if (!c || typeof c !== "object") return bad.push(`${at} is not an object`);
      for (const k of Object.keys(c)) if (!["name", "color", "personality", "infected", "kind", "map", "x", "y"].includes(k)) bad.push(`${at}: unknown key "${k}"`);
      if (!str(c.name, 16) || !/^[A-Za-z0-9 ._-]{1,16}$/.test(c.name)) bad.push(`${at}.name: 1-16 letters, numbers, spaces . _ -`);
      if (!hex(c.color)) bad.push(`${at}.color: like "#ff8800"`);
      if (!str(c.personality, 200) || !c.personality) bad.push(`${at}.personality: 1-200 characters`);
      if (typeof c.infected !== "boolean") bad.push(`${at}.infected: true or false`);
      if (c.kind !== undefined && typeof c.kind !== "boolean") bad.push(`${at}.kind: true or false`);
      if (c.kind && c.infected) bad.push(`${at}: can't be both kind and infected`);
      if (!Number.isInteger(c.map) || c.map < 0 || c.map >= Math.max(mapCount, 1)) bad.push(`${at}.map: 0-${Math.max(mapCount, 1) - 1}`);
      if (!num(c.x, 1, 28) || !num(c.y, 1, 16)) bad.push(`${at}: x 1-28, y 1-16`);
    });
  }
  if (content.remove !== undefined && (!Array.isArray(content.remove) || content.remove.length > 20 || !content.remove.every((n) => str(n, 16)))) bad.push("remove: a list of character names");
  return bad;
}
