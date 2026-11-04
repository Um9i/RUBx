//! Maps the keyboard and mouse to game commands, layer drags, and camera movement.

use bevy::input::mouse::{AccumulatedMouseScroll, MouseScrollUnit};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::camera::OrbitCamera;
use crate::cube::Turn;
use crate::drag::{FaceHit, LayerDrag, pick_face, resolve_drag};
use crate::game::{CubeState, GameCommand, GameSet, Session, SolvePhase, release_drag};
use crate::hud::PointerOverHud;
use crate::screens::{Screen, cube_controls_active};

/// Cursor travel in pixels before a press on the cube commits to a layer and direction.
const DRAG_LOCK_PIXELS: f32 = 6.0;

const TURN_KEYS: [(KeyCode, &str); 12] = [
    (KeyCode::KeyR, "R"),
    (KeyCode::KeyL, "L"),
    (KeyCode::KeyU, "U"),
    (KeyCode::KeyD, "D"),
    (KeyCode::KeyF, "F"),
    (KeyCode::KeyB, "B"),
    (KeyCode::KeyM, "M"),
    (KeyCode::KeyE, "E"),
    (KeyCode::KeyS, "S"),
    (KeyCode::KeyX, "x"),
    (KeyCode::KeyY, "y"),
    (KeyCode::KeyZ, "z"),
];

/// What the mouse is currently doing.
#[derive(Resource, Clone, Copy, Debug, Default)]
enum PointerMode {
    #[default]
    Idle,
    /// Pressed on the cube, but not yet moved far enough to pick a direction.
    Grabbed { hit: FaceHit, start: Vec2 },
    /// Holding a layer, which follows the cursor.
    Turning { drag: LayerDrag, start: Vec2 },
    /// Moving the camera.
    Orbiting { last: Vec2 },
}

/// Adds keyboard and mouse controls.
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PointerMode>().add_systems(
            Update,
            (keyboard_commands, pointer_actions, scroll_zoom)
                .in_set(GameSet::Input)
                .run_if(cube_controls_active),
        );
    }
}

fn keyboard_commands(
    keys: Res<ButtonInput<KeyCode>>,
    screen: Res<State<Screen>>,
    state: Res<CubeState>,
    mut commands: MessageWriter<GameCommand>,
) {
    if state.locked {
        return;
    }
    // Training positions come from the activity, so only Play may scramble or reset freely.
    let free_play = *screen.get() == Screen::Play;
    let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    let shift = keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    if ctrl {
        if keys.just_pressed(KeyCode::KeyZ) {
            commands.write(if shift {
                GameCommand::Redo
            } else {
                GameCommand::Undo
            });
        }
        if keys.just_pressed(KeyCode::KeyY) {
            commands.write(GameCommand::Redo);
        }
        if free_play && keys.just_pressed(KeyCode::KeyR) {
            commands.write(GameCommand::Reset);
        }
        return;
    }

    if free_play && keys.just_pressed(KeyCode::Space) {
        commands.write(GameCommand::Scramble);
    }
    if keys.just_pressed(KeyCode::Backspace) {
        commands.write(GameCommand::Undo);
    }

    for (key, name) in TURN_KEYS {
        if keys.just_pressed(key) {
            let turn = name
                .parse::<Turn>()
                .map(|turn| if shift { turn.inverse() } else { turn });
            match turn {
                Ok(turn) => {
                    commands.write(GameCommand::Turn(turn));
                }
                Err(error) => error!("{error}"),
            }
        }
    }
}

// The pointer state machine needs all of these at once, and splitting it would only spread the
// transitions across systems that must then coordinate through more shared state.
#[expect(clippy::too_many_arguments)]
fn pointer_actions(
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    view: Single<(&Camera, &GlobalTransform)>,
    mut orbit: Single<&mut OrbitCamera>,
    over_hud: Res<PointerOverHud>,
    mut mode: ResMut<PointerMode>,
    mut state: ResMut<CubeState>,
    mut session: ResMut<Session>,
) {
    let cursor = window.cursor_position();
    let (camera, camera_transform) = *view;

    match *mode {
        PointerMode::Idle => {
            let Some(cursor) = cursor else { return };
            if mouse.just_pressed(MouseButton::Right) {
                *mode = PointerMode::Orbiting { last: cursor };
            } else if mouse.just_pressed(MouseButton::Left) && !over_hud.0 {
                let hit = camera
                    .viewport_to_world(camera_transform, cursor)
                    .ok()
                    .and_then(|ray| pick_face(ray.origin, *ray.direction));
                *mode = match hit {
                    Some(hit) if session.phase != SolvePhase::Scrambling && !state.locked => {
                        state.request_flush();
                        PointerMode::Grabbed { hit, start: cursor }
                    }
                    // A press on the cube while it's busy does nothing rather than orbiting by
                    // surprise. A press on the background always orbits.
                    Some(_) => PointerMode::Idle,
                    None => PointerMode::Orbiting { last: cursor },
                };
            }
        }
        PointerMode::Grabbed { hit, start } => {
            if !mouse.pressed(MouseButton::Left) || session.phase == SolvePhase::Scrambling {
                *mode = PointerMode::Idle;
                return;
            }
            let Some(cursor) = cursor else { return };
            let travel = cursor - start;
            if travel.length() < DRAG_LOCK_PIXELS {
                return;
            }
            // Keystrokes may have queued more turns since the press, and a layer can't be held
            // while others are moving through it.
            if state.is_busy() {
                state.request_flush();
                return;
            }
            let project = |point| camera.world_to_viewport(camera_transform, point).ok();
            *mode = match resolve_drag(hit, travel, project) {
                Some(drag) => {
                    state.set_drag(drag.axis, drag.layer, drag.angle(travel));
                    PointerMode::Turning { drag, start }
                }
                None => PointerMode::Idle,
            };
        }
        PointerMode::Turning { drag, start } => {
            if !mouse.pressed(MouseButton::Left) {
                release_drag(&mut state, &mut session);
                *mode = PointerMode::Idle;
                return;
            }
            // A reset clears the held layer, so there is nothing left to follow.
            if !state.is_dragging() {
                *mode = PointerMode::Idle;
                return;
            }
            if let Some(cursor) = cursor {
                state.set_drag(drag.axis, drag.layer, drag.angle(cursor - start));
            }
        }
        PointerMode::Orbiting { last } => {
            if !mouse.any_pressed([MouseButton::Left, MouseButton::Right]) {
                *mode = PointerMode::Idle;
                return;
            }
            if let Some(cursor) = cursor {
                orbit.orbit_pixels(cursor - last);
                *mode = PointerMode::Orbiting { last: cursor };
            }
        }
    }
}

fn scroll_zoom(scroll: Res<AccumulatedMouseScroll>, mut orbit: Single<&mut OrbitCamera>) {
    if scroll.delta.y == 0.0 {
        return;
    }
    let steps = match scroll.unit {
        MouseScrollUnit::Line => scroll.delta.y,
        MouseScrollUnit::Pixel => scroll.delta.y / 40.0,
    };
    orbit.zoom_steps(steps);
}
