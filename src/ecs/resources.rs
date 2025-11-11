#![allow(dead_code)]

use bracket_geometry::prelude::Point;

use crate::map::{FloorId, MapLayer, World};

#[derive(Clone)]
pub struct MovementContext {
    pub floor: FloorId,
    pub world: World,
    pub player_point: Point,
    pub width: i32,
    pub height: i32,
    walkable: Vec<bool>,
    blocks_sight: Vec<bool>,
}

impl MovementContext {
    pub fn from_layer(layer: &MapLayer, floor: FloorId, world: World, player_point: Point) -> Self {
        let walkable = layer
            .tiles
            .iter()
            .map(|tile| !tile.blocks_move)
            .collect::<Vec<bool>>();
        let blocks_sight = layer
            .tiles
            .iter()
            .map(|tile| tile.blocks_sight)
            .collect::<Vec<bool>>();

        Self {
            floor,
            world,
            player_point,
            width: layer.width,
            height: layer.height,
            walkable,
            blocks_sight,
        }
    }

    pub fn is_walkable(&self, point: Point) -> bool {
        if point.x < 0 || point.x >= self.width || point.y < 0 || point.y >= self.height {
            return false;
        }
        let idx = (point.y * self.width + point.x) as usize;
        self.walkable.get(idx).copied().unwrap_or(false)
    }

    pub fn blocks_sight(&self, point: Point) -> bool {
        if point.x < 0 || point.x >= self.width || point.y < 0 || point.y >= self.height {
            return true;
        }
        let idx = (point.y * self.width + point.x) as usize;
        self.blocks_sight.get(idx).copied().unwrap_or(true)
    }

    pub fn in_bounds(&self, point: Point) -> bool {
        point.x >= 0 && point.x < self.width && point.y >= 0 && point.y < self.height
    }
}

#[derive(Default)]
pub struct CombatLog {
    pub entries: Vec<String>,
}

impl CombatLog {
    pub fn push<S: Into<String>>(&mut self, entry: S) {
        self.entries.push(entry.into());
    }
}
