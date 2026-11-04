# RUBx

A Rubik's cube simulator built with Rust and [Bevy](https://bevy.org) 0.20.

```sh
cargo run --release   # the first build takes several minutes
```

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
| `Space` | Scramble. Your first move starts the timer |
| `Ctrl+Z` or `Backspace` | Undo |
| `Ctrl+Y` or `Ctrl+Shift+Z` | Redo |
| `Ctrl+R` | Reset to solved |

Whole-cube rotations don't start the timer or count as moves, so you can inspect the cube freely.

## Layout

| Path | Purpose |
| --- | --- |
| `src/cube.rs` | The puzzle model and notation, with no Bevy dependency |
| `src/game.rs` | Turn queue, animation timing, undo history, and solve timer. Runs headless |
| `src/drag.rs` | Picking and mouse drag geometry |
| `src/view.rs` | Procedural meshes, materials, lights, and placing cubies |
| `src/camera.rs`, `src/input.rs`, `src/hud.rs` | Camera, keyboard and mouse, and on-screen display |
| `tests/game_flow.rs` | Game rules, run through real Bevy schedules without a window |
| `examples/drive.rs` | Plays the game in a real window and takes screenshots |

The cube state is stored as exact integer rotations. Transforms are rebuilt from it every frame, so
floating point error can't build up however long you play.

## Testing

```sh
cargo test
cargo clippy --all-targets
cargo run --example drive -- /tmp/shots   # opens a window for about 10 seconds
```

`drive` feeds synthetic keyboard and mouse events through the real input path, checks the resulting
moves, and saves screenshots to the given directory, which must already exist.
