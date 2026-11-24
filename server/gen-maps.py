#!/usr/bin/env python3
"""Copies the game's built-in maps (src/world.rs) into the website (server/src/maps.js), for the Places page."""
import json, re, pathlib
root = pathlib.Path(__file__).resolve().parent.parent
s = (root / "src/world.rs").read_text()
maps = [{"name": m.group(1), "rows": re.findall(r'"([^"]*)"', m.group(2))} for m in re.finditer(r'name: "([^"]+)",\s*rows: \[(.*?)\]', s, re.S)]
out = "// Made by server/gen-maps.py from the game's src/world.rs: don't edit by hand.\nexport const MAPS = " + json.dumps(maps, indent=0) + ";\n"
(root / "server/src/maps.js").write_text(out)
print(len(maps), "maps")
