# RainbowRogue Scaffolding — 2025-11-10

## Context
- Reviewed `ROADMAP.md` to align modules with planned milestones.
- Used Bracket Productions tutorial patterns (console loop, `BTermBuilder::simple80x50`, spectrum HUD) as inspiration for the initial terminal harness.

## Work Completed
- Initialized `Cargo` binary crate with resolver `2` and all required `bracket-*` dependencies (`bracket-terminal`, `bracket-geometry`, `bracket-pathfinding`, `bracket-noise`, `bracket-random`).
- Added foundational modules: `ecs/`, `map/`, `render/`, `ai/`, and `data/` with type shells matching roadmap tables (World enum, ECS components, HUD helpers).
- Implemented `RainbowRogueState` game loop scaffold that:
  - Opens a crossterm-driven 80×50 console window.
  - Tracks active world/floor state, message log, and placeholder ECS turn counter.
  - Provides world cycling and floor shifting inputs (Tab, Backspace, PageUp/PageDown).
  - Renders a temporary HUD ring plus event log placeholder.
- Created demo dungeon/substrate constructors so later systems can plug in deterministic generation.

## Outstanding Follow-Ups
1. Install system dependencies (`fontconfig`, `X11`, `pkg-config`) or switch to a pure `crossterm` backend that avoids the `winit` stack so `cargo check` passes locally.
2. Replace stub ECS world with Specs/Legion per roadmap Milestone M1.
3. Flesh out map generation to populate `Substrate` rooms/corridors and feed render systems.
4. Port early tutorial chapters (input handling, player movement) from Bracket Productions into the new module layout.
