//! Runs the game rules headless through real Bevy schedules, with a fixed time step.

use std::time::Duration;

use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use rubx::cube::{Axis, Cube, Layer, Turn, parse_sequence};
use rubx::game::{
    CubeState, GameCommand, GamePlugin, ScrambleRng, Session, SolvePhase, release_drag,
};

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, GamePlugin))
        .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
            16,
        )))
        .insert_resource(ScrambleRng(fastrand::Rng::with_seed(7)));
    app.update();
    app
}

fn send(app: &mut App, command: GameCommand) {
    app.world_mut().write_message(command);
}

fn turn(text: &str) -> GameCommand {
    GameCommand::Turn(text.parse().unwrap())
}

/// Updates until all turns have finished, failing if that takes implausibly long.
fn settle(app: &mut App) {
    for _ in 0..5000 {
        app.update();
        if !app.world().resource::<CubeState>().is_busy() {
            return;
        }
    }
    panic!("turns never settled");
}

fn cube(app: &App) -> &Cube {
    &app.world().resource::<CubeState>().cube
}

fn session(app: &App) -> &Session {
    app.world().resource::<Session>()
}

fn expected(text: &str) -> Cube {
    let mut cube = Cube::solved();
    cube.apply_all(parse_sequence(text).unwrap());
    cube
}

#[test]
fn queued_turns_should_apply_in_order() {
    let mut app = app();
    for text in ["R", "U", "R'"] {
        send(&mut app, turn(text));
    }
    settle(&mut app);
    assert_eq!(*cube(&app), expected("R U R'"));
}

#[test]
fn turn_should_animate_before_it_applies() {
    let mut app = app();
    send(&mut app, turn("R"));
    app.update();
    app.update();
    let state = app.world().resource::<CubeState>();
    assert_eq!(
        (
            state.rotating_layer().is_some(),
            state.cube == Cube::solved()
        ),
        (true, true)
    );
}

#[test]
fn undo_should_reverse_the_last_turn_and_redo_should_restore_it() {
    let mut app = app();
    send(&mut app, turn("R"));
    send(&mut app, turn("U"));
    send(&mut app, GameCommand::Undo);
    settle(&mut app);
    assert_eq!(*cube(&app), expected("R"));
    send(&mut app, GameCommand::Redo);
    settle(&mut app);
    assert_eq!(*cube(&app), expected("R U"));
}

#[test]
fn new_turn_should_discard_the_redo_stack() {
    let mut app = app();
    send(&mut app, turn("R"));
    send(&mut app, GameCommand::Undo);
    send(&mut app, turn("U"));
    settle(&mut app);
    assert!(!session(&app).can_redo());
}

#[test]
fn undo_with_empty_history_should_do_nothing() {
    let mut app = app();
    send(&mut app, GameCommand::Undo);
    settle(&mut app);
    assert_eq!(*cube(&app), Cube::solved());
}

#[test]
fn move_count_should_ignore_whole_cube_rotations() {
    let mut app = app();
    for text in ["R", "x", "U"] {
        send(&mut app, turn(text));
    }
    settle(&mut app);
    assert_eq!(session(&app).move_count(), 2);
}

/// Returns an app whose scramble has just finished.
fn scrambled_app() -> App {
    let mut app = app();
    send(&mut app, GameCommand::Scramble);
    settle(&mut app);
    app.update();
    app
}

#[test]
fn scramble_should_end_ready_and_unsolved() {
    let app = scrambled_app();
    assert_eq!(
        (session(&app).phase, cube(&app).is_solved()),
        (SolvePhase::Ready, false)
    );
}

#[test]
fn scramble_should_not_count_as_moves() {
    let app = scrambled_app();
    assert_eq!(session(&app).move_count(), 0);
}

#[test]
fn turn_during_a_scramble_should_be_ignored() {
    let mut app = app();
    send(&mut app, GameCommand::Scramble);
    app.update();
    send(&mut app, turn("R"));
    settle(&mut app);
    assert_eq!(session(&app).move_count(), 0);
}

#[test]
fn whole_cube_rotation_should_not_start_the_timer() {
    let mut app = scrambled_app();
    send(&mut app, turn("y"));
    settle(&mut app);
    assert_eq!(session(&app).phase, SolvePhase::Ready);
}

#[test]
fn first_move_should_start_the_timer() {
    let mut app = scrambled_app();
    send(&mut app, turn("R"));
    settle(&mut app);
    assert!(matches!(session(&app).phase, SolvePhase::Solving { .. }));
}

#[test]
fn timer_should_advance_while_solving() {
    let mut app = scrambled_app();
    send(&mut app, turn("R"));
    settle(&mut app);
    let before = solving_secs(&app);
    for _ in 0..10 {
        app.update();
    }
    assert!(solving_secs(&app) > before);
}

fn solving_secs(app: &App) -> f32 {
    match session(app).phase {
        SolvePhase::Solving { secs } => secs,
        phase => panic!("expected Solving, got {phase:?}"),
    }
}

/// Solves a cube from [`app_scrambled_with`] by playing its known scramble backwards.
fn solve_by_reversing(app: &mut App, scramble: &[Turn]) {
    for turn in scramble.iter().rev() {
        send(app, GameCommand::Turn(turn.inverse()));
    }
    settle(app);
    app.update();
}

fn app_scrambled_with(text: &str) -> (App, Vec<Turn>) {
    let mut app = app();
    let turns = parse_sequence(text).unwrap();
    // A real scramble leaves the game in the Ready phase, then a known state replaces its result.
    send(&mut app, GameCommand::Scramble);
    settle(&mut app);
    app.update();
    let mut cube = Cube::solved();
    cube.apply_all(turns.iter().copied());
    app.world_mut().resource_mut::<CubeState>().cube = cube;
    (app, turns)
}

#[test]
fn solving_should_stop_the_timer_and_record_the_result() {
    let (mut app, turns) = app_scrambled_with("R U F2 L'");
    solve_by_reversing(&mut app, &turns);
    assert!(matches!(
        session(&app).phase,
        SolvePhase::Solved { moves: 4, .. }
    ));
}

#[test]
fn solving_should_record_a_best_time() {
    let (mut app, turns) = app_scrambled_with("R U");
    solve_by_reversing(&mut app, &turns);
    assert!(session(&app).best_secs.is_some());
}

#[test]
fn move_after_solving_should_return_to_free_play() {
    let (mut app, turns) = app_scrambled_with("R U");
    solve_by_reversing(&mut app, &turns);
    send(&mut app, turn("R"));
    settle(&mut app);
    assert_eq!(session(&app).phase, SolvePhase::Free);
}

#[test]
fn reset_should_restore_the_solved_cube_immediately() {
    let mut app = scrambled_app();
    send(&mut app, GameCommand::Reset);
    app.update();
    assert_eq!(
        (cube(&app).clone(), session(&app).phase),
        (Cube::solved(), SolvePhase::Free)
    );
}

#[test]
fn reset_during_a_scramble_should_stop_it() {
    let mut app = app();
    send(&mut app, GameCommand::Scramble);
    app.update();
    send(&mut app, GameCommand::Reset);
    settle(&mut app);
    assert_eq!(*cube(&app), Cube::solved());
}

/// Holds the R layer at `angle` radians about `+X`, as a mouse drag would.
fn hold_r_layer(app: &mut App, angle: f32) {
    app.world_mut()
        .resource_mut::<CubeState>()
        .set_drag(Axis::X, Layer::Pos, angle);
}

fn release(app: &mut App) {
    app.world_mut()
        .resource_scope(|world, mut state: Mut<CubeState>| {
            release_drag(&mut state, &mut world.resource_mut::<Session>());
        });
}

const CLOCKWISE_R_DRAG: f32 = -1.2;

#[test]
fn released_drag_should_apply_the_turn() {
    let mut app = app();
    hold_r_layer(&mut app, CLOCKWISE_R_DRAG);
    app.update();
    release(&mut app);
    settle(&mut app);
    assert_eq!(*cube(&app), expected("R"));
}

#[test]
fn small_drag_should_leave_the_cube_and_history_unchanged() {
    let mut app = app();
    hold_r_layer(&mut app, -0.2);
    app.update();
    release(&mut app);
    settle(&mut app);
    assert_eq!(
        (cube(&app).clone(), session(&app).move_count()),
        (Cube::solved(), 0)
    );
}

#[test]
fn drag_should_start_the_timer_when_ready() {
    let mut app = scrambled_app();
    hold_r_layer(&mut app, CLOCKWISE_R_DRAG);
    release(&mut app);
    settle(&mut app);
    assert!(matches!(session(&app).phase, SolvePhase::Solving { .. }));
}

#[test]
fn turn_pressed_while_holding_a_layer_should_not_corrupt_undo() {
    let mut app = app();
    hold_r_layer(&mut app, CLOCKWISE_R_DRAG);
    send(&mut app, turn("U"));
    app.update();
    release(&mut app);
    settle(&mut app);
    send(&mut app, GameCommand::Undo);
    settle(&mut app);
    assert_eq!(*cube(&app), Cube::solved());
}

#[test]
fn scramble_while_holding_a_layer_should_be_ignored() {
    let mut app = app();
    hold_r_layer(&mut app, CLOCKWISE_R_DRAG);
    send(&mut app, GameCommand::Scramble);
    app.update();
    assert_eq!(session(&app).phase, SolvePhase::Free);
}

#[test]
fn reset_while_holding_a_layer_should_release_it() {
    let mut app = app();
    hold_r_layer(&mut app, CLOCKWISE_R_DRAG);
    send(&mut app, GameCommand::Reset);
    app.update();
    assert!(!app.world().resource::<CubeState>().is_dragging());
}
