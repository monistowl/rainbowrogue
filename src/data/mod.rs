#![allow(dead_code)]

pub mod items;
pub mod monsters;

use crate::map::World;

#[derive(Clone, Debug)]
pub struct WorldRuleSet {
    pub world: World,
    pub notes: &'static str,
}

pub fn builtin_rules() -> Vec<WorldRuleSet> {
    vec![
        WorldRuleSet {
            world: World::Red,
            notes: "Heat blooms amplify melee damage.",
        },
        WorldRuleSet {
            world: World::Orange,
            notes: "Chemical clouds respond to wind tunnels.",
        },
        WorldRuleSet {
            world: World::Yellow,
            notes: "Lens-prisms extend FOV and detect traps.",
        },
        WorldRuleSet {
            world: World::Green,
            notes: "Regrowth tiles slowly mend allies.",
        },
        WorldRuleSet {
            world: World::Blue,
            notes: "Stillwater grants crit bonuses to ranged.",
        },
        WorldRuleSet {
            world: World::Indigo,
            notes: "Mindstorms favor teleport talent rolls.",
        },
        WorldRuleSet {
            world: World::Violet,
            notes: "Curses thread through unseen resonance.",
        },
    ]
}
