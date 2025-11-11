# rainbowrogue · Development Roadmap
_Seven-world 4D roguelike built on `bracket-lib` (Rust)_


## OVERVIEW
- **Engine**: Rust + `bracket-lib` (terminal, pathfinding, noise, random)
- **Core ECS**: Specs (or Legion later)
- **Concept**: Seven parallel worlds — Red, Orange, Yellow, Green, Blue, Indigo, Violet — each a full roguelike layer of the same dungeon substrate.  
- **Player**: chooses a **home world** at character creation.  
- **Traversal**: between-world via portals, items, or attunements; between-floor via stairs.  
- **Goal**: reach the deepest layer and unify the spectrum.

---

## ARCHITECTURE

### Core modules
| Module | Description |
|---------|-------------|
| `ecs/` | Components, systems, resources |
| `map/` | Floor generation, substrate, per-world layers |
| `render/` | Drawing, HUD ring, colorblind support |
| `ai/` | Behavior trees, Dijkstra/flee/pursuit systems |
| `data/` | RON definitions for monsters, items, world rules |
| `main.rs` | GameState loop, tick() dispatcher |

### Core data types
```rust
enum World { Red, Orange, Yellow, Green, Blue, Indigo, Violet }
struct FloorId(pub u32);

struct Substrate { width: i32, height: i32, rooms: Vec<Rect>, corridors: Vec<Vec<Point>>, stairs_up: Vec<Point>, stairs_down: Vec<Point> }

struct Tile { glyph: u16, fg: RGB, bg: RGB, blocks_move: bool, blocks_sight: bool, tag: u32 }

struct MapLayer { world: World, tiles: Vec<Tile> }
struct WorldFloor { id: FloorId, substrate: Substrate, layers: [MapLayer; 7] }

struct Dungeon { floors: Vec<WorldFloor> }
````

### ECS key components

* `Position { x, y, floor: FloorId, world: World }`
* `Renderable { glyph, color, order }`
* `Viewshed { radius, dirty, visible }`
* `Actor { energy, speed }`
* `Portal { to_world, cost, key_mask, cooldown }`
* `WorldAffinity { primary, resist, vuln }`
* `PlaneAttunements { unlocked: Vec<World>, perks: u64 }`
* `ConcordanceId(u64)` — link echo entities across worlds

### Systems

* `InputSystem` → `IntentSystem` → `MovementSystem`
* `FovSystem(world)` → `AiSystem(world)` → `CombatSystem`
* `WorldShiftSystem` handles portal traversal & cooldowns
* `RenderSystem` reads `Dungeon.floors[floor].layers[world]`

---

## WORLD DESIGN

### Thematic mechanics

| World  | Traits / Hazards      | Example Perks           |
| ------ | --------------------- | ----------------------- |
| Red    | Heat, lava, eruptions | Fire resist, heat aura  |
| Orange | Chemistry, gas, acid  | Explosive concoctions   |
| Yellow | Light, insight        | Trap-sense, wider FOV   |
| Green  | Growth, life          | Regen zones, vine doors |
| Blue   | Cold, depth           | Slow projectiles, crits |
| Indigo | Mind, space           | Blink, teleport nodes   |
| Violet | Arcane, curse         | Phase armor, cursecraft |

### Traversal rules

* **Move (x,y)** within current world layer.
* **Stairs**: change `floor` (same world).
* **Portals**: change `world` (same floor).
* **Prisms/Abilities**: self-shift, limited range or cooldown.

---

## IMPLEMENTATION ROADMAP

### M0 — PROJECT SCAFFOLD

* [ ] Create Cargo project `rainbowrogue`
* [ ] Add `bracket-lib` dependencies (`bracket-terminal`, `bracket-geometry`, `bracket-pathfinding`, `bracket-noise`, `bracket-random`)
* [ ] Basic console window, game loop, state machine
* [ ] GitHub CI & web (wasm) target

### M1 — SINGLE-WORLD MVP

* [ ] Implement ECS skeleton with Specs
* [ ] Map generator (BSP or drunkard walk)
* [ ] Player, movement, stairs, FOV, basic monsters
* [ ] Combat, death, log, save/load

### M2 — MULTI-FLOOR & ECS REFACTOR

* [ ] Intent/action queue system
* [ ] Inventory, items, pickups
* [ ] Pathfinding & simple AI
* [ ] Refactor to use `FloorId`

### M3 — WORLD FRAMEWORK

* [ ] Add `World` enum (R,O,Y,G,B,I,V)
* [ ] Create `Substrate + MapLayer[7]` model
* [ ] Rendering reads from active world layer
* [ ] HUD ring showing seven-world status
* [ ] Debug controls to switch worlds

### M4 — PORTALS & SHIFTING

* [ ] Implement `Portal` entities and `WorldShiftSystem`
* [ ] Portal peek rendering (preview destination cell)
* [ ] Portal cooldowns & energy costs
* [ ] Dev hotkey (`Tab`) to cycle worlds for testing

### M5 — WORLD MATERIALIZATION

* [ ] Deterministic per-world transforms over substrate
* [ ] Apply Perlin noise or CA to stamp hazards
* [ ] Implement 2–3 unique hazards per world
* [ ] Introduce world-affine mobs

### M6 — LEVEL GENERATION PIPELINE

* [ ] Generate substrate once per floor
* [ ] Materialize 7 MapLayers using rule packs
* [ ] Add paired portals (adjacent colors by default)
* [ ] Guarantee connectivity & softlock prevention

### M7 — CHARACTER GENERATION & ATTUNEMENTS

* [ ] Character creation: choose starting world
* [ ] Assign starting items & perks by world
* [ ] Implement attunement progression system
* [ ] Add world-key and prism item archetypes

### M8 — AI & WORLD ECOSYSTEMS

* [ ] Extend AI to handle world affinity/hostility
* [ ] Add cross-world elite behavior (limited pursuit)
* [ ] Implement regenerative ecology (Green) and spreading hazards (Red)

### M9 — UI/UX & ACCESSIBILITY

* [ ] Full ROYGBIV HUD ring with cooldown pips
* [ ] World-preview ghost mode (`[` `]`)
* [ ] Colorblind-safe glyph sets / icons
* [ ] Particle text & log polish

### M10 — PACKAGING & MOD SUPPORT

* [ ] Serialize `Dungeon` + entities to RON/JSON
* [ ] WebAssembly build + itch.io test
* [ ] Data-driven rules (`data/world_rules.ron`)
* [ ] CLI tools for map/asset previews

---

## TESTING & QA

### Invariants

* [ ] Alignment: portals never land on blocked tiles.
* [ ] Connectivity: each world floor is fully connected.
* [ ] Safety: each floor has at least one reachable portal.
* [ ] Performance: FOV ≤ 2 ms, render ≤ 2 ms per tick.

### Test targets

* Unit tests for FOV/world shift
* Property tests for portal routing & seed determinism
* Benchmarks for Dijkstra & render timing

---

## STRETCH GOALS

* [ ] Non-Euclidean Indigo corridors (wraparound)
* [ ] Violet curse-socket crafting system
* [ ] Replay ghost (“echo” of your past run)
* [ ] Steam integration, achievements

---

## LICENSE & REFERENCES

* Code: MIT / Apache-2.0 dual license
* Built on [`bracket-lib`](https://github.com/amethyst/bracket-lib)
* Inspired by [Bracket Productions roguelike tutorial](https://bfnightly.bracketproductions.com/)
