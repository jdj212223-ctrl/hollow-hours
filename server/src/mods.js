// The .hh mod format (Hollow Hours): a zip with
//   hh.json       { "name", "version", "description" }
//   content.json  { "intro": "...",
//                   "rules": { "infected_speed", "tornado_min", "tornado_max", "food_minutes", "water_minutes", "sneak_speed" },
//                   "characters": [ { "name", "color": "#rrggbb", "personality", "infected": true|false, "map": 0-2, "x", "y" } ] }
// Data only, never code: a mod can't run anything on anyone's computer. Every field is checked here (and again in the
// game), unknown keys are refused, and everything has limits.

export const RULES = {
  infected_speed: [1.0, 4.0],
  tornado_min: [20, 600],
  tornado_max: [20, 900],
  food_minutes: [1, 30],
  water_minutes: [1, 30],
  sneak_speed: [0.8, 2.5],
};

const str = (v, max) => typeof v === "string" && v.length <= max;

export function validate(hh, content) {
  const bad = [];
  if (!hh || typeof hh !== "object") return ["hh.json is missing or not JSON"];
  if (!str(hh.name, 24) || !/^[A-Za-z0-9 _!.-]{1,24}$/.test(hh.name)) bad.push("hh.json: name must be 1-24 letters, numbers, spaces, _ ! . -");
  if (!str(hh.version, 16) || !hh.version) bad.push("hh.json: version is missing");
  if (hh.description !== undefined && !str(hh.description, 300)) bad.push("hh.json: description is over 300 characters");
  if (!content || typeof content !== "object" || Array.isArray(content)) return [...bad, "content.json is missing or not an object"];
  for (const k of Object.keys(content)) if (!["intro", "rules", "characters"].includes(k)) bad.push(`content.json: unknown key "${k}"`);
  if (content.intro !== undefined && !str(content.intro, 200)) bad.push("intro: over 200 characters");
  if (content.rules !== undefined) {
    if (typeof content.rules !== "object" || Array.isArray(content.rules)) bad.push("rules must be an object");
    else for (const [k, v] of Object.entries(content.rules)) {
      if (!RULES[k]) bad.push(`rules: unknown rule "${k}"`);
      else if (typeof v !== "number" || v < RULES[k][0] || v > RULES[k][1]) bad.push(`rules.${k} must be a number from ${RULES[k][0]} to ${RULES[k][1]}`);
    }
    const r = content.rules || {};
    if (r.tornado_min && r.tornado_max && r.tornado_max < r.tornado_min) bad.push("rules: tornado_max is smaller than tornado_min");
  }
  if (content.characters !== undefined) {
    if (!Array.isArray(content.characters) || content.characters.length > 12) bad.push("characters: a list of at most 12");
    else content.characters.forEach((c, i) => {
      const at = `characters[${i}]`;
      if (!c || typeof c !== "object") return bad.push(`${at} is not an object`);
      for (const k of Object.keys(c)) if (!["name", "color", "personality", "infected", "map", "x", "y"].includes(k)) bad.push(`${at}: unknown key "${k}"`);
      if (!str(c.name, 16) || !/^[A-Za-z0-9 ._-]{1,16}$/.test(c.name)) bad.push(`${at}.name: 1-16 letters, numbers, spaces . _ -`);
      if (!/^#[0-9a-fA-F]{6}$/.test(String(c.color))) bad.push(`${at}.color: like "#ff8800"`);
      if (!str(c.personality, 200) || !c.personality) bad.push(`${at}.personality: 1-200 characters`);
      if (typeof c.infected !== "boolean") bad.push(`${at}.infected: true or false`);
      if (![0, 1, 2].includes(c.map)) bad.push(`${at}.map: 0 (house), 1 (beach) or 2 (pool)`);
      if (typeof c.x !== "number" || c.x < 1 || c.x > 28 || typeof c.y !== "number" || c.y < 1 || c.y > 16) bad.push(`${at}: x 1-28, y 1-16`);
    });
  }
  return bad;
}
