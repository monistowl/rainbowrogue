#![allow(dead_code)]

use bracket_geometry::prelude::Point;
use bracket_pathfinding::prelude::{Algorithm2D, BaseMap, DistanceAlg, field_of_view};
use bracket_random::prelude::RandomNumberGenerator;
use smallvec::SmallVec;
use specs::prelude::*;

use super::{
    components::{
        Actor, CombatStats, IntentStep, Monster, MonsterBrain, MonsterTag, PlayerTag, Position,
        Viewshed,
    },
    resources::{CombatLog, MovementContext},
};

#[derive(Default)]
pub struct EnergySystem;

impl<'a> System<'a> for EnergySystem {
    type SystemData = WriteStorage<'a, Actor>;

    fn run(&mut self, mut actors: Self::SystemData) {
        for actor in (&mut actors).join() {
            actor.energy = actor.energy.saturating_add(actor.speed);
        }
    }
}

#[derive(Default)]
pub struct WanderSystem;

impl<'a> System<'a> for WanderSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, IntentStep>,
        ReadStorage<'a, Position>,
        ReadStorage<'a, MonsterTag>,
        ReadStorage<'a, MonsterBrain>,
        ReadExpect<'a, MovementContext>,
        ReadStorage<'a, CombatStats>,
        WriteExpect<'a, RandomNumberGenerator>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut intents,
            positions,
            monsters,
            brains,
            movement,
            stats,
            mut rng,
        ): Self::SystemData,
    ) {
        let dirs = [
            Point::new(1, 0),
            Point::new(-1, 0),
            Point::new(0, 1),
            Point::new(0, -1),
        ];
        for (entity, pos, _, brain) in (&entities, &positions, &monsters, &brains).join() {
            if pos.floor != movement.floor || pos.world != movement.world {
                continue;
            }

            let mut acted = false;

            if let Some(stat) = stats.get(entity) {
                let player_distance =
                    DistanceAlg::Pythagoras.distance2d(pos.point, movement.player_point);
                let hp_ratio = stat.hp as f32 / stat.max_hp as f32;
                if pos.floor == movement.floor && pos.world == movement.world {
                    if hp_ratio <= 0.3 && player_distance < 6.0 {
                        if let Some(step) = step_away(pos.point, movement.player_point, &movement) {
                            let _ = intents.insert(entity, IntentStep { delta: step });
                            acted = true;
                        }
                    } else if player_distance <= 8.0 {
                        if let Some(step) =
                            step_towards(pos.point, movement.player_point, &movement)
                        {
                            let _ = intents.insert(entity, IntentStep { delta: step });
                            acted = true;
                        }
                    }
                }
            }

            if acted {
                continue;
            }

            let roll = rng.range(0, 100) as f32 / 100.0;
            if roll > brain.wander_chance {
                continue;
            }
            let dir = dirs[rng.range(0, dirs.len() as i32) as usize];
            let target = Point::new(pos.point.x + dir.x, pos.point.y + dir.y);
            if movement.is_walkable(target) {
                let _ = intents.insert(entity, IntentStep { delta: dir });
            }
        }
    }
}

#[derive(Default)]
pub struct MovementSystem;

impl<'a> System<'a> for MovementSystem {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, IntentStep>,
        ReadExpect<'a, MovementContext>,
        WriteStorage<'a, Viewshed>,
        ReadStorage<'a, PlayerTag>,
        WriteStorage<'a, CombatStats>,
        ReadStorage<'a, Monster>,
        WriteExpect<'a, CombatLog>,
    );

    fn run(
        &mut self,
        (
            entities,
            mut positions,
            mut intents,
            movement,
            mut viewsheds,
            players,
            mut stats,
            monsters,
            mut combat_log,
        ): Self::SystemData,
    ) {
        let mut player_snapshot = {
            let positions_ref: &WriteStorage<Position> = &positions;
            (&entities, positions_ref, &players)
                .join()
                .next()
                .map(|(entity, pos, _)| (entity, pos.clone()))
        };

        let mut to_clear = Vec::new();
        for (entity, pos, intent) in (&entities, &mut positions, &intents).join() {
            if pos.floor != movement.floor || pos.world != movement.world {
                continue;
            }
            let target = Point::new(pos.point.x + intent.delta.x, pos.point.y + intent.delta.y);

            if let Some((player_entity_id, player_pos)) = player_snapshot.as_mut() {
                if target == player_pos.point
                    && pos.floor == player_pos.floor
                    && pos.world == player_pos.world
                    && entity != *player_entity_id
                {
                    if let (Some(attacker_stats), Some(player_stats)) =
                        (stats.get(entity).cloned(), stats.get_mut(*player_entity_id))
                    {
                        let damage = (attacker_stats.power - player_stats.defense).max(1);
                        player_stats.hp = player_stats.hp.saturating_sub(damage);
                        let name = monsters
                            .get(entity)
                            .map(|m| m.name.clone())
                            .unwrap_or_else(|| "foe".to_string());
                        combat_log.push(format!("{name} claws you for {damage}"));
                        if player_stats.hp == 0 {
                            combat_log.push("You feel your spectrum shatter.".to_string());
                        }
                    }
                    continue;
                }
            }

            if movement.is_walkable(target) {
                pos.point = target;
                if let Some(vs) = viewsheds.get_mut(entity) {
                    vs.dirty = true;
                }

                if let Some((player_entity_id, player_pos)) = player_snapshot.as_mut() {
                    if entity == *player_entity_id {
                        player_pos.point = pos.point;
                        player_pos.floor = pos.floor;
                        player_pos.world = pos.world;
                    }
                }
            }
            to_clear.push(entity);
        }

        for entity in to_clear {
            intents.remove(entity);
        }
    }
}

#[derive(Default)]
pub struct FovSystem;

impl<'a> System<'a> for FovSystem {
    type SystemData = (
        ReadExpect<'a, MovementContext>,
        WriteStorage<'a, Viewshed>,
        ReadStorage<'a, Position>,
    );

    fn run(&mut self, (movement, mut viewsheds, positions): Self::SystemData) {
        let map = MovementFov { ctx: &*movement };
        for (viewshed, pos) in (&mut viewsheds, &positions).join() {
            if !viewshed.dirty || pos.floor != movement.floor || pos.world != movement.world {
                continue;
            }
            viewshed.visible = field_of_view(pos.point, viewshed.radius, &map)
                .into_iter()
                .filter(|point| movement.in_bounds(*point))
                .collect();
            for point in &viewshed.visible {
                if !viewshed.remembered.contains(point) {
                    viewshed.remembered.push(*point);
                }
            }
            viewshed.dirty = false;
        }
    }
}

struct MovementFov<'a> {
    ctx: &'a MovementContext,
}

impl<'a> BaseMap for MovementFov<'a> {
    fn is_opaque(&self, idx: usize) -> bool {
        let point = self.index_to_point2d(idx);
        self.ctx.blocks_sight(point)
    }

    fn get_available_exits(&self, idx: usize) -> SmallVec<[(usize, f32); 10]> {
        let mut exits = SmallVec::new();
        let point = self.index_to_point2d(idx);
        let steps = [
            Point::new(1, 0),
            Point::new(-1, 0),
            Point::new(0, 1),
            Point::new(0, -1),
        ];
        for dir in steps {
            let dest = Point::new(point.x + dir.x, point.y + dir.y);
            if self.in_bounds(dest) && self.ctx.is_walkable(dest) {
                exits.push((self.point2d_to_index(dest), 1.0));
            }
        }
        exits
    }

    fn get_pathing_distance(&self, idx1: usize, idx2: usize) -> f32 {
        let p1 = self.index_to_point2d(idx1);
        let p2 = self.index_to_point2d(idx2);
        DistanceAlg::Pythagoras.distance2d(p1, p2)
    }
}

impl<'a> Algorithm2D for MovementFov<'a> {
    fn dimensions(&self) -> Point {
        Point::new(self.ctx.width, self.ctx.height)
    }

    fn in_bounds(&self, point: Point) -> bool {
        self.ctx.in_bounds(point)
    }
}

fn step_towards(from: Point, to: Point, movement: &MovementContext) -> Option<Point> {
    let dx = (to.x - from.x).clamp(-1, 1);
    let dy = (to.y - from.y).clamp(-1, 1);
    try_steps(from, dx, dy, movement)
}

fn step_away(from: Point, to: Point, movement: &MovementContext) -> Option<Point> {
    let dx = (from.x - to.x).clamp(-1, 1);
    let dy = (from.y - to.y).clamp(-1, 1);
    try_steps(from, dx, dy, movement)
}

fn try_steps(from: Point, dx: i32, dy: i32, movement: &MovementContext) -> Option<Point> {
    let axes = if dx.abs() >= dy.abs() {
        [Point::new(dx, 0), Point::new(0, dy)]
    } else {
        [Point::new(0, dy), Point::new(dx, 0)]
    };
    for dir in axes {
        if dir == Point::new(0, 0) {
            continue;
        }
        let target = Point::new(from.x + dir.x, from.y + dir.y);
        if movement.is_walkable(target) {
            return Some(dir);
        }
    }
    None
}
