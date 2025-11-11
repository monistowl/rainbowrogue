#![allow(dead_code)]

use bracket_geometry::prelude::Point;
use bracket_terminal::prelude::RGB;
use specs::prelude::{Component, NullStorage, VecStorage};

use crate::map::{FloorId, World};

#[derive(Clone, Debug)]
pub struct Position {
    pub point: Point,
    pub floor: FloorId,
    pub world: World,
}

impl Component for Position {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug)]
pub struct Renderable {
    pub glyph: u16,
    pub color: RGB,
    pub order: i32,
}

impl Component for Renderable {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug, Default)]
pub struct Viewshed {
    pub radius: i32,
    pub dirty: bool,
    pub visible: Vec<Point>,
    pub remembered: Vec<Point>,
}

impl Component for Viewshed {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug)]
pub struct Actor {
    pub energy: i32,
    pub speed: i32,
}

impl Component for Actor {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug)]
pub struct Portal {
    pub to_world: World,
    pub cost: i32,
    pub key_mask: u32,
    pub cooldown: i32,
}

impl Component for Portal {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug, Default)]
pub struct WorldAffinity {
    pub primary: World,
    pub resist: Option<World>,
    pub vulnerable: Option<World>,
}

impl Component for WorldAffinity {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug, Default)]
pub struct PlaneAttunements {
    pub unlocked: Vec<World>,
    pub perks: u64,
}

impl Component for PlaneAttunements {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug, Copy, Default, PartialEq, Eq, Hash)]
pub struct ConcordanceId(pub u64);

impl Component for ConcordanceId {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug)]
pub struct IntentStep {
    pub delta: Point,
}

impl Default for IntentStep {
    fn default() -> Self {
        Self {
            delta: Point::new(0, 0),
        }
    }
}

impl Component for IntentStep {
    type Storage = VecStorage<Self>;
}

#[derive(Default)]
pub struct PlayerTag;

impl Component for PlayerTag {
    type Storage = NullStorage<Self>;
}

#[derive(Default)]
pub struct MonsterTag;

impl Component for MonsterTag {
    type Storage = NullStorage<Self>;
}

#[derive(Clone, Debug)]
pub struct Monster {
    pub name: String,
}

impl Component for Monster {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug)]
pub struct MonsterBrain {
    pub wander_chance: f32,
}

impl Component for MonsterBrain {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug)]
pub struct CombatStats {
    pub max_hp: i32,
    pub hp: i32,
    pub power: i32,
    pub defense: i32,
}

impl Component for CombatStats {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug, Default)]
pub struct Inventory {
    pub slots: Vec<InventorySlot>,
}

#[derive(Clone, Debug)]
pub struct InventorySlot {
    pub name: String,
    pub description: String,
    pub uses_remaining: i32,
    pub effect: InventoryEffect,
    pub color: RGB,
}

#[derive(Clone, Debug)]
pub enum InventoryEffect {
    Heal { amount: i32 },
    Cleanse,
    Blink { range: i32 },
    Nova { damage: i32, radius: i32 },
}

impl Component for Inventory {
    type Storage = VecStorage<Self>;
}
