//! A hint engine that solves the way the beginner method is taught.
//!
//! Instead of finding the shortest solution, each stage searches over the same moves a learner is
//! taught for that stage. Each step of a hint is therefore something they recognise, like
//! `R U R' U'`, and following hints demonstrates the method.
//!
//! Searches stay tiny because they compare states by only what the stage cares about. A corner
//! has just 24 placements, for example, and every available move is one that can't disturb
//! pieces already solved.

use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::Hash;
use std::sync::LazyLock;

use crate::cube::{Axis, Cube, Layer, Rotation, Turn, parse_sequence};
use crate::method::{
    CROSS_EDGES, FIRST_CORNERS, MIDDLE_EDGES, MethodView, Stage, TOP_CORNERS, TOP_EDGES,
    piece_solved, stage_of_view,
};

/// What to do next, in the player's frame of reference.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hint {
    /// The stage the cube is on.
    pub stage: Stage,
    /// The steps that finish this part of the stage, in order.
    pub steps: Vec<HintStep>,
    /// The index in [`Cube::cubies`] of the piece being worked on, if there is just one.
    pub target: Option<usize>,
}

/// A chunk of a hint: one taught move sequence and why to make it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HintStep {
    /// The turns to make.
    pub turns: Vec<Turn>,
    /// Why this step helps.
    pub why: &'static str,
}

/// Returns what to do next, or `None` if the cube is already solved.
pub fn hint(cube: &Cube) -> Option<Hint> {
    let view = MethodView::of(cube);
    let stage = view.stage();
    let (moves, target) = match stage {
        Stage::Cross => cross_steps(&view),
        Stage::FirstCorners => first_corner_steps(&view)?,
        Stage::MiddleEdges => middle_edge_steps(&view)?,
        Stage::YellowCross => (
            last_layer_steps(&view, &MOVES.yellow_cross, |v| {
                orientation_key(v, &TOP_EDGES)
            })?,
            None,
        ),
        Stage::YellowCorners => (
            last_layer_steps(&view, &MOVES.sune, |v| orientation_key(v, &TOP_CORNERS))?,
            None,
        ),
        Stage::PlaceCorners => (
            last_layer_steps(&view, &MOVES.a_perm, |v| placement_key(v, &TOP_CORNERS))?,
            None,
        ),
        Stage::PlaceEdges => (
            last_layer_steps(&view, &MOVES.u_perm, |v| {
                let mut key = placement_key(v, &TOP_CORNERS);
                key.extend(placement_key(v, &TOP_EDGES));
                key
            })?,
            None,
        ),
        Stage::Done => return None,
    };
    let steps = moves
        .into_iter()
        .map(|step| HintStep {
            turns: step
                .turns
                .iter()
                .map(|&turn| view.world_turn(turn))
                .collect(),
            why: step.why,
        })
        .collect();
    Some(Hint {
        stage,
        steps,
        target: target.map(|home| view.world_index(home)),
    })
}

/// Keeps a hint steady while the player works through it, one turn at a time.
///
/// Partway through a trigger like `R U R' U'` the cube looks worse than when it started. Asking
/// for a fresh hint at that point would just say to undo the last turn, so the tracker holds onto
/// its plan and only replans once the player has finished it or gone their own way.
#[derive(Clone, Debug, Default)]
pub struct HintTracker {
    plan: Option<Plan>,
}

#[derive(Clone, Debug)]
struct Plan {
    hint: Hint,
    /// The state of the cube before each turn of the plan, and after its last.
    states: Vec<Cube>,
    /// How many turns of the plan have been made.
    made: usize,
}

impl Plan {
    fn new(cube: &Cube) -> Option<Self> {
        let hint = hint(cube)?;
        let mut states = vec![cube.clone()];
        let mut current = cube.clone();
        for &turn in hint.steps.iter().flat_map(|step| &step.turns) {
            current.apply(turn);
            states.push(current.clone());
        }
        Some(Self {
            hint,
            states,
            made: 0,
        })
    }

    fn is_finished(&self) -> bool {
        self.made + 1 >= self.states.len()
    }
}

impl HintTracker {
    /// Updates the plan to match the cube.
    pub fn sync(&mut self, cube: &Cube) {
        if let Some(plan) = &mut self.plan {
            if plan.states.get(plan.made + 1) == Some(cube) {
                plan.made += 1;
            }
            if plan.states[plan.made] == *cube && !plan.is_finished() {
                return;
            }
        }
        self.plan = Plan::new(cube);
    }

    /// Returns the current hint, if there is one.
    pub fn hint(&self) -> Option<&Hint> {
        self.plan.as_ref().map(|plan| &plan.hint)
    }

    /// Returns the next turn the plan calls for.
    pub fn next_turn(&self) -> Option<Turn> {
        let plan = self.plan.as_ref()?;
        plan.hint
            .steps
            .iter()
            .flat_map(|step| &step.turns)
            .nth(plan.made)
            .copied()
    }

    /// Returns the turns left in the current step, then the steps after it.
    pub fn remaining(&self) -> Option<(&HintStep, &[Turn], &[HintStep])> {
        let plan = self.plan.as_ref()?;
        let mut before = 0;
        for (index, step) in plan.hint.steps.iter().enumerate() {
            if plan.made < before + step.turns.len() {
                let rest = &plan.hint.steps[index + 1..];
                return Some((step, &step.turns[plan.made - before..], rest));
            }
            before += step.turns.len();
        }
        None
    }
}

/// A move sequence the method teaches.
#[derive(Clone, Debug)]
struct Move {
    turns: Vec<Turn>,
    why: &'static str,
}

impl Move {
    fn new(text: &str, why: &'static str) -> Self {
        let turns = parse_sequence(text).unwrap_or_else(|error| panic!("{error}"));
        Self { turns, why }
    }
}

/// All the moves taught by the method. They're literals, so a parse failure is a typo here that
/// any test run would catch.
struct Moves {
    top_turns: [Move; 3],
    corner_trigger: [Move; 4],
    middle_insert: [[Move; 2]; 4],
    yellow_cross: Move,
    sune: Move,
    a_perm: Move,
    u_perm: Move,
}

static MOVES: LazyLock<Moves> = LazyLock::new(|| {
    const TOP: &str = "Turn the top layer to line the pieces up";
    // Hints use whichever face reaches the slot, while lessons teach the front right version. Say
    // that it's the same move, or a hint looks like it contradicts the lesson.
    let trigger = |text, why| Move::new(text, why);
    let insert =
        |right, right_why, left, left_why| [Move::new(right, right_why), Move::new(left, left_why)];
    Moves {
        top_turns: ["U", "U'", "U2"].map(|text| Move::new(text, TOP)),
        corner_trigger: [
            trigger("R U R' U'", "The trigger, on the front right slot"),
            trigger(
                "B U B' U'",
                "The same trigger on the back right slot, with B doing R's job",
            ),
            trigger(
                "L U L' U'",
                "The same trigger on the back left slot, with L doing R's job",
            ),
            trigger(
                "F U F' U'",
                "The same trigger on the front left slot, with F doing R's job",
            ),
        ],
        middle_insert: [
            insert(
                "U R U' R' U' F' U F",
                "Insert to the right, into the front right slot",
                "U' F' U F U R U' R'",
                "Insert to the left, into the front right slot, with R at the front",
            ),
            insert(
                "U B U' B' U' R' U R",
                "Insert to the right, into the back right slot, with R at the front",
                "U' R' U R U B U' B'",
                "Insert to the left, into the back right slot, with B at the front",
            ),
            insert(
                "U L U' L' U' B' U B",
                "Insert to the right, into the back left slot, with B at the front",
                "U' B' U B U L U' L'",
                "Insert to the left, into the back left slot, with L at the front",
            ),
            insert(
                "U F U' F' U' L' U L",
                "Insert to the right, into the front left slot, with L at the front",
                "U' L' U L U F U' F'",
                "Insert to the left, into the front left slot",
            ),
        ],
        yellow_cross: Move::new(
            "F R U R' U' F'",
            "This flips yellow edges: dot to L shape, L shape to line, line to cross",
        ),
        sune: Move::new(
            "R U R' U R U2 R'",
            "This twists the corners without disturbing the yellow cross",
        ),
        a_perm: Move::new(
            "R' F R' B2 R F' R' B2 R2",
            "This cycles three corners and keeps the top yellow",
        ),
        u_perm: Move::new(
            "R2 U R U R' U' R' U' R' U R'",
            "This cycles three edges and keeps everything else in place",
        ),
    }
});

/// Finds the shortest list of `moves` from `start` to a state meeting `goal`.
///
/// States with equal keys count as the same. A key must therefore capture everything that affects
/// the goal and how the moves change it.
fn search<K: Eq + Hash>(
    start: &Cube,
    moves: &[&Move],
    key: impl Fn(&Cube) -> K,
    goal: impl Fn(&Cube) -> bool,
) -> Option<Vec<Move>> {
    struct Node {
        cube: Cube,
        parent: usize,
        via: usize,
    }
    let mut nodes = vec![Node {
        cube: start.clone(),
        parent: 0,
        via: 0,
    }];
    let mut seen = HashSet::from([key(start)]);
    let mut frontier = VecDeque::from([0]);
    while let Some(index) = frontier.pop_front() {
        if goal(&nodes[index].cube) {
            let mut path = Vec::new();
            let mut at = index;
            while at != 0 {
                path.push(moves[nodes[at].via].clone());
                at = nodes[at].parent;
            }
            path.reverse();
            return Some(path);
        }
        for (via, step) in moves.iter().enumerate() {
            let mut cube = nodes[index].cube.clone();
            cube.apply_all(step.turns.iter().copied());
            if seen.insert(key(&cube)) {
                frontier.push_back(nodes.len());
                nodes.push(Node {
                    cube,
                    parent: index,
                    via,
                });
            }
        }
    }
    None
}

/// Searches each piece in `homes` and returns the plan for the one that is quickest to solve.
fn best_piece_plan(
    view: &MethodView,
    homes: &[[i8; 3]],
    moves: &[&Move],
) -> Option<(Vec<Move>, Option<[i8; 3]>)> {
    homes
        .iter()
        .filter(|&&home| !view.solved(home))
        .filter_map(|&home| {
            let plan = search(
                &view.cube,
                moves,
                |cube| cube.cubie_at_home(home).rotation(),
                |cube| piece_solved(cube, home),
            )?;
            Some((plan, Some(home)))
        })
        .min_by_key(|(plan, _)| plan.iter().map(|step| step.turns.len()).sum::<usize>())
}

fn first_corner_steps(view: &MethodView) -> Option<(Vec<Move>, Option<[i8; 3]>)> {
    // A trigger disturbs only the corner and edge of its own slot. Staying away from finished
    // corners means none of them can be undone.
    let mut moves: Vec<&Move> = MOVES.top_turns.iter().collect();
    for (slot, &corner) in FIRST_CORNERS.iter().enumerate() {
        if !view.solved(corner) {
            moves.push(&MOVES.corner_trigger[slot]);
        }
    }
    best_piece_plan(view, &FIRST_CORNERS, &moves)
}

fn middle_edge_steps(view: &MethodView) -> Option<(Vec<Move>, Option<[i8; 3]>)> {
    let mut moves: Vec<&Move> = MOVES.top_turns.iter().collect();
    for (slot, &edge) in MIDDLE_EDGES.iter().enumerate() {
        if !view.solved(edge) {
            moves.extend(&MOVES.middle_insert[slot]);
        }
    }
    best_piece_plan(view, &MIDDLE_EDGES, &moves)
}

/// Plans a last-layer stage, which uses one taught algorithm and turns of the top layer.
fn last_layer_steps<K: Eq + Hash>(
    view: &MethodView,
    algorithm: &Move,
    key: impl Fn(&Cube) -> K,
) -> Option<Vec<Move>> {
    let stage = view.stage();
    let mut moves: Vec<&Move> = MOVES.top_turns.iter().collect();
    moves.push(algorithm);
    search(&view.cube, &moves, key, |cube| stage_of_view(cube) > stage)
}

/// Describes where yellow stickers are facing, position by position, regardless of which piece
/// each one belongs to.
fn orientation_key(view: &Cube, homes: &[[i8; 3]]) -> Vec<([i8; 3], [i8; 3])> {
    let mut key: Vec<_> = homes
        .iter()
        .map(|&home| {
            let cubie = view.cubie_at_home(home);
            (cubie.position(), cubie.rotation().apply([0, 1, 0]))
        })
        .collect();
    key.sort_unstable();
    key
}

/// Describes which piece is at which position.
fn placement_key(view: &Cube, homes: &[[i8; 3]]) -> Vec<([i8; 3], [i8; 3])> {
    homes
        .iter()
        .map(|&home| (home, view.cubie_at_home(home).position()))
        .collect()
}

/// The 18 turns of the outer layers.
fn face_turns() -> Vec<Turn> {
    let mut turns = Vec::with_capacity(18);
    for axis in Axis::ALL {
        for layer in [Layer::Pos, Layer::Neg] {
            for quarters in [1, -1, 2] {
                turns.push(Turn::new(axis, layer, quarters));
            }
        }
    }
    turns
}

/// Every way four white edges can be placed, with the distance of each from a finished cross.
///
/// There are only 190,080 reachable states, so unlike a search over whole cubes this is quick to
/// build and gives a perfect answer at any time.
struct CrossTable {
    turns: Vec<Turn>,
    /// `next[turn][placement]` is where a turn sends an edge in that placement.
    next: Vec<[u8; PLACEMENTS]>,
    /// The number of turns to finish the cross from each state, or [`UNREACHED`].
    distance: Vec<u8>,
}

/// An edge is in one of 12 positions, either way round.
const PLACEMENTS: usize = 24;
const UNREACHED: u8 = u8::MAX;

static CROSS_TABLE: LazyLock<CrossTable> = LazyLock::new(CrossTable::build);

/// A placement of an edge: its position and where its white sticker faces.
type Placement = ([i8; 3], [i8; 3]);

fn all_placements() -> Vec<Placement> {
    let mut placements = Vec::with_capacity(PLACEMENTS);
    for x in -1..=1_i8 {
        for y in -1..=1_i8 {
            for z in -1..=1_i8 {
                let position = [x, y, z];
                if position.iter().filter(|&&coord| coord != 0).count() != 2 {
                    continue;
                }
                for axis in 0..3 {
                    if position[axis] != 0 {
                        let mut normal = [0; 3];
                        normal[axis] = position[axis];
                        placements.push((position, normal));
                    }
                }
            }
        }
    }
    placements
}

fn placement_of(view: &Cube, home: [i8; 3]) -> Placement {
    let cubie = view.cubie_at_home(home);
    (cubie.position(), cubie.rotation().apply([0, -1, 0]))
}

impl CrossTable {
    fn build() -> Self {
        let placements = all_placements();
        let index: HashMap<Placement, u8> = placements
            .iter()
            .enumerate()
            .map(|(i, &placement)| (placement, i as u8))
            .collect();
        let turns = face_turns();
        let next: Vec<[u8; PLACEMENTS]> = turns
            .iter()
            .map(|&turn| {
                let rotation = Rotation::quarter_turns(turn.axis, turn.quarters);
                std::array::from_fn(|i| {
                    let (position, normal) = placements[i];
                    if turn.layer.contains(position[turn.axis.index()]) {
                        index[&(rotation.apply(position), rotation.apply(normal))]
                    } else {
                        i as u8
                    }
                })
            })
            .collect();

        let mut distance = vec![UNREACHED; PLACEMENTS.pow(4)];
        let solved = Cube::solved().reframed(Rotation::IDENTITY).0;
        let start = state_index(CROSS_EDGES.map(|home| index[&placement_of(&solved, home)]));
        distance[start] = 0;
        let mut frontier = VecDeque::from([start]);
        while let Some(state) = frontier.pop_front() {
            let edges = state_edges(state);
            for table in &next {
                let moved = state_index(edges.map(|edge| table[usize::from(edge)]));
                if distance[moved] == UNREACHED {
                    distance[moved] = distance[state] + 1;
                    frontier.push_back(moved);
                }
            }
        }
        Self {
            turns,
            next,
            distance,
        }
    }

    /// Returns the shortest list of turns that finishes the cross.
    fn solve(&self, view: &Cube) -> Vec<Turn> {
        let placements = all_placements();
        let edges = CROSS_EDGES.map(|home| {
            let placement = placement_of(view, home);
            placements
                .iter()
                .position(|&candidate| candidate == placement)
                .unwrap_or_default() as u8
        });
        let mut state = state_index(edges);
        let mut path = Vec::new();
        while self.distance[state] != 0 && self.distance[state] != UNREACHED {
            let edges = state_edges(state);
            let step = self.next.iter().enumerate().find_map(|(i, table)| {
                let moved = state_index(edges.map(|edge| table[usize::from(edge)]));
                (self.distance[moved] < self.distance[state]).then_some((i, moved))
            });
            let Some((turn, moved)) = step else { break };
            path.push(self.turns[turn]);
            state = moved;
        }
        path
    }
}

fn state_index(edges: [u8; 4]) -> usize {
    edges
        .iter()
        .fold(0, |state, &edge| state * PLACEMENTS + usize::from(edge))
}

fn state_edges(state: usize) -> [u8; 4] {
    let digit = |place: u32| ((state / PLACEMENTS.pow(place)) % PLACEMENTS) as u8;
    [digit(3), digit(2), digit(1), digit(0)]
}

fn cross_steps(view: &MethodView) -> (Vec<Move>, Option<[i8; 3]>) {
    const WHY: &str = "This is the shortest way to finish the white cross";
    let steps = CROSS_TABLE
        .solve(&view.cube)
        .into_iter()
        .map(|turn| Move {
            turns: vec![turn],
            why: WHY,
        })
        .collect();
    let target = CROSS_EDGES.into_iter().find(|&home| !view.solved(home));
    (steps, target)
}

/// Returns the fewest turns that finish the white cross, for comparing a player's cross to it.
pub fn cross_distance(cube: &Cube) -> usize {
    CROSS_TABLE.solve(&MethodView::of(cube).cube).len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cube::{parse_sequence, scramble};
    use crate::method::training_start;

    /// Follows hints to the end, returning the stages seen and the number of turns made.
    fn follow_hints(mut cube: Cube) -> Result<(Cube, usize), String> {
        let mut turns = 0;
        let mut best = MethodView::of(&cube).stage();
        for _ in 0..200 {
            let Some(next) = hint(&cube) else {
                return Ok((cube, turns));
            };
            let stage = MethodView::of(&cube).stage();
            if stage < best {
                return Err(format!("went back from {best:?} to {stage:?}"));
            }
            best = stage;
            let Some(step) = next.steps.first() else {
                return Err(format!("no steps at {stage:?}"));
            };
            cube.apply_all(step.turns.iter().copied());
            turns += step.turns.len();
        }
        Err("hints never finished".to_owned())
    }

    fn scrambled(seed: u64) -> Cube {
        let mut cube = training_start();
        cube.apply_all(scramble(&mut fastrand::Rng::with_seed(seed), 25));
        cube
    }

    #[test]
    fn solved_cube_should_have_no_hint() {
        assert_eq!(hint(&training_start()), None);
    }

    #[test]
    fn following_hints_should_solve_any_scramble() {
        for seed in 0..300 {
            let (cube, _) = follow_hints(scrambled(seed)).unwrap_or_else(|e| panic!("{seed}: {e}"));
            assert!(cube.is_solved(), "seed {seed}");
        }
    }

    #[test]
    fn following_hints_should_solve_any_scramble_held_any_way() {
        let holds = ["x", "y", "z2", "x y", "y' z", "x2 y'"];
        for (seed, hold) in holds.iter().cycle().take(60).enumerate() {
            let mut cube = scrambled(seed as u64);
            cube.apply_all(parse_sequence(hold).unwrap());
            let (cube, _) = follow_hints(cube).unwrap_or_else(|e| panic!("{seed}: {e}"));
            assert!(cube.is_solved(), "seed {seed}");
        }
    }

    #[test]
    fn following_hints_should_solve_a_cube_with_moved_centres() {
        for seed in 0..60 {
            let mut cube = scrambled(seed);
            cube.apply_all(parse_sequence("M E' S2 R M'").unwrap());
            let (cube, _) = follow_hints(cube).unwrap_or_else(|e| panic!("{seed}: {e}"));
            assert!(cube.is_solved(), "seed {seed}");
        }
    }

    #[test]
    fn following_a_tracked_plan_one_turn_at_a_time_should_solve_any_scramble() {
        for seed in 0..150 {
            let mut cube = scrambled(seed);
            let mut tracker = HintTracker::default();
            tracker.sync(&cube);
            let mut turns = 0;
            while let Some(turn) = tracker.next_turn() {
                cube.apply(turn);
                tracker.sync(&cube);
                turns += 1;
                assert!(turns < 600, "seed {seed} never finished");
            }
            assert!(cube.is_solved(), "seed {seed}");
        }
    }

    #[test]
    fn tracker_should_keep_its_plan_midway_through_a_step() {
        let mut cube = training_start();
        cube.apply_all(parse_sequence("U R U' R'").unwrap());
        let mut tracker = HintTracker::default();
        tracker.sync(&cube);
        let plan = tracker.hint().cloned().unwrap();
        let first = tracker.next_turn().unwrap();
        cube.apply(first);
        tracker.sync(&cube);
        assert_eq!(tracker.hint(), Some(&plan));
    }

    #[test]
    fn tracker_should_replan_when_the_player_goes_their_own_way() {
        let mut cube = scrambled(5);
        let mut tracker = HintTracker::default();
        tracker.sync(&cube);
        let wrong = "x".parse().unwrap();
        cube.apply(wrong);
        tracker.sync(&cube);
        assert_eq!(tracker.hint().cloned(), hint(&cube));
    }

    #[test]
    fn tracker_should_report_the_turns_left_in_the_current_step() {
        let mut cube = training_start();
        // Pull a corner out of its slot, so that the plan is a run of triggers.
        cube.apply_all(parse_sequence("R U R' U'").unwrap());
        let mut tracker = HintTracker::default();
        tracker.sync(&cube);
        let total = |tracker: &HintTracker| {
            tracker.remaining().map_or(0, |(_, left, rest)| {
                left.len() + rest.iter().map(|step| step.turns.len()).sum::<usize>()
            })
        };
        let at_start = total(&tracker);
        cube.apply(tracker.next_turn().unwrap());
        tracker.sync(&cube);
        assert_eq!(total(&tracker), at_start - 1);
    }

    #[test]
    fn tracker_should_have_nothing_to_say_about_a_solved_cube() {
        let mut tracker = HintTracker::default();
        tracker.sync(&training_start());
        assert_eq!(tracker.next_turn(), None);
    }

    #[test]
    fn finished_first_corners_should_stay_finished() {
        for seed in 0..100 {
            let mut cube = scrambled(seed);
            while MethodView::of(&cube).stage() == Stage::Cross {
                let step = hint(&cube).unwrap().steps.remove(0);
                cube.apply_all(step.turns);
            }
            let mut solved = 0;
            while MethodView::of(&cube).stage() == Stage::FirstCorners {
                let view = MethodView::of(&cube);
                let count = FIRST_CORNERS.iter().filter(|&&c| view.solved(c)).count();
                assert!(count >= solved, "seed {seed}");
                solved = count;
                let step = hint(&cube).unwrap().steps.remove(0);
                cube.apply_all(step.turns);
            }
        }
    }

    #[test]
    fn hints_should_be_in_the_players_frame() {
        // With white on top, the method's bottom layer is the player's top layer.
        let mut cube = Cube::solved();
        cube.apply("U".parse().unwrap());
        let next = hint(&cube).unwrap();
        let first = next.steps[0].turns[0];
        assert_eq!((first.axis, first.layer), (Axis::Y, Layer::Pos));
    }

    #[test]
    fn hint_should_name_a_piece_that_is_not_solved() {
        let cube = scrambled(1);
        let target = hint(&cube).unwrap().target.unwrap();
        // The target is a white edge in the player's cube, and white is `home` Y = +1 in there.
        assert_eq!(cube.cubies()[target].home()[1], 1);
    }

    #[test]
    fn cross_table_should_reach_exactly_the_possible_states() {
        let reached = CROSS_TABLE
            .distance
            .iter()
            .filter(|&&distance| distance != UNREACHED)
            .count();
        assert_eq!(reached, 190_080);
    }

    #[test]
    fn hardest_cross_should_take_eight_turns() {
        // A well-known result for the cross, and a good check on the table.
        let hardest = CROSS_TABLE
            .distance
            .iter()
            .filter(|&&distance| distance != UNREACHED)
            .max();
        assert_eq!(hardest, Some(&8));
    }

    #[test]
    fn cross_distance_should_count_a_single_broken_turn() {
        let mut cube = training_start();
        cube.apply("D".parse().unwrap());
        assert_eq!(cross_distance(&cube), 1);
    }
}
