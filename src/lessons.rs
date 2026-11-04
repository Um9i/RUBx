//! The learn-to-solve course: what each lesson teaches and how it is practised.

use crate::cube::{Cube, Turn, parse_sequence};
use crate::method::Stage;

/// How a lesson is practised.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exercise {
    /// Make exactly these move sequences, one sequence per position.
    Follow(&'static [&'static str]),
    /// Finish this stage of the method from several generated positions.
    Stage(Stage),
}

/// One lesson of the course.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lesson {
    /// A stable name for saved progress.
    pub id: &'static str,
    /// The lesson's title.
    pub title: &'static str,
    /// The teaching text.
    pub body: &'static str,
    /// The key algorithm for this lesson, if it has one.
    pub algorithm: Option<&'static str>,
    /// How the lesson is practised.
    pub exercise: Exercise,
}

/// How many positions a stage lesson asks the player to finish.
pub const STAGE_POSITIONS: u32 = 3;

impl Lesson {
    /// Returns how many positions must be finished to complete this lesson.
    pub const fn positions(&self) -> u32 {
        match self.exercise {
            Exercise::Follow(sequences) => sequences.len() as u32,
            Exercise::Stage(_) => STAGE_POSITIONS,
        }
    }
}

/// The course, in teaching order.
pub const LESSONS: [Lesson; 8] = [
    Lesson {
        id: "notation",
        title: "How to read moves",
        body: "Yellow is on top, white on the bottom, and green faces you. Letters name a face: \
               R(ight), L(eft), U(p), D(own), F(ront), B(ack).\n\n\
               A letter alone means turn that face clockwise, as if you were looking straight at \
               it. An apostrophe, as in R', means anticlockwise. A 2, as in U2, means a half \
               turn.\n\n\
               Follow the arrow, using the keyboard (hold Shift for ') or by dragging the cube. \
               The second exercise repeats one trigger six times. Watch the cube come back to \
               solved by itself.",
        algorithm: None,
        exercise: Exercise::Follow(&[
            "R R' U U' F F' L L' D D' B B'",
            "R U R' U' R U R' U' R U R' U' R U R' U' R U R' U' R U R' U'",
        ]),
    },
    Lesson {
        id: "cross",
        title: "The white cross",
        body: "Make a white plus on the bottom. It's not enough for the white stickers to line \
               up: each edge's other colour must match the centre beside it.\n\n\
               Centres never move relative to each other, so they tell you where everything goes. \
               An easy way to place an edge: get it into the top layer, turn the top until its \
               side colour matches a centre, then turn that face twice to drop it into the \
               bottom.\n\n\
               There's no algorithm to learn here. It's all about seeing where pieces go.",
        algorithm: None,
        exercise: Exercise::Stage(Stage::Cross),
    },
    Lesson {
        id: "corners",
        title: "The white corners",
        body: "Fill in the corners of the white layer, one at a time.\n\n\
               Find a white corner in the top layer. Turn the top until it sits directly above \
               the slot it belongs in, with that slot at the front right. Then repeat the trigger \
               until the corner drops in the right way round. It takes 1, 3, or 5 repeats.\n\n\
               If a white corner is stuck in the bottom layer in the wrong place, one trigger \
               lifts it out into the top layer.",
        algorithm: Some("R U R' U'"),
        exercise: Exercise::Stage(Stage::FirstCorners),
    },
    Lesson {
        id: "middle",
        title: "The middle layer",
        body: "Now place the four edges of the middle layer.\n\n\
               Find a top-layer edge with no yellow on it. Turn the top until its front colour \
               matches the centre below it. Its top colour tells you which way it goes: to the \
               right, or to the left.\n\n\
               To the right: U R U' R' U' F' U F\n\
               To the left: U' L' U L U F U' F'\n\n\
               If an edge is stuck in the wrong slot, insert any top edge there to push it out.",
        algorithm: Some("U R U' R' U' F' U F"),
        exercise: Exercise::Stage(Stage::MiddleEdges),
    },
    Lesson {
        id: "yellow-cross",
        title: "The yellow cross",
        body: "Two layers done. Time to make a yellow plus on top. Ignore the corners for now.\n\n\
               The top edges show one of four patterns: a dot, an L shape, a line, or the cross. \
               The algorithm moves you along that list. Hold the L shape at the back left, or the \
               line horizontal, then apply it.\n\n\
               The cube looks scrambled mid-algorithm, but the finished layers always come back.",
        algorithm: Some("F R U R' U' F'"),
        exercise: Exercise::Stage(Stage::YellowCross),
    },
    Lesson {
        id: "yellow-face",
        title: "The yellow face",
        body: "Make the whole top yellow. This algorithm is called the Sune, and it twists three \
               corners without breaking your cross.\n\n\
               Count the corners already yellow on top, and hold the cube by that number. With \
               one, put it at the front left. With none, turn the top so that the front left \
               corner's yellow faces left. With two, so that it faces you.\n\n\
               Apply the Sune, count again, and repeat until the face is done.",
        algorithm: Some("R U R' U R U2 R'"),
        exercise: Exercise::Stage(Stage::YellowCorners),
    },
    Lesson {
        id: "place-corners",
        title: "Place the corners",
        body: "The top is yellow, but the pieces are in the wrong spots. Fix the corners first.\n\n\
               Look around the sides of the top layer for two corners on the same side showing \
               the same colour. These are called headlights. Hold them at the back and apply the \
               algorithm. If there are no headlights anywhere, apply it from any side and they \
               will appear. Keep going until the corners match their sides when you turn the top.",
        algorithm: Some("R' F R' B2 R F' R' B2 R2"),
        exercise: Exercise::Stage(Stage::PlaceCorners),
    },
    Lesson {
        id: "place-edges",
        title: "Place the edges",
        body: "The last step. Turn the top so that the corners match their sides. If one side is \
               now completely finished, hold it at the back and apply the algorithm, once or \
               twice, until the edges cycle home.\n\n\
               If no side is finished, apply it from any side, then look again.\n\n\
               Do this and you've solved a Rubik's cube.",
        algorithm: Some("R2 U R U R' U' R' U' R' U R'"),
        exercise: Exercise::Stage(Stage::PlaceEdges),
    },
];

/// Tracks the player through a list of turns they have been told to make.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Follow {
    turns: Vec<Turn>,
    /// The state of the cube before each turn, and after the last.
    states: Vec<Cube>,
    made: usize,
    off_track: bool,
}

impl Follow {
    /// Starts following a sequence from the given position.
    pub fn new(start: &Cube, text: &str) -> Self {
        let turns = parse_sequence(text).unwrap_or_default();
        let mut states = vec![start.clone()];
        let mut current = start.clone();
        for &turn in &turns {
            current.apply(turn);
            states.push(current.clone());
        }
        Self {
            turns,
            states,
            made: 0,
            off_track: false,
        }
    }

    /// Compares the cube to the plan. Called each time the cube settles.
    pub fn sync(&mut self, cube: &Cube) {
        if self.states.get(self.made + 1) == Some(cube) {
            self.made += 1;
            self.off_track = false;
        } else {
            self.off_track = self.states[self.made] != *cube;
        }
    }

    /// Returns the next turn to make, unless the player needs to undo a wrong turn first.
    pub fn next_turn(&self) -> Option<Turn> {
        if self.off_track {
            None
        } else {
            self.turns.get(self.made).copied()
        }
    }

    /// Returns whether every turn has been made.
    pub fn is_finished(&self) -> bool {
        self.made == self.turns.len()
    }

    /// Returns whether the player has made a turn that isn't next in the list.
    pub fn is_off_track(&self) -> bool {
        self.off_track
    }

    /// Returns the turns made so far and the turns still to make.
    pub fn progress(&self) -> (&[Turn], &[Turn]) {
        self.turns.split_at(self.made)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cube::parse_algorithm;
    use crate::method::{MethodView, training_start};
    use crate::setups::stage_position;

    #[test]
    fn every_lesson_id_should_be_unique() {
        let ids: std::collections::HashSet<_> = LESSONS.iter().map(|lesson| lesson.id).collect();
        assert_eq!(ids.len(), LESSONS.len());
    }

    #[test]
    fn stage_lessons_should_cover_every_stage_in_order() {
        let stages: Vec<_> = LESSONS
            .iter()
            .filter_map(|lesson| match lesson.exercise {
                Exercise::Stage(stage) => Some(stage),
                Exercise::Follow(_) => None,
            })
            .collect();
        assert_eq!(stages, Stage::STEPS);
    }

    #[test]
    fn every_algorithm_and_exercise_should_parse() {
        for lesson in &LESSONS {
            assert!(parse_algorithm(lesson.algorithm.unwrap_or("")).is_ok());
            if let Exercise::Follow(sequences) = lesson.exercise {
                assert!(sequences.iter().all(|text| parse_sequence(text).is_ok()));
            }
        }
    }

    // The default font has only a basic character set, and anything else shows as an empty box.
    #[test]
    fn lesson_text_should_only_use_plain_ascii() {
        for lesson in &LESSONS {
            assert!(lesson.body.is_ascii(), "{}", lesson.title);
            assert!(lesson.title.is_ascii(), "{}", lesson.title);
        }
    }

    #[test]
    fn second_notation_exercise_should_return_to_solved() {
        let Exercise::Follow(sequences) = LESSONS[0].exercise else {
            panic!("the first lesson should follow moves");
        };
        let mut cube = training_start();
        cube.apply_all(parse_sequence(sequences[1]).unwrap());
        assert!(cube.is_solved());
    }

    // The tests below carry out each lesson's written instructions on real positions, exactly as
    // a learner would. A recipe that doesn't work would otherwise look fine in every other test.
    const YELLOW: [i8; 3] = [0, -1, 0];
    const UP: [i8; 3] = [0, 1, 0];
    const BACK: [i8; 3] = [0, 0, -1];
    const FRONT: [i8; 3] = [0, 0, 1];
    const LEFT: [i8; 3] = [-1, 0, 0];
    const UBL: [i8; 3] = [-1, 1, -1];
    const UBR: [i8; 3] = [1, 1, -1];
    const UFL: [i8; 3] = [-1, 1, 1];
    const UFR: [i8; 3] = [1, 1, 1];
    const UB: [i8; 3] = [0, 1, -1];
    const UL: [i8; 3] = [-1, 1, 0];
    const UR: [i8; 3] = [1, 1, 0];
    const UF: [i8; 3] = [0, 1, 1];

    /// Returns the solved-position normal, which stands for the colour, of a visible sticker.
    fn sticker(cube: &Cube, position: [i8; 3], face: [i8; 3]) -> [i8; 3] {
        let cubie = cube
            .cubies()
            .iter()
            .find(|cubie| cubie.position() == position)
            .unwrap();
        cubie
            .home_normals()
            .find(|&normal| cubie.rotation().apply(normal) == face)
            .unwrap()
    }

    fn yellow_on_top(cube: &Cube, position: [i8; 3]) -> bool {
        sticker(cube, position, UP) == YELLOW
    }

    fn apply(cube: &mut Cube, text: &str) {
        cube.apply_all(parse_sequence(text).unwrap());
    }

    /// Turns the whole cube until `pattern` holds, leaving it as it was if it never does.
    fn hold_until(cube: &mut Cube, pattern: impl Fn(&Cube) -> bool) -> bool {
        for _ in 0..4 {
            if pattern(cube) {
                return true;
            }
            apply(cube, "y");
        }
        false
    }

    /// Runs a recipe on many positions of a stage, returning the most repeats any position needed.
    fn repeats_needed(stage: Stage, algorithm: &str, hold: impl Fn(&mut Cube)) -> u32 {
        let mut rng = fastrand::Rng::with_seed(99);
        let mut most = 0;
        for _ in 0..300 {
            let mut cube = stage_position(stage, &mut rng);
            let mut repeats = 0;
            while MethodView::of(&cube).stage() == stage {
                hold(&mut cube);
                apply(&mut cube, algorithm);
                repeats += 1;
                assert!(repeats < 10, "the recipe never finished {stage:?}");
            }
            assert!(MethodView::of(&cube).stage() > stage);
            most = most.max(repeats);
        }
        most
    }

    #[test]
    fn corner_lesson_trigger_should_insert_a_top_corner_in_one_three_or_five_repeats() {
        for repeats in [1, 3, 5] {
            let mut cube = training_start();
            for _ in 0..repeats {
                apply(&mut cube, "U R U' R'");
            }
            let view = MethodView::of(&cube);
            assert_eq!(
                view.cube.cubie_at_home([1, -1, 1]).position(),
                UFR,
                "{repeats}"
            );
            for _ in 0..repeats {
                apply(&mut cube, "R U R' U'");
            }
            assert!(MethodView::of(&cube).solved([1, -1, 1]), "{repeats}");
        }
    }

    #[test]
    fn middle_lesson_right_insert_should_take_the_edge_whose_top_matches_the_right() {
        let mut cube = training_start();
        apply(&mut cube, "F' U' F U R U R' U'");
        let view = MethodView::of(&cube);
        let edge = view.cube.cubie_at_home([1, 0, 1]);
        assert_eq!(
            (
                edge.position(),
                edge.rotation().apply([0, 0, 1]),
                edge.rotation().apply([1, 0, 0])
            ),
            (UF, FRONT, UP),
        );
    }

    #[test]
    fn middle_lesson_left_insert_should_take_the_edge_whose_top_matches_the_left() {
        let mut cube = training_start();
        apply(&mut cube, "F U F' U' L' U' L U");
        let view = MethodView::of(&cube);
        let edge = view.cube.cubie_at_home([-1, 0, 1]);
        assert_eq!(
            (
                edge.position(),
                edge.rotation().apply([0, 0, 1]),
                edge.rotation().apply([-1, 0, 0])
            ),
            (UF, FRONT, UP),
        );
    }

    #[test]
    fn yellow_cross_recipe_should_finish_in_at_most_three_repeats() {
        let most = repeats_needed(Stage::YellowCross, "F R U R' U' F'", |cube| {
            hold_until(cube, |cube| {
                let (b, l, r, f) = (
                    yellow_on_top(cube, UB),
                    yellow_on_top(cube, UL),
                    yellow_on_top(cube, UR),
                    yellow_on_top(cube, UF),
                );
                // Either an L at the back left or a horizontal line. A dot needs no holding.
                (b && l && !r && !f) || (l && r && !b && !f)
            });
        });
        assert!(most <= 3, "{most}");
    }

    #[test]
    fn yellow_face_recipe_should_finish() {
        let most = repeats_needed(Stage::YellowCorners, "R U R' U R U2 R'", |cube| {
            let count = [UFL, UFR, UBL, UBR]
                .iter()
                .filter(|&&corner| yellow_on_top(cube, corner))
                .count();
            hold_until(cube, |cube| match count {
                1 => yellow_on_top(cube, UFL),
                0 => sticker(cube, UFL, LEFT) == YELLOW,
                _ => sticker(cube, UFL, FRONT) == YELLOW,
            });
        });
        assert!(most <= 4, "{most}");
    }

    #[test]
    fn place_corners_recipe_should_finish() {
        let most = repeats_needed(Stage::PlaceCorners, "R' F R' B2 R F' R' B2 R2", |cube| {
            hold_until(cube, |cube| {
                sticker(cube, UBL, BACK) == sticker(cube, UBR, BACK)
            });
        });
        assert!(most <= 3, "{most}");
    }

    #[test]
    fn place_edges_recipe_should_finish_in_the_promised_repeats() {
        let most = repeats_needed(Stage::PlaceEdges, "R2 U R U R' U' R' U' R' U R'", |cube| {
            // Turn the top so that the corners match their sides.
            for _ in 0..4 {
                if sticker(cube, UBL, BACK) == sticker(cube, [0, 0, -1], BACK)
                    && sticker(cube, UBR, BACK) == sticker(cube, [0, 0, -1], BACK)
                {
                    break;
                }
                apply(cube, "U");
            }
            hold_until(cube, |cube| {
                sticker(cube, UB, BACK) == sticker(cube, [0, 0, -1], BACK)
            });
        });
        assert!(most <= 3, "{most}");
    }

    #[test]
    fn follow_should_advance_on_the_next_turn() {
        let start = training_start();
        let mut follow = Follow::new(&start, "R U");
        let mut cube = start;
        cube.apply("R".parse().unwrap());
        follow.sync(&cube);
        assert_eq!(follow.next_turn(), Some("U".parse().unwrap()));
    }

    #[test]
    fn follow_should_not_finish_early_when_a_state_repeats() {
        let start = training_start();
        let mut follow = Follow::new(&start, "R R'");
        follow.sync(&start);
        assert_eq!(follow.next_turn(), Some("R".parse().unwrap()));
    }

    #[test]
    fn follow_should_flag_a_wrong_turn_until_it_is_undone() {
        let start = training_start();
        let mut follow = Follow::new(&start, "R U");
        let mut cube = start.clone();
        cube.apply("F".parse().unwrap());
        follow.sync(&cube);
        assert!(follow.is_off_track());
        follow.sync(&start);
        assert!(!follow.is_off_track());
    }

    #[test]
    fn follow_should_finish_after_the_last_turn() {
        let start = training_start();
        let mut follow = Follow::new(&start, "R");
        let mut cube = start;
        cube.apply("R".parse().unwrap());
        follow.sync(&cube);
        assert!(follow.is_finished());
    }
}
