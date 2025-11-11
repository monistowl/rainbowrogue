# RainbowRogue
![image](./screenshot.png)
_Console-native roguelike exploring a seven-world (4D) dungeon rendered with `bracket-lib`._

## Description
RainbowRogue drops you into a stacked dungeon where every floor exists simultaneously across seven attunement planes: Red, Orange, Yellow, Green, Blue, Indigo, and Violet. Each plane is its own tactical layer with bespoke hazards, monster templates, and traversal rules. You can phase between worlds mid-run, scout overlapping geometries, and line up multi-world plays—stepping through a smoldering Red corridor before blinking into Indigo for a flank, or freezing foes in Blue so their echoes in Violet arrive staggered.

### Core pillars
- **Spectrum shifting**: Cycle worlds on demand to discover alternate tile sets, portals, and monster behaviors sharing the same substrate.
- **Floor and world mobility**: Climb floors via stairs while keeping your current world, or phase worlds on the fly while staying anchored to a floor.
- **ECS-driven encounters**: Specs-powered components drive AI, visibility, quick actions, and cross-world synchronization so every action can ripple between planes.
- **Readable, moddable data**: Monsters, world rules, and procedural substrates live in `src/data` so you can iterate on balance without touching the loop.

## Quick Start
### Prerequisites
- Rust toolchain (1.80+ recommended; install via [rustup](https://rustup.rs)).
- A Unicode-friendly terminal; `bracket-terminal` defaults to crossterm on desktop.

### Build & run
```bash
# clone and enter the repo
git clone https://github.com/<your-org>/rainbowrogue.git
cd rainbowrogue

# run in debug (fast iteration)
cargo run

# for smoother animation and optimized AI ticks
cargo run --release
```
_The window opens at 80x50 characters; resize your terminal beforehand for best results._

### Troubleshooting
- On macOS, grant the terminal “Input Monitoring” to ensure crossterm receives PageUp/PageDown events.
- If fonts render incorrectly, force ASCII glyph mode by exporting `BRACKET_ASCII_FONT=1` before launching.

## Player Manual
### Objective
Descend through as many dungeon floors as you can, unify the spectrum by activating portals in each world, and survive the cross-world ecosystems trying to collapse the lattice.

### Controls
| Action | Keys |
| --- | --- |
| Move (orthogonal) | Arrow keys, `WASD`, or `HJKL` |
| Cycle worlds forward/backward | `Tab` / `Backspace` |
| Change dungeon floor | `PageUp` (downward) / `PageDown` (upward) |
| Use quickbar item slots | `1`–`4` |
| Close the game | `Esc` or close the terminal window |

Tips:
- Movement spends a turn even if blocked; watch the combat log to know whether you bumped an enemy or a wall.
- Cycling worlds re-centers AI intent, so you can shake pursuit or force monsters to rematerialize on safer tiles.

### HUD & feedback
- **Top banner**: Shows build tag, current frame, and overall turn counter.
- **World + floor readout**: Highlights your active world name and floor index.
- **Vitality line**: Displays HP in color-coded text (orange warning ≤60%, flashing alert ≤30%).
- **HUD ring**: Seven wedges represent ROYGBIV worlds with cooldown pips and modifiers.
- **Quickbar**: Appears on row 5 with `[slot] name (uses)` entries for consumables bound to keys `1`–`4`.
- **Message log**: Bottom six rows narrate movement, discoveries, combat rolls, and health warnings.

### Traversal & combat
1. **Substrate awareness**: Every floor shares geometry across worlds; walls in one plane might be passable or hazardous in another.
2. **Portals vs. stairs**: Stairs move floors but preserve your current world. Portals (and attunements) swap worlds while staying on the same floor.
3. **Visibility**: Exploring reveals tiles per-world. Swapping worlds can expose unseen tiles even on rooms you already visited.
4. **Monsters**: Each world seeds its own monster templates; leverage vulnerabilities (e.g., frost-stalled Blue mobs, psychic Indigo casters).
5. **Consumables**: Slots trigger instant abilities (heals, prisms, buffs). When empty, the log will remind you the slot is vacant.

### Progression pointers
- Revisit earlier floors after unlocking more worlds to loot gated rooms or rescue stranded portals.
- Keep an eye on the log after shifts—newly visible tile counts hint at unexplored branches.
- If HP warnings trigger, stabilize in a safer world (Green regen zones, Yellow visibility) before diving back into harsher planes.

## Contributing
See `ROADMAP.md` for the staged milestone breakdown and use `bd` (beads) issues for task tracking. Planning documents belong in `history/` if you generate new ones.
