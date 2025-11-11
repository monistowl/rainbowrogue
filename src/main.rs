mod ai;
mod data;
mod ecs;
mod map;
mod render;

use ai::BehaviorContext;
use bracket_geometry::prelude::Point;
use bracket_random::prelude::RandomNumberGenerator;
use bracket_terminal::prelude::*;
use data::monsters::MonsterTemplate;
use ecs::EcsWorld;
use map::{Dungeon, FloorId, SPECTRUM, World};
use render::{HudRing, draw_log, draw_map};
use std::collections::HashSet;

const SCREEN_HEIGHT: i32 = 50;
const MAP_ORIGIN_X: i32 = 2;
const MAP_ORIGIN_Y: i32 = 7;
const LOG_RESERVED_ROWS: i32 = 7;
const LOG_PANEL_START: i32 = SCREEN_HEIGHT - 6;
const LOG_MAX_ENTRIES: usize = 8;

struct RainbowRogueState {
    dungeon: Dungeon,
    ecs: EcsWorld,
    behavior: BehaviorContext,
    hud: HudRing,
    active_world: World,
    active_floor: FloorId,
    frame: u64,
    message_log: Vec<String>,
    last_move_attempt: Option<(Point, Point)>,
    visible_tiles: HashSet<Point>,
    hp_alerted: bool,
    hp_ratio: f32,
}

impl Default for RainbowRogueState {
    fn default() -> Self {
        let dungeon = Dungeon::scaffolding_demo();
        let active_world = World::Red;
        let active_floor = FloorId(0);
        let mut message_log: Vec<String> = data::builtin_rules()
            .into_iter()
            .map(|rule| format!("{} focus: {}", rule.world.as_str(), rule.notes))
            .collect();
        message_log.truncate(LOG_MAX_ENTRIES);
        let player_pos = dungeon.spawn_point(active_floor);
        let ecs = EcsWorld::new(player_pos, active_floor, active_world);

        let mut state = Self {
            dungeon,
            ecs,
            behavior: BehaviorContext::new(active_world),
            hud: HudRing::new(),
            active_world,
            active_floor,
            frame: 0,
            message_log,
            last_move_attempt: None,
            visible_tiles: HashSet::new(),
            hp_alerted: false,
            hp_ratio: 1.0,
        };
        state.seed_monsters();
        state
    }
}

impl GameState for RainbowRogueState {
    fn tick(&mut self, ctx: &mut BTerm) {
        self.handle_input(ctx);
        self.frame = self.frame.wrapping_add(1);
        let previous_point = self.ecs.player_point();
        if let Some(layer) = self
            .dungeon
            .active_layer(self.active_floor, self.active_world)
        {
            self.ecs
                .advance(layer, self.active_floor, self.active_world);
        } else {
            self.ecs.clear_player_intent();
        }
        self.resolve_move_attempt(previous_point);
        self.update_visibility();
        self.flush_combat_log();
        self.check_health_warning();
        ctx.cls();
        self.draw_scene(ctx);
    }
}

impl RainbowRogueState {
    fn handle_input(&mut self, ctx: &mut BTerm) {
        if let Some(key) = ctx.key {
            match key {
                VirtualKeyCode::Left | VirtualKeyCode::A | VirtualKeyCode::H => {
                    self.try_step(-1, 0)
                }
                VirtualKeyCode::Right | VirtualKeyCode::D | VirtualKeyCode::L => {
                    self.try_step(1, 0)
                }
                VirtualKeyCode::Up | VirtualKeyCode::W | VirtualKeyCode::K => self.try_step(0, -1),
                VirtualKeyCode::Down | VirtualKeyCode::S | VirtualKeyCode::J => self.try_step(0, 1),
                VirtualKeyCode::Tab => self.cycle_world(1),
                VirtualKeyCode::Back => self.cycle_world(-1),
                VirtualKeyCode::PageUp => self.shift_floor(1),
                VirtualKeyCode::PageDown => self.shift_floor(-1),
                VirtualKeyCode::Key1 => self.activate_consumable(0),
                VirtualKeyCode::Key2 => self.activate_consumable(1),
                VirtualKeyCode::Key3 => self.activate_consumable(2),
                VirtualKeyCode::Key4 => self.activate_consumable(3),
                _ => {}
            }
        }
    }

    fn draw_scene(&mut self, ctx: &mut BTerm) {
        let header = format!(
            "RainbowRogue pre-alpha · Frame {} · Turn {}",
            self.frame, self.ecs.turn
        );
        ctx.print_color_centered(1, RGB::named(YELLOW), RGB::named(BLACK), &header);

        let info = format!(
            "Active world: {} · Floor {}",
            self.active_world.as_str(),
            self.active_floor.0
        );
        ctx.print_color_centered(3, RGB::named(LIGHT_CYAN), RGB::named(BLACK), &info);
        if let Some(stats) = self.ecs.player_stats() {
            let vitality = format!("HP {}/{}", stats.hp, stats.max_hp);
            let hp_color = if self.hp_ratio <= 0.3 {
                RGB::named(ORANGE)
            } else if self.hp_ratio <= 0.6 {
                RGB::from_u8(255, 120, 120)
            } else {
                RGB::named(RED)
            };
            ctx.print_color_centered(4, hp_color, RGB::named(BLACK), &vitality);
        }

        self.hud
            .draw(ctx, self.active_world, self.active_floor, self.frame);
        self.draw_quickbar(ctx);

        if let Some(layer) = self
            .dungeon
            .active_layer(self.active_floor, self.active_world)
        {
            draw_map(
                ctx,
                layer,
                Point::new(MAP_ORIGIN_X, MAP_ORIGIN_Y),
                LOG_RESERVED_ROWS,
                &self.visible_tiles,
            );
            self.ecs.each_renderable(
                self.active_floor,
                self.active_world,
                true,
                |point, renderable| {
                    if !self.visible_tiles.contains(&point) {
                        return;
                    }
                    let screen_x = MAP_ORIGIN_X + point.x;
                    let screen_y = MAP_ORIGIN_Y + point.y;
                    ctx.set(
                        screen_x,
                        screen_y,
                        renderable.color,
                        RGB::named(BLACK),
                        renderable.glyph,
                    );
                },
            );
        }

        draw_log(ctx, &self.message_log, LOG_PANEL_START);
    }

    fn cycle_world(&mut self, delta: i32) {
        self.active_world = self.active_world.cycle(delta);
        self.behavior = BehaviorContext::new(self.active_world);
        let point = self.ecs.player_point();
        self.ecs
            .set_player_position(point, self.active_floor, self.active_world);
        self.ecs.clear_player_intent();
        self.last_move_attempt = None;
        self.push_log_entry(format!(
            "Shifted attunement to {} on frame {}",
            self.active_world.as_str(),
            self.frame
        ));
    }

    fn shift_floor(&mut self, delta: i32) {
        let current = self.active_floor.0 as i32;
        let max_floor = (self.dungeon.floors.len().max(1) - 1) as i32;
        let next = (current + delta).clamp(0, max_floor);
        self.active_floor = FloorId(next as u32);
        let spawn = self.dungeon.spawn_point(self.active_floor);
        self.ecs
            .set_player_position(spawn, self.active_floor, self.active_world);
        self.ecs.clear_player_intent();
        self.last_move_attempt = None;
        self.push_log_entry(format!(
            "Reindexed to floor {} (delta {delta})",
            self.active_floor.0
        ));
    }

    fn try_step(&mut self, dx: i32, dy: i32) {
        if dx == 0 && dy == 0 {
            return;
        }

        let current = self.ecs.player_point();
        let target = Point::new(current.x + dx, current.y + dy);
        if let Some(entity) = self
            .ecs
            .entity_at(target, self.active_floor, self.active_world)
        {
            if entity != self.ecs.player_entity() {
                if let Some(report) =
                    self.ecs
                        .player_attack(target, self.active_floor, self.active_world)
                {
                    self.push_log_entry(report.hit);
                    if let Some(kill) = report.kill {
                        self.push_log_entry(kill);
                        self.ecs.queue_player_step(Point::new(dx, dy));
                        self.last_move_attempt = Some((current, target));
                    } else {
                        self.last_move_attempt = None;
                    }
                    return;
                }
            }
        }
        self.ecs.queue_player_step(Point::new(dx, dy));
        self.last_move_attempt = Some((current, target));
    }

    fn push_log_entry<S: Into<String>>(&mut self, entry: S) {
        self.message_log.insert(0, entry.into());
        self.message_log.truncate(LOG_MAX_ENTRIES);
    }

    fn draw_quickbar(&self, ctx: &mut BTerm) {
        let entries = self.ecs.player_inventory();
        if entries.is_empty() {
            return;
        }
        let mut x = 2;
        for (idx, slot) in entries.iter().take(5) {
            let label = format!("[{}] {} (x{})", idx + 1, slot.name, slot.uses_remaining);
            ctx.print_color(x, 5, slot.color, RGB::named(BLACK), &label);
            x += label.len() as i32 + 2;
        }
    }

    fn resolve_move_attempt(&mut self, previous_point: Point) {
        if let Some((origin, target)) = self.last_move_attempt.take() {
            let current = self.ecs.player_point();
            if current == target {
                self.push_log_entry(format!(
                    "Stepped to {},{} in {}",
                    current.x,
                    current.y,
                    self.active_world.as_str()
                ));
            } else if origin == previous_point {
                self.push_log_entry(format!("Blocked at {},{}", target.x, target.y));
            }
        }
    }

    fn update_visibility(&mut self) {
        let previous = self.visible_tiles.clone();
        if let Some(layer) = self
            .dungeon
            .active_layer_mut(self.active_floor, self.active_world)
        {
            let visible = self.ecs.player_visible_tiles();
            for point in &visible {
                layer.reveal_point(*point);
            }
            self.visible_tiles = visible.into_iter().collect();
            let newly_visible = self
                .visible_tiles
                .iter()
                .filter(|point| !previous.contains(point))
                .count();
            if newly_visible > 0 {
                self.push_log_entry(format!(
                    "Glimpsed {newly_visible} new tiles in {}",
                    self.active_world.as_str()
                ));
            }
        } else {
            self.visible_tiles.clear();
        }
    }

    fn flush_combat_log(&mut self) {
        for entry in self.ecs.drain_combat_log() {
            self.push_log_entry(entry);
        }
    }

    fn check_health_warning(&mut self) {
        if let Some(stats) = self.ecs.player_stats() {
            let ratio = stats.hp as f32 / stats.max_hp as f32;
            let critical = ratio <= 0.3;
            if critical && !self.hp_alerted {
                self.push_log_entry("!! Vitality critical !!");
                self.hp_alerted = true;
            } else if !critical && self.hp_alerted && ratio > 0.5 {
                self.push_log_entry("Vitality stabilizes.");
                self.hp_alerted = false;
            }
            self.hp_ratio = ratio;
        }
    }

    fn activate_consumable(&mut self, slot_index: usize) {
        if let Some(messages) = self.ecs.use_consumable(
            slot_index,
            &mut self.dungeon,
            self.active_floor,
            self.active_world,
        ) {
            for message in messages {
                self.push_log_entry(message);
            }
            self.last_move_attempt = None;
            self.update_visibility();
        } else {
            self.push_log_entry(format!("Slot {} is empty.", slot_index + 1));
        }
    }

    fn seed_monsters(&mut self) {
        let mut rng = RandomNumberGenerator::seeded(0xdead_beef);
        let spawn_floor = self.active_floor;
        if let Some(floor) = self.dungeon.active_floor(spawn_floor) {
            for &world in SPECTRUM.iter() {
                let layer = floor.layer(world);
                let mut walkable = layer.walkable_points();
                if walkable.is_empty() {
                    continue;
                }
                let templates = MonsterTemplate::for_world(world);
                if templates.is_empty() {
                    continue;
                }
                let spawn_count = (walkable.len() / 80).max(2).min(6);
                for _ in 0..spawn_count {
                    if walkable.is_empty() {
                        break;
                    }
                    let idx = rng.range(0, walkable.len() as i32) as usize;
                    let point = walkable.swap_remove(idx);
                    if world == self.active_world && point == self.ecs.player_point() {
                        continue;
                    }
                    let template_idx = rng.range(0, templates.len() as i32) as usize;
                    let template = templates[template_idx].clone();
                    self.ecs.spawn_monster(&template, point, spawn_floor, world);
                }
            }
        }
    }
}

fn main() -> BError {
    let context = BTermBuilder::simple80x50()
        .with_title("RainbowRogue · Spectrum Seed")
        .build()?;
    let game_state = RainbowRogueState::default();
    main_loop(context, game_state)
}
