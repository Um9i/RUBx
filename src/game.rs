//! Game rules: the turn queue, turn animation timing, undo history, and the solve timer.
//!
//! Nothing here touches rendering or input, so it can run headless in tests.

use std::collections::VecDeque;
use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;

use crate::cube::{self, Axis, Cube, Layer, Turn};
use crate::method::Goal;

/// How long a quarter turn takes when nothing is waiting behind it, in seconds.
const QUARTER_TURN_SECS: f32 = 0.18;
/// How long a quarter turn takes during a scramble, in seconds.
const SCRAMBLE_TURN_SECS: f32 = 0.055;
/// The shortest any turn animation may take, in seconds.
const MIN_TURN_SECS: f32 = 0.04;
/// The number of turns in a scramble.
pub const SCRAMBLE_LENGTH: usize = 25;

/// A request to change the game, from the keyboard, the HUD, or a script.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameCommand {
    /// Queue a turn made by the player.
    Turn(Turn),
    /// Reverse the most recent player turn.
    Undo,
    /// Replay the most recently undone turn.
    Redo,
    /// Scramble the cube and arm the solve timer.
    Scramble,
    /// Return immediately to a solved cube and clear all progress.
    Reset,
}

/// A layer has begun moving on its own, from a queued turn or a released drag.
///
/// Whole-cube rotations and drags that snap back to where they started don't count.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct TurnStarted {
    /// Whether this turn is part of a scramble.
    pub scramble: bool,
}

/// Runs the game systems in a stable order within `Update`.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub enum GameSet {
    /// Turns device input into commands and drag updates.
    Input,
    /// Applies commands and advances the simulation.
    Simulate,
    /// Shows the resulting state.
    Present,
}

/// The authoritative state of the cube and everything being done to it.
#[derive(Resource, Default)]
pub struct CubeState {
    /// The cube with all finished turns applied. Queued and active turns are not included.
    pub cube: Cube,
    queue: VecDeque<QueuedTurn>,
    active: Option<ActiveTurn>,
    drag: Option<LayerAngle>,
    flush_requested: bool,
    /// Whether the player's input is being ignored, such as during a demonstration.
    pub locked: bool,
}

/// A layer shown rotated away from its resting position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerAngle {
    /// The axis the layer rotates about.
    pub axis: Axis,
    /// The layer being rotated.
    pub layer: Layer,
    /// The angle in radians, positive by the right-hand rule about `+axis`.
    pub angle: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TurnSource {
    Player,
    /// An undo or redo. It affects the cube and timer but its history is already recorded.
    Replay,
    Scramble,
}

#[derive(Clone, Copy, Debug)]
struct QueuedTurn {
    turn: Turn,
    source: TurnSource,
}

#[derive(Clone, Copy, Debug)]
struct ActiveTurn {
    /// The turn to apply on completion, which is an identity turn if a drag snapped back.
    turn: Turn,
    source: TurnSource,
    axis: Axis,
    layer: Layer,
    from: f32,
    to: f32,
    elapsed: f32,
    duration: f32,
    /// Whether [`TurnStarted`] has been sent for this turn.
    announced: bool,
}

impl ActiveTurn {
    fn angle(&self) -> f32 {
        let t = (self.elapsed / self.duration).clamp(0.0, 1.0);
        let eased = 1.0 - (1.0 - t).powi(3);
        self.from + (self.to - self.from) * eased
    }
}

impl CubeState {
    /// Returns the layer currently shown away from rest, if any.
    pub fn rotating_layer(&self) -> Option<LayerAngle> {
        self.drag.or_else(|| {
            self.active.as_ref().map(|active| LayerAngle {
                axis: active.axis,
                layer: active.layer,
                angle: active.angle(),
            })
        })
    }

    /// Returns whether a turn is animating or waiting to.
    pub fn is_busy(&self) -> bool {
        self.active.is_some() || !self.queue.is_empty()
    }

    /// Returns whether the player is holding a layer with the mouse.
    pub fn is_dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Asks the simulation to finish all animating and queued turns on its next update.
    ///
    /// This keeps fast mouse play responsive: grabbing the cube completes a turn still settling
    /// from the previous drag rather than ignoring the new one.
    pub fn request_flush(&mut self) {
        self.flush_requested = true;
    }

    /// Shows a layer following the mouse. Queued turns wait until the drag ends.
    pub fn set_drag(&mut self, axis: Axis, layer: Layer, angle: f32) {
        self.drag = Some(LayerAngle { axis, layer, angle });
    }

    /// Abandons a drag without moving anything.
    pub fn cancel_drag(&mut self) {
        self.drag = None;
    }
}

/// Player progress: move history and where the current solve stands.
#[derive(Resource)]
pub struct Session {
    history: Vec<Turn>,
    redo: Vec<Turn>,
    undone_quarters: usize,
    /// Where the current solve stands.
    pub phase: SolvePhase,
    /// What counts as finished. Training sets this to the part of the solve being practised.
    pub goal: Goal,
    /// The fastest time for the current activity, in seconds.
    pub best_secs: Option<f32>,
}

impl Default for Session {
    fn default() -> Self {
        Self {
            history: Vec::new(),
            redo: Vec::new(),
            undone_quarters: 0,
            phase: SolvePhase::default(),
            goal: Cube::is_solved,
            best_secs: None,
        }
    }
}

impl Session {
    /// Returns the number of moves made, ignoring whole-cube rotations.
    pub fn move_count(&self) -> usize {
        self.history
            .iter()
            .filter(|turn| !turn.is_whole_cube())
            .count()
    }

    /// Returns the quarter turns made so far, including any that were undone.
    ///
    /// A half turn costs two either way, so this doesn't depend on whether it was made with one
    /// drag or two key presses.
    pub fn quarters_turned(&self) -> usize {
        let kept: usize = self.history.iter().map(|turn| turn.quarter_cost()).sum();
        kept + self.undone_quarters
    }

    /// Returns whether there is a turn to redo.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    fn record(&mut self, turn: Turn) {
        self.history.push(turn);
        self.redo.clear();
    }

    fn clear_history(&mut self) {
        self.history.clear();
        self.redo.clear();
        self.undone_quarters = 0;
    }
}

/// Where the current solve stands.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum SolvePhase {
    /// Free play, with no timer.
    #[default]
    Free,
    /// A scramble is being played out.
    Scrambling,
    /// Scrambled and waiting for the player's first move, which starts the timer.
    Ready,
    /// The timer is running.
    Solving { secs: f32 },
    /// Solved. The result stays visible until the next move.
    Solved { secs: f32, moves: usize },
}

/// The random source for scrambles.
#[derive(Resource)]
pub struct ScrambleRng(pub fastrand::Rng);

impl Default for ScrambleRng {
    fn default() -> Self {
        Self(fastrand::Rng::new())
    }
}

/// Adds the game rules. Doesn't need a window, so tests can use it with `MinimalPlugins`.
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<GameCommand>()
            .add_message::<TurnStarted>()
            .init_resource::<CubeState>()
            .init_resource::<Session>()
            .init_resource::<ScrambleRng>()
            .configure_sets(
                Update,
                (GameSet::Input, GameSet::Simulate, GameSet::Present).chain(),
            )
            .add_systems(
                Update,
                (handle_commands, advance_turns)
                    .chain()
                    .in_set(GameSet::Simulate),
            );
    }
}

fn handle_commands(
    mut commands: MessageReader<GameCommand>,
    mut state: ResMut<CubeState>,
    mut session: ResMut<Session>,
    mut rng: ResMut<ScrambleRng>,
) {
    for &command in commands.read() {
        // A scramble must play out undisturbed, or history would mix with scramble turns. A held
        // layer finishes before anything queued, so accepting a command during a drag would record
        // it in history ahead of a turn that actually happens first, and undo would then corrupt
        // the cube.
        let locked = session.phase == SolvePhase::Scrambling || state.is_dragging();
        if locked && command != GameCommand::Reset {
            continue;
        }
        match command {
            GameCommand::Turn(turn) => {
                if !turn.is_identity() {
                    session.record(turn);
                    enqueue(&mut state, turn, TurnSource::Player);
                }
            }
            GameCommand::Undo => {
                if let Some(turn) = session.history.pop() {
                    session.redo.push(turn);
                    session.undone_quarters += turn.quarter_cost();
                    enqueue(&mut state, turn.inverse(), TurnSource::Replay);
                }
            }
            GameCommand::Redo => {
                if let Some(turn) = session.redo.pop() {
                    session.history.push(turn);
                    session.undone_quarters -= turn.quarter_cost();
                    enqueue(&mut state, turn, TurnSource::Replay);
                }
            }
            GameCommand::Scramble => {
                session.clear_history();
                session.phase = SolvePhase::Scrambling;
                for turn in cube::scramble(&mut rng.0, SCRAMBLE_LENGTH) {
                    enqueue(&mut state, turn, TurnSource::Scramble);
                }
            }
            GameCommand::Reset => {
                load_free(&mut state, &mut session, Cube::solved());
            }
        }
    }
}

fn enqueue(state: &mut CubeState, turn: Turn, source: TurnSource) {
    state.queue.push_back(QueuedTurn { turn, source });
}

fn advance_turns(
    time: Res<Time>,
    mut state: ResMut<CubeState>,
    mut session: ResMut<Session>,
    mut started: MessageWriter<TurnStarted>,
) {
    let dt = time.delta_secs();
    let state = &mut *state;

    if let SolvePhase::Solving { secs } = &mut session.phase {
        *secs += dt;
    }

    if std::mem::take(&mut state.flush_requested) && session.phase != SolvePhase::Scrambling {
        if let Some(active) = state.active.take() {
            finish_turn(&mut state.cube, &mut session, active.turn, active.source);
        }
        while let Some(queued) = state.queue.pop_front() {
            finish_turn(&mut state.cube, &mut session, queued.turn, queued.source);
        }
    }

    if let Some(active) = &mut state.active {
        active.elapsed += dt;
        if active.elapsed >= active.duration {
            let (turn, source) = (active.turn, active.source);
            state.active = None;
            finish_turn(&mut state.cube, &mut session, turn, source);
        }
    }

    if state.active.is_none()
        && state.drag.is_none()
        && let Some(queued) = state.queue.pop_front()
    {
        let backlog = state.queue.len();
        state.active = Some(start_turn(queued, backlog));
    }

    if let Some(active) = &mut state.active
        && !std::mem::replace(&mut active.announced, true)
        && !active.turn.is_identity()
        && !active.turn.is_whole_cube()
    {
        started.write(TurnStarted {
            scramble: active.source == TurnSource::Scramble,
        });
    }

    if session.phase == SolvePhase::Scrambling && !state.is_busy() {
        session.phase = SolvePhase::Ready;
    }
}

/// Builds the animation for a queued turn, speeding up when more turns wait behind it.
fn start_turn(queued: QueuedTurn, backlog: usize) -> ActiveTurn {
    let QueuedTurn { turn, source } = queued;
    let base = match source {
        TurnSource::Scramble => SCRAMBLE_TURN_SECS,
        TurnSource::Player | TurnSource::Replay => QUARTER_TURN_SECS / (1.0 + backlog as f32 * 0.6),
    };
    let to = f32::from(turn.quarters) * FRAC_PI_2;
    ActiveTurn {
        turn,
        source,
        axis: turn.axis,
        layer: turn.layer,
        from: 0.0,
        to,
        elapsed: 0.0,
        duration: scaled_duration(base, to),
        announced: false,
    }
}

/// Scales a quarter turn's duration to a rotation of `distance` radians.
///
/// The square root keeps a half turn from feeling sluggish and a small settle from being instant.
fn scaled_duration(quarter_turn_secs: f32, distance: f32) -> f32 {
    (quarter_turn_secs * (distance.abs() / FRAC_PI_2).sqrt()).max(MIN_TURN_SECS)
}

/// Applies a completed turn to the cube and updates the solve timer.
fn finish_turn(cube: &mut Cube, session: &mut Session, turn: Turn, source: TurnSource) {
    cube.apply(turn);
    if source == TurnSource::Scramble || turn.is_identity() || turn.is_whole_cube() {
        return;
    }
    match session.phase {
        SolvePhase::Ready => session.phase = SolvePhase::Solving { secs: 0.0 },
        SolvePhase::Solved { .. } => session.phase = SolvePhase::Free,
        SolvePhase::Free | SolvePhase::Scrambling | SolvePhase::Solving { .. } => {}
    }
    if let SolvePhase::Solving { secs } = session.phase
        && (session.goal)(cube)
    {
        session.best_secs = Some(session.best_secs.map_or(secs, |best| best.min(secs)));
        session.phase = SolvePhase::Solved {
            secs,
            moves: session.move_count(),
        };
    }
}

/// Replaces the cube with a position to work on. The player's first move will start the timer.
pub fn load_position(state: &mut CubeState, session: &mut Session, cube: Cube, goal: Goal) {
    *state = CubeState { cube, ..default() };
    session.clear_history();
    session.goal = goal;
    session.phase = SolvePhase::Ready;
}

/// Replaces the cube with a position for free play, with no timer.
pub fn load_free(state: &mut CubeState, session: &mut Session, cube: Cube) {
    load_position(state, session, cube, Cube::is_solved);
    session.phase = SolvePhase::Free;
}

/// The fraction of a quarter turn past which a dragged layer continues to the next position.
pub const SNAP_THRESHOLD_QUARTERS: f32 = 0.3;

/// Ends a mouse drag, animating the held layer from `angle` to its nearest resting place.
///
/// Dragging at least [`SNAP_THRESHOLD_QUARTERS`] of a quarter turn counts as a turn, so the player
/// doesn't have to drag the full 45° that plain rounding would require.
pub fn release_drag(state: &mut CubeState, session: &mut Session) {
    let Some(LayerAngle { axis, layer, angle }) = state.drag.take() else {
        return;
    };
    let quarters = snap_quarters(angle);
    let turn = Turn::new(axis, layer, quarters);
    if !turn.is_identity() {
        session.record(turn);
    }
    let to = quarters as f32 * FRAC_PI_2;
    state.active = Some(ActiveTurn {
        turn,
        source: TurnSource::Player,
        axis,
        layer,
        from: angle,
        to,
        elapsed: 0.0,
        duration: scaled_duration(QUARTER_TURN_SECS * 0.8, to - angle),
        announced: false,
    });
}

/// Returns the whole number of quarter turns a drag of `angle` radians settles on.
fn snap_quarters(angle: f32) -> i32 {
    let quarters = angle.abs() / FRAC_PI_2;
    let snapped = (quarters + 1.0 - SNAP_THRESHOLD_QUARTERS).floor() as i32;
    if angle < 0.0 { -snapped } else { snapped }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_drag_should_snap_back() {
        assert_eq!(snap_quarters(0.2 * FRAC_PI_2), 0);
    }

    #[test]
    fn drag_past_threshold_should_complete_the_quarter_turn() {
        assert_eq!(snap_quarters(0.35 * FRAC_PI_2), 1);
    }

    #[test]
    fn negative_drag_past_threshold_should_turn_the_other_way() {
        assert_eq!(snap_quarters(-0.35 * FRAC_PI_2), -1);
    }

    #[test]
    fn drag_past_a_quarter_turn_should_not_reach_the_next_without_the_threshold() {
        assert_eq!(snap_quarters(1.2 * FRAC_PI_2), 1);
    }

    #[test]
    fn drag_near_a_half_turn_should_settle_on_two_quarters() {
        assert_eq!(snap_quarters(1.8 * FRAC_PI_2), 2);
    }
}
