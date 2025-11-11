#![allow(dead_code)]

use std::collections::HashSet;

use bracket_geometry::prelude::Point;
use bracket_terminal::prelude::*;

use crate::map::{FloorId, MapLayer, SPECTRUM, World};

pub struct HudRing;

impl HudRing {
    pub const fn new() -> Self {
        Self
    }

    pub fn draw(&self, ctx: &mut BTerm, active_world: World, active_floor: FloorId, frame: u64) {
        let (width, _) = ctx.get_char_size();
        ctx.draw_box(0, 0, width - 1, 6, RGB::named(GRAY), RGB::named(BLACK));
        ctx.print_color(
            2,
            1,
            RGB::named(WHITE),
            RGB::named(BLACK),
            format!("Spectrum HUD · Floor {}", active_floor.0),
        );
        ctx.print_color(
            2,
            2,
            RGB::named(LIGHT_BLUE),
            RGB::named(BLACK),
            format!("Frame {}", frame),
        );

        for (idx, world) in SPECTRUM.iter().enumerate() {
            let x = 2 + (idx as i32 * 10);
            let (fg, glyph) = if *world == active_world {
                (RGB::named(LIGHT_GREEN), '*')
            } else {
                (RGB::named(DARK_GRAY), '·')
            };
            ctx.set(x, 4, fg, RGB::named(BLACK), glyph as u16);
            ctx.print_color(x + 2, 4, fg, RGB::named(BLACK), world.as_str());
        }
    }
}

pub fn draw_log(ctx: &mut BTerm, log: &[String], start_y: i32) {
    let (width, _) = ctx.get_char_size();
    let height = (log.len() as i32).min(5) + 2;
    let top = (start_y - 1).max(0);
    ctx.draw_box(
        0,
        top,
        width - 1,
        height,
        RGB::named(DARK_GRAY),
        RGB::named(BLACK),
    );
    ctx.print_color(
        2,
        top + 1,
        RGB::named(WHITE),
        RGB::named(BLACK),
        "Event Log",
    );
    for (row, entry) in log.iter().take(5).enumerate() {
        ctx.print(2, top + 2 + row as i32, entry);
    }
}

pub fn draw_map(
    ctx: &mut BTerm,
    layer: &MapLayer,
    map_origin: Point,
    reserved_rows: i32,
    visible: &HashSet<Point>,
) {
    let (screen_w, screen_h) = ctx.get_char_size();
    let screen_w = screen_w as i32;
    let screen_h = screen_h as i32;
    let max_draw_y = screen_h - reserved_rows;
    let max_draw_x = screen_w - 2;

    for y in 0..layer.height {
        let screen_y = map_origin.y + y;
        if screen_y >= max_draw_y {
            break;
        }
        for x in 0..layer.width {
            let screen_x = map_origin.x + x;
            if screen_x >= max_draw_x {
                break;
            }
            let point = Point::new(x, y);
            if let Some(tile) = layer.tile_at(point) {
                if visible.contains(&point) {
                    ctx.set(screen_x, screen_y, tile.fg, tile.bg, tile.glyph);
                } else if tile.revealed {
                    ctx.set(
                        screen_x,
                        screen_y,
                        RGB::named(DARK_GRAY),
                        RGB::named(BLACK),
                        tile.glyph,
                    );
                } else {
                    ctx.set(
                        screen_x,
                        screen_y,
                        RGB::named(BLACK),
                        RGB::named(BLACK),
                        b' ' as u16,
                    );
                }
            }
        }
    }
}
