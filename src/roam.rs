//! The Claude family's own lives (Infection mode). Instead of trailing behind you, each kind AI has a task:
//!  - Follow: stay close and guard you (there's always at least one bodyguard).
//!  - Roam: walk around the house and the other places, checking on things.
//!  - Forage: go and find food or water anywhere (walking through doors), pick it up and carry it back to you.
//! They find their way with a path search over the tiles and the doors between maps, run from the infected, and
//! report over the radio ("(radio) Haiku 4.5: green eyes on the beach").

use super::*;

#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Task {
    #[default]
    Follow,
    Roam,
    Forage,
}

impl Task {
    pub fn name(self) -> &'static str {
        match self { Task::Follow => "guarding you", Task::Roam => "roaming", Task::Forage => "looking for food and water" }
    }
}

fn walkable(m: usize, x: i32, y: i32) -> bool {
    !matches!(tile(m, x, y), '#' | 'b' | '~' | 'm')
}

fn tile_of(p: Vec2) -> (i32, i32) { ((p.x / T) as i32, (p.y / T) as i32) }
fn center(t: (i32, i32)) -> Vec2 { vec2(t.0 as f32 + 0.5, t.1 as f32 + 0.5) * T }

/// The next tile to step on from `from` toward `to` on map `m` (a breadth-first search over the tiles), or None
/// if `to` can't be reached.
pub fn next_step(m: usize, from: (i32, i32), to: (i32, i32)) -> Option<(i32, i32)> {
    if from == to { return Some(to); }
    let idx = |t: (i32, i32)| (t.1 * MW as i32 + t.0) as usize;
    let mut came = vec![u16::MAX; MW * MH];
    let mut queue = std::collections::VecDeque::new();
    came[idx(from)] = idx(from) as u16;
    queue.push_back(from);
    while let Some(cur) = queue.pop_front() {
        if cur == to {
            // Walk back to the step right after `from`.
            let mut t = cur;
            loop {
                let p = came[idx(t)] as usize;
                let prev = ((p % MW) as i32, (p / MW) as i32);
                if prev == from { return Some(t); }
                t = prev;
            }
        }
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            let n = (cur.0 + dx, cur.1 + dy);
            if n.0 < 0 || n.1 < 0 || n.0 >= MW as i32 || n.1 >= MH as i32 { continue; }
            if came[idx(n)] != u16::MAX || !(walkable(m, n.0, n.1) || n == to) { continue; }
            came[idx(n)] = idx(cur) as u16;
            queue.push_back(n);
        }
    }
    None
}

/// The next map on the way from `from` to `to` (through the doors), or None if there's no way.
fn next_map(from: usize, to: usize) -> Option<usize> {
    if from == to { return Some(to); }
    let n = maps_len();
    let mut came = vec![usize::MAX; n];
    came[from] = from;
    let mut queue = std::collections::VecDeque::from([from]);
    while let Some(m) = queue.pop_front() {
        if m == to {
            let mut t = m;
            while came[t] != from { t = came[t]; }
            return Some(t);
        }
        let exits: Vec<usize> = with_map(m, |d| d.doors.iter().map(|x| x.1).collect());
        for e in exits {
            if e < n && came[e] == usize::MAX { came[e] = m; queue.push_back(e); }
        }
    }
    None
}

/// The door tile on map `m` that leads to map `to`, nearest to `near`.
fn door_to(m: usize, to: usize, near: (i32, i32)) -> Option<(i32, i32)> {
    let mut best: Option<((i32, i32), i32)> = None;
    for y in 0..MH as i32 {
        for x in 0..MW as i32 {
            let c = tile(m, x, y);
            if door(c, m).map(|d| d.0) == Some(to) {
                let d = (x - near.0).abs() + (y - near.1).abs();
                if best.map_or(true, |b| d < b.1) { best = Some(((x, y), d)); }
            }
        }
    }
    best.map(|b| b.0)
}

/// Points character `c` at its goal (map, spot): the next tile on the way, through doors if it's on another map.
/// Walking onto the right door takes it to the next map. False if there's no way there.
fn walk_toward(c: &mut Character, goal_map: usize, goal: Vec2) -> bool {
    let here = tile_of(c.pos);
    let target = if goal_map == c.map {
        tile_of(goal)
    } else {
        let Some(next) = next_map(c.map, goal_map) else { return false };
        let Some(d) = door_to(c.map, next, here) else { return false };
        if here == d {
            // Through the door.
            if let Some((m, at)) = door(tile(c.map, d.0, d.1), c.map) {
                c.map = m;
                c.pos = at * T;
                c.target = c.pos;
            }
            return true;
        }
        d
    };
    match next_step(c.map, here, target) {
        Some(t) => { c.target = if t == target && goal_map == c.map { goal } else { center(t) }; true }
        None => false,
    }
}

/// A random spot to wander to: mostly around the house, sometimes the beach or further.
fn roam_spot() -> (usize, Vec2) {
    let m = match rand::gen_range(0, 10) { 0..=4 => 0, 5..=7 => 1.min(maps_len() - 1), _ => rand::gen_range(0, maps_len()) };
    (m, random_floor(m).unwrap_or(vec2(3.5, 6.0) * T))
}

fn radio(g: &mut Game, who: &str, text: String) {
    g.log.push(format!("(radio) {who}: {text}"));
    g.sfx.radio();
}

/// Runs every kind AI for this frame (the host's game only; guests get their positions from the host).
pub fn update(g: &mut Game, dt: f32) {
    let player = (g.map, g.pos);
    let lost = g.lost_control;
    let low = g.food < 0.6 || g.water < 0.6;
    let mut events: Vec<(String, String)> = vec![];
    let mut delivered: Vec<(bool, &'static str)> = vec![];
    let mut taken: Vec<usize> = vec![];
    // Infected (and you, once you've turned) are dangerous: (map, where).
    let threats: Vec<(usize, Vec2)> = g.chars.iter().filter(|c| c.infected && c.stun <= 0.0).map(|c| (c.map, c.pos))
        .chain(lost.then_some(player)).collect();
    let kinds: Vec<usize> = (0..g.chars.len()).filter(|&i| g.chars[i].kind && !g.chars[i].infected && g.chars[i].shop <= 0.0).collect();
    let mut guards = kinds.iter().filter(|&&i| g.chars[i].task == Task::Follow).count();
    for (k, &i) in kinds.iter().enumerate() {
        // Choose what to do next when the current task runs out.
        g.chars[i].task_t -= dt;
        if g.chars[i].task_t <= 0.0 && g.chars[i].carry.is_none() {
            let was = g.chars[i].task;
            let roll = rand::gen_range(0.0, 1.0);
            let forage_chance = if low { 0.6 } else { 0.25 };
            let mut task = if roll < forage_chance { Task::Forage } else if roll < forage_chance + 0.35 { Task::Roam } else { Task::Follow };
            // Always keep a bodyguard with you.
            if was == Task::Follow && task != Task::Follow && guards <= 1 { task = Task::Follow; }
            if was == Task::Follow && task != Task::Follow { guards -= 1; }
            if was != Task::Follow && task == Task::Follow { guards += 1; }
            let c = &mut g.chars[i];
            c.task = task;
            c.task_t = rand::gen_range(20.0, 45.0);
            c.goal = match task {
                Task::Roam => Some(roam_spot()),
                _ => None,
            };
            if task != was && task != Task::Follow {
                events.push((c.name.to_string(), match task { Task::Forage => "Heading out to find food and water.".into(), _ => "Going to check the other rooms.".into() }));
            }
        }
        let items = &g.items;
        let c = &mut g.chars[i];
        // An infected close by (or you, once you've turned): run away from it, whatever the task.
        if let Some(t) = threats.iter().filter(|t| t.0 == c.map && t.1.distance(c.pos) < 4.0 * T).map(|t| t.1)
            .min_by(|a, b| a.distance(c.pos).partial_cmp(&b.distance(c.pos)).unwrap()) {
            c.target = c.pos + (c.pos - t).normalize_or_zero() * 4.0 * T;
            if c.spotted <= 0.0 && c.map != player.0 {
                events.push((c.name.to_string(), format!("Green eyes in {}! I'm getting away.", map_name(c.map))));
            }
            c.spotted = 20.0;
            continue;
        }
        c.spotted = (c.spotted - dt).max(0.0);
        if lost { continue; }
        match c.task {
            Task::Follow => {
                let angle = k as f32 * 1.6;
                let spot = player.1 + vec2(angle.cos(), angle.sin()) * 1.4 * T;
                if c.map != player.0 || c.pos.distance(player.1) > 2.5 * T {
                    if !walk_toward(c, player.0, spot) { c.target = c.pos; }
                } else {
                    c.target = c.pos; // close enough: stand guard
                }
            }
            Task::Roam => {
                let (gm, gp) = c.goal.unwrap_or((c.map, c.pos));
                if c.map == gm && c.pos.distance(gp) < 0.6 * T || !walk_toward(c, gm, gp) {
                    c.goal = Some(roam_spot());
                }
            }
            Task::Forage => {
                if let Some(water) = c.carry {
                    // Bring it home to you.
                    if c.map == player.0 && c.pos.distance(player.1) < 1.6 * T {
                        delivered.push((water, c.name));
                        c.carry = None;
                        c.task = Task::Follow;
                        c.task_t = rand::gen_range(10.0, 25.0);
                    } else if !walk_toward(c, player.0, player.1) {
                        c.target = c.pos;
                    }
                } else {
                    // The nearest food or water nobody else has picked (same map first).
                    let pick = items.iter().enumerate().filter(|(j, _)| !taken.contains(j))
                        .min_by(|a, b| {
                            let d = |x: &(usize, Vec2, bool)| if x.0 == c.map { x.1.distance(c.pos) } else { 1e6 + (x.0 as f32) * 1e3 };
                            d(a.1).partial_cmp(&d(b.1)).unwrap()
                        }).map(|(j, x)| (j, *x));
                    match pick {
                        Some((j, (im, ip, water))) => {
                            if c.map == im && c.pos.distance(ip) < 0.6 * T {
                                taken.push(j);
                                c.carry = Some(water);
                                events.push((c.name.to_string(), format!("Got {}. Bringing it to you.", if water { "water" } else { "food" })));
                            } else if !walk_toward(c, im, ip) {
                                c.task = Task::Roam;
                                c.goal = Some(roam_spot());
                            }
                        }
                        None => { c.task = Task::Roam; c.goal = Some(roam_spot()); }
                    }
                }
            }
        }
    }
    // Picked-up supplies leave the world (and new ones turn up somewhere else, like when you pick them up).
    taken.sort_unstable_by(|a, b| b.cmp(a));
    for j in taken {
        let (im, ip, _) = g.items.remove(j);
        if let Some(n) = &g.net { n.send(json!({"t": "pick", "map": im, "x": ip.x / T, "y": ip.y / T})); }
        let m = rand::gen_range(0, maps_len());
        if let Some(p) = random_floor(m) { g.items.push((m, p, rand::gen_range(0, 2) == 1)); }
    }
    for (water, name) in delivered {
        if water { g.water = (g.water + g.rules.supply_amount).min(1.0); } else { g.food = (g.food + g.rules.supply_amount).min(1.0); }
        g.sfx.pickup();
        g.log.push(format!("{name} brought you {}.", if water { "water" } else { "food" }));
        if let Some(c) = g.chars.iter_mut().find(|c| c.name == name) {
            c.bubble = if water { "Here, drink. Slowly.".into() } else { "Here. Eat something.".into() };
            c.via = String::new();
            c.bubble_t = 3.0;
        }
    }
    for (who, text) in events { radio(g, &who, text); }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// They find their way around walls, and through doors to other maps.
    #[test]
    fn paths_go_around_walls_and_through_doors() {
        set_maps(vec![]);
        // The cozy house: from the top-left room to the top-right room, the wall at x=13 is in the way.
        let mut t = (3, 3);
        for _ in 0..200 {
            if t == (20, 3) { break; }
            let n = next_step(0, t, (20, 3)).expect("a way through the gap");
            assert!(walkable(0, n.0, n.1), "never through a wall: {n:?}");
            t = n;
        }
        assert_eq!(t, (20, 3), "got there");
        // From the house to the Marketplace you go through the beach.
        assert_eq!(next_map(0, 3), Some(1));
        assert_eq!(next_map(1, 3), Some(3));
        let mut c = kind_characters().remove(0);
        c.map = 0;
        c.pos = vec2(3.5, 3.5) * T;
        for _ in 0..4000 {
            if c.map == 3 { break; }
            assert!(walk_toward(&mut c, 3, vec2(14.5, 5.5) * T), "always a way");
            let to = c.target - c.pos;
            if to.length() > 0.5 { c.pos += to.normalize() * (to.length().min(4.0)); }
        }
        assert_eq!(c.map, 3, "walked through the house, the beach and into the Marketplace");
    }
}
