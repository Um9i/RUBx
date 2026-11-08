//! Runs every Training activity headless, through real Bevy schedules and state transitions.

use std::time::Duration;

use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::time::TimeUpdateStrategy;
use rubx::algs::{Case, PLL};
use rubx::cube::{Cube, Turn};
use rubx::drills::{DrillSet, stats_key};
use rubx::game::{CubeState, GameCommand, GamePlugin, ScrambleRng, Session, SolvePhase};
use rubx::lessons::LESSONS;
use rubx::method::{MethodView, Stage, first_two_layers_done, last_layer_oriented};
use rubx::progress::Progress;
use rubx::screens::{Screen, ScreenPlugin, UiAction};
use rubx::setups::PracticeStage;
use rubx::training::{Activity, Run, TrainingPlugin, practice_key};

fn app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        StatesPlugin,
        GamePlugin,
        ScreenPlugin,
        TrainingPlugin,
    ))
    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        16,
    )))
    .insert_resource(ScrambleRng(fastrand::Rng::with_seed(11)))
    // A default `Progress` has no path, so these tests never touch real saved progress.
    .insert_resource(Progress::default());
    app.update();
    app
}

fn act(app: &mut App, action: UiAction) {
    app.world_mut().write_message(action);
    // One update for the action, and one for any screen change to take effect.
    app.update();
    app.update();
}

fn turn(app: &mut App, text: &str) {
    let parsed: Turn = text.parse().unwrap();
    app.world_mut().write_message(GameCommand::Turn(parsed));
    settle(app);
}

fn settle(app: &mut App) {
    for _ in 0..3000 {
        app.update();
        let state = app.world().resource::<CubeState>();
        if !state.is_busy() && !state.locked {
            app.update();
            return;
        }
    }
    panic!("the cube never settled");
}

fn run(app: &App) -> &Run {
    app.world().resource::<Run>()
}

fn cube(app: &App) -> &Cube {
    &app.world().resource::<CubeState>().cube
}

fn session(app: &App) -> &Session {
    app.world().resource::<Session>()
}

fn progress(app: &App) -> &Progress {
    app.world().resource::<Progress>()
}

fn screen(app: &App) -> Screen {
    *app.world().resource::<State<Screen>>().get()
}

/// Keeps pressing Show Me until the attempt finishes.
fn finish_with_show_me(app: &mut App) {
    for _ in 0..200 {
        if run(app).is_finished() {
            return;
        }
        act(app, UiAction::ShowMe);
        settle(app);
    }
    panic!("the attempt never finished");
}

/// Finishes the attempt by making each suggested turn. Hints are allowed in lessons.
fn finish_following_hints(app: &mut App) {
    act(app, UiAction::Hint);
    for _ in 0..500 {
        if run(app).is_finished() {
            return;
        }
        let next = run(app).arrow_turn().expect("a suggested turn");
        app.world_mut().write_message(GameCommand::Turn(next));
        settle(app);
    }
    panic!("the attempt never finished");
}

#[test]
fn game_should_start_on_the_menu() {
    assert_eq!(screen(&app()), Screen::Menu);
}

#[test]
fn back_should_climb_to_the_menu() {
    let mut app = app();
    act(&mut app, UiAction::StartCoach);
    act(&mut app, UiAction::Back);
    assert_eq!(screen(&app), Screen::Hub);
    act(&mut app, UiAction::Back);
    assert_eq!(screen(&app), Screen::Menu);
}

#[test]
fn leaving_an_activity_should_clear_it() {
    let mut app = app();
    act(&mut app, UiAction::StartCoach);
    act(&mut app, UiAction::Back);
    assert_eq!(run(&app).activity, Activity::None);
}

#[test]
fn play_should_start_with_a_solved_cube_and_no_timer() {
    let mut app = app();
    act(&mut app, UiAction::Go(Screen::Play));
    assert_eq!(
        (cube(&app).clone(), session(&app).phase),
        (Cube::solved(), SolvePhase::Free)
    );
}

#[test]
fn stage_lessons_should_start_on_their_stage() {
    let mut app = app();
    for (index, stage) in Stage::STEPS.iter().enumerate() {
        act(&mut app, UiAction::StartLesson(index + 1));
        assert_eq!(MethodView::of(cube(&app)).stage(), *stage, "{stage:?}");
    }
}

#[test]
fn notation_lesson_should_accept_the_listed_moves() {
    let mut app = app();
    act(&mut app, UiAction::StartLesson(0));
    for text in "R R' U U' F F' L L' D D' B B'".split_whitespace() {
        assert_eq!(
            run(&app).arrow_turn(),
            Some(text.parse().unwrap()),
            "{text}"
        );
        turn(&mut app, text);
    }
    assert!(run(&app).is_finished());
}

#[test]
fn notation_lesson_should_flag_a_wrong_move_and_recover_after_undo() {
    let mut app = app();
    act(&mut app, UiAction::StartLesson(0));
    turn(&mut app, "L");
    assert_eq!(run(&app).arrow_turn(), None);
    app.world_mut().write_message(GameCommand::Undo);
    settle(&mut app);
    assert_eq!(run(&app).arrow_turn(), Some("R".parse().unwrap()));
}

#[test]
fn notation_lesson_should_complete_after_both_exercises() {
    let mut app = app();
    act(&mut app, UiAction::StartLesson(0));
    finish_following_hints(&mut app);
    assert!(!run(&app).lesson_complete());
    act(&mut app, UiAction::Next);
    finish_following_hints(&mut app);
    assert!(run(&app).lesson_complete());
    assert!(progress(&app).lesson_done("notation"));
}

#[test]
fn every_stage_lesson_should_finish_and_be_saved_after_three_positions() {
    for (index, lesson) in LESSONS.iter().enumerate().skip(1) {
        let mut app = app();
        act(&mut app, UiAction::StartLesson(index));
        for position in 0..3 {
            assert!(!run(&app).lesson_complete(), "lesson {index}, {position}");
            finish_following_hints(&mut app);
            if position < 2 {
                act(&mut app, UiAction::Next);
            }
        }
        assert!(progress(&app).lesson_done(lesson.id), "{index}");
    }
}

#[test]
fn next_after_finishing_a_lesson_should_start_the_next_one() {
    let mut app = app();
    act(&mut app, UiAction::StartLesson(7));
    for _ in 0..2 {
        finish_following_hints(&mut app);
        act(&mut app, UiAction::Next);
    }
    finish_following_hints(&mut app);
    act(&mut app, UiAction::Next);
    // Lesson 8 is the last, so Next returns to the list.
    assert_eq!(screen(&app), Screen::LessonList);
}

fn lesson_positions_finished(app: &App) -> u32 {
    match run(app).activity {
        Activity::Lesson { finished, .. } => finished,
        activity => panic!("expected a lesson, got {activity:?}"),
    }
}

#[test]
fn lesson_position_finished_with_show_me_should_not_count() {
    let mut app = app();
    act(&mut app, UiAction::StartLesson(1));
    finish_with_show_me(&mut app);
    assert_eq!(lesson_positions_finished(&app), 0);
}

#[test]
fn retrying_a_finished_lesson_position_should_not_count_twice() {
    let mut app = app();
    act(&mut app, UiAction::StartLesson(1));
    finish_following_hints(&mut app);
    act(&mut app, UiAction::Restart);
    finish_following_hints(&mut app);
    assert_eq!(lesson_positions_finished(&app), 1);
}

#[test]
fn coach_should_solve_a_scramble_by_following_hints() {
    let mut app = app();
    act(&mut app, UiAction::StartCoach);
    finish_with_show_me(&mut app);
    assert!(cube(&app).is_solved());
}

#[test]
fn coach_should_not_count_a_helped_solve_as_clean() {
    let mut app = app();
    act(&mut app, UiAction::StartCoach);
    finish_with_show_me(&mut app);
    let stats = progress(&app).stats("coach");
    assert_eq!((stats.attempts, stats.clean), (1, 0));
}

#[test]
fn hint_should_point_at_a_turn_only_while_visible() {
    let mut app = app();
    act(&mut app, UiAction::StartCoach);
    assert_eq!(run(&app).arrow_turn(), None);
    act(&mut app, UiAction::Hint);
    assert!(run(&app).arrow_turn().is_some());
    act(&mut app, UiAction::Hint);
    assert_eq!(run(&app).arrow_turn(), None);
}

#[test]
fn hint_should_keep_its_plan_through_a_trigger() {
    let mut app = app();
    act(&mut app, UiAction::StartLesson(2));
    act(&mut app, UiAction::Hint);
    // Walk the plan by hand, checking that every suggested turn is accepted without replanning.
    let mut previous = None;
    for _ in 0..4 {
        let Some(next) = run(&app).arrow_turn() else {
            break;
        };
        let plan = run(&app).tracker.hint().cloned();
        if let Some(previous) = &previous {
            assert_eq!(plan.as_ref(), Some(previous));
        }
        previous = plan;
        app.world_mut().write_message(GameCommand::Turn(next));
        settle(&mut app);
        if run(&app).is_finished() {
            break;
        }
    }
}

#[test]
fn input_should_be_locked_during_a_demonstration() {
    let mut app = app();
    act(&mut app, UiAction::StartCoach);
    act(&mut app, UiAction::ShowMe);
    assert!(app.world().resource::<CubeState>().locked);
    settle(&mut app);
    assert!(!app.world().resource::<CubeState>().locked);
}

/// Starts a drill and returns its case and how to line it up.
fn drill(app: &mut App) -> (&'static Case, i32, i32) {
    act(app, UiAction::StartDrill(DrillSet::StarterPll));
    match run(app).activity {
        Activity::Drill {
            case,
            before,
            after,
            ..
        } => (case, before, after),
        activity => panic!("expected a drill, got {activity:?}"),
    }
}

/// Solves a drill as an expert would, one turn at a time.
fn solve_drill(app: &mut App, case: &Case, before: i32, after: i32) {
    let top = |quarters: i32| Turn::new(rubx::cube::Axis::Y, rubx::cube::Layer::Pos, -quarters);
    let turns = [top(before)]
        .into_iter()
        .chain(case.turns())
        .chain([top(after)])
        .filter(|turn| !turn.is_identity());
    for next in turns {
        app.world_mut().write_message(GameCommand::Turn(next));
        settle(app);
    }
}

#[test]
fn drill_solved_unaided_should_count_as_clean() {
    let mut app = app();
    let (case, before, after) = drill(&mut app);
    solve_drill(&mut app, case, before, after);
    let stats = progress(&app).stats(&stats_key(case));
    assert_eq!((stats.attempts, stats.clean), (1, 1));
}

#[test]
fn drill_solved_after_revealing_should_not_count_as_clean() {
    let mut app = app();
    let (case, before, after) = drill(&mut app);
    act(&mut app, UiAction::Reveal);
    solve_drill(&mut app, case, before, after);
    let stats = progress(&app).stats(&stats_key(case));
    assert_eq!((stats.attempts, stats.clean), (1, 0));
}

#[test]
fn retrying_a_revealed_drill_should_still_not_count_as_clean() {
    let mut app = app();
    let (case, before, after) = drill(&mut app);
    act(&mut app, UiAction::Reveal);
    act(&mut app, UiAction::Restart);
    solve_drill(&mut app, case, before, after);
    assert_eq!(progress(&app).stats(&stats_key(case)).clean, 0);
}

#[test]
fn drill_solved_the_long_way_should_not_count_as_clean() {
    let mut app = app();
    let (case, before, after) = drill(&mut app);
    for _ in 0..4 {
        turn(&mut app, "R");
    }
    solve_drill(&mut app, case, before, after);
    let stats = progress(&app).stats(&stats_key(case));
    assert_eq!((stats.attempts, stats.clean), (1, 0));
}

/// Solves a drill the way a keyboard does, with every half turn as two key presses.
fn solve_drill_by_keyboard(app: &mut App, case: &Case, before: i32, after: i32) {
    let top = |quarters: i32| Turn::new(rubx::cube::Axis::Y, rubx::cube::Layer::Pos, -quarters);
    let turns: Vec<Turn> = [top(before)]
        .into_iter()
        .chain(case.turns())
        .chain([top(after)])
        .filter(|turn| !turn.is_identity())
        .collect();
    for next in turns {
        let presses = if next.quarters == 2 { 2 } else { 1 };
        let single = Turn::new(next.axis, next.layer, i32::from(next.quarters) / presses);
        for _ in 0..presses {
            app.world_mut().write_message(GameCommand::Turn(single));
            settle(app);
        }
    }
}

#[test]
fn drill_solved_by_keyboard_should_count_as_clean() {
    let mut app = app();
    // The starter set is mostly half-turn algorithms, so several runs are sure to include some.
    for _ in 0..12 {
        let (case, before, after) = drill(&mut app);
        solve_drill_by_keyboard(&mut app, case, before, after);
        assert!(run(&app).result.unwrap().clean, "{}", case.name);
    }
}

fn undo(app: &mut App) {
    app.world_mut().write_message(GameCommand::Undo);
    settle(app);
}

#[test]
fn drill_with_one_undone_slip_should_still_count_as_clean() {
    let mut app = app();
    let (case, before, after) = drill(&mut app);
    turn(&mut app, "R");
    undo(&mut app);
    solve_drill(&mut app, case, before, after);
    assert_eq!(progress(&app).stats(&stats_key(case)).clean, 1);
}

#[test]
fn drill_with_several_undone_moves_should_not_count_as_clean() {
    let mut app = app();
    let (case, before, after) = drill(&mut app);
    for text in ["R", "U", "F"] {
        turn(&mut app, text);
    }
    for _ in 0..3 {
        undo(&mut app);
    }
    solve_drill(&mut app, case, before, after);
    assert_eq!(progress(&app).stats(&stats_key(case)).clean, 0);
}

#[test]
fn demonstration_should_not_set_a_best_time() {
    let mut app = app();
    let (case, ..) = drill(&mut app);
    act(&mut app, UiAction::ShowMe);
    settle(&mut app);
    let stats = progress(&app).stats(&stats_key(case));
    assert_eq!(
        (stats.attempts, stats.best_secs, session(&app).best_secs),
        (1, None, None),
    );
}

#[test]
fn retrying_a_finished_drill_should_not_be_scored_again() {
    let mut app = app();
    let (case, before, after) = drill(&mut app);
    solve_drill(&mut app, case, before, after);
    act(&mut app, UiAction::Restart);
    solve_drill(&mut app, case, before, after);
    let stats = progress(&app).stats(&stats_key(case));
    assert_eq!((stats.attempts, stats.clean), (1, 1));
}

#[test]
fn retrying_an_unfinished_drill_should_not_count_as_clean() {
    let mut app = app();
    let (case, before, after) = drill(&mut app);
    turn(&mut app, "R");
    act(&mut app, UiAction::Restart);
    solve_drill(&mut app, case, before, after);
    let stats = progress(&app).stats(&stats_key(case));
    assert_eq!((stats.attempts, stats.clean, stats.best_secs), (1, 0, None));
}

#[test]
fn undo_button_should_do_nothing_during_a_demonstration() {
    let mut app = app();
    drill(&mut app);
    act(&mut app, UiAction::ShowMe);
    for _ in 0..40 {
        app.update();
    }
    act(&mut app, UiAction::Game(GameCommand::Undo));
    settle(&mut app);
    assert!(cube(&app).is_solved());
}

#[test]
fn show_me_should_finish_a_drill() {
    let mut app = app();
    drill(&mut app);
    act(&mut app, UiAction::ShowMe);
    settle(&mut app);
    assert!(cube(&app).is_solved());
}

#[test]
fn oll_drill_should_finish_when_the_top_is_yellow() {
    let mut app = app();
    act(&mut app, UiAction::StartDrill(DrillSet::CrossOll));
    act(&mut app, UiAction::ShowMe);
    settle(&mut app);
    assert!(run(&app).is_finished() && last_layer_oriented(cube(&app)));
}

#[test]
fn next_drill_should_offer_a_different_case() {
    let mut app = app();
    let (first, ..) = drill(&mut app);
    act(&mut app, UiAction::Next);
    let Activity::Drill { case, .. } = run(&app).activity else {
        panic!("expected a drill");
    };
    assert_ne!(case, first);
}

#[test]
fn every_pll_case_should_be_finishable_by_its_own_solution() {
    for case in &PLL {
        for (before, after) in [(0, 0), (1, 3), (2, 1)] {
            let mut cube = case.setup(before, after);
            let top = |q: i32| Turn::new(rubx::cube::Axis::Y, rubx::cube::Layer::Pos, -q);
            cube.apply(top(before));
            cube.apply_all(case.turns());
            cube.apply(top(after));
            assert!(cube.is_solved(), "{}", case.name);
        }
    }
}

#[test]
fn every_practice_stage_should_finish_by_following_hints() {
    let reached: [fn(&Cube) -> bool; 4] = [
        Stage::Cross.goal(),
        first_two_layers_done,
        last_layer_oriented,
        Cube::is_solved,
    ];
    for (stage, reached) in PracticeStage::ALL.into_iter().zip(reached) {
        let mut app = app();
        act(&mut app, UiAction::StartPractice(stage));
        assert!(!reached(cube(&app)), "{stage:?}");
        finish_with_show_me(&mut app);
        assert!(reached(cube(&app)), "{stage:?}");
        assert_eq!(progress(&app).stats(&practice_key(stage)).attempts, 1);
    }
}

#[test]
fn cross_practice_should_report_the_best_possible_length() {
    let mut app = app();
    act(&mut app, UiAction::StartPractice(PracticeStage::Cross));
    assert!(run(&app).cross_par.is_some_and(|par| par <= 8));
}

#[test]
fn retry_should_return_to_the_same_position() {
    let mut app = app();
    act(&mut app, UiAction::StartPractice(PracticeStage::Cross));
    let start = cube(&app).clone();
    turn(&mut app, "R");
    act(&mut app, UiAction::Restart);
    assert_eq!(*cube(&app), start);
}

#[test]
fn next_should_give_a_new_position() {
    let mut app = app();
    act(&mut app, UiAction::StartCoach);
    let start = cube(&app).clone();
    act(&mut app, UiAction::Next);
    assert_ne!(*cube(&app), start);
}

#[test]
fn first_move_in_an_activity_should_start_the_timer() {
    let mut app = app();
    act(&mut app, UiAction::StartPractice(PracticeStage::Cross));
    assert_eq!(session(&app).phase, SolvePhase::Ready);
    turn(&mut app, "R");
    assert!(matches!(session(&app).phase, SolvePhase::Solving { .. }));
}

#[test]
fn solving_in_play_should_save_the_time() {
    let mut app = app();
    act(&mut app, UiAction::Go(Screen::Play));
    app.world_mut().write_message(GameCommand::Scramble);
    settle(&mut app);
    // Replace the scramble with a known position, so that this test can solve it.
    let mut known = Cube::solved();
    known.apply("R".parse().unwrap());
    app.world_mut().resource_mut::<CubeState>().cube = known;
    turn(&mut app, "R'");
    assert_eq!(progress(&app).stats("play").attempts, 1);
}

#[test]
fn drill_time_should_be_shown_as_the_best_when_the_case_returns() {
    let mut app = app();
    let (case, before, after) = drill(&mut app);
    solve_drill(&mut app, case, before, after);
    act(&mut app, UiAction::Restart);
    assert!(session(&app).best_secs.is_some());
}
