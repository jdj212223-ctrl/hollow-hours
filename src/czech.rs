//! Czech cooking on the house's stove: ingredients from the Marketplace (or the Helix Store's grocery bag),
//! and the stove cooks the best Czech dish you can make with what you've got.

/// An ingredient: (Czech, English).
pub const ING: [(&str, &str); 13] = [
    ("hovězí", "beef"),
    ("vepřové", "pork"),
    ("brambory", "potatoes"),
    ("zelí", "sauerkraut"),
    ("mouka", "flour"),
    ("cibule", "onions"),
    ("smetana", "cream"),
    ("vejce", "eggs"),
    ("eidam", "cheese"),
    ("kapr", "carp"),
    ("houby", "mushrooms"),
    ("paprika", "paprika"),
    ("česnek", "garlic"),
];
const BEEF: usize = 0;
const PORK: usize = 1;
const POTATO: usize = 2;
const KRAUT: usize = 3;
const FLOUR: usize = 4;
const ONION: usize = 5;
const CREAM: usize = 6;
const EGGS: usize = 7;
const CHEESE: usize = 8;
const CARP: usize = 9;
const SHROOM: usize = 10;
const PAPRIKA: usize = 11;
const GARLIC: usize = 12;

pub type Pantry = [u32; 13];

pub struct Dish {
    pub name: &'static str,
    pub what: &'static str,
    pub needs: &'static [usize],
    /// Seconds on the stove (loud the whole time).
    pub secs: f32,
    pub food: f32,
    pub water: f32,
}

/// Best first: the stove cooks the first one you have everything for.
pub const DISHES: [Dish; 12] = [
    Dish { name: "Svíčková na smetaně", what: "beef in creamy vegetable sauce with knedlíky (bread dumplings) and cranberries", needs: &[BEEF, CREAM, FLOUR, ONION], secs: 11.0, food: 0.95, water: 0.0 },
    Dish { name: "Guláš s knedlíky", what: "a thick beef goulash with onions and paprika, six knedlíky on the side", needs: &[BEEF, ONION, PAPRIKA, FLOUR], secs: 10.0, food: 0.9, water: 0.05 },
    Dish { name: "Vepřo knedlo zelo", what: "roast pork, knedlíky and sauerkraut: the Czech classic", needs: &[PORK, FLOUR, KRAUT], secs: 10.0, food: 0.85, water: 0.0 },
    Dish { name: "Smažený kapr s bramborovým salátem", what: "fried carp with potato salad, like on Christmas Eve", needs: &[CARP, POTATO, EGGS], secs: 9.0, food: 0.75, water: 0.0 },
    Dish { name: "Řízek s bramborem", what: "a pork schnitzel, fried golden, with boiled potatoes", needs: &[PORK, EGGS, FLOUR], secs: 8.0, food: 0.7, water: 0.0 },
    Dish { name: "Smažený sýr s hranolky", what: "fried cheese with fries and a spoon of tatarka", needs: &[CHEESE, EGGS, FLOUR], secs: 7.0, food: 0.65, water: 0.0 },
    Dish { name: "Kulajda", what: "a creamy soup with mushrooms, potatoes and dill", needs: &[SHROOM, POTATO, CREAM], secs: 8.0, food: 0.45, water: 0.35 },
    Dish { name: "Bramboračka", what: "potato soup with mushrooms and marjoram", needs: &[POTATO, SHROOM, ONION], secs: 8.0, food: 0.45, water: 0.3 },
    Dish { name: "Bramboráky", what: "crispy potato pancakes with garlic and marjoram", needs: &[POTATO, FLOUR, GARLIC], secs: 7.0, food: 0.55, water: 0.0 },
    Dish { name: "Palačinky", what: "thin pancakes rolled up with cream", needs: &[FLOUR, EGGS, CREAM], secs: 6.0, food: 0.5, water: 0.0 },
    Dish { name: "Česnečka", what: "garlic soup with potatoes (it fixes everything, Czech grandmas say)", needs: &[GARLIC, POTATO], secs: 6.0, food: 0.35, water: 0.35 },
    Dish { name: "Míchaná vajíčka s cibulkou", what: "scrambled eggs with onion", needs: &[EGGS, ONION], secs: 5.0, food: 0.4, water: 0.0 },
];

pub fn total(p: &Pantry) -> u32 { p.iter().sum() }

/// A random ingredient (the common ones more often).
pub fn random_ingredient() -> usize {
    const BAG: [usize; 20] = [BEEF, PORK, POTATO, POTATO, POTATO, KRAUT, FLOUR, FLOUR, FLOUR, ONION, ONION, CREAM, EGGS, EGGS, CHEESE, CARP, SHROOM, PAPRIKA, GARLIC, GARLIC];
    BAG[macroquad::rand::gen_range(0, BAG.len())]
}

pub fn add(p: &mut Pantry, i: usize) -> &'static str {
    p[i] += 1;
    ING[i].1
}

/// What the stove would cook now: a dish, or (if nothing fits) a single ingredient fried on its own.
pub enum Cook { Dish(usize), Plain(usize) }

pub fn best(p: &Pantry) -> Option<Cook> {
    if let Some(d) = DISHES.iter().position(|d| d.needs.iter().all(|&i| p[i] > 0)) { return Some(Cook::Dish(d)); }
    p.iter().position(|&n| n > 0).map(Cook::Plain)
}

pub fn label(c: &Cook) -> String {
    match c {
        Cook::Dish(d) => DISHES[*d].name.to_string(),
        Cook::Plain(i) => format!("opečené {} (just {})", ING[*i].0, ING[*i].1),
    }
}

/// Takes the ingredients out of the pantry; returns (seconds, food, water).
pub fn start(p: &mut Pantry, c: &Cook) -> (f32, f32, f32) {
    match c {
        Cook::Dish(d) => {
            for &i in DISHES[*d].needs { p[i] -= 1; }
            (DISHES[*d].secs, DISHES[*d].food, DISHES[*d].water)
        }
        Cook::Plain(i) => { p[*i] -= 1; (5.0, 0.25, 0.0) }
    }
}

/// A short list for the HUD: "2 brambory, 1 cibule, ...".
pub fn summary(p: &Pantry) -> String {
    p.iter().enumerate().filter(|(_, &n)| n > 0).map(|(i, n)| format!("{n} {}", ING[i].0)).collect::<Vec<_>>().join(", ")
}

/// What a kind AI says when the food's ready.
pub fn compliment(d: Option<usize>) -> &'static str {
    match d {
        Some(0) => "Svíčková! Dobrou chuť. Save me a knedlík?",
        Some(1) => "That guláš smells like a real home. Dobrou chuť!",
        Some(2) => "Vepřo knedlo zelo. You know what you're doing.",
        Some(3) => "Kapr and potato salad... it feels like Christmas.",
        Some(6) | Some(7) | Some(10) => "Soup. Good: that's water too. Dobrou chuť.",
        Some(9) => "Palačinky! Okay, now it's a good day.",
        _ => "Dobrou chuť! Eat while it's hot.",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dishes_and_pantry() {
        let mut p: Pantry = [0; 13];
        assert!(best(&p).is_none());
        for i in [BEEF, ONION, PAPRIKA, FLOUR] { p[i] += 1; }
        assert!(matches!(best(&p), Some(Cook::Dish(1))));
        p[CREAM] += 1;
        let c = best(&p).unwrap();
        assert!(matches!(c, Cook::Dish(0)));
        let (secs, food, _) = start(&mut p, &c);
        assert!(secs > 5.0 && food > 0.9);
        assert_eq!(total(&p), 1);
        assert!(matches!(best(&p), Some(Cook::Plain(PAPRIKA))));
        for d in &DISHES { assert!(d.needs.iter().all(|&i| i < ING.len())); }
    }
}
