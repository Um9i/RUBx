//! Plays the real game in a real window through synthetic mouse events and checks the results.
//!
//! This covers what headless tests can't: the camera projection, drag handling through the actual
//! viewport, HUD hit testing, and how the result looks. It opens a window for a few seconds.
//!
//! ```text
//! cargo run --example drive -- <directory for screenshots>
//! ```
//!
//! Exits with a non-zero status if any check fails.

use std::collections::VecDeque;
use std::path::PathBuf;

use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::MouseButtonInput;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::ui::UiGlobalTransform;
use bevy::ui_widgets::Button;
use bevy::window::{CursorMoved, PrimaryWindow, WindowEvent};
use rubx::RubxPlugin;
use rubx::camera::OrbitCamera;
use rubx::cube::{Cube, parse_sequence};
use rubx::game::{CubeState, GameCommand, Session, SolvePhase};

#[derive(Clone, Debug)]
enum Step {
    Wait(u32),
    WaitIdle,
    Shot(&'static str),
    Command(GameCommand),
    Turns(&'static str),
    /// Move the cursor to the screen position of a world point.
    MoveToWorld(Vec3),
    /// Move the cursor to a position in window coordinates.
    MoveToWindow(Vec2),
    /// Move the cursor to the middle of the button with this label.
    MoveToButton(&'static str),
    Press(MouseButton),
    Release(MouseButton),
    KeyDown(KeyCode),
    KeyUp(KeyCode),
    /// Check the cube against a solved cube with these turns applied.
    ExpectCube(&'static str),
    ExpectPhase(SolvePhase),
    ExpectDragging(bool),
    ExpectCameraMoved,
}

fn world_drag(from: Vec3, to: Vec3, shot: Option<&'static str>) -> Vec<Step> {
    let mut steps = vec![
        Step::MoveToWorld(from),
        Step::Wait(2),
        Step::Press(MouseButton::Left),
    ];
    steps.push(Step::Wait(1));
    for i in 1..=10 {
        steps.push(Step::MoveToWorld(from.lerp(to, i as f32 / 10.0)));
        steps.push(Step::Wait(1));
    }
    if let Some(name) = shot {
        steps.push(Step::Shot(name));
    }
    steps.push(Step::Release(MouseButton::Left));
    steps.push(Step::WaitIdle);
    steps
}

/// Taps `key` while holding `modifiers`, as a real keyboard would report it.
fn chord(modifiers: &[KeyCode], key: KeyCode) -> Vec<Step> {
    let mut steps: Vec<Step> = modifiers.iter().map(|&m| Step::KeyDown(m)).collect();
    steps.push(Step::Wait(1));
    steps.push(Step::KeyDown(key));
    steps.push(Step::Wait(1));
    steps.push(Step::KeyUp(key));
    steps.extend(modifiers.iter().map(|&m| Step::KeyUp(m)));
    steps.push(Step::Wait(1));
    steps.push(Step::WaitIdle);
    steps
}

fn script() -> VecDeque<Step> {
    use Step::*;
    let top_of = |x: f32, z: f32| Vec3::new(x, 1.5, z);
    let front_of = |x: f32, y: f32| Vec3::new(x, y, 1.5);
    let right_of = |y: f32, z: f32| Vec3::new(1.5, y, z);

    // Wayland delivers the window's first real frame some time after the app loop starts.
    let mut steps = vec![Wait(150), Shot("01-solved"), ExpectCube("")];

    // Keyboard-style commands with animation.
    steps.extend([
        Turns("R U R' U'"),
        Wait(4),
        Shot("02-mid-turn"),
        WaitIdle,
        Shot("03-after-turns"),
        ExpectCube("R U R' U'"),
        Command(GameCommand::Reset),
        Wait(2),
        ExpectCube(""),
    ]);

    // Real keyboard events, including the modifier handling.
    let (shift, ctrl) = (KeyCode::ShiftLeft, KeyCode::ControlLeft);
    steps.extend(chord(&[], KeyCode::KeyR));
    steps.push(ExpectCube("R"));
    steps.extend(chord(&[shift], KeyCode::KeyU));
    steps.push(ExpectCube("R U'"));
    steps.extend(chord(&[ctrl], KeyCode::KeyZ));
    steps.push(ExpectCube("R"));
    steps.extend(chord(&[ctrl], KeyCode::KeyY));
    steps.push(ExpectCube("R U'"));
    steps.extend(chord(&[ctrl, shift], KeyCode::KeyZ));
    steps.push(ExpectCube("R U'"));
    steps.extend(chord(&[], KeyCode::Backspace));
    steps.push(ExpectCube("R"));
    steps.extend(chord(&[ctrl, shift], KeyCode::KeyZ));
    steps.push(ExpectCube("R U'"));
    steps.extend(chord(&[], KeyCode::KeyM));
    steps.push(ExpectCube("R U' M"));
    steps.extend(chord(&[], KeyCode::KeyY));
    steps.push(ExpectCube("R U' M y"));
    steps.extend(chord(&[ctrl], KeyCode::KeyR));
    steps.push(Wait(2));
    steps.push(ExpectCube(""));

    // Each physical drag must produce the move a person would expect.
    steps.extend(world_drag(
        front_of(1.0, 0.0),
        front_of(1.0, 1.0),
        Some("04-holding-r"),
    ));
    steps.push(ExpectCube("R"));
    steps.push(Command(GameCommand::Reset));
    steps.push(Wait(2));

    steps.extend(world_drag(front_of(0.0, 1.0), front_of(-1.0, 1.0), None));
    steps.push(ExpectCube("U"));
    steps.push(Command(GameCommand::Reset));
    steps.push(Wait(2));

    steps.extend(world_drag(top_of(1.0, 0.0), top_of(1.0, -1.0), None));
    steps.push(ExpectCube("R"));
    steps.push(Command(GameCommand::Reset));
    steps.push(Wait(2));

    steps.extend(world_drag(right_of(-1.0, 1.0), right_of(-1.0, 0.0), None));
    // Moving the bottom row of the R face from front to back is how D carries it: F, R, B, L.
    steps.push(ExpectCube("D"));
    steps.push(Command(GameCommand::Reset));
    steps.push(Wait(2));

    // A drag too small to reach the snap threshold should put the layer back.
    steps.extend(world_drag(front_of(1.0, 0.0), front_of(1.0, 0.3), None));
    steps.push(ExpectCube(""));
    steps.push(ExpectDragging(false));

    // Quick successive drags: the second grab lands while the first is still settling.
    steps.extend(
        world_drag(front_of(1.0, 0.0), front_of(1.0, 1.0), None)
            .into_iter()
            .filter(|step| !matches!(step, WaitIdle)),
    );
    steps.extend(world_drag(front_of(0.0, 1.0), front_of(-1.0, 1.0), None));
    steps.push(ExpectCube("R U"));
    steps.push(Command(GameCommand::Undo));
    steps.push(WaitIdle);
    steps.push(ExpectCube("R"));
    steps.push(Command(GameCommand::Reset));
    steps.push(Wait(2));

    // Dragging the background orbits the camera and leaves the cube alone.
    steps.extend([
        MoveToWindow(Vec2::new(150.0, 400.0)),
        Wait(2),
        Press(MouseButton::Left),
        Wait(1),
        MoveToWindow(Vec2::new(240.0, 340.0)),
        Wait(2),
        ExpectDragging(false),
        Release(MouseButton::Left),
        Wait(30),
        ExpectCameraMoved,
        ExpectCube(""),
        Shot("05-orbited"),
    ]);

    // A real click on a HUD button goes through UI picking.
    steps.extend([
        MoveToButton("Scramble"),
        Wait(3),
        Press(MouseButton::Left),
        Wait(2),
        Release(MouseButton::Left),
        Wait(6),
        ExpectPhase(SolvePhase::Scrambling),
        Shot("06-scrambling"),
        WaitIdle,
        Wait(2),
        ExpectPhase(SolvePhase::Ready),
        Shot("07-scrambled"),
        // Give the last screenshot time to reach the disk before the app exits.
        Wait(30),
    ]);

    VecDeque::from(steps)
}

#[derive(Resource)]
struct Driver {
    steps: VecDeque<Step>,
    wait: u32,
    out_dir: PathBuf,
    failures: u32,
}

fn main() -> AppExit {
    let out_dir = std::env::args()
        .nth(1)
        .map_or_else(|| PathBuf::from("."), PathBuf::from);

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "RUBx drive".into(),
                resolution: (1280, 800).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(RubxPlugin)
        .insert_resource(Driver {
            steps: script(),
            wait: 0,
            out_dir,
            failures: 0,
        })
        // In `First`, so that input systems see these events exactly as they would real ones.
        .add_systems(First, drive)
        .run()
}

#[expect(clippy::too_many_arguments)]
fn drive(
    mut commands: Commands,
    mut driver: ResMut<Driver>,
    mut game: MessageWriter<GameCommand>,
    mut cursor_events: MessageWriter<WindowEvent>,
    mut button_events: MessageWriter<MouseButtonInput>,
    mut key_events: MessageWriter<KeyboardInput>,
    window: Single<(Entity, &mut Window), With<PrimaryWindow>>,
    view: Single<(&Camera, &GlobalTransform)>,
    orbit: Single<&OrbitCamera>,
    buttons: Query<(&Children, &UiGlobalTransform), With<Button>>,
    texts: Query<&Text>,
    state: Res<CubeState>,
    session: Res<Session>,
    mut exit: MessageWriter<AppExit>,
) {
    let (window_entity, mut window) = window.into_inner();
    let (camera, camera_transform) = *view;

    if driver.wait > 0 {
        driver.wait -= 1;
        return;
    }
    let Some(step) = driver.steps.front().cloned() else {
        let failures = driver.failures;
        println!("drive finished with {failures} failure(s)");
        exit.write(if failures == 0 {
            AppExit::Success
        } else {
            AppExit::error()
        });
        return;
    };

    let mut move_cursor = |window: &mut Window, position: Vec2| {
        window.set_cursor_position(Some(position));
        cursor_events.write(WindowEvent::CursorMoved(CursorMoved {
            window: window_entity,
            position,
            delta: None,
        }));
    };

    match &step {
        Step::Wait(frames) => driver.wait = *frames,
        Step::WaitIdle => {
            if state.is_busy() {
                return;
            }
        }
        Step::Shot(name) => {
            let path = driver.out_dir.join(format!("{name}.png"));
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path));
        }
        Step::Command(command) => {
            game.write(*command);
        }
        Step::Turns(text) => {
            let turns = parse_sequence(text).unwrap_or_default();
            game.write_batch(turns.into_iter().map(GameCommand::Turn));
        }
        Step::MoveToWorld(point) => match camera.world_to_viewport(camera_transform, *point) {
            Ok(position) => move_cursor(&mut window, position),
            Err(error) => fail(&mut driver, format!("can't project {point}: {error:?}")),
        },
        Step::MoveToWindow(position) => move_cursor(&mut window, *position),
        Step::MoveToButton(label) => {
            let found = buttons.iter().find(|(children, _)| {
                children
                    .iter()
                    .any(|child| texts.get(child).is_ok_and(|text| text.0 == *label))
            });
            match found {
                Some((_, transform)) => {
                    let scale = window.scale_factor();
                    move_cursor(&mut window, transform.translation / scale);
                }
                None => fail(&mut driver, format!("no button labelled {label}")),
            }
        }
        Step::Press(button) | Step::Release(button) => {
            let pressed = matches!(step, Step::Press(_));
            let event = MouseButtonInput {
                button: *button,
                state: if pressed {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                },
                window: window_entity,
            };
            button_events.write(event);
            cursor_events.write(WindowEvent::MouseButtonInput(event));
        }
        Step::KeyDown(key) | Step::KeyUp(key) => {
            key_events.write(KeyboardInput {
                key_code: *key,
                logical_key: Key::Unidentified(NativeKey::Unidentified),
                state: if matches!(step, Step::KeyDown(_)) {
                    ButtonState::Pressed
                } else {
                    ButtonState::Released
                },
                text: None,
                repeat: false,
                window: window_entity,
            });
        }
        Step::ExpectCube(text) => {
            let mut expected = Cube::solved();
            expected.apply_all(parse_sequence(text).unwrap_or_default());
            check(
                &mut driver,
                state.cube == expected,
                format!("cube is {text:?}"),
            );
        }
        Step::ExpectPhase(phase) => {
            let actual = session.phase;
            check(
                &mut driver,
                actual == *phase,
                format!("phase {actual:?} is {phase:?}"),
            );
        }
        Step::ExpectDragging(dragging) => {
            check(
                &mut driver,
                state.is_dragging() == *dragging,
                format!("dragging is {dragging}"),
            );
        }
        Step::ExpectCameraMoved => {
            let default = OrbitCamera::default();
            let moved = orbit.yaw != default.yaw && orbit.pitch != default.pitch;
            check(&mut driver, moved, "camera moved".to_owned());
        }
    }
    driver.steps.pop_front();
}

fn check(driver: &mut Driver, passed: bool, description: String) {
    if passed {
        println!("ok    {description}");
    } else {
        fail(driver, description);
    }
}

fn fail(driver: &mut Driver, description: String) {
    println!("FAIL  {description}");
    driver.failures += 1;
}
