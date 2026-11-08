# RUBx

A Rubik's cube simulator built with Rust and [Bevy](https://bevy.org) 0.20.

```sh
cargo run --release   # the first build takes several minutes
```

## Modes

**Training** is the heart of the game, with four activities:

- **Learn to solve:** eight lessons on the beginner layer-by-layer method, from reading notation to
  the last step. Each stage lesson explains the idea, then has you finish three generated positions.
- **Coach:** solve a scramble your own way, with a checklist of stages and hints when you want them.
- **Algorithm drills:** recognise and solve OLL and PLL cases from memory. Weak cases come up more
  often, and results are saved per case.
- **Stage practice:** time one part of a CFOP solve: cross, first two layers, OLL, or PLL.

**Play** is the classic game: scramble, then solve against the clock.

Progress is saved in `~/.local/share/rubx/progress.tsv`. Set `RUBX_DATA_DIR` to keep it elsewhere.

## Controls

| Input | Action |
| --- | --- |
| Drag the cube | Turn a layer. It follows the cursor and snaps when released |
| Drag the background, or right-drag | Orbit the camera |
| Arrow keys | Orbit the camera |
| Scroll | Zoom |
| `R` `L` `U` `D` `F` `B` | Turn a face clockwise |
| `M` `E` `S` | Turn a middle slice |
| `X` `Y` `Z` | Rotate the whole cube |
| `Shift` + any of the above | Turn the other way |
| `Ctrl+Z` or `Backspace` | Undo |
| `Ctrl+Y` or `Ctrl+Shift+Z` | Redo |
| `Esc` | Go back |

In Training:

| Input | Action |
| --- | --- |
| `H` | Show or hide a hint, with an arrow on the cube |
| `J` | Show me: the game makes the next moves |
| `V` | Reveal the algorithm in a drill |
| `G` | Retry the position |
| `N` | Next position |

In Play:

| Input | Action |
| --- | --- |
| `Space` | Scramble. Your first move starts the timer |
| `Ctrl+R` | Reset to solved |

Whole-cube rotations don't start the timer or count as moves, so you can inspect the cube freely.

## Layout

| Path | Purpose |
| --- | --- |
| `src/cube.rs` | The puzzle model and notation, with no Bevy dependency |
| `src/method.rs` | The stages of the beginner method, and how to recognise them |
| `src/solver.rs` | The hint engine, which solves using the moves the lessons teach |
| `src/algs.rs` | The OLL and PLL tables |
| `src/setups.rs` | Generates positions with only the part being practised left to do |
| `src/lessons.rs` | The course text, and tracking of follow-along exercises |
| `src/drills.rs` | Algorithm sets, and choosing which case comes next |
| `src/progress.rs` | Saved progress |
| `src/game.rs` | Turn queue, animation timing, undo history, and solve timer. Runs headless |
| `src/screens.rs`, `src/training.rs` | Navigation, and running the activities. Run headless |
| `src/drag.rs` | Picking and mouse drag geometry |
| `src/view.rs` | Procedural meshes, materials, lights, and placing cubies |
| `src/hint_view.rs` | Arrows and outlines drawn on the cube |
| `src/camera.rs`, `src/input.rs` | Camera, keyboard, and mouse |
| `src/hud.rs`, `src/menus.rs` | Timer, buttons, menus, and training panels |
| `src/audio.rs` | Turn sounds and the music playlist |
| `assets/audio/` | CC0 sounds and music, with credits alongside. The game looks for `assets/` next to the executable |
| `tests/game_flow.rs`, `tests/training_flow.rs` | Rules and activities, run through real Bevy schedules without a window |
| `examples/drive.rs` | Plays the game in a real window and takes screenshots |

The cube state is stored as exact integer rotations. Transforms are rebuilt from it every frame, so
floating point error can't build up however long you play.

All solving logic works in a fixed frame with white on the bottom and green at the front, then
converts its answers to however the player is holding the cube.

## Testing

```sh
cargo test
cargo clippy --all-targets
cargo run --example drive -- /tmp/shots   # opens a window for about a minute
```

`drive` feeds synthetic keyboard and mouse events through the real input path, checks the resulting
moves, clicks through every mode, and saves screenshots to the given directory, which must already
exist. It doesn't touch your saved progress.

The tests also carry out each lesson's written instructions on real positions, and check both
algorithm tables against the cube, so a wrong recipe or a typo in an algorithm fails the build.
