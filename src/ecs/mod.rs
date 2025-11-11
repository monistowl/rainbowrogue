#![allow(dead_code)]

pub mod components;
pub mod resources;
pub mod systems;

use bracket_geometry::prelude::Point;
use bracket_pathfinding::prelude::DistanceAlg;
use bracket_random::prelude::RandomNumberGenerator;
use specs::prelude::{
    Builder, Dispatcher, DispatcherBuilder, Entity, Join, World as SpecsWorld, WorldExt,
};

use crate::{
    data::{
        items::{ConsumableEffect, starter_consumables},
        monsters::MonsterTemplate,
    },
    map::{Dungeon, FloorId, MapLayer, World, world_color},
};

use self::{
    components::{
        Actor, CombatStats, IntentStep, Inventory, InventoryEffect, InventorySlot, Monster,
        MonsterBrain, MonsterTag, PlaneAttunements, PlayerTag, Position, Renderable, Viewshed,
        WorldAffinity,
    },
    resources::{CombatLog, MovementContext},
    systems::{EnergySystem, FovSystem, MovementSystem, WanderSystem},
};

pub struct EcsWorld {
    specs_world: SpecsWorld,
    dispatcher: Dispatcher<'static, 'static>,
    player: Entity,
    pub turn: u64,
}

pub struct AttackReport {
    pub hit: String,
    pub kill: Option<String>,
}

#[derive(Clone)]
pub struct ConsumableMessage {
    pub lines: Vec<String>,
}
impl EcsWorld {
    pub fn new(spawn: Point, floor: FloorId, world: World) -> Self {
        let mut specs_world = SpecsWorld::new();
        Self::register_components(&mut specs_world);
        specs_world.insert(RandomNumberGenerator::seeded(0x51ec5ead));
        specs_world.insert(CombatLog::default());
        let player = Self::spawn_player(&mut specs_world, spawn, floor, world);
        let dispatcher = DispatcherBuilder::new()
            .with(EnergySystem::default(), "energy", &[])
            .with(WanderSystem::default(), "wander", &[])
            .with(MovementSystem::default(), "movement", &["wander"])
            .with(FovSystem::default(), "fov", &["movement"])
            .build();

        Self {
            specs_world,
            dispatcher,
            player,
            turn: 0,
        }
    }

    fn register_components(world: &mut SpecsWorld) {
        world.register::<Position>();
        world.register::<Renderable>();
        world.register::<Viewshed>();
        world.register::<Actor>();
        world.register::<WorldAffinity>();
        world.register::<PlaneAttunements>();
        world.register::<IntentStep>();
        world.register::<PlayerTag>();
        world.register::<Monster>();
        world.register::<MonsterBrain>();
        world.register::<MonsterTag>();
        world.register::<CombatStats>();
        world.register::<Inventory>();
    }

    fn spawn_player(
        world: &mut SpecsWorld,
        spawn: Point,
        floor: FloorId,
        world_affinity: World,
    ) -> Entity {
        world
            .create_entity()
            .with(Position {
                point: spawn,
                floor,
                world: world_affinity,
            })
            .with(Renderable {
                glyph: b'@' as u16,
                color: world_color(world_affinity),
                order: 2,
            })
            .with(Viewshed {
                radius: 8,
                dirty: true,
                visible: Vec::new(),
                remembered: Vec::new(),
            })
            .with(Actor {
                energy: 0,
                speed: 60,
            })
            .with(CombatStats {
                max_hp: 20,
                hp: 20,
                power: 5,
                defense: 1,
            })
            .with(WorldAffinity {
                primary: world_affinity,
                resist: None,
                vulnerable: None,
            })
            .with(PlaneAttunements {
                unlocked: vec![world_affinity],
                perks: 0,
            })
            .with(PlayerTag)
            .with(Inventory {
                slots: starter_consumables(world_affinity)
                    .into_iter()
                    .map(|template| InventorySlot {
                        name: template.name.to_string(),
                        description: template.description.to_string(),
                        uses_remaining: 1,
                        effect: match template.effect {
                            ConsumableEffect::Heal { amount } => InventoryEffect::Heal { amount },
                            ConsumableEffect::Cleanse => InventoryEffect::Cleanse,
                            ConsumableEffect::Blink { range } => InventoryEffect::Blink { range },
                            ConsumableEffect::Nova { damage, radius } => {
                                InventoryEffect::Nova { damage, radius }
                            }
                        },
                        color: template.color,
                    })
                    .collect(),
            })
            .build()
    }

    pub fn advance(&mut self, layer: &MapLayer, floor: FloorId, world: World) {
        let context = MovementContext::from_layer(layer, floor, world, self.player_point());
        self.specs_world.insert(context);
        self.dispatcher.dispatch(&mut self.specs_world);
        self.specs_world.maintain();
        self.turn = self.turn.wrapping_add(1);
    }

    pub fn queue_player_step(&mut self, delta: Point) {
        let mut intents = self.specs_world.write_component::<IntentStep>();
        let _ = intents.insert(self.player, IntentStep { delta });
    }

    pub fn clear_player_intent(&mut self) {
        let mut intents = self.specs_world.write_component::<IntentStep>();
        let _ = intents.remove(self.player);
    }

    pub fn use_consumable(
        &mut self,
        slot_index: usize,
        dungeon: &mut Dungeon,
        floor: FloorId,
        world: World,
    ) -> Option<Vec<String>> {
        let (name, effect, remove_slot) = {
            let mut inventories = self.specs_world.write_component::<Inventory>();
            let inventory = inventories.get_mut(self.player)?;
            if slot_index >= inventory.slots.len() {
                return None;
            }
            let slot = &mut inventory.slots[slot_index];
            if slot.uses_remaining <= 0 {
                return None;
            }
            slot.uses_remaining -= 1;
            (
                slot.name.clone(),
                slot.effect.clone(),
                slot.uses_remaining <= 0,
            )
        };

        let mut log = vec![format!("Activated {name}")];
        match effect {
            InventoryEffect::Heal { amount } => {
                let mut stats = self.specs_world.write_component::<CombatStats>();
                if let Some(player_stats) = stats.get_mut(self.player) {
                    let before = player_stats.hp;
                    player_stats.hp = (player_stats.hp + amount).min(player_stats.max_hp);
                    let gained = player_stats.hp - before;
                    if gained > 0 {
                        log.push(format!("Recovered {gained} HP."));
                    } else {
                        log.push("No further vitality restored.".to_string());
                    }
                }
            }
            InventoryEffect::Cleanse => {
                log.push("Resonance cleansed of spectral grime.".to_string());
            }
            InventoryEffect::Blink { range } => {
                if let Some(dest) = self.blink_destination(range, dungeon, floor, world) {
                    self.set_player_position(dest, floor, world);
                    log.push(format!("Blink to {},{}", dest.x, dest.y));
                } else {
                    log.push("Blink fizzles; nowhere to anchor.".to_string());
                }
            }
            InventoryEffect::Nova { damage, radius } => {
                log.extend(self.spectral_nova(damage, radius, floor, world));
            }
        }

        if remove_slot {
            let mut inventories = self.specs_world.write_component::<Inventory>();
            if let Some(inv) = inventories.get_mut(self.player) {
                if slot_index < inv.slots.len() {
                    inv.slots.remove(slot_index);
                }
            }
        }

        Some(log)
    }

    pub fn entity_at(&self, point: Point, floor: FloorId, world: World) -> Option<Entity> {
        let entities = self.specs_world.entities();
        let positions = self.specs_world.read_component::<Position>();
        for (entity, pos) in (&entities, &positions).join() {
            if pos.floor == floor && pos.world == world && pos.point == point {
                return Some(entity);
            }
        }
        None
    }

    pub fn player_attack(
        &mut self,
        target_point: Point,
        floor: FloorId,
        world: World,
    ) -> Option<AttackReport> {
        let target = self.entity_at(target_point, floor, world)?;
        if target == self.player {
            return None;
        }

        let entities = self.specs_world.entities();
        let mut stats = self.specs_world.write_component::<CombatStats>();
        let monsters = self.specs_world.read_component::<Monster>();

        let attacker_stats = stats.get(self.player)?.clone();
        let target_stats = stats.get_mut(target)?;
        let damage = (attacker_stats.power - target_stats.defense).max(1);
        target_stats.hp = target_stats.hp.saturating_sub(damage);

        let name = monsters
            .get(target)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| "foe".to_string());

        let mut kill = None;
        if target_stats.hp == 0 {
            kill = Some(format!("{name} collapses into specter dust."));
            let _ = entities.delete(target);
        }

        Some(AttackReport {
            hit: format!("You strike {name} for {damage}"),
            kill,
        })
    }

    pub fn player_visible_tiles(&self) -> Vec<Point> {
        let storage = self.specs_world.read_component::<Viewshed>();
        storage
            .get(self.player)
            .map(|vs| vs.visible.clone())
            .unwrap_or_default()
    }

    pub fn player_stats(&self) -> Option<CombatStats> {
        let stats = self.specs_world.read_component::<CombatStats>();
        stats.get(self.player).cloned()
    }

    pub fn player_inventory(&self) -> Vec<(usize, InventorySlot)> {
        let inventories = self.specs_world.read_component::<Inventory>();
        inventories
            .get(self.player)
            .map(|inv| {
                inv.slots
                    .iter()
                    .cloned()
                    .enumerate()
                    .collect::<Vec<(usize, InventorySlot)>>()
            })
            .unwrap_or_default()
    }

    pub fn drain_combat_log(&mut self) -> Vec<String> {
        let mut log = self.specs_world.write_resource::<CombatLog>();
        std::mem::take(&mut log.entries)
    }

    pub fn spawn_monster(
        &mut self,
        template: &MonsterTemplate,
        point: Point,
        floor: FloorId,
        world: World,
    ) {
        self.specs_world
            .create_entity()
            .with(Position {
                point,
                floor,
                world,
            })
            .with(Renderable {
                glyph: template.glyph as u16,
                color: template.color,
                order: 1,
            })
            .with(Monster {
                name: template.name.to_string(),
            })
            .with(MonsterBrain {
                wander_chance: template.wander_chance,
            })
            .with(CombatStats {
                max_hp: template.hp,
                hp: template.hp,
                power: template.power,
                defense: template.defense,
            })
            .with(WorldAffinity {
                primary: world,
                resist: None,
                vulnerable: None,
            })
            .with(MonsterTag::default())
            .build();
    }

    pub fn each_renderable<F>(&self, floor: FloorId, world: World, include_player: bool, mut f: F)
    where
        F: FnMut(Point, &Renderable),
    {
        let entities = self.specs_world.entities();
        let positions = self.specs_world.read_component::<Position>();
        let renderables = self.specs_world.read_component::<Renderable>();
        let players = self.specs_world.read_component::<PlayerTag>();
        for (entity, pos, renderable) in (&entities, &positions, &renderables).join() {
            if pos.floor != floor || pos.world != world {
                continue;
            }
            if !include_player && players.contains(entity) {
                continue;
            }
            f(pos.point, renderable);
        }
    }

    pub fn player_position(&self) -> Position {
        let storage = self.specs_world.read_component::<Position>();
        storage.get(self.player).cloned().unwrap_or(Position {
            point: Point::new(0, 0),
            floor: FloorId(0),
            world: World::Red,
        })
    }

    pub fn player_point(&self) -> Point {
        self.player_position().point
    }

    pub fn player_entity(&self) -> Entity {
        self.player
    }

    pub fn set_player_position(&mut self, point: Point, floor: FloorId, world: World) {
        {
            let mut positions = self.specs_world.write_component::<Position>();
            if let Some(pos) = positions.get_mut(self.player) {
                pos.point = point;
                pos.floor = floor;
                pos.world = world;
            }
        }

        {
            let mut renderables = self.specs_world.write_component::<Renderable>();
            if let Some(render) = renderables.get_mut(self.player) {
                render.color = world_color(world);
            }
        }

        {
            let mut viewsheds = self.specs_world.write_component::<Viewshed>();
            if let Some(vs) = viewsheds.get_mut(self.player) {
                vs.dirty = true;
            }
        }
    }

    fn blink_destination(
        &mut self,
        range: i32,
        dungeon: &mut Dungeon,
        floor: FloorId,
        world: World,
    ) -> Option<Point> {
        let current = self.player_point();
        let mut candidates = Vec::new();
        for dy in -range..=range {
            for dx in -range..=range {
                let point = Point::new(current.x + dx, current.y + dy);
                if dungeon.is_walkable(floor, world, point)
                    && self.entity_at(point, floor, world).is_none()
                {
                    candidates.push(point);
                }
            }
        }
        if candidates.is_empty() {
            return None;
        }
        let mut rng = self.specs_world.write_resource::<RandomNumberGenerator>();
        let idx = rng.range(0, candidates.len() as i32) as usize;
        Some(candidates[idx])
    }

    fn spectral_nova(
        &mut self,
        damage: i32,
        radius: i32,
        floor: FloorId,
        world: World,
    ) -> Vec<String> {
        let mut log = Vec::new();
        let mut stats = self.specs_world.write_component::<CombatStats>();
        let positions = self.specs_world.read_component::<Position>();
        let monsters = self.specs_world.read_component::<Monster>();
        let entities = self.specs_world.entities();
        let mut deaths = Vec::new();
        let origin = self.player_point();
        let mut affected = 0;

        for (entity, pos, stat, monster) in (&entities, &positions, &mut stats, &monsters).join() {
            if pos.floor != floor || pos.world != world {
                continue;
            }
            let dist = DistanceAlg::Pythagoras.distance2d(origin, pos.point);
            if dist <= radius as f32 {
                affected += 1;
                stat.hp = stat.hp.saturating_sub(damage);
                log.push(format!("{} sears for {} damage.", monster.name, damage));
                if stat.hp == 0 {
                    deaths.push((entity, monster.name.clone()));
                }
            }
        }

        for (entity, name) in deaths {
            log.push(format!("{name} disintegrates in prismatic fire."));
            let _ = entities.delete(entity);
        }

        if affected == 0 {
            log.push("Nova crackles harmlessly.".to_string());
        }

        log
    }
}
