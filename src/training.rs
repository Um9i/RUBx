//! Runs the Training activities and records results in both modes.
//!
//! This only deals with game state. The panels in [`crate::menus`] present it, so this all runs in
//! headless tests.

use std::collections::VecDeque;

use bevy::prelude::*;

use crate::algs::Case;
use crate::cube::{Axis, Cube, Layer, Turn};
use crate::drills::{DrillSet, pick_case, stats_key};
use crate::game::{
    CubeState, GameCommand, GameSet, ScrambleRng, Session, SolvePhase, load_free, load_position,
};
use crate::lessons::{Exercise, Follow, LESSONS, Lesson};
use crate::method::{Goal, training_start};
use crate::progress::Progress;
use crate::screens::{NavigateSet, Screen, UiAction};
use crate::setups::{PracticeStage, scrambled, stage_position};
use crate::solver::{HintTracker, cross_distance};

/// The pause between turns of a demonstration, in seconds, so that the player can follow it.
const DEMO_PAUSE_SECS: f32 = 0.22;
/// Extra turns a clean drill attempt may take, for lining up the top layer before and after.
const DRILL_SPARE_TURNS: usize = 2;

const PLAY_KEY: &str = "play";
const COACH_KEY: &str = "coach";

/// The activity the player is doing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Activity {
    /// Nothing from Training. This is the state on menus and in Play.
    #[default]
    None,
    /// A lesson, and how many of its positions have been finished.
    Lesson { index: usize, finished: u32 },
    /// Solving a scramble with the coach.
    Coach,
    /// Drilling a set, on a specific case, with the top turned by these quarter turns.
    Drill {
        set: DrillSet,
        case: &'static Case,
        before: i32,
        after: i32,
    },
    /// Practising one stage of a solve.
    Practice(PracticeStage),
}

/// What happened in the most recently finished attempt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttemptResult {
    /// How long it took, in seconds.
    pub secs: f32,
    /// How many moves it took.
    pub moves: usize,
    /// Whether it was done without any help.
    pub clean: bool,
}

/// The state of the current training activity.
#[derive(Resource, Default)]
pub struct Run {
    /// The activity.
    pub activity: Activity,
    /// The plan behind hints.
    pub tracker: HintTracker,
    /// Whether the next move is being shown.
    pub hint_visible: bool,
    /// How many times the player asked for a hint this attempt.
    pub hints_used: u32,
    /// Whether the player revealed the algorithm this attempt.
    pub revealed: bool,
    /// Whether the game made some moves for the player this attempt.
    pub assisted: bool,
    /// The result, once the attempt is finished.
    pub result: Option<AttemptResult>,
    /// The turns being followed, in a lesson that uses them.
    pub follow: Option<Follow>,
    /// The fewest turns that could finish the cross, when practising the cross.
    pub cross_par: Option<usize>,
    start: Option<(Cube, Goal)>,
    demo: VecDeque<Turn>,
    demo_wait: f32,
}

impl Run {
    /// Returns whether the current attempt has finished.
    pub fn is_finished(&self) -> bool {
        self.result.is_some()
    }

    /// Returns the turn to point out on the cube, if any.
    pub fn arrow_turn(&self) -> Option<Turn> {
        if let Some(&turn) = self.demo.front() {
            return Some(turn);
        }
        if self.is_finished() {
            return None;
        }
        if let Some(follow) = &self.follow {
            return follow.next_turn();
        }
        if self.hint_visible {
            return self.tracker.next_turn();
        }
        None
    }

    /// Returns the index of the piece to point out on the cube, if any.
    pub fn target_piece(&self) -> Option<usize> {
        if self.hint_visible && !self.is_finished() {
            self.tracker.hint()?.target
        } else {
            None
        }
    }

    /// Returns the current lesson, if the player is on one.
    pub fn lesson(&self) -> Option<&'static Lesson> {
        match self.activity {
            Activity::Lesson { index, .. } => LESSONS.get(index),
            _ => None,
        }
    }

    /// Returns whether all positions of the current lesson are finished.
    pub fn lesson_complete(&self) -> bool {
        match (self.activity, self.lesson()) {
            (Activity::Lesson { finished, .. }, Some(lesson)) => finished >= lesson.positions(),
            _ => false,
        }
    }

    /// Clears everything about the current attempt.
    fn begin_attempt(&mut self, start: (Cube, Goal)) {
        self.tracker = HintTracker::default();
        self.hint_visible = false;
        self.hints_used = 0;
        self.revealed = false;
        self.assisted = false;
        self.result = None;
        self.follow = None;
        self.cross_par = None;
        self.demo.clear();
        self.start = Some(start);
    }
}

/// Adds the training activities. Needs [`crate::game::GamePlugin`] and
/// [`crate::screens::ScreenPlugin`], but no window.
pub struct TrainingPlugin;

impl Plugin for TrainingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Run>()
            .insert_resource(Progress::load_default())
            .add_systems(
                Update,
                handle_actions.in_set(GameSet::Input).after(NavigateSet),
            )
            .add_systems(
                Update,
                (play_demo, follow_along, detect_finish, record_lessons)
                    .chain()
                    .after(GameSet::Simulate)
                    .before(GameSet::Present),
            )
            .add_systems(OnEnter(Screen::Play), enter_play);
        for screen in [
            Screen::Menu,
            Screen::Hub,
            Screen::LessonList,
            Screen::DrillList,
            Screen::PracticeList,
        ] {
            app.add_systems(OnEnter(screen), enter_menu);
        }
    }
}

fn enter_play(
    mut run: ResMut<Run>,
    mut state: ResMut<CubeState>,
    mut session: ResMut<Session>,
    progress: Res<Progress>,
) {
    *run = Run::default();
    load_free(&mut state, &mut session, Cube::solved());
    session.best_secs = progress.stats(PLAY_KEY).best_secs;
}

/// Shows a neat, solved cube behind the menus.
fn enter_menu(mut run: ResMut<Run>, mut state: ResMut<CubeState>, mut session: ResMut<Session>) {
    *run = Run::default();
    load_free(&mut state, &mut session, training_start());
}

/// Bundles everything an action might need to change.
struct Context<'a> {
    run: &'a mut Run,
    state: &'a mut CubeState,
    session: &'a mut Session,
    rng: &'a mut fastrand::Rng,
    progress: &'a Progress,
}

impl Context<'_> {
    /// Loads a position, ready for the player's first move.
    fn load(&mut self, cube: Cube, goal: Goal) {
        self.run.begin_attempt((cube.clone(), goal));
        load_position(self.state, self.session, cube, goal);
    }

    fn start_lesson(&mut self, index: usize, finished: u32) {
        let Some(lesson) = LESSONS.get(index) else {
            return;
        };
        self.session.best_secs = None;
        match lesson.exercise {
            Exercise::Follow(sequences) => {
                let start = training_start();
                self.load(start.clone(), |_| false);
                let text = sequences
                    .get(finished as usize)
                    .or(sequences.last())
                    .copied()
                    .unwrap_or_default();
                self.run.follow = Some(Follow::new(&start, text));
                self.session.phase = SolvePhase::Free;
            }
            Exercise::Stage(stage) => {
                let cube = stage_position(stage, self.rng);
                self.load(cube, stage.goal());
            }
        }
        self.run.activity = Activity::Lesson { index, finished };
    }

    fn start_coach(&mut self) {
        let cube = scrambled(self.rng);
        self.load(cube, Cube::is_solved);
        self.run.activity = Activity::Coach;
        self.session.best_secs = self.progress.stats(COACH_KEY).best_secs;
    }

    fn start_drill(&mut self, set: DrillSet, last: Option<&Case>) {
        let case = pick_case(set, self.progress, last, self.rng);
        let (before, after) = (self.rng.i32(0..4), self.rng.i32(0..4));
        let goal = if set.is_pll() {
            Cube::is_solved
        } else {
            crate::method::last_layer_oriented
        };
        self.load(case.setup(before, after), goal);
        self.run.activity = Activity::Drill {
            set,
            case,
            before,
            after,
        };
        self.session.best_secs = self.progress.stats(&stats_key(case)).best_secs;
    }

    fn start_practice(&mut self, stage: PracticeStage) {
        let cube = stage.position(self.rng);
        self.load(cube.clone(), stage.goal());
        self.run.activity = Activity::Practice(stage);
        if stage == PracticeStage::Cross {
            self.run.cross_par = Some(cross_distance(&cube));
        }
        self.session.best_secs = self.progress.stats(&practice_key(stage)).best_secs;
    }

    fn next(&mut self, screen: &mut NextState<Screen>) {
        match self.run.activity {
            Activity::None => {}
            Activity::Lesson { index, finished } => {
                if self.run.lesson_complete() {
                    if index + 1 < LESSONS.len() {
                        self.start_lesson(index + 1, 0);
                    } else {
                        screen.set(Screen::LessonList);
                    }
                } else {
                    self.start_lesson(index, finished);
                }
            }
            Activity::Coach => self.start_coach(),
            Activity::Drill { set, case, .. } => self.start_drill(set, Some(case)),
            Activity::Practice(stage) => self.start_practice(stage),
        }
    }

    /// Puts the current attempt back to its start.
    fn restart(&mut self) {
        let Some((cube, goal)) = self.run.start.clone() else {
            return;
        };
        let follow_text = self.follow_text();
        let par = self.run.cross_par;
        // Having seen the answer still counts on a retry, or a reveal could never cost anything.
        let (revealed, hints_used, assisted) =
            (self.run.revealed, self.run.hints_used, self.run.assisted);
        self.load(cube.clone(), goal);
        self.run.cross_par = par;
        self.run.revealed = revealed;
        self.run.hints_used = hints_used;
        self.run.assisted = assisted;
        if let Some(text) = follow_text {
            self.run.follow = Some(Follow::new(&cube, text));
            self.session.phase = SolvePhase::Free;
        }
    }

    fn follow_text(&self) -> Option<&'static str> {
        match (self.run.activity, self.run.lesson()?.exercise) {
            (Activity::Lesson { finished, .. }, Exercise::Follow(sequences)) => sequences
                .get(finished as usize)
                .or(sequences.last())
                .copied(),
            _ => None,
        }
    }

    fn toggle_hint(&mut self) {
        if self.run.is_finished() {
            return;
        }
        if matches!(self.run.activity, Activity::Drill { .. }) {
            self.run.revealed = true;
            return;
        }
        self.run.hint_visible = !self.run.hint_visible;
        if self.run.hint_visible {
            self.run.hints_used += 1;
            self.run.tracker.sync(&self.state.cube);
        }
    }

    /// Plays moves for the player. They watch, and the attempt no longer counts as unaided.
    fn show_me(&mut self) {
        if self.run.is_finished() || !self.run.demo.is_empty() {
            return;
        }
        if let Activity::Drill {
            case,
            before,
            after,
            ..
        } = self.run.activity
        {
            self.restart();
            self.run.demo = drill_solution(case, before, after).into();
        } else if let Some(follow) = &self.run.follow {
            self.run.demo.extend(follow.next_turn());
        } else {
            self.run.tracker.sync(&self.state.cube);
            if let Some((_, turns, _)) = self.run.tracker.remaining() {
                self.run.demo = turns.iter().copied().collect();
            }
        }
        if !self.run.demo.is_empty() {
            self.run.assisted = true;
            self.run.demo_wait = 0.0;
            self.state.locked = true;
        }
    }
}

/// Returns the full solution to a drill: line up the top, make the moves, and line it up again.
fn drill_solution(case: &Case, before: i32, after: i32) -> Vec<Turn> {
    let top = |quarters: i32| Turn::new(Axis::Y, Layer::Pos, -quarters);
    [top(before)]
        .into_iter()
        .chain(case.turns())
        .chain([top(after)])
        .filter(|turn| !turn.is_identity())
        .collect()
}

/// Returns the key that a practice stage's results are saved under.
pub fn practice_key(stage: PracticeStage) -> String {
    format!("practice:{}", stage.name())
}

/// Returns whether actions that change the position apply to this screen.
fn is_training_screen(screen: Screen) -> bool {
    matches!(
        screen,
        Screen::Lesson | Screen::Coach | Screen::Drill | Screen::Practice
    )
}

#[expect(clippy::too_many_arguments)]
fn handle_actions(
    mut actions: MessageReader<UiAction>,
    current: Res<State<Screen>>,
    mut screen: ResMut<NextState<Screen>>,
    mut run: ResMut<Run>,
    mut state: ResMut<CubeState>,
    mut session: ResMut<Session>,
    mut rng: ResMut<ScrambleRng>,
    progress: Res<Progress>,
) {
    let mut context = Context {
        run: &mut run,
        state: &mut state,
        session: &mut session,
        rng: &mut rng.0,
        progress: &progress,
    };
    for action in actions.read() {
        let on_training_screen = is_training_screen(*current.get());
        match *action {
            UiAction::StartLesson(index) => {
                context.start_lesson(index, 0);
                screen.set(Screen::Lesson);
            }
            UiAction::StartCoach => {
                context.start_coach();
                screen.set(Screen::Coach);
            }
            UiAction::StartDrill(set) => {
                context.start_drill(set, None);
                screen.set(Screen::Drill);
            }
            UiAction::StartPractice(stage) => {
                context.start_practice(stage);
                screen.set(Screen::Practice);
            }
            UiAction::Hint if on_training_screen => context.toggle_hint(),
            UiAction::ShowMe if on_training_screen => context.show_me(),
            UiAction::Next if on_training_screen => context.next(&mut screen),
            UiAction::Restart if on_training_screen => context.restart(),
            UiAction::Reveal
                if on_training_screen
                    && matches!(context.run.activity, Activity::Drill { .. })
                    && !context.run.is_finished() =>
            {
                context.run.revealed = true;
            }
            _ => {}
        }
    }
}

/// Plays queued demonstration turns, pausing between them.
fn play_demo(
    time: Res<Time>,
    mut run: ResMut<Run>,
    mut state: ResMut<CubeState>,
    mut commands: MessageWriter<GameCommand>,
) {
    if !state.locked || state.is_busy() {
        return;
    }
    if run.demo.is_empty() {
        state.locked = false;
        return;
    }
    run.demo_wait -= time.delta_secs();
    if run.demo_wait <= 0.0
        && let Some(turn) = run.demo.pop_front()
    {
        commands.write(GameCommand::Turn(turn));
        run.demo_wait = DEMO_PAUSE_SECS;
    }
}

/// Keeps hint plans and followed moves up to date, and finishes follow-along exercises.
fn follow_along(mut run: ResMut<Run>, state: Res<CubeState>, session: Res<Session>) {
    if state.is_busy() || state.is_dragging() {
        return;
    }
    let run = &mut *run;
    if run.hint_visible {
        run.tracker.sync(&state.cube);
    }
    if let Some(follow) = &mut run.follow {
        follow.sync(&state.cube);
        if follow.is_finished() && run.result.is_none() {
            run.result = Some(AttemptResult {
                secs: 0.0,
                moves: session.move_count(),
                clean: !run.assisted,
            });
            finish_lesson_position(&mut run.activity);
        }
    }
}

fn finish_lesson_position(activity: &mut Activity) {
    if let Activity::Lesson { finished, .. } = activity {
        *finished += 1;
    }
}

/// Notices when an attempt reaches its goal, and records the result.
fn detect_finish(
    mut run: ResMut<Run>,
    session: Res<Session>,
    current: Res<State<Screen>>,
    mut progress: ResMut<Progress>,
) {
    let run = &mut *run;
    let SolvePhase::Solved { secs, moves } = session.phase else {
        // Play has no attempts to start explicitly, so it re-arms when the result clears.
        if *current.get() == Screen::Play {
            run.result = None;
        }
        return;
    };
    if run.result.is_some() {
        return;
    }
    let unaided = !run.assisted && !run.revealed && run.hints_used == 0;
    let mut clean = unaided;
    match run.activity {
        Activity::None => {
            if *current.get() == Screen::Play {
                progress.record(PLAY_KEY, secs, true);
            }
        }
        Activity::Lesson { .. } => finish_lesson_position(&mut run.activity),
        Activity::Coach => progress.record(COACH_KEY, secs, unaided),
        Activity::Drill { case, .. } => {
            clean = unaided && moves <= case.turn_count() + DRILL_SPARE_TURNS;
            progress.record(&stats_key(case), secs, clean);
        }
        Activity::Practice(stage) => progress.record(&practice_key(stage), secs, unaided),
    }
    run.result = Some(AttemptResult { secs, moves, clean });
}

/// Saves a lesson as complete once its last position is finished.
fn record_lessons(run: Res<Run>, mut progress: ResMut<Progress>) {
    if !run.is_changed() || !run.lesson_complete() {
        return;
    }
    if let Some(lesson) = run.lesson()
        && !progress.lesson_done(lesson.id)
    {
        progress.complete_lesson(lesson.id);
    }
}
