#![allow(dead_code)]

use bracket_terminal::prelude::RGB;

use crate::map::World;

#[derive(Clone, Debug)]
pub struct MonsterTemplate {
    pub name: &'static str,
    pub glyph: char,
    pub color: RGB,
    pub wander_chance: f32,
    pub hp: i32,
    pub power: i32,
    pub defense: i32,
}

impl MonsterTemplate {
    pub fn for_world(world: World) -> Vec<Self> {
        match world {
            World::Red => vec![
                Self::new("Ember Imp", 'i', RGB::from_u8(255, 140, 76), 0.65, 6, 3, 0),
                Self::new(
                    "Cinder Wolf",
                    'w',
                    RGB::from_u8(255, 90, 90),
                    0.55,
                    10,
                    4,
                    1,
                ),
            ],
            World::Orange => vec![
                Self::new("Acid Puff", 'a', RGB::from_u8(255, 180, 90), 0.6, 8, 3, 0),
                Self::new(
                    "Flask Golem",
                    'g',
                    RGB::from_u8(210, 140, 60),
                    0.45,
                    14,
                    5,
                    2,
                ),
            ],
            World::Yellow => vec![
                Self::new(
                    "Prism Ghost",
                    'p',
                    RGB::from_u8(255, 255, 140),
                    0.35,
                    7,
                    2,
                    1,
                ),
                Self::new("Sun Mite", 'm', RGB::from_u8(250, 230, 120), 0.5, 5, 2, 0),
            ],
            World::Green => vec![
                Self::new(
                    "Thorn Hopper",
                    'h',
                    RGB::from_u8(140, 220, 120),
                    0.5,
                    9,
                    3,
                    1,
                ),
                Self::new(
                    "Bloom Sentinel",
                    'b',
                    RGB::from_u8(90, 200, 90),
                    0.3,
                    16,
                    4,
                    3,
                ),
            ],
            World::Blue => vec![
                Self::new(
                    "Glacier Crab",
                    'c',
                    RGB::from_u8(120, 170, 255),
                    0.4,
                    12,
                    3,
                    2,
                ),
                Self::new(
                    "Stillwater Shade",
                    's',
                    RGB::from_u8(90, 140, 255),
                    0.35,
                    8,
                    4,
                    1,
                ),
            ],
            World::Indigo => vec![
                Self::new("Mindworm", 'n', RGB::from_u8(170, 140, 255), 0.45, 6, 4, 0),
                Self::new(
                    "Phase Stalker",
                    'q',
                    RGB::from_u8(150, 120, 220),
                    0.55,
                    10,
                    5,
                    1,
                ),
            ],
            World::Violet => vec![
                Self::new("Hex Bat", 'x', RGB::from_u8(220, 120, 255), 0.5, 7, 3, 0),
                Self::new(
                    "Veil Revenant",
                    'v',
                    RGB::from_u8(200, 90, 255),
                    0.4,
                    13,
                    5,
                    2,
                ),
            ],
        }
    }

    fn new(
        name: &'static str,
        glyph: char,
        color: RGB,
        wander_chance: f32,
        hp: i32,
        power: i32,
        defense: i32,
    ) -> Self {
        Self {
            name,
            glyph,
            color,
            wander_chance,
            hp,
            power,
            defense,
        }
    }
}
