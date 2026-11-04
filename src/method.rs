//! The vocabulary of solving: the stages of the beginner method and how to recognise them.
//!
//! Every check works in a *method view* of the cube, which fixes white on the bottom and green at
//! the front, whichever way the player is holding it. This is the standard way to teach and write
//! algorithms, so `R U R'` here means what any tutorial means by it.

use crate::cube::{Axis, Cube, Rotation, Turn, index_of_home};

/// The white edges, which form the cross.
pub const CROSS_EDGES: [[i8; 3]; 4] = [[0, -1, 1], [1, -1, 0], [0, -1, -1], [-1, -1, 0]];
/// The white corners, ordered so that each index is the same slot as in [`MIDDLE_EDGES`].
pub const FIRST_CORNERS: [[i8; 3]; 4] = [[1, -1, 1], [1, -1, -1], [-1, -1, -1], [-1, -1, 1]];
/// The edges of the middle layer, ordered so that each index is a slot: FR, BR, BL, and FL.
pub const MIDDLE_EDGES: [[i8; 3]; 4] = [[1, 0, 1], [1, 0, -1], [-1, 0, -1], [-1, 0, 1]];
/// The yellow edges.
pub const TOP_EDGES: [[i8; 3]; 4] = [[0, 1, 1], [1, 1, 0], [0, 1, -1], [-1, 1, 0]];
/// The yellow corners.
pub const TOP_CORNERS: [[i8; 3]; 4] = [[1, 1, 1], [1, 1, -1], [-1, 1, -1], [-1, 1, 1]];

const UP: [i8; 3] = [0, 1, 0];

/// The rotation from the game's solved colours to the method's: white down and green in front.
fn method_colours() -> Rotation {
    Rotation::quarter_turns(Axis::Z, 2)
}

/// A solved cube held the way the method expects, ready for training.
pub fn training_start() -> Cube {
    let mut cube = Cube::solved();
    cube.apply(Turn::new(Axis::Z, crate::cube::Layer::All, 2));
    cube
}

/// A cube seen in the method's fixed frame, whichever way it is actually held.
#[derive(Clone, Debug)]
pub struct MethodView {
    /// The cube in the method's frame. A piece is solved when its rotation is the identity.
    pub cube: Cube,
    to_world: Rotation,
}

impl MethodView {
    /// Views a cube in the method's frame.
    pub fn of(cube: &Cube) -> Self {
        let (cube, to_world) = cube.reframed(method_colours());
        Self { cube, to_world }
    }

    /// Converts a turn in the method's frame to the turn the player must make.
    pub fn world_turn(&self, turn: Turn) -> Turn {
        self.to_world.rotate_turn(turn)
    }

    /// Converts a piece's home in the method's frame to its index in the player's cube.
    pub fn world_index(&self, home: [i8; 3]) -> usize {
        index_of_home(method_colours().apply(home))
    }

    /// Returns whether a piece is in its solved place, the right way round.
    pub fn solved(&self, home: [i8; 3]) -> bool {
        piece_solved(&self.cube, home)
    }

    /// Returns whether a yellow piece is showing its yellow sticker on top.
    pub fn yellow_up(&self, home: [i8; 3]) -> bool {
        self.cube.cubie_at_home(home).rotation().apply(UP) == UP
    }

    /// Returns the beginner stage this cube is on.
    pub fn stage(&self) -> Stage {
        stage_of_view(&self.cube)
    }
}

/// Returns the beginner stage of a cube that is already in the method's frame.
pub fn stage_of_view(view: &Cube) -> Stage {
    let solved = |homes: &[[i8; 3]]| homes.iter().all(|&home| piece_solved(view, home));
    let yellow_up = |homes: &[[i8; 3]]| {
        homes
            .iter()
            .all(|&home| view.cubie_at_home(home).rotation().apply(UP) == UP)
    };
    if !solved(&CROSS_EDGES) {
        Stage::Cross
    } else if !solved(&FIRST_CORNERS) {
        Stage::FirstCorners
    } else if !solved(&MIDDLE_EDGES) {
        Stage::MiddleEdges
    } else if !yellow_up(&TOP_EDGES) {
        Stage::YellowCross
    } else if !yellow_up(&TOP_CORNERS) {
        Stage::YellowCorners
    } else if !corners_placed(view) {
        Stage::PlaceCorners
    } else if !solved(&TOP_CORNERS) || !solved(&TOP_EDGES) {
        Stage::PlaceEdges
    } else {
        Stage::Done
    }
}

/// Returns whether the yellow corners are in the right order around the top layer.
///
/// This allows for the top layer being turned. Lining the pieces up is part of working on the
/// next stage, and a plan that turns the top mustn't appear to undo this one.
fn corners_placed(view: &Cube) -> bool {
    (0..4).any(|quarters| {
        let top = Rotation::quarter_turns(Axis::Y, quarters);
        TOP_CORNERS
            .iter()
            .all(|&home| top.apply(view.cubie_at_home(home).position()) == home)
    })
}

/// Returns whether a piece of a reframed cube is in its solved place, the right way round.
pub fn piece_solved(view: &Cube, home: [i8; 3]) -> bool {
    view.cubie_at_home(home).rotation() == Rotation::IDENTITY
}

/// The stages of the beginner layer-by-layer method, in solving order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Stage {
    Cross,
    FirstCorners,
    MiddleEdges,
    YellowCross,
    YellowCorners,
    PlaceCorners,
    PlaceEdges,
    Done,
}

impl Stage {
    /// The stages that have work to do, in order.
    pub const STEPS: [Stage; 7] = [
        Stage::Cross,
        Stage::FirstCorners,
        Stage::MiddleEdges,
        Stage::YellowCross,
        Stage::YellowCorners,
        Stage::PlaceCorners,
        Stage::PlaceEdges,
    ];

    /// A short name for checklists.
    pub const fn name(self) -> &'static str {
        match self {
            Stage::Cross => "White cross",
            Stage::FirstCorners => "White corners",
            Stage::MiddleEdges => "Middle layer",
            Stage::YellowCross => "Yellow cross",
            Stage::YellowCorners => "Yellow face",
            Stage::PlaceCorners => "Place corners",
            Stage::PlaceEdges => "Place edges",
            Stage::Done => "Solved",
        }
    }

    /// Returns a goal that is met once the cube has moved past this stage.
    pub fn goal(self) -> Goal {
        match self {
            Stage::Cross => |cube| MethodView::of(cube).stage() > Stage::Cross,
            Stage::FirstCorners => |cube| MethodView::of(cube).stage() > Stage::FirstCorners,
            Stage::MiddleEdges => |cube| MethodView::of(cube).stage() > Stage::MiddleEdges,
            Stage::YellowCross => |cube| MethodView::of(cube).stage() > Stage::YellowCross,
            Stage::YellowCorners => |cube| MethodView::of(cube).stage() > Stage::YellowCorners,
            Stage::PlaceCorners => |cube| MethodView::of(cube).stage() > Stage::PlaceCorners,
            Stage::PlaceEdges | Stage::Done => |cube| MethodView::of(cube).stage() == Stage::Done,
        }
    }
}

/// A test for whether the player has finished what they were asked to do.
pub type Goal = fn(&Cube) -> bool;

/// Returns whether the first two layers are complete, which is the goal of F2L practice.
pub fn first_two_layers_done(cube: &Cube) -> bool {
    MethodView::of(cube).stage() > Stage::MiddleEdges
}

/// Returns whether the last layer is all yellow on top, which is the goal of OLL practice.
pub fn last_layer_oriented(cube: &Cube) -> bool {
    MethodView::of(cube).stage() > Stage::YellowCorners
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cube::parse_sequence;

    fn after(text: &str) -> MethodView {
        let mut cube = training_start();
        cube.apply_all(parse_sequence(text).unwrap());
        MethodView::of(&cube)
    }

    #[test]
    fn solved_cube_should_be_done() {
        assert_eq!(after("").stage(), Stage::Done);
    }

    #[test]
    fn solved_cube_should_be_done_when_held_any_way() {
        for hold in ["x", "y2", "z'", "x y z"] {
            assert_eq!(after(hold).stage(), Stage::Done, "{hold}");
        }
    }

    #[test]
    fn training_start_should_put_white_on_the_bottom() {
        let view = MethodView::of(&training_start());
        assert_eq!(view.world_turn("R".parse().unwrap()).to_string(), "R");
    }

    #[test]
    fn method_turns_should_translate_when_held_upside_down() {
        // Holding white on top swaps D with U and R with L, as seen from the front.
        let view = MethodView::of(&crate::cube::Cube::solved());
        assert_eq!(view.world_turn("R".parse().unwrap()).to_string(), "L");
    }

    #[test]
    fn white_piece_should_map_back_to_the_players_cube() {
        // White is on top in the game's solved colours, so `home` must map to +Y there.
        let view = MethodView::of(&crate::cube::Cube::solved());
        assert_eq!(view.world_index(CROSS_EDGES[0]), index_of_home([0, 1, 1]),);
    }

    #[test]
    fn breaking_the_bottom_layer_should_return_to_the_cross_stage() {
        assert_eq!(after("D").stage(), Stage::Cross);
    }

    #[test]
    fn stage_should_track_each_broken_part() {
        let cases = [
            ("R U R' U'", Stage::FirstCorners),
            ("U R U' R' U' F' U F", Stage::MiddleEdges),
            ("F R U R' U' F'", Stage::YellowCross),
            ("R U R' U R U2 R'", Stage::YellowCorners),
            ("R' F R' B2 R F' R' B2 R2", Stage::PlaceCorners),
            ("R2 U R U R' U' R' U' R' U R'", Stage::PlaceEdges),
            ("U", Stage::PlaceEdges),
        ];
        for (text, stage) in cases {
            assert_eq!(after(text).stage(), stage, "{text}");
        }
    }

    #[test]
    fn stage_goal_should_be_met_only_after_that_stage() {
        let mut cube = training_start();
        cube.apply_all(parse_sequence("R U R' U R U2 R'").unwrap());
        assert_eq!(
            (
                Stage::YellowCross.goal()(&cube),
                Stage::YellowCorners.goal()(&cube),
            ),
            (true, false),
        );
    }

    #[test]
    fn stages_should_be_ordered() {
        assert!(Stage::STEPS.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
