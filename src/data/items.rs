#![allow(dead_code)]
use bracket_terminal::prelude::{LIGHT_BLUE, MAGENTA, ORANGE, RED, RGB};

use crate::map::World;

#[derive(Clone, Debug)]
pub struct ConsumableTemplate {
    pub name: &'static str,
    pub description: &'static str,
    pub color: RGB,
    pub effect: ConsumableEffect,
}

#[derive(Clone, Debug)]
pub enum ConsumableEffect {
    Heal { amount: i32 },
    Cleanse,
    Blink { range: i32 },
    Nova { damage: i32, radius: i32 },
}

pub fn starter_consumables(world: World) -> Vec<ConsumableTemplate> {
    match world {
        World::Red => vec![
            ConsumableTemplate::new(
                "Thermal Draft",
                "Restores 8 HP with a warming rush.",
                RGB::named(ORANGE),
                ConsumableEffect::Heal { amount: 8 },
            ),
            ConsumableTemplate::new(
                "Ember Nova",
                "Detonates a 3-tile blast for 6 damage.",
                RGB::named(RED),
                ConsumableEffect::Nova {
                    damage: 6,
                    radius: 3,
                },
            ),
        ],
        World::Blue => vec![ConsumableTemplate::new(
            "Stillwater Draught",
            "Heals 10 HP and purges slowing chills.",
            RGB::named(LIGHT_BLUE),
            ConsumableEffect::Heal { amount: 10 },
        )],
        _ => vec![ConsumableTemplate::new(
            "Prismatic Tonic",
            "Heals 6 HP and cleanses curse residue.",
            RGB::named(MAGENTA),
            ConsumableEffect::Cleanse,
        )],
    }
}

impl ConsumableTemplate {
    pub const fn new(
        name: &'static str,
        description: &'static str,
        color: RGB,
        effect: ConsumableEffect,
    ) -> Self {
        Self {
            name,
            description,
            color,
            effect,
        }
    }
}
