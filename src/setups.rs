//! Generates practice positions: cubes with only the part being practised left to do.

use crate::algs::{Case, OLL, PLL};
use crate::cube::{Cube, Turn, parse_sequence, scramble};
use crate::method::{
    Goal, MethodView, Stage, first_two_layers_done, last_layer_oriented, training_start,
};

/// Moves that keep the white cross intact.
const TRIGGERS: [&str; 4] = ["R U R' U'", "B U B' U'", "L U L' U'", "F U F' U'"];
/// Moves that keep the first layer intact.
const INSERTS: [&str; 8] = [
    "U R U' R' U' F' U F",
    "U' F' U F U R U' R'",
    "U B U' B' U' R' U R",
    "U' R' U R U B U' B'",
    "U L U' L' U' B' U B",
    "U' B' U B U L U' L'",
    "U F U' F' U' L' U L",
    "U' L' U L U F U' F'",
];
const TOP_TURNS: [&str; 3] = ["U", "U'", "U2"];
const YELLOW_CROSS: &str = "F R U R' U' F'";
const SUNE: &str = "R U R' U R U2 R'";
const A_PERM: &str = "R' F R' B2 R F' R' B2 R2";
const U_PERM: &str = "R2 U R U R' U' R' U' R' U R'";

const SCRAMBLE_LENGTH: usize = 25;
const SHUFFLE_MOVES: usize = 14;
/// A random shuffle can land on a position with nothing to do, so allow several tries. Each one
/// almost always succeeds, so running out would mean a bug that the tests would catch.
const SHUFFLE_ATTEMPTS: usize = 200;

/// Returns a cube made by shuffling a solved one with moves that keep some part of it intact.
fn shuffled(rng: &mut fastrand::Rng, pool: &[&str], accept: impl Fn(&Cube) -> bool) -> Cube {
    let mut cube = training_start();
    for _ in 0..SHUFFLE_ATTEMPTS {
        cube = training_start();
        for _ in 0..SHUFFLE_MOVES {
            let text = pool[rng.usize(..pool.len())];
            cube.apply_all(parse_sequence(text).unwrap_or_default());
        }
        if accept(&cube) {
            break;
        }
    }
    cube
}

/// Returns a scrambled cube, held the way the method expects.
pub fn scrambled(rng: &mut fastrand::Rng) -> Cube {
    let mut cube = training_start();
    let turns: Vec<Turn> = scramble(rng, SCRAMBLE_LENGTH);
    cube.apply_all(turns);
    cube
}

/// Returns a cube on the given stage of the beginner method, with earlier stages complete.
pub fn stage_position(stage: Stage, rng: &mut fastrand::Rng) -> Cube {
    let pool: Vec<&str> = match stage {
        Stage::Cross | Stage::Done => return scrambled(rng),
        Stage::FirstCorners => [&TRIGGERS[..], &TOP_TURNS].concat(),
        Stage::MiddleEdges => [&INSERTS[..], &TOP_TURNS].concat(),
        Stage::YellowCross => [&[YELLOW_CROSS, SUNE][..], &TOP_TURNS].concat(),
        Stage::YellowCorners => [&[SUNE, A_PERM, U_PERM][..], &TOP_TURNS].concat(),
        Stage::PlaceCorners => [&[A_PERM, U_PERM][..], &TOP_TURNS].concat(),
        Stage::PlaceEdges => [&[U_PERM][..], &TOP_TURNS].concat(),
    };
    shuffled(rng, &pool, |cube| MethodView::of(cube).stage() == stage)
}

/// A stage of a CFOP solve that can be practised on its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PracticeStage {
    /// Build the white cross from a scramble.
    Cross,
    /// Finish the first two layers from a solved cross.
    FirstTwoLayers,
    /// Make the top all yellow.
    Oll,
    /// Finish the cube from an all-yellow top.
    Pll,
}

impl PracticeStage {
    /// All stages, in solving order.
    pub const ALL: [PracticeStage; 4] = [
        PracticeStage::Cross,
        PracticeStage::FirstTwoLayers,
        PracticeStage::Oll,
        PracticeStage::Pll,
    ];

    /// The stage's name.
    pub const fn name(self) -> &'static str {
        match self {
            PracticeStage::Cross => "Cross",
            PracticeStage::FirstTwoLayers => "First two layers",
            PracticeStage::Oll => "OLL",
            PracticeStage::Pll => "PLL",
        }
    }

    /// What the player must achieve.
    pub const fn instructions(self) -> &'static str {
        match self {
            PracticeStage::Cross => {
                "Build the white cross on the bottom. Each white edge must also match the centre \
                 above it. Experts plan the whole cross before making a move and finish in 8 \
                 turns or fewer."
            }
            PracticeStage::FirstTwoLayers => {
                "The cross is done. Finish the bottom two layers. Try pairing each white corner \
                 with its middle edge in the top layer, then inserting the pair in one go."
            }
            PracticeStage::Oll => {
                "The first two layers are done. Make the whole top yellow, keeping everything \
                 below it intact."
            }
            PracticeStage::Pll => {
                "The top is all yellow. Move the last-layer pieces into place to finish the cube."
            }
        }
    }

    /// The test for having finished this stage.
    pub fn goal(self) -> Goal {
        match self {
            PracticeStage::Cross => Stage::Cross.goal(),
            PracticeStage::FirstTwoLayers => first_two_layers_done,
            PracticeStage::Oll => last_layer_oriented,
            PracticeStage::Pll => Cube::is_solved,
        }
    }

    /// Returns a position to practise this stage on.
    pub fn position(self, rng: &mut fastrand::Rng) -> Cube {
        match self {
            PracticeStage::Cross => scrambled(rng),
            PracticeStage::FirstTwoLayers => {
                let pool = [&TRIGGERS[..], &INSERTS, &TOP_TURNS].concat();
                shuffled(rng, &pool, |cube| !first_two_layers_done(cube))
            }
            PracticeStage::Oll => random_case(&OLL, rng),
            PracticeStage::Pll => random_case(&PLL, rng),
        }
    }
}

fn random_case(cases: &[Case], rng: &mut fastrand::Rng) -> Cube {
    cases[rng.usize(..cases.len())].setup(rng.i32(0..4), rng.i32(0..4))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_positions_should_be_on_the_requested_stage() {
        let mut rng = fastrand::Rng::with_seed(1);
        for stage in Stage::STEPS {
            for _ in 0..40 {
                let cube = stage_position(stage, &mut rng);
                assert_eq!(MethodView::of(&cube).stage(), stage);
            }
        }
    }

    #[test]
    fn scrambles_should_start_with_white_on_the_bottom() {
        let cube = scrambled(&mut fastrand::Rng::with_seed(2));
        let turn = MethodView::of(&cube).world_turn("R".parse().unwrap());
        assert_eq!(turn.to_string(), "R");
    }

    #[test]
    fn practice_positions_should_not_already_meet_their_goal() {
        let mut rng = fastrand::Rng::with_seed(3);
        for stage in PracticeStage::ALL {
            for _ in 0..40 {
                let cube = stage.position(&mut rng);
                assert!(!stage.goal()(&cube), "{stage:?}");
            }
        }
    }

    #[test]
    fn first_two_layers_practice_should_keep_the_cross() {
        let mut rng = fastrand::Rng::with_seed(4);
        for _ in 0..40 {
            let cube = PracticeStage::FirstTwoLayers.position(&mut rng);
            assert!(Stage::Cross.goal()(&cube));
        }
    }

    #[test]
    fn oll_practice_should_keep_the_first_two_layers() {
        let mut rng = fastrand::Rng::with_seed(5);
        for _ in 0..40 {
            let cube = PracticeStage::Oll.position(&mut rng);
            assert!(first_two_layers_done(&cube));
        }
    }

    #[test]
    fn pll_practice_should_keep_the_top_yellow() {
        let mut rng = fastrand::Rng::with_seed(6);
        for _ in 0..40 {
            let cube = PracticeStage::Pll.position(&mut rng);
            assert!(last_layer_oriented(&cube));
        }
    }
}
