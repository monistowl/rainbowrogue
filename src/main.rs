mod ai;
mod data;
mod ecs;
mod map;
mod render;
mod scripted_input;

use ai::BehaviorContext;
use bracket_geometry::prelude::Point;
use bracket_random::prelude::RandomNumberGenerator;
use bracket_terminal::prelude::*;
use chrono;

use data::monsters::MonsterTemplate;
use ecs::EcsWorld;
use map::{Dungeon, FloorId, SPECTRUM, Tile, World};
use render::{HudRing, draw_log, draw_map};
use scripted_input::ScriptedInput;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, env, fs, io, path::Path};

const MAP_ORIGIN_X: i32 = 2;
const MAP_ORIGIN_Y: i32 = 7;
const LOG_RESERVED_ROWS: i32 = 7;
const LOG_MAX_ENTRIES: usize = 8;
const RUN_STATS_PATH: &str = "run_stats.json";
const RESET_CONFIRM_WINDOW_FRAMES: u64 = 300; // ~5 seconds at 60 FPS

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RunStats {
    run_number: u32,
    best_depth: u32,
}

impl Default for RunStats {
    fn default() -> Self {
        Self {
            run_number: 1,
            best_depth: 0,
        }
    }
}

impl RunStats {
    fn load_from_disk() -> Self {
        let path = Path::new(RUN_STATS_PATH);
        if let Ok(bytes) = fs::read(path) {
            serde_json::from_slice(&bytes).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    fn persist_to_disk(&self) -> io::Result<()> {
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        fs::write(RUN_STATS_PATH, bytes)
    }
}

#[derive(Clone)]
struct StairCue {
    icon: &'static str,
    description: &'static str,
    color: RGB,
}

enum InputSource {
    Keyboard,
    Scripted,
}

enum RunState {
    AwaitingInput,
    PlayerTurn,
    MonsterTurn,
}

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
    seeded_floors: HashSet<u32>,
    run_stats: RunStats,
    run_max_floor: u32,
    is_dead: bool,
    reset_prompt_frame: Option<u64>,
    needs_prime_tick: bool,
    verbose: bool,
    play_history: Vec<String>,
    input_source: InputSource,
    scripted_input: Option<ScriptedInput>,
    last_player_point: Option<Point>,
    run_state: RunState,
}

impl Default for RainbowRogueState {
    fn default() -> Self {
        Self::bootstrap(RunStats::load_from_disk())
    }
}

impl GameState for RainbowRogueState {
    fn tick(&mut self, ctx: &mut BTerm) {
        self.expire_reset_prompt();
        let mut player_acted = false;
        let mut monsters_acted = false;
        let mut guard = 0;

        loop {
            guard += 1;
            if guard > 4 {
                debug_assert!(false, "turn state machine exceeded expected iterations");
                break;
            }

            match self.run_state {
                RunState::AwaitingInput => {
                    let acted = self.handle_input(ctx);
                    if acted {
                        player_acted = true;
                        self.run_state = RunState::PlayerTurn;
                        continue;
                    }
                    break;
                }
                RunState::PlayerTurn => {
                    self.run_turn(true);
                    self.run_state = RunState::MonsterTurn;
                    continue;
                }
                RunState::MonsterTurn => {
                    let has_monster_intent = self.ecs.has_monster_intent();
                    if has_monster_intent {
                        self.run_turn(false);
                        monsters_acted = true;
                    }
                    self.run_state = RunState::AwaitingInput;
                    break;
                }
            }
        }

        ctx.cls_bg(BLACK);
        self.draw_scene(ctx);

        if self.verbose && (player_acted || monsters_acted) {
            self.dump_verbose_frame(player_acted);
        }
    }
}

impl RainbowRogueState {
    fn bootstrap(meta: RunStats) -> Self {
        let args: Vec<String> = env::args().collect();
        let verbose = env::var("RR_VERBOSE")
            .map(|v| ["1", "true", "TRUE", "on", "ON"].contains(&v.as_str()))
            .unwrap_or(false)
            || args.contains(&"--verbose".to_string());

        let mut input_source = InputSource::Keyboard;
        let mut scripted_input: Option<ScriptedInput> = None;

        if let Some(script_path_idx) = args.iter().position(|arg| arg == "--scripted-input") {
            if let Some(path) = args.get(script_path_idx + 1) {
                match ScriptedInput::from_file(path) {
                    Ok(si) => {
                        scripted_input = Some(si);
                        input_source = InputSource::Scripted;
                        println!("[RR-SCRIPT] Running with scripted input from: {}", path);
                    }
                    Err(e) => {
                        eprintln!("[RR-ERROR] Failed to load script from {}: {}", path, e);
                        // Fallback to keyboard input
                    }
                }
            } else {
                eprintln!("[RR-ERROR] --scripted-input requires a path argument.");
                // Fallback to keyboard input
            }
        }

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
            seeded_floors: HashSet::new(),
            run_stats: meta,
            run_max_floor: active_floor.0,
            is_dead: false,
            reset_prompt_frame: None,
            needs_prime_tick: true,
            verbose,
            play_history: Vec::new(),
            input_source,
            scripted_input,
            last_player_point: Some(player_pos),
            run_state: RunState::AwaitingInput,
        };
        state.seed_floor_monsters(state.active_floor);
        state.record_depth(state.active_floor);
        state.update_visibility();
        state
    }

    fn handle_input(&mut self, ctx: &mut BTerm) -> bool {
        let mut consumed_turn = false;
        let key = match self.input_source {
            InputSource::Keyboard => {
                let k = ctx.key;
                ctx.key = None; // Clear BTerm's key for keyboard input
                k
            }
            InputSource::Scripted => {
                let k = self.scripted_input.as_mut().and_then(|si| si.next_key());
                if k.is_none() {
                    // If script is exhausted, signal to quit the game
                    // by returning VirtualKeyCode::Escape, which will be handled below.
                    Some(VirtualKeyCode::Escape)
                } else {
                    k
                }
            }
        };

        if let Some(key) = key {
            if self.is_dead {
                match key {
                    VirtualKeyCode::R => {
                        self.reset_run();
                        return false;
                    }
                    VirtualKeyCode::Escape => {
                        ctx.quit();
                        return false;
                    }
                    _ => return false,
                }
            }

            consumed_turn = match key {
                VirtualKeyCode::Left | VirtualKeyCode::A | VirtualKeyCode::H | VirtualKeyCode::Numpad4 => {
                    self.try_step(-1, 0)
                }
                VirtualKeyCode::Right | VirtualKeyCode::D | VirtualKeyCode::L | VirtualKeyCode::Numpad6 => {
                    self.try_step(1, 0)
                }
                VirtualKeyCode::Up | VirtualKeyCode::W | VirtualKeyCode::K | VirtualKeyCode::Numpad8 => {
                    self.try_step(0, -1)
                }
                VirtualKeyCode::Down | VirtualKeyCode::S | VirtualKeyCode::J | VirtualKeyCode::Numpad2 => {
                    self.try_step(0, 1)
                }

                // Diagonals
                VirtualKeyCode::Y | VirtualKeyCode::Numpad7 => self.try_step(-1, -1),
                VirtualKeyCode::U | VirtualKeyCode::Numpad9 => self.try_step(1, -1),
                VirtualKeyCode::B | VirtualKeyCode::Numpad1 => self.try_step(-1, 1),
                VirtualKeyCode::N | VirtualKeyCode::Numpad3 => self.try_step(1, 1),

                VirtualKeyCode::Tab => self.cycle_world(1),
                VirtualKeyCode::Back => self.cycle_world(-1),
                VirtualKeyCode::PageUp => self.shift_floor(1),
                VirtualKeyCode::PageDown => self.shift_floor(-1),
                VirtualKeyCode::Key1 => self.activate_consumable(0),
                VirtualKeyCode::Key2 => self.activate_consumable(1),
                VirtualKeyCode::Key3 => self.activate_consumable(2),
                VirtualKeyCode::Key4 => self.activate_consumable(3),
                VirtualKeyCode::R => {
                    self.handle_reset_request();
                    false
                }
                VirtualKeyCode::Escape => {
                    ctx.quit();
                    if matches!(self.input_source, InputSource::Scripted) {
                        std::process::exit(0); // Force exit for scripted runs
                    }
                    false
                }
                VirtualKeyCode::Period => {
                    // This is a "wait" command, consumes a turn but does nothing
                    true
                }
                VirtualKeyCode::T => {
                    // Step Turn command: forces a turn advancement
                    self.run_state = RunState::PlayerTurn; // Force player turn to trigger run_turn
                    true
                }
                VirtualKeyCode::P => {
                    // Dump State command: dumps current game state to verbose log
                    self.dump_current_state();
                    false // Does not consume a turn
                }
                _ => false,
            };
        }
        consumed_turn
    }

    fn dump_current_state(&self) {
        if !self.verbose {
            return;
        }
        let player_pos = self.ecs.player_point();
        println!("[RR-DEBUG] --- Current Game State ---");
        println!("[RR-DEBUG] Frame: {}, Turn: {}", self.frame, self.ecs.turn);
        println!(
            "[RR-DEBUG] Player Pos: ({}, {}), Floor: {}, World: {}",
            player_pos.x,
            player_pos.y,
            self.active_floor.0,
            self.active_world.as_str()
        );
        // Dump visible monsters
        let mut monster_positions = Vec::new();
        self.ecs.each_renderable(
            self.active_floor,
            self.active_world,
            false, // Don't include player
            |point, renderable| {
                if self.visible_tiles.contains(&point) {
                    monster_positions.push(format!(
                        "({}, {}) [{}]",
                        point.x, point.y, renderable.glyph as u8 as char
                    ));
                }
            },
        );
        if !monster_positions.is_empty() {
            println!(
                "[RR-DEBUG] Visible Monsters: {}",
                monster_positions.join(", ")
            );
        } else {
            println!("[RR-DEBUG] No visible monsters.");
        }
        println!("[RR-DEBUG] Message Log (last 3):");
        for entry in self.message_log.iter().take(3) {
            println!("[RR-DEBUG]   {}", entry);
        }
        println!("[RR-DEBUG] --------------------------");
    }

    fn run_turn(&mut self, action_taken: bool) {
        if action_taken {
            self.frame = self.frame.wrapping_add(1);
        }
        self.last_player_point = Some(self.ecs.player_point()); // Store previous player point
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
        self.needs_prime_tick = false;
    }

    fn draw_scene(&mut self, ctx: &mut BTerm) {
        let stair_cue = self.stair_cue();
        let header = format!(
            "RainbowRogue pre-alpha · Frame {} · Turn {}",
            self.frame, self.ecs.turn
        );
        ctx.print_color_centered(1, RGB::named(YELLOW), RGB::named(BLACK), &header);
        let meta_line = format!(
            "Run {} · Deepest cleared floor {}",
            self.run_stats.run_number, self.run_stats.best_depth
        );
        ctx.print_color_centered(2, RGB::named(LIGHT_GREEN), RGB::named(BLACK), &meta_line);

        let info = format!(
            "Active world: {} · Floor {}{}",
            self.active_world.as_str(),
            self.active_floor.0,
            stair_cue
                .as_ref()
                .map(|cue| format!(" · {}", cue.description))
                .unwrap_or_default()
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
        if let Some(cue) = stair_cue {
            let label = format!("{} {}", cue.icon, cue.description);
            ctx.print_color(2, 6, cue.color, RGB::named(BLACK), &label);
        }

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

            // Clear player's old position if they moved (this is now redundant with the below, but kept for clarity)
            if let Some(last_point) = self.last_player_point {
                let current_point = self.ecs.player_point();
                if last_point != current_point {
                    if let Some(tile) = layer.tile_at(last_point) {
                        let screen_x = MAP_ORIGIN_X + last_point.x;
                        let screen_y = MAP_ORIGIN_Y + last_point.y;
                        ctx.set(screen_x, screen_y, tile.fg, RGB::named(BLACK), tile.glyph);
                    }
                }
            }

            // Draw background tiles under all visible entities to ensure no lingering artifacts
            self.ecs.each_renderable(
                self.active_floor,
                self.active_world,
                true, // Include player for this pass
                |point, _|
                 {
                    if self.visible_tiles.contains(&point) {
                        if let Some(tile) = layer.tile_at(point) {
                            let screen_x = MAP_ORIGIN_X + point.x;
                            let screen_y = MAP_ORIGIN_Y + point.y;
                            ctx.set(screen_x, screen_y, tile.fg, RGB::named(BLACK), tile.glyph);
                        }
                    }
                },
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

        let (_, screen_h_raw) = ctx.get_char_size();
        let screen_h = screen_h_raw as i32;
        let log_panel_start = self.calculate_log_start(screen_h);
        draw_log(ctx, &self.message_log, log_panel_start);
        if self.is_dead {
            self.draw_game_over(ctx);
        }
    }

    fn calculate_log_start(&self, screen_height: i32) -> i32 {
        if screen_height <= 0 {
            return 0;
        }
        let min_anchor = MAP_ORIGIN_Y + 5;
        let mut start = screen_height.saturating_sub(LOG_RESERVED_ROWS + 1);
        if start < min_anchor {
            start = min_anchor;
        }
        start = start.min(screen_height.saturating_sub(2));
        start.max(1)
    }

    fn dump_verbose_frame(&self, action_taken: bool) {
        let pos = self.ecs.player_point();
        println!(
            "[RR-VERBOSE] frame={} turn={} acted={} world={} floor={} pos=({}, {})",
            self.frame,
            self.ecs.turn,
            action_taken,
            self.active_world.as_str(),
            self.active_floor.0,
            pos.x,
            pos.y
        );
        for entry in &self.message_log {
            println!("  log> {entry}");
        }
        println!("--");
    }

    fn handle_reset_request(&mut self) {
        if self.is_dead {
            return;
        }
        let confirmed = self
            .reset_prompt_frame
            .map(|frame| self.frame.saturating_sub(frame) <= RESET_CONFIRM_WINDOW_FRAMES)
            .unwrap_or(false);
        if confirmed {
            self.reset_prompt_frame = None;
            self.clear_run_stats();
        } else {
            self.reset_prompt_frame = Some(self.frame);
            self.push_log_entry(
                "Press R again within 5s to wipe run stats (resets run counter & best depth).",
            );
        }
    }

    fn cycle_world(&mut self, delta: i32) -> bool {
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
        true
    }

    fn shift_floor(&mut self, delta: i32) -> bool {
        if delta == 0 {
            return false;
        }

        if delta < 0 && self.active_floor.0 == 0 {
            self.push_log_entry("You already stand on the surface anchor.");
            return false;
        }

        let Some(tile) = self.tile_under_player() else {
            self.push_log_entry("The void yawns beneath you; no stairs here.");
            return false;
        };

        let required_tag = if delta > 0 {
            Tile::TAG_STAIR_DOWN
        } else {
            Tile::TAG_STAIR_UP
        };

        if tile.tag != required_tag {
            let msg = if delta > 0 {
                "Need to stand on a downward stair (>) to descend."
            } else {
                "Need to stand on an upward stair (<) to ascend."
            };
            self.push_log_entry(msg);
            return false;
        }

        let current = self.active_floor.0 as i32;
        let target = current + delta;
        if target < 0 {
            self.push_log_entry("The prism lock refuses to go higher.");
            return false;
        }

        let target_floor = FloorId(target as u32);
        let created = self.dungeon.ensure_floor(target_floor);
        if created {
            self.push_log_entry(format!("Floor {} takes shape.", target_floor.0));
            self.seed_floor_monsters(target_floor);
        }

        self.active_floor = target_floor;
        let arrival = self.arrival_point(delta > 0);
        self.ecs
            .set_player_position(arrival, self.active_floor, self.active_world);
        self.ecs.clear_player_intent();
        self.last_move_attempt = None;
        self.visible_tiles.clear();
        self.update_visibility();
        self.record_depth(self.active_floor);
        let verb = if delta > 0 { "Descended" } else { "Ascended" };
        self.push_log_entry(format!("{verb} to floor {}", self.active_floor.0));
        true
    }

    fn try_step(&mut self, dx: i32, dy: i32) -> bool {
        if dx == 0 && dy == 0 {
            return false;
        }

        let current = self.ecs.player_point();
        let target = Point::new(current.x + dx, current.y + dy);
        if let Some(entity) = self
            .ecs
            .entity_at(target, self.active_floor, self.active_world)
        {
            if entity != self.ecs.player_entity() {
                if let Some(report) = self.ecs.player_attack(target, self.active_floor, self.active_world) {
                    self.push_log_entry(report.hit);
                    if let Some(kill) = report.kill {
                        self.push_log_entry(kill);
                        self.ecs.queue_player_step(Point::new(dx, dy));
                        self.last_move_attempt = Some((current, target));
                    } else {
                        self.last_move_attempt = None;
                    }
                    return true;
                }
            }
        }
        self.ecs.queue_player_step(Point::new(dx, dy));
        self.last_move_attempt = Some((current, target));
        true
    }

    fn push_log_entry<S: Into<String>>(&mut self, entry: S) {
        let entry = entry.into();
        self.play_history.push(entry.clone());
        self.message_log.insert(0, entry);
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
            if stats.hp <= 0 && !self.is_dead {
                self.on_player_death();
            }
        }
    }

    fn activate_consumable(&mut self, slot_index: usize) -> bool {
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
            true
        } else {
            self.push_log_entry(format!("Slot {} is empty.", slot_index + 1));
            false
        }
    }

    fn tile_under_player(&self) -> Option<Tile> {
        let point = self.ecs.player_point();
        self.dungeon
            .active_layer(self.active_floor, self.active_world)
            .and_then(|layer| layer.tile_at(point).cloned())
    }

    fn stair_cue(&self) -> Option<StairCue> {
        self.tile_under_player().and_then(|tile| match tile.tag {
            Tile::TAG_STAIR_UP => Some(StairCue {
                icon: "^",
                description: "On < : PageDown to ascend",
                color: RGB::named(LIGHT_GREEN),
            }),
            Tile::TAG_STAIR_DOWN => Some(StairCue {
                icon: "v",
                description: "On > : PageUp to descend",
                color: RGB::named(ORANGE),
            }),
            _ => None,
        })
    }

    fn expire_reset_prompt(&mut self) {
        if let Some(frame) = self.reset_prompt_frame {
            if self.frame.saturating_sub(frame) > RESET_CONFIRM_WINDOW_FRAMES {
                self.reset_prompt_frame = None;
            }
        }
    }

    fn clear_run_stats(&mut self) {
        let _ = fs::remove_file(RUN_STATS_PATH);
        self.run_stats = RunStats::default();
        self.run_max_floor = self.active_floor.0;
        self.persist_run_stats();
        self.push_log_entry("Run stats reset. Run counter back to 1.");
    }

    fn arrival_point(&self, descending: bool) -> Point {
        if let Some(floor) = self.dungeon.active_floor(self.active_floor) {
            let anchor = if descending {
                floor.stairs_up().first()
            } else {
                floor.stairs_down().first()
            };
            if let Some(point) = anchor {
                return *point;
            }
            return floor.spawn_point();
        }
        Point::new(1, 1)
    }

    fn persist_run_stats(&self) {
        if let Err(err) = self.run_stats.persist_to_disk() {
            eprintln!("Failed to persist run stats: {err}");
        }
    }

    fn record_depth(&mut self, floor: FloorId) {
        self.run_max_floor = self.run_max_floor.max(floor.0);
        self.run_stats.best_depth = self.run_stats.best_depth.max(self.run_max_floor);
        self.persist_run_stats();
    }

    fn seed_floor_monsters(&mut self, floor_id: FloorId) {
        if self.seeded_floors.contains(&floor_id.0) {
            return;
        }
        let mut rng = RandomNumberGenerator::seeded(0xdead_beef ^ floor_id.0 as u64);
        if let Some(floor) = self.dungeon.active_floor(floor_id) {
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
                let spawn_target = (walkable.len() / 90).max(2).min(6);
                let mut spawned = 0;
                while spawned < spawn_target && !walkable.is_empty() {
                    let idx = rng.range(0, walkable.len() as i32) as usize;
                    let point = walkable.swap_remove(idx);
                    if self.ecs.entity_at(point, floor_id, world).is_some()
                        || (floor_id == self.active_floor
                            && world == self.active_world
                            && point == self.ecs.player_point())
                    {
                        continue;
                    }
                    let template_idx = rng.range(0, templates.len() as i32) as usize;
                    let template = templates[template_idx].clone();
                    self.ecs.spawn_monster(&template, point, floor_id, world);
                    spawned += 1;
                }
            }
        }
        self.seeded_floors.insert(floor_id.0);
    }

    fn on_player_death(&mut self) {
        self.is_dead = true;
        self.ecs.clear_player_intent();
        self.last_move_attempt = None;
        self.run_stats.best_depth = self.run_stats.best_depth.max(self.run_max_floor);
        self.persist_run_stats();
        self.push_log_entry("Your spectrum shatters. Press R to restart or Esc to quit.");
    }

    fn reset_run(&mut self) {
        let mut next_stats = self.run_stats.clone();
        next_stats.best_depth = next_stats.best_depth.max(self.run_max_floor);
        next_stats.run_number = next_stats.run_number.saturating_add(1);
        *self = Self::bootstrap(next_stats);
        self.persist_run_stats();
        self.push_log_entry(format!(
                    "Run {} anchors. Best depth {}",
                    self.run_stats.run_number, self.run_stats.best_depth
                ));
    }

    fn draw_game_over(&self, ctx: &mut BTerm) {
        let (width_raw, height_raw) = ctx.get_char_size();
        let screen_h = height_raw as i32;
        let banner = "S P E C T R U M   S H A T T E R E D";
        let hint = "Press R to restart or Esc to quit.";
        let box_top = (MAP_ORIGIN_Y + 6).min(screen_h.saturating_sub(8));
        let box_height = 6.min(screen_h.saturating_sub(box_top).saturating_sub(1).max(3));
        ctx.draw_box(
            1,
            box_top,
            width_raw.saturating_sub(2),
            box_height,
            RGB::named(DARK_GRAY),
            RGB::named(BLACK),
        );
        let banner_y = (box_top + 2).min(screen_h.saturating_sub(2));
        let hint_y = (banner_y + 2).min(screen_h.saturating_sub(1));
        ctx.print_color_centered(banner_y, RGB::named(RED), RGB::named(BLACK), banner);
        ctx.print_color_centered(hint_y, RGB::named(WHITE), RGB::named(BLACK), hint);
    }
}

impl Drop for RainbowRogueState {
    fn drop(&mut self) {
        if !self.verbose {
            return;
        }
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let filename = format!("history/play_history_{}.log", timestamp);
        if let Err(e) = fs::create_dir_all("history") {
            eprintln!("Failed to create history directory: {}", e);
            return;
        }
        if let Err(e) = fs::write(&filename, self.play_history.join("\n")) {
            eprintln!("Failed to write play history to {}: {}", filename, e);
        } else {
            println!("Play history saved to {}", filename);
        }
    }
}

fn main() -> BError {
    let args: Vec<String> = env::args().collect();
    let is_scripted = args.iter().any(|arg| arg == "--scripted-input");

    let (console_width, console_height) = console_dimensions(is_scripted);
    let context = BTermBuilder::simple(console_width, console_height)?
        .with_title("RainbowRogue · Spectrum Seed")
        .with_font("vga8x16.png", 8, 16)
        .with_tile_dimensions(8, 16)

        .build()?;

    let game_state = RainbowRogueState::default();
    main_loop(context, game_state)
}

fn console_dimensions(_is_scripted: bool) -> (i32, i32) {
    (132, 43)
}