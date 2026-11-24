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
pub(crate) const BUILTIN: [Map; 11] = [
    Map {
        name: "The cozy house",
        rows: [
            "##############################",
            "#ss......m...#...............#",
            "#............#.......d.......#",
            "#........bb..#...............#",
            "#........bb..#.......bbbb....#",
            "#.....L......................#",
            "#............#...............#",
            "######.#######.......L.......#",
            "#............#####.###########",
            "#............#...............#",
            "P..d.........#...............#",
            "#............................#",
            "#....bbb.....#.......bb......#",
            "#....bbb.....#.......bb......B",
            "#............#...............B",
            "#.....L......#.....d.........#",
            "#...........S#..............K#",
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
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,,,L,,,,,,,,,,,,,,,,,,L,,,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,,,,,,,,,d,,,,,,,,,,,,,,,,,,",
            "A,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            "A,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,C",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,C",
            ",,,,,,,,,,,,,,,,,,,,,,,,,,,,,,",
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
            "#............................#",
            "#............................#",
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
            "#..........d.................#",
            "#............................#",
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
            "#............................#",
            "#.bbbbbb..bbbbbb..bbbbbb.....#",
            "#............................#",
            "#.bbbbbb..bbbbbb..bbbbbb..d..#",
            "#............................#",
            "#.bbbbbb..bbbbbb..bbbbbb.....#",
            "#............................X",
            "#............................X",
            "#.bbbbbb..bbbbbb..bbbbbb.....#",
            "#............................#",
            "#.bbbbbb..bbbbbb..bbbbbb.....#",
            "#............................#",
            "#.........................bb.#",
            "#..L.......L.......L......bb.#",
            "#............................#",
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
            "*,,,,,,,,L,,,,,,,,,,L,,,,,,,,*",
            "*,,,,,,,,,,,,,,,,,,,,,,,,,,,,*",
            "*,,,,,*,,,,,,,,,,,,,,,,,*,,,,*",
            "*,,,,,,,,,,,,nnnn,,,,,,,,,,,,*",
            "*,,,,,,,,,,,,,,,,,,,,,,,,,,,,*",
            "*,,*,,,,,,,,,,,,,,,,,,,,,,,,,H",
            "*,,,,,,,,,,,,L,,,,,,,,,,,*,,,H",
            "*,,,,,,*,,,,,,,,,,,,,,,,,,,,,*",
            "*,,,,,,,,,,,,,,,,,,,,,,,,,,,,*",
            "*,,,,,,,,,,,,,,,,,,,*,,,,,,,,*",
            "*,,*,,,,,,,,,,,,,,,,,,,,,,,*,*",
            "*,,,,,,,,,,,,,,,,,,,,,,,,,,,,*",
            "*,,,,,,,*,,,,,,,,,,,,,,*,,,,,*",
            "*,,,,,,,,,,,,,,,,,,,,,,,,,,,,*",
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
            "#t.t.t........#........t.t.t.#",
            "#.............#..............#",
            "#..bbb...bbb..#..bbb....bbb..#",
            "#.............#..............#",
            "#............................#",
            "######.########.######.#######",
            "#............................#",
            "X............ooo.............#",
            "X............ooo.............#",
            "#............ooo.............#",
            "#............................#",
            "#..t..........d.........t....#",
            "#..bb....................bb..#",
            "#..bb....................bb..#",
            "#............................#",
            "#.t........................t.#",
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
            "#............................#",
            "#...L.......L.......L.....L..#",
            "#............................#",
            "#..bb.....t........bb.....d..#",
            "#............................#",
            "#............................#",
            "#............................#",
            "#............................#",
            "#..===========================",
            "#..===========================",
            "#............................#",
            "#.bb..bb..bb..bb..bb..bb..bb.#",
            "#............................#",
            "#............................#",
            "#.bb..bb..bb..bb..bb..bb..bb.#",
            "#..........t.................#",
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
            "#..yy........#.............T.#",
            "#..yy........#......bbb......#",
            "#............#......bbb......#",
            "#.....L......#...............#",
            "#............#.......L.......#",
            "######.###########.###########",
            "#............................#",
            "#............................#",
            "######.###########.###########",
            "#............#...............#",
            "#..d.........#....bb.........#",
            "#............#....bb.........#",
            "#............................#",
            "#..bb........#...............#",
            "#..bb........#.......L.......#",
            "#...........S#...............#",
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
            "#.bb.bb.bb.bb.bb.bb..........#",
            "#............................#",
            "#.bb.bb.bb.bb.bb.bb...ggg....#",
            "#.....................ggg....#",
            "#............................#",
            "#..L.........L.........L.....#",
            "#............................#",
            "#.bb.bb.bb.bb.....bb.bb.bb...#",
            "#............................#",
            "#............................#",
            "#.bb.bb.bb.bb.....bb.bb.bb...#",
            "#............................#",
            "#.....d......................#",
            "#............................#",
            "#............................#",
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
            "#.bb.bb....bb.......w........#",
            "#............................#",
            "#..d......bbb.......bbb......#",
            "#.........bbb.......bbb......#",
            "#....L..................L....#",
            "#............................#",
            "#.bb.....bb.....bb.....bb....#",
            "#............................#",
            "#............................#",
            "#.bb.....bb.....bb.....bb....#",
            "#............................#",
            "#....L.........d........L....#",
            "#............................#",
            "#..bbb...................bbb.#",
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
            "#............aaaa............#",
            "#..o.o.......aaaa.......o.o..#",
            "#............................#",
            "#.t.t..................t.t...#",
            "#####.######.....#######.#####",
            "#............................#",
            "#..bb..bb..bb....bb..bb..bb..#",
            "#............................#",
            "#..bb..bb..bb....bb..bb..bb..#",
            "#............................#",
            "######.####...L...####.#######",
            "#............................#",
            "#..L......t.......t......L...#",
            "#............................#",
            "#.....bbb............bbb.....#",
            "#............................#",
            "##############Z###############",
        ],
        dark: 0.75,
        liminal: 0.9,
        floor: Color::new(0.1, 0.1, 0.13, 1.0),
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
    let doors: [Vec<(char, usize, Vec2)>; 11] = [
        vec![('B', 1, vec2(1.5, 9.5)), ('P', 4, vec2(27.5, 8.5)), ('S', 7, vec2(11.5, 15.5)), ('K', 8, vec2(26.5, 15.5))],
        vec![('A', 0, vec2(28.5, 13.5)), ('C', 2, vec2(1.5, 8.5)), ('M', 3, vec2(14.5, 1.5))],
        vec![('B', 1, vec2(28.5, 14.5))],
        vec![('M', 1, vec2(14.5, 15.5)), ('X', 5, vec2(1.5, 8.5))],
        vec![('H', 0, vec2(1.5, 10.5)), ('U', 6, vec2(14.5, 1.5))],
        vec![('X', 3, vec2(27.5, 8.5)), ('Z', 10, vec2(14.5, 15.5))],
        vec![('U', 4, vec2(14.5, 15.5))],
        vec![('S', 0, vec2(11.5, 15.5)), ('T', 9, vec2(3.5, 15.5))],
        vec![('K', 0, vec2(27.5, 15.5))],
        vec![('T', 7, vec2(26.5, 1.5))],
        vec![('Z', 5, vec2(7.5, 1.5))],
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
    match tile(m, (x / T).floor() as i32, (y / T).floor() as i32) {
        '#' | 'b' | '~' | 'm' | 's' | '*' | 'n' | 't' | 'o' | '=' | 'y' | 'g' | 'w' | 'a' => true,
        'Z' => !super::story::tower_open(),
        _ => false,
    }
}

/// Door letter → (map, where you arrive)
pub(crate) fn door(c: char, from: usize) -> Option<(usize, Vec2)> {
    if !c.is_ascii_uppercase() || (c == 'Z' && !super::story::tower_open()) { return None; }
    with_map(from, |d| d.doors.iter().find(|(l, _, _)| *l == c).map(|(_, to, at)| (*to, *at)))
}

