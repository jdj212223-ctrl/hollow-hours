//! The world: the built-in maps (as text, one letter per tile), the maps of the game being played, and the
//! helpers to read them (tiles, walls, doors). Split out of main.rs in 4.0 (the big revamp).

use super::*;

// ---------------- Maps ----------------

pub(crate) struct Map {
    pub(crate) name: &'static str,
    pub(crate) rows: [&'static str; MH],
    /// 0 = bright, 1 = pitch black (when the Dark phobia is on)
    pub(crate) dark: f32,
    /// how "empty and wrong" it feels
    pub(crate) liminal: f32,
    pub(crate) floor: Color,
}

/// The three built-in maps. Mods can add to them or replace them (see MapData).
pub(crate) const BUILTIN: [Map; 12] = [
    Map {
        name: "The cozy house",
        rows: [
            "##############################",
            "#ss.kkkf.m...#..cccccc....p..#",
            "#............#...rrrrrr..d...#",
            "#........bb..#...rrrrrr......#",
            "#..p.....bb..#...rrrrrr.bbb..#",
            "#.....L......................#",
            "#p...........#..............p#",
            "######.#######.......L.......#",
            "#............#####.###########",
            "#rrrrrrr.....#...............#",
            "P..d.........#..p.........p..#",
            "#............................#",
            "#....ccc.....#.......bb......#",
            "#....bbb.....#.......bb......B",
            "#............#...............B",
            "#p....L......#.....d.....z...#",
            "#k.........rS#...z..........K#",
            "##############################",
        ],
        dark: 0.35,
        liminal: 0.1,
        floor: Color::new(0.55, 0.38, 0.25, 1.0),
    },
    Map {
        name: "The beach at 3 AM",
        rows: [
            "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~",
            "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~",
            "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~",
            "~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~",
            ",,~~,,,,,,,~~~,,,,,,,,,~~,,,,,",
            ",,e,,,,,,,,,,,,,,,,,,,,,,,e,,,",
            ",,,,,L,,,,,,,,u,,,,,,,,,L,,,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,,,,,,u,,,d,,,,,,,,u,,,,,,,,",
            "A,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            "A,,,,,,,,,,,,,,,,,,,,,,,e,,,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,,e,,,,,,,,,,u,,,,,,,,,,,,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,C",
            ",,,,,,,,,,e,,,,,,,,,,,,,,,,,,C",
            ",,,,,,,,,,,,,,rr,,,,,,,,,,,,,,",
            "##############MM##############",
        ],
        dark: 0.75,
        liminal: 0.65,
        floor: Color::new(0.72, 0.65, 0.48, 1.0),
    },
    Map {
        name: "The empty pool",
        rows: [
            "##############################",
            "#p..q.....q.....q.....q.....p#",
            "#..c.c.c................c.c..#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "B............................#",
            "B............................#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "#....~~~~~~~~~~~~~~~~~~~~....#",
            "#.............L..............#",
            "#..........d.........c.c.c...#",
            "#p..q.....q.....q.....q.....p#",
            "##############################",
        ],
        dark: 0.45,
        liminal: 0.95,
        floor: Color::new(0.78, 0.86, 0.85, 1.0),
    },
    Map {
        name: "The Marketplace",
        rows: [
            "##############MM##############",
            "#v...........................#",
            "#.hhhhh..hhhh....hhhh..hhh.ff#",
            "#............................#",
            "#.hhhhh..hhhh....hhhh..hhh.d.#",
            "#............................#",
            "#.hhhhh..hhhh....hhhh..hhh.ff#",
            "#............................X",
            "#............................X",
            "#.hhhhh..hhhh....hhhh..hhh...#",
            "#............................#",
            "#.hhhhh..hhhh....hhhh..hhh.p.#",
            "#............................#",
            "#..kk...kk...kk...kk......bb.#",
            "#..L.......L.......L......bb.#",
            "#v...........................#",
            "#............................#",
            "##############################",
        ],
        dark: 0.55,
        liminal: 0.8,
        floor: Color::new(0.62, 0.64, 0.6, 1.0),
    },
    Map {
        name: "The Park",
        rows: [
            "******************************",
            "*,,,,,,,,,,,,,,,,,,,,,,,,,,,,*",
            "*,,*,,,,,,,,,,,,,,,,,,,,*,,,,*",
            "*,,,,,,,,L,,,,rr,,,,L,,,,,,,,*",
            "*,,,,,,,,,,,,,rr,,,,,,,,,,,,,*",
            "*,,,,,*,,,,,,,rr,,,,,,,,*,,,,*",
            "*,,,,,,,,,,,,nnnn,,,,,,,,,,,,*",
            "*,,,,,,,,,,,,,rr,,,,,,,,,,,,,*",
            "*,,*,,,,,,,,,,rrrrrrrrrrrrrrrH",
            "*,,,,jjj,,,,,Lrr,,,,,,,,,*,,,H",
            "*,,,,jjj,*,,,,rr,,,,,,,,,,,,,*",
            "*,,,,,,,,,,,,,rr,,,,,,,,e,,,,*",
            "*,,,,,,,,,,,,,rr,,,,,*,,,,,,,*",
            "*,,*,,,,,,,,,,rr,,,,,,,,,,,*,*",
            "*,,,,,,,e,,,,,rr,,,,,,,,,,,,,*",
            "*,,,,,,,*,,,,,rr,,,,,,,*,,,,,*",
            "*,,,,,,,,,,,,,rr,,,,,,,,,,,,,*",
            "**************UU**************",
        ],
        dark: 0.7,
        liminal: 0.55,
        floor: Color::new(0.22, 0.36, 0.2, 1.0),
    },
    Map {
        name: "The Helix Lab",
        rows: [
            "#######Z######################",
            "#t.t.t........#.ii.....t.t.t.#",
            "#.............#..............#",
            "#..kkk...kkk..#..kkk....kkk..#",
            "#.............#..............#",
            "#..x.........................#",
            "######.########.######.#######",
            "#.....q..............q.......#",
            "X............ooo.............#",
            "X............ooo........x....#",
            "#............ooo.............#",
            "#.....q..............q.......#",
            "#..t..........d.........t....#",
            "#..kk...................kk..v#",
            "#..kk.......x............kk..#",
            "#...p....................p...#",
            "#.t....i.i.........i.i.....t.#",
            "##############################",
        ],
        dark: 0.5,
        liminal: 0.85,
        floor: Color::new(0.82, 0.85, 0.86, 1.0),
    },
    Map {
        name: "The Subway",
        rows: [
            "##############UU##############",
            "#v...........................#",
            "#...L.......L.......L.....L..#",
            "#............................#",
            "#..bb.....t........bb.....d..#",
            "#....q.......q.......q.......#",
            "#............................#",
            "#............................#",
            "#rrrrrrrrrrrrrrrrrrrrrrrrrrrr#",
            "#..===========================",
            "#..===========================",
            "#rrrrrrrrrrrrrrrrrrrrrrrrrrrr#",
            "#.bb..bb..bb..bb..bb..bb..bb.#",
            "#....q.......q.......q.......#",
            "#............................#",
            "#.bb..bb..bb..bb..bb..bb..bb.#",
            "#v.........t...............v.#",
            "##############################",
        ],
        dark: 0.8,
        liminal: 0.7,
        floor: Color::new(0.4, 0.4, 0.42, 1.0),
    },
    Map {
        name: "The cozy house: upstairs",
        rows: [
            "##############################",
            "#..yy...p....#rrrrrr.......T.#",
            "#..yy........#......bbb......#",
            "#rrrr........#..c...bbb......#",
            "#rrrr.L......#...............#",
            "#............#.......L......p#",
            "######.###########.###########",
            "#p..........................p#",
            "#...........rrrrrr...........#",
            "######.###########.###########",
            "#............#...............#",
            "#..d.........#....bb.....kk..#",
            "#............#....bb.........#",
            "#............................#",
            "#..bb........#.z.............#",
            "#..bb........#.......L.......#",
            "#...........S#..............p#",
            "##############################",
        ],
        dark: 0.4,
        liminal: 0.2,
        floor: Color::new(0.5, 0.36, 0.26, 1.0),
    },
    Map {
        name: "The cozy house: basement",
        rows: [
            "##############################",
            "#.bb.bb.bb.bb.bb.bb......zz..#",
            "#............................#",
            "#.bb.bb.bb.bb.bb.bb...ggg..z.#",
            "#.....................ggg....#",
            "#.......................x....#",
            "#..L.........L.........L.....#",
            "#............................#",
            "#.bb.bb.bb.bb.....bb.bb.bb...#",
            "#............................#",
            "#..........zz................#",
            "#.bb.bb.bb.bb.....bb.bb.bb...#",
            "#............................#",
            "#.....d..........z...........#",
            "#..zz........................#",
            "#..zz.....x..................#",
            "#..........................K.#",
            "##############################",
        ],
        dark: 0.75,
        liminal: 0.5,
        floor: Color::new(0.3, 0.3, 0.32, 1.0),
    },
    Map {
        name: "The cozy house: attic",
        rows: [
            "##############################",
            "#.zz.zz....zz.......w........#",
            "#............................#",
            "#..d......bbb.......bbb......#",
            "#.........bbb.......bbb......#",
            "#....L..................L....#",
            "#....x.......................#",
            "#.zz.....bb.....zz.....bb....#",
            "#............................#",
            "#..........c.................#",
            "#.bb.....zz.....bb.....zz....#",
            "#............................#",
            "#....L.........d........L....#",
            "#...................x........#",
            "#..zzz...................bbb.#",
            "#............................#",
            "#.T..........................#",
            "##############################",
        ],
        dark: 0.85,
        liminal: 0.6,
        floor: Color::new(0.42, 0.33, 0.24, 1.0),
    },
    Map {
        name: "Helix Tower",
        rows: [
            "##############################",
            "#q...........aaaa...........q#",
            "#..o.o.......aaaa.......o.o..#",
            "#............................#",
            "#.t.t...p...........p..t.t...#",
            "#####.######.....#######.#####",
            "#............................#",
            "#..ii..ii..ii....ii..ii..ii..#",
            "#............................#",
            "#..ii..ii..ii....ii..ii..ii..#",
            "#............................#",
            "######.####...L...####.#######",
            "#............................#",
            "#..L......t..q....q..t...L...#",
            "#............................#",
            "#p....kkk............kkk....p#",
            "#............................#",
            "##############Z###############",
        ],
        dark: 0.75,
        liminal: 0.9,
        floor: Color::new(0.1, 0.1, 0.13, 1.0),
    },
    Map {
        name: "The Town",
        rows: [
            "######,,,,,,,,MM,,,,,,,,######",
            "######,,,,,,,,,,,,,,,,,,######",
            "######,,L,,,,,,,,,,,,L,,######",
            "######,,,,,,,,,,,,,,,,,,######",
            "######,,,zz,,,,,,,,,e,,,######",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,rrrrrrrrrrrrrrrrrrrrrrrr,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,q,,,,,,,,,,,,,,,,,,,,,,,,q,,",
            ",,,,,,,v,,,,,,,,,,,,,,,,,,,,,,",
            ",,,rrrrrrrrrrrrrrrrrrrrrrrr,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            "######,,,,e,,,,,,,,,,,,,######",
            "######,,L,,,,,,,,,,,,L,,######",
            "######,,,,,,,,,,,,,,zz,,######",
            "######,,,,,,,,,,,,,,,,,,######",
            "######,,,,,,,,,,,,,,,,,,######",
            "######,,,,,,,,NN,,,,,,,,######",
        ],
        dark: 0.65,
        liminal: 0.6,
        floor: Color::new(0.17, 0.17, 0.19, 1.0),
    },
];

/// A map as the game uses it: the built-in ones, or one from a .hh mod.
#[derive(Clone)]
pub(crate) struct MapData {
    pub(crate) name: String,
    pub(crate) rows: Vec<Vec<u8>>,
    pub(crate) dark: f32,
    pub(crate) liminal: f32,
    pub(crate) floor: Color,
    pub(crate) wall: Color,
    /// Door letter → (map it leads to, where you arrive in tiles)
    pub(crate) doors: Vec<(char, usize, Vec2)>,
    /// The empty pool look for its water tiles.
    pub(crate) drained: bool,
}

pub(crate) fn builtin_maps() -> Vec<MapData> {
    let doors: [Vec<(char, usize, Vec2)>; 12] = [
        vec![('B', 1, vec2(1.5, 9.5)), ('P', 4, vec2(27.5, 8.5)), ('S', 7, vec2(11.5, 15.5)), ('K', 8, vec2(26.5, 15.5))],
        vec![('A', 0, vec2(28.5, 13.5)), ('C', 2, vec2(1.5, 8.5)), ('M', 11, vec2(14.5, 1.5))],
        vec![('B', 1, vec2(28.5, 14.5))],
        vec![('M', 11, vec2(14.5, 15.5)), ('X', 5, vec2(1.5, 8.5))],
        vec![('H', 0, vec2(1.5, 10.5)), ('U', 6, vec2(14.5, 1.5))],
        vec![('X', 3, vec2(27.5, 8.5)), ('Z', 10, vec2(14.5, 15.5))],
        vec![('U', 4, vec2(14.5, 15.5))],
        vec![('S', 0, vec2(11.5, 15.5)), ('T', 9, vec2(3.5, 15.5))],
        vec![('K', 0, vec2(27.5, 15.5))],
        vec![('T', 7, vec2(26.5, 1.5))],
        vec![('Z', 5, vec2(7.5, 1.5))],
        vec![('M', 1, vec2(14.5, 15.5)), ('N', 3, vec2(14.5, 1.5))],
    ];
    BUILTIN.iter().zip(doors).enumerate().map(|(i, (m, d))| MapData {
        name: m.name.into(), rows: m.rows.iter().map(|r| r.as_bytes().to_vec()).collect(), dark: m.dark, liminal: m.liminal,
        floor: m.floor, wall: if m.name == "Helix Tower" { Color::new(0.3, 0.1, 0.12, 1.0) } else { Color::new(0.22, 0.2, 0.24, 1.0) }, doors: d, drained: i == 2,
    }).collect()
}

thread_local! {
    /// The maps of the game being played (built-in, or changed by mods).
    pub(crate) static WORLD: std::cell::RefCell<Vec<MapData>> = std::cell::RefCell::new(builtin_maps());
    /// The same maps as they started (walls the infected broke come back in the next game).
    pub(crate) static PRISTINE: std::cell::RefCell<Vec<MapData>> = std::cell::RefCell::new(builtin_maps());
}

pub(crate) fn set_maps(maps: Vec<MapData>) {
    let maps = if maps.is_empty() { builtin_maps() } else { maps };
    PRISTINE.with(|p| *p.borrow_mut() = maps.clone());
    WORLD.with(|w| *w.borrow_mut() = maps);
}
/// Undo everything that changed in the maps last game (broken walls).
pub(crate) fn reset_world() {
    let maps = PRISTINE.with(|p| p.borrow().clone());
    WORLD.with(|w| *w.borrow_mut() = maps);
}
/// Changes one tile (a broken wall).
pub(crate) fn set_tile(m: usize, x: i32, y: i32, c: u8) {
    if x < 0 || y < 0 || x >= MW as i32 || y >= MH as i32 { return; }
    WORLD.with(|w| if let Some(d) = w.borrow_mut().get_mut(m) { d.rows[y as usize][x as usize] = c; });
}
pub(crate) fn maps_len() -> usize { WORLD.with(|w| w.borrow().len()) }
pub(crate) fn with_map<R>(m: usize, f: impl FnOnce(&MapData) -> R) -> R {
    WORLD.with(|w| { let w = w.borrow(); f(&w[m.min(w.len() - 1)]) })
}
pub(crate) fn map_name(m: usize) -> String { with_map(m, |d| d.name.clone()) }
pub(crate) fn map_dark(m: usize) -> f32 { with_map(m, |d| d.dark) }
pub(crate) fn map_liminal(m: usize) -> f32 { with_map(m, |d| d.liminal) }
pub(crate) fn map_floor(m: usize) -> Color { with_map(m, |d| d.floor) }
pub(crate) fn map_wall(m: usize) -> Color { with_map(m, |d| d.wall) }
pub(crate) fn map_drained(m: usize) -> bool { with_map(m, |d| d.drained) }
pub(crate) fn map_has_water(m: usize) -> bool { with_map(m, |d| d.rows.iter().any(|r| r.contains(&b'~'))) }

pub(crate) fn tile(m: usize, x: i32, y: i32) -> char {
    if x < 0 || y < 0 || x >= MW as i32 || y >= MH as i32 {
        return '#';
    }
    WORLD.with(|w| w.borrow().get(m).map(|d| d.rows[y as usize][x as usize] as char).unwrap_or('#'))
}

pub(crate) fn solid(m: usize, x: f32, y: f32) -> bool {
    let c = tile(m, (x / T).floor() as i32, (y / T).floor() as i32);
    blocks(c) || (c == 'Z' && !super::story::tower_open())
}

/// Tiles you can't walk through: walls, water, furniture. 4.0 furniture: c couch, k counter, f fridge, p plant,
/// h shelf, v vending machine, q pillar, u beach umbrella, e rock, j pond, i server rack, z boxes.
/// (r rug / path and x debris are floor.)
pub(crate) fn blocks(c: char) -> bool {
    matches!(c, '#' | 'b' | '~' | 'm' | 's' | '*' | 'n' | 't' | 'o' | '=' | 'y' | 'g' | 'w' | 'a'
        | 'c' | 'k' | 'f' | 'p' | 'h' | 'v' | 'q' | 'u' | 'e' | 'j' | 'i' | 'z')
}

/// Door letter → (map, where you arrive)
pub(crate) fn door(c: char, from: usize) -> Option<(usize, Vec2)> {
    if !c.is_ascii_uppercase() || (c == 'Z' && !super::story::tower_open()) { return None; }
    with_map(from, |d| d.doors.iter().find(|(l, _, _)| *l == c).map(|(_, to, at)| (*to, *at)))
}


#[cfg(test)]
mod tests {
    use super::*;

    fn open(m: usize, x: f32, y: f32) -> bool { !solid(m, x * T, y * T) }

    /// The 4.0 maps: every floor tile and every door can be reached, and every fixed spot (where characters start,
    /// where doors drop you, the Mac Studio and stove spots...) is on open floor.
    #[test]
    fn every_map_is_walkable_and_every_spot_is_free() {
        set_maps(vec![]);
        super::super::story::set_tower(true);
        for m in 0..maps_len() {
            let name = map_name(m);
            // Flood fill from the first place a door drops you.
            let start = (0..maps_len()).flat_map(|f| with_map(f, |d| d.doors.clone())).find(|(_, to, _)| *to == m).map(|(_, _, at)| at).unwrap();
            let mut seen = vec![vec![false; MW]; MH];
            let mut todo = vec![(start.x as i32, start.y as i32)];
            while let Some((x, y)) = todo.pop() {
                if x < 0 || y < 0 || x >= MW as i32 || y >= MH as i32 || seen[y as usize][x as usize] || blocks(tile(m, x, y)) { continue; }
                seen[y as usize][x as usize] = true;
                todo.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
            }
            for y in 0..MH { for x in 0..MW {
                let c = tile(m, x as i32, y as i32);
                assert!(blocks(c) || seen[y][x], "{name}: tile ({x},{y}) '{c}' can't be reached");
            } }
            for (_, to, at) in with_map(m, |d| d.doors.clone()) {
                assert!(open(to, at.x, at.y), "a door from {name} drops you inside something in {}", map_name(to));
            }
        }
        for c in characters().iter().chain(kind_characters().iter()) {
            assert!(open(c.map, c.pos.x / T, c.pos.y / T), "{} starts inside something in {}", c.name, map_name(c.map));
        }
        let spots = [(0, 4.5, 5.0), (0, 9.5, 2.5), (0, 2.5, 2.6), (0, 10.3, 16.5), (0, 26.0, 16.5), (0, 2.6, 10.5), (0, 27.5, 13.5), (0, 2.5, 9.5), (0, 20.5, 5.5)];
        for (m, x, y) in spots { assert!(open(m, x, y), "spot ({x},{y}) on {} is blocked", map_name(m)); }
        // The things the game looks for are still there.
        for (m, c) in [(0, 'm'), (0, 's'), (7, 'y'), (8, 'g'), (9, 'w'), (4, 'n'), (5, 't'), (5, 'o'), (6, 't'), (10, 'a')] {
            assert!(with_map(m, |d| d.rows.iter().any(|r| r.contains(&(c as u8)))), "{} lost its '{c}'", map_name(m));
        }
        super::super::story::set_tower(false);
    }
}

/// A safe spot near `p` on map `m`: a floor tile you can actually reach from one of the map's doors.
/// Old saves can put someone inside a wall or outside the map's outer wall (maps changed between versions);
/// this moves them to the nearest reachable floor instead.
pub(crate) fn safe_spot(m: usize, p: Vec2) -> Vec2 {
    let free = |x: i32, y: i32| { let c = tile(m, x, y); !blocks(c) && !(c == 'Z' && !super::story::tower_open()) };
    let (w, h) = (MW as i32, MH as i32);
    let mut seen = vec![false; (w * h) as usize];
    let mut q = std::collections::VecDeque::new();
    for y in 0..h { for x in 0..w {
        if door(tile(m, x, y), m).is_some() { seen[(y * w + x) as usize] = true; q.push_back((x, y)); }
    } }
    if q.is_empty() { // a map with no doors: any free tile counts
        for y in 0..h { for x in 0..w { if free(x, y) { seen[(y * w + x) as usize] = true; } } }
    }
    while let Some((x, y)) = q.pop_front() {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= w || ny >= h { continue; }
            let i = (ny * w + nx) as usize;
            if !seen[i] && free(nx, ny) { seen[i] = true; q.push_back((nx, ny)); }
        }
    }
    let (px, py) = ((p.x / T).floor() as i32, (p.y / T).floor() as i32);
    let ok = |x: i32, y: i32| x >= 0 && y >= 0 && x < w && y < h && seen[(y * w + x) as usize] && free(x, y);
    if ok(px, py) { return p; }
    let mut best: Option<(i32, i32, i32)> = None;
    for y in 0..h { for x in 0..w {
        if ok(x, y) { let d = (x - px).pow(2) + (y - py).pow(2); if best.map_or(true, |b| d < b.2) { best = Some((x, y, d)); } }
    } }
    best.map(|(x, y, _)| vec2(x as f32 + 0.5, y as f32 + 0.5) * T).unwrap_or(p)
}

#[cfg(test)]
mod safe_spot_tests {
    use super::*;

    #[test]
    fn old_save_spots_outside_the_walls_come_back_inside() {
        set_maps(vec![]);
        for m in 0..maps_len() {
            // Every map's corner is outer wall (or beyond it): whoever was saved there lands on reachable floor.
            for p in [vec2(0.5, 0.5) * T, vec2(-3.0, -3.0) * T, vec2(MW as f32 + 2.0, MH as f32 + 2.0) * T] {
                let s = safe_spot(m, p);
                assert!(!solid(m, s.x, s.y), "map {m}: {p:?} -> {s:?} is solid");
            }
        }
        // Twin's spot from the save that came from an older version (map 1).
        let s = safe_spot(1, vec2(11.63, 2.29) * T);
        assert!(!solid(1, s.x, s.y));
        eprintln!("Twin's old spot (11.6, 2.3) on map 1 -> ({:.1}, {:.1}); tile there was {:?}", s.x / T, s.y / T, tile(1, 11, 2));
    }
}
