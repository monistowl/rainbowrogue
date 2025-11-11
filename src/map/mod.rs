#![allow(dead_code)]

use bracket_geometry::prelude::{Point, Rect};
use bracket_random::prelude::RandomNumberGenerator;
use bracket_terminal::prelude::{BLACK, RGB};

pub const DEFAULT_MAP_WIDTH: i32 = 80;
pub const DEFAULT_MAP_HEIGHT: i32 = 48;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum World {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Indigo,
    Violet,
}

impl Default for World {
    fn default() -> Self {
        World::Red
    }
}

pub fn world_color(world: World) -> RGB {
    match world {
        World::Red => RGB::from_u8(255, 95, 86),
        World::Orange => RGB::from_u8(255, 170, 64),
        World::Yellow => RGB::from_u8(241, 241, 87),
        World::Green => RGB::from_u8(126, 211, 33),
        World::Blue => RGB::from_u8(96, 165, 255),
        World::Indigo => RGB::from_u8(120, 98, 240),
        World::Violet => RGB::from_u8(193, 126, 255),
    }
}

impl World {
    pub fn as_str(&self) -> &'static str {
        match self {
            World::Red => "Red",
            World::Orange => "Orange",
            World::Yellow => "Yellow",
            World::Green => "Green",
            World::Blue => "Blue",
            World::Indigo => "Indigo",
            World::Violet => "Violet",
        }
    }

    pub fn spectrum_index(&self) -> usize {
        match self {
            World::Red => 0,
            World::Orange => 1,
            World::Yellow => 2,
            World::Green => 3,
            World::Blue => 4,
            World::Indigo => 5,
            World::Violet => 6,
        }
    }

    pub fn cycle(self, delta: i32) -> Self {
        let idx = self.spectrum_index() as i32;
        let next = (idx + delta).rem_euclid(SPECTRUM.len() as i32) as usize;
        SPECTRUM[next]
    }
}

pub const SPECTRUM: [World; 7] = [
    World::Red,
    World::Orange,
    World::Yellow,
    World::Green,
    World::Blue,
    World::Indigo,
    World::Violet,
];

fn corridor_path(start: Point, end: Point) -> Vec<Point> {
    let mut path = Vec::new();
    let mut cursor = start;
    path.push(cursor);

    while cursor.x != end.x {
        cursor.x += if end.x > cursor.x { 1 } else { -1 };
        path.push(cursor);
    }

    while cursor.y != end.y {
        cursor.y += if end.y > cursor.y { 1 } else { -1 };
        path.push(cursor);
    }

    if *path.last().unwrap() != end {
        path.push(end);
    }

    path
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FloorId(pub u32);

#[derive(Clone, Debug)]
pub struct Substrate {
    pub width: i32,
    pub height: i32,
    pub rooms: Vec<Rect>,
    pub corridors: Vec<Vec<Point>>,
    pub stairs_up: Vec<Point>,
    pub stairs_down: Vec<Point>,
    pub spawn: Point,
}

impl Substrate {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            rooms: Vec::new(),
            corridors: Vec::new(),
            stairs_up: Vec::new(),
            stairs_down: Vec::new(),
            spawn: Point::new(width / 2, height / 2),
        }
    }

    pub fn procedural(width: i32, height: i32, seed: u64) -> Self {
        const MAX_ROOMS: usize = 24;
        const MIN_ROOM_W: i32 = 6;
        const MAX_ROOM_W: i32 = 14;
        const MIN_ROOM_H: i32 = 5;
        const MAX_ROOM_H: i32 = 10;

        let mut rng = RandomNumberGenerator::seeded(seed);
        let mut substrate = Self::new(width, height);

        for _ in 0..MAX_ROOMS {
            let room_w = rng.range(MIN_ROOM_W, MAX_ROOM_W);
            let room_h = rng.range(MIN_ROOM_H, MAX_ROOM_H);
            if room_w >= width - 4 || room_h >= height - 4 {
                continue;
            }

            let x_max = width - room_w - 2;
            let y_max = height - room_h - 2;
            if x_max <= 2 || y_max <= 2 {
                continue;
            }

            let room_x = rng.range(2, x_max);
            let room_y = rng.range(4, y_max);
            let candidate = Rect::with_size(room_x, room_y, room_w, room_h);

            if substrate
                .rooms
                .iter()
                .any(|room| room.intersect(&candidate))
            {
                continue;
            }

            let candidate_center = candidate.center();
            if let Some(prev_center) = substrate.rooms.last().map(|room| room.center()) {
                substrate
                    .corridors
                    .push(corridor_path(prev_center, candidate_center));
            } else {
                substrate.spawn = candidate_center;
                substrate.stairs_up = vec![candidate_center];
            }

            substrate.rooms.push(candidate);
        }

        if let Some(last_room) = substrate.rooms.last() {
            substrate.stairs_down = vec![last_room.center()];
        }

        if substrate.rooms.is_empty() {
            Self::demo_layout(width, height)
        } else {
            substrate
        }
    }

    pub fn demo_layout(width: i32, height: i32) -> Self {
        let mut substrate = Self::new(width, height);
        let room_width = 12;
        let room_height = 8;
        let mut x = 2;
        while x + room_width < width - 2 {
            let room = Rect::with_size(x, 8, room_width, room_height);
            substrate.rooms.push(room);
            x += room_width + 3;
        }

        if substrate.rooms.is_empty() {
            substrate
                .rooms
                .push(Rect::with_size(2, 8, width.saturating_sub(4), room_height));
        }

        substrate.spawn = substrate.rooms[0].center();
        substrate.stairs_up.push(substrate.rooms[0].center());
        let exit_point = substrate
            .rooms
            .last()
            .map(|rect| rect.center())
            .unwrap_or(substrate.spawn);
        substrate.stairs_down.push(exit_point);

        for window in substrate.rooms.windows(2) {
            let start = window[0].center();
            let end = window[1].center();
            substrate.corridors.push(corridor_path(start, end));
        }

        substrate
    }
}

#[derive(Clone, Debug)]
pub struct Tile {
    pub glyph: u16,
    pub fg: RGB,
    pub bg: RGB,
    pub blocks_move: bool,
    pub blocks_sight: bool,
    pub tag: u32,
    pub revealed: bool,
}

impl Default for Tile {
    fn default() -> Self {
        Tile::wall()
    }
}

impl Tile {
    pub fn wall() -> Self {
        Self {
            glyph: b'#' as u16,
            fg: RGB::from_u8(90, 90, 90),
            bg: RGB::named(BLACK),
            blocks_move: true,
            blocks_sight: true,
            tag: 0,
            revealed: false,
        }
    }

    pub fn floor(world: World) -> Self {
        Self {
            glyph: b'.' as u16,
            fg: world_color(world),
            bg: RGB::named(BLACK),
            blocks_move: false,
            blocks_sight: false,
            tag: 1,
            revealed: false,
        }
    }

    pub fn stair_up(world: World) -> Self {
        Self {
            glyph: b'<' as u16,
            fg: world_color(world),
            bg: RGB::named(BLACK),
            blocks_move: false,
            blocks_sight: false,
            tag: 2,
            revealed: false,
        }
    }

    pub fn stair_down(world: World) -> Self {
        Self {
            glyph: b'>' as u16,
            fg: world_color(world),
            bg: RGB::named(BLACK),
            blocks_move: false,
            blocks_sight: false,
            tag: 3,
            revealed: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MapLayer {
    pub world: World,
    pub width: i32,
    pub height: i32,
    pub tiles: Vec<Tile>,
}

impl MapLayer {
    pub fn empty(world: World, width: i32, height: i32) -> Self {
        let size = (width * height) as usize;
        Self {
            world,
            width,
            height,
            tiles: vec![Tile::default(); size],
        }
    }

    pub fn from_substrate(world: World, substrate: &Substrate) -> Self {
        let mut layer = Self::empty(world, substrate.width, substrate.height);
        layer.tiles.iter_mut().for_each(|tile| *tile = Tile::wall());

        for room in &substrate.rooms {
            room.for_each(|pt| layer.paint_floor(pt));
        }

        for corridor in &substrate.corridors {
            for &pt in corridor {
                layer.paint_floor(pt);
            }
        }

        for &stair in &substrate.stairs_up {
            layer.set_tile(stair, Tile::stair_up(world));
        }

        for &stair in &substrate.stairs_down {
            layer.set_tile(stair, Tile::stair_down(world));
        }

        layer
    }

    fn idx(&self, x: i32, y: i32) -> Option<usize> {
        if self.in_bounds(Point::new(x, y)) {
            Some((y * self.width + x) as usize)
        } else {
            None
        }
    }

    pub fn in_bounds(&self, point: Point) -> bool {
        point.x >= 0 && point.x < self.width && point.y >= 0 && point.y < self.height
    }

    fn paint_floor(&mut self, point: Point) {
        self.set_tile(point, Tile::floor(self.world));
    }

    pub fn set_tile(&mut self, point: Point, tile: Tile) {
        if let Some(idx) = self.idx(point.x, point.y) {
            self.tiles[idx] = tile;
        }
    }

    pub fn tile_at(&self, point: Point) -> Option<&Tile> {
        self.idx(point.x, point.y).map(|idx| &self.tiles[idx])
    }

    pub fn tile_at_mut(&mut self, point: Point) -> Option<&mut Tile> {
        self.idx(point.x, point.y).map(|idx| &mut self.tiles[idx])
    }

    pub fn reveal_point(&mut self, point: Point) {
        if let Some(tile) = self.tile_at_mut(point) {
            tile.revealed = true;
        }
    }

    pub fn is_walkable(&self, point: Point) -> bool {
        self.tile_at(point).map_or(false, |tile| !tile.blocks_move)
    }

    pub fn first_walkable(&self) -> Point {
        for y in 0..self.height {
            for x in 0..self.width {
                let point = Point::new(x, y);
                if self.is_walkable(point) {
                    return point;
                }
            }
        }
        Point::new(0, 0)
    }

    pub fn walkable_points(&self) -> Vec<Point> {
        let mut points = Vec::new();
        for y in 0..self.height {
            for x in 0..self.width {
                let point = Point::new(x, y);
                if self.is_walkable(point) {
                    points.push(point);
                }
            }
        }
        points
    }
}

#[derive(Clone, Debug)]
pub struct WorldFloor {
    pub id: FloorId,
    pub substrate: Substrate,
    pub layers: [MapLayer; 7],
}

impl WorldFloor {
    pub fn empty(id: FloorId, width: i32, height: i32) -> Self {
        let substrate = Substrate::new(width, height);
        let layers = std::array::from_fn(|idx| MapLayer::empty(SPECTRUM[idx], width, height));
        Self {
            id,
            substrate,
            layers,
        }
    }

    pub fn demo(id: FloorId, width: i32, height: i32) -> Self {
        let seed = id.0 as u64 + 1;
        let substrate = Substrate::procedural(width, height, seed);
        Self::from_substrate(id, substrate)
    }

    pub fn from_seed(id: FloorId, width: i32, height: i32, seed: u64) -> Self {
        let substrate = Substrate::procedural(width, height, seed);
        Self::from_substrate(id, substrate)
    }

    fn from_substrate(id: FloorId, substrate: Substrate) -> Self {
        let layers = std::array::from_fn(|idx| MapLayer::from_substrate(SPECTRUM[idx], &substrate));
        Self {
            id,
            substrate,
            layers,
        }
    }

    pub fn layer(&self, world: World) -> &MapLayer {
        let idx = world.spectrum_index();
        &self.layers[idx]
    }

    pub fn layer_mut(&mut self, world: World) -> &mut MapLayer {
        let idx = world.spectrum_index();
        &mut self.layers[idx]
    }

    pub fn spawn_point(&self) -> Point {
        self.substrate.spawn
    }
}

#[derive(Clone, Debug, Default)]
pub struct Dungeon {
    pub floors: Vec<WorldFloor>,
}

impl Dungeon {
    pub fn scaffolding_demo() -> Self {
        let floor = WorldFloor::demo(FloorId(0), DEFAULT_MAP_WIDTH, DEFAULT_MAP_HEIGHT);
        Self {
            floors: vec![floor],
        }
    }

    pub fn active_floor(&self, floor: FloorId) -> Option<&WorldFloor> {
        self.floors.get(floor.0 as usize)
    }

    pub fn active_layer(&self, floor: FloorId, world: World) -> Option<&MapLayer> {
        self.active_floor(floor).map(|wf| wf.layer(world))
    }

    pub fn active_layer_mut(&mut self, floor: FloorId, world: World) -> Option<&mut MapLayer> {
        self.floors
            .get_mut(floor.0 as usize)
            .map(|wf| wf.layer_mut(world))
    }

    pub fn spawn_point(&self, floor: FloorId) -> Point {
        self.active_floor(floor)
            .map(|wf| wf.spawn_point())
            .unwrap_or(Point::new(1, 1))
    }

    pub fn is_walkable(&self, floor: FloorId, world: World, point: Point) -> bool {
        self.active_layer(floor, world)
            .map(|layer| layer.is_walkable(point))
            .unwrap_or(false)
    }
}
