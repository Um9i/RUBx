//! The standard OLL and PLL algorithm sets for the final stage of a CFOP solve.
//!
//! Each algorithm is written the way it's normally published: white down, green front, ready to
//! apply when the case shows on top. Tests check the whole of both tables against the cube itself.

use crate::cube::{Cube, Layer, Turn, invert_sequence, parse_algorithm};
use crate::method::training_start;

/// A last-layer case and an algorithm that solves it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Case {
    /// The standard name of the case.
    pub name: &'static str,
    /// The algorithm, in standard notation.
    pub alg: &'static str,
}

impl Case {
    const fn new(name: &'static str, alg: &'static str) -> Self {
        Self { name, alg }
    }

    /// Returns the algorithm's turns. The tables are tested, so this cannot fail in practice.
    pub fn turns(&self) -> Vec<Turn> {
        parse_algorithm(self.alg).unwrap_or_default()
    }

    /// Returns a cube showing this case, with the top layer turned by `before` and `after`
    /// quarter turns so that it can't be recognised by position alone.
    pub fn setup(&self, before: i32, after: i32) -> Cube {
        let mut cube = training_start();
        cube.apply(Turn::new(crate::cube::Axis::Y, Layer::Pos, after));
        cube.apply_all(invert_sequence(&self.turns()));
        cube.apply(Turn::new(crate::cube::Axis::Y, Layer::Pos, before));
        cube
    }
}

/// The 21 permutation cases of the last layer.
pub const PLL: [Case; 21] = [
    Case::new("Aa perm", "R' F R' B2 R F' R' B2 R2"),
    Case::new("Ab perm", "R2 B2 R F R' B2 R F' R"),
    Case::new("E perm", "x' R U' R' D R U R' D' R U R' D R U' R' D' x"),
    Case::new("F perm", "R' U' F' R U R' U' R' F R2 U' R' U' R U R' U R"),
    Case::new("Ga perm", "R2 U R' U R' U' R U' R2 U' D R' U R D'"),
    Case::new("Gb perm", "R' U' R U D' R2 U R' U R U' R U' R2 D"),
    Case::new("Gc perm", "R2 U' R U' R U R' U R2 U D' R U' R' D"),
    Case::new("Gd perm", "R U R' U' D R2 U' R U' R' U R' U R2 D'"),
    Case::new("H perm", "M2 U M2 U2 M2 U M2"),
    Case::new("Ja perm", "R' U L' U2 R U' R' U2 R L"),
    Case::new("Jb perm", "R U R' F' R U R' U' R' F R2 U' R'"),
    Case::new(
        "Na perm",
        "R U R' U R U R' F' R U R' U' R' F R2 U' R' U2 R U' R'",
    ),
    Case::new("Nb perm", "R' U R U' R' F' U' F R U R' F R' F' R U' R"),
    Case::new("Ra perm", "R U' R' U' R U R D R' U' R D' R' U2 R'"),
    Case::new("Rb perm", "R2 F R U R U' R' F' R U2 R' U2 R"),
    Case::new("T perm", "R U R' U' R' F R2 U' R' U' R U R' F'"),
    Case::new("Ua perm", "R U' R U R U R U' R' U' R2"),
    Case::new("Ub perm", "R2 U R U R' U' R' U' R' U R'"),
    Case::new("V perm", "R' U R' U' y R' F' R2 U' R' U R' F R F"),
    Case::new("Y perm", "F R U' R' U' R U R' F' R U R' U' R' F R F'"),
    Case::new("Z perm", "M2 U M2 U M' U2 M2 U2 M' U2"),
];

/// The 57 orientation cases of the last layer, numbered in the standard order.
pub const OLL: [Case; 57] = [
    Case::new("OLL 1", "R U2 R2 F R F' U2 R' F R F'"),
    Case::new("OLL 2", "F R U R' U' F' f R U R' U' f'"),
    Case::new("OLL 3", "f R U R' U' f' U' F R U R' U' F'"),
    Case::new("OLL 4", "f R U R' U' f' U F R U R' U' F'"),
    Case::new("OLL 5", "r' U2 R U R' U r"),
    Case::new("OLL 6", "r U2 R' U' R U' r'"),
    Case::new("OLL 7", "r U R' U R U2 r'"),
    Case::new("OLL 8", "r' U' R U' R' U2 r"),
    Case::new("OLL 9", "R U R' U' R' F R2 U R' U' F'"),
    Case::new("OLL 10", "R U R' U R' F R F' R U2 R'"),
    Case::new("OLL 11", "r' R2 U R' U R U2 R' U M'"),
    Case::new("OLL 12", "M' R' U' R U' R' U2 R U' M"),
    Case::new("OLL 13", "r U' r' U' r U r' F' U F"),
    Case::new("OLL 14", "R' F R U R' F' R F U' F'"),
    Case::new("OLL 15", "r' U' r R' U' R U r' U r"),
    Case::new("OLL 16", "r U r' R U R' U' r U' r'"),
    Case::new("OLL 17", "R U R' U R' F R F' U2 R' F R F'"),
    Case::new("OLL 18", "r U R' U R U2 r2 U' R U' R' U2 r"),
    Case::new("OLL 19", "M U R U R' U' M' R' F R F'"),
    Case::new("OLL 20", "M U R U R' U' M2 U R U' r'"),
    Case::new("OLL 21", "R U2 R' U' R U R' U' R U' R'"),
    Case::new("OLL 22", "R U2 R2 U' R2 U' R2 U2 R"),
    Case::new("OLL 23", "R2 D R' U2 R D' R' U2 R'"),
    Case::new("OLL 24", "r U R' U' r' F R F'"),
    Case::new("OLL 25", "F' r U R' U' r' F R"),
    Case::new("OLL 26 (Antisune)", "R U2 R' U' R U' R'"),
    Case::new("OLL 27 (Sune)", "R U R' U R U2 R'"),
    Case::new("OLL 28", "r U R' U' r' R U R U' R'"),
    Case::new("OLL 29", "R U R' U' R U' R' F' U' F R U R'"),
    Case::new("OLL 30", "F R' F R2 U' R' U' R U R' F2"),
    Case::new("OLL 31", "R' U' F U R U' R' F' R"),
    Case::new("OLL 32", "L U F' U' L' U L F L'"),
    Case::new("OLL 33", "R U R' U' R' F R F'"),
    Case::new("OLL 34", "R U R2 U' R' F R U R U' F'"),
    Case::new("OLL 35", "R U2 R2 F R F' R U2 R'"),
    Case::new("OLL 36", "L' U' L U' L' U L U L F' L' F"),
    Case::new("OLL 37", "F R' F' R U R U' R'"),
    Case::new("OLL 38", "R U R' U R U' R' U' R' F R F'"),
    Case::new("OLL 39", "L F' L' U' L U F U' L'"),
    Case::new("OLL 40", "R' F R U R' U' F' U R"),
    Case::new("OLL 41", "R U R' U R U2 R' F R U R' U' F'"),
    Case::new("OLL 42", "R' U' R U' R' U2 R F R U R' U' F'"),
    Case::new("OLL 43", "F' U' L' U L F"),
    Case::new("OLL 44", "F U R U' R' F'"),
    Case::new("OLL 45", "F R U R' U' F'"),
    Case::new("OLL 46", "R' U' R' F R F' U R"),
    Case::new("OLL 47", "R' U' R' F R F' R' F R F' U R"),
    Case::new("OLL 48", "F R U R' U' R U R' U' F'"),
    Case::new("OLL 49", "r U' r2 U r2 U r2 U' r"),
    Case::new("OLL 50", "r' U r2 U' r2 U' r2 U r'"),
    Case::new("OLL 51", "F U R U' R' U R U' R' F'"),
    Case::new("OLL 52", "R U R' U R U' B U' B' R'"),
    Case::new("OLL 53", "r' U' R U' R' U R U' R' U2 r"),
    Case::new("OLL 54", "r U R' U R U' R' U R U2 r'"),
    Case::new("OLL 55", "R' F R U R U' R2 F' R2 U' R' U R U R'"),
    Case::new("OLL 56", "r' U' r U' R' U R U' R' U R r' U r"),
    Case::new("OLL 57", "R U R' U' M' U R U' r'"),
];

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;
    use crate::cube::parse_sequence;
    use crate::method::{MethodView, Stage, TOP_CORNERS, TOP_EDGES};

    fn top_turned(view: &Cube, quarters: i32) -> Cube {
        let mut cube = view.clone();
        cube.apply(Turn::new(crate::cube::Axis::Y, Layer::Pos, quarters));
        cube
    }

    /// Identifies a case by its yellow stickers, however the top layer is turned.
    fn orientation_signature(case: &Case) -> Vec<([i8; 3], [i8; 3])> {
        let view = MethodView::of(&case.setup(0, 0)).cube;
        (0..4)
            .map(|quarters| {
                let turned = top_turned(&view, quarters);
                let mut signature: Vec<_> = TOP_CORNERS
                    .iter()
                    .chain(&TOP_EDGES)
                    .map(|&home| {
                        let cubie = turned.cubie_at_home(home);
                        (cubie.position(), cubie.rotation().apply([0, 1, 0]))
                    })
                    .collect();
                signature.sort_unstable();
                signature
            })
            .min()
            .unwrap()
    }

    /// Identifies a case by how it moves pieces, however the top layer is turned.
    ///
    /// Turning the top before and after a case also covers seeing it from another side, because
    /// that amounts to one turn before it and the opposite turn after it.
    fn permutation_signature(case: &Case) -> Vec<([i8; 3], [i8; 3])> {
        let homes: Vec<[i8; 3]> = TOP_CORNERS.iter().chain(&TOP_EDGES).copied().collect();
        let mut signatures = Vec::new();
        for before in 0..4 {
            for after in 0..4 {
                let view = MethodView::of(&case.setup(before, after)).cube;
                signatures.push(
                    homes
                        .iter()
                        .map(|&home| (home, view.cubie_at_home(home).position()))
                        .collect::<Vec<_>>(),
                );
            }
        }
        signatures.into_iter().min().unwrap()
    }

    #[test]
    fn pll_should_list_21_cases() {
        assert_eq!(PLL.len(), 21);
    }

    #[test]
    fn every_algorithm_should_parse() {
        for case in PLL.iter().chain(&OLL) {
            assert!(parse_algorithm(case.alg).is_ok(), "{}", case.name);
        }
    }

    #[test]
    fn every_pll_should_only_permute_the_last_layer() {
        for case in &PLL {
            assert_eq!(
                MethodView::of(&case.setup(0, 0))
                    .stage()
                    .max(Stage::PlaceCorners),
                MethodView::of(&case.setup(0, 0)).stage(),
                "{}",
                case.name,
            );
        }
    }

    #[test]
    fn every_pll_should_solve_its_own_setup() {
        for case in &PLL {
            let mut cube = case.setup(0, 0);
            cube.apply_all(case.turns());
            assert!(cube.is_solved(), "{}", case.name);
        }
    }

    #[test]
    fn every_pll_should_be_a_different_case() {
        let distinct: HashSet<_> = PLL.iter().map(permutation_signature).collect();
        assert_eq!(distinct.len(), 21);
    }

    #[test]
    fn every_oll_should_only_disturb_the_last_layer() {
        for case in &OLL {
            let stage = MethodView::of(&case.setup(0, 0)).stage();
            assert!(stage >= Stage::YellowCross, "{}: {stage:?}", case.name);
        }
    }

    #[test]
    fn every_oll_should_leave_the_top_yellow() {
        for case in &OLL {
            let mut cube = case.setup(0, 0);
            cube.apply_all(case.turns());
            assert!(
                MethodView::of(&cube).stage() > Stage::YellowCorners,
                "{}",
                case.name
            );
        }
    }

    #[test]
    fn every_oll_should_start_with_something_to_orient() {
        for case in &OLL {
            let view = MethodView::of(&case.setup(0, 0));
            assert!(view.stage() <= Stage::YellowCorners, "{}", case.name);
        }
    }

    #[test]
    fn every_oll_should_be_a_different_case() {
        let mut seen: HashMap<_, &str> = HashMap::new();
        for case in &OLL {
            if let Some(other) = seen.insert(orientation_signature(case), case.name) {
                panic!("{} and {other} are the same case", case.name);
            }
        }
        assert_eq!(seen.len(), 57);
    }

    fn oll(number: usize) -> &'static Case {
        &OLL[number - 1]
    }

    fn yellow_up_count(case: &Case, homes: &[[i8; 3]]) -> usize {
        let view = MethodView::of(&case.setup(0, 0));
        homes.iter().filter(|&&home| view.yellow_up(home)).count()
    }

    // The tests above can't tell if numbers are swapped between cases, but the published groups
    // can catch most of that.
    #[test]
    fn dot_cases_should_have_no_yellow_edges_on_top() {
        for number in [1, 2, 3, 4, 17, 18, 19, 20] {
            assert_eq!(yellow_up_count(oll(number), &TOP_EDGES), 0, "OLL {number}");
        }
    }

    #[test]
    fn cross_cases_should_have_every_yellow_edge_on_top() {
        for number in 21..=27 {
            assert_eq!(yellow_up_count(oll(number), &TOP_EDGES), 4, "OLL {number}");
        }
    }

    #[test]
    fn corners_oriented_cases_should_have_every_yellow_corner_on_top() {
        for number in [20, 28, 57] {
            assert_eq!(
                yellow_up_count(oll(number), &TOP_CORNERS),
                4,
                "OLL {number}"
            );
        }
    }

    /// Returns how many corners and edges a PLL moves, for each way of turning the top layer.
    fn pieces_moved(case: &Case) -> Vec<(usize, usize)> {
        let view = MethodView::of(&case.setup(0, 0)).cube;
        (0..4)
            .map(|quarters| {
                let turned = top_turned(&view, quarters);
                let moved = |homes: &[[i8; 3]]| {
                    homes
                        .iter()
                        .filter(|&&home| turned.cubie_at_home(home).position() != home)
                        .count()
                };
                (moved(&TOP_CORNERS), moved(&TOP_EDGES))
            })
            .collect()
    }

    #[test]
    fn pll_cases_should_move_the_pieces_their_names_promise() {
        let expected = [
            ("Aa perm", (3, 0)),
            ("Ab perm", (3, 0)),
            ("E perm", (4, 0)),
            ("H perm", (0, 4)),
            ("Z perm", (0, 4)),
            ("Ua perm", (0, 3)),
            ("Ub perm", (0, 3)),
            ("T perm", (2, 2)),
            ("F perm", (2, 2)),
            ("Ja perm", (2, 2)),
            ("Jb perm", (2, 2)),
            ("Ra perm", (2, 2)),
            ("Rb perm", (2, 2)),
            ("V perm", (2, 2)),
            ("Y perm", (2, 2)),
            ("Na perm", (2, 2)),
            ("Nb perm", (2, 2)),
            ("Ga perm", (3, 3)),
            ("Gb perm", (3, 3)),
            ("Gc perm", (3, 3)),
            ("Gd perm", (3, 3)),
        ];
        for (name, moved) in expected {
            let case = PLL.iter().find(|case| case.name == name).unwrap();
            assert!(pieces_moved(case).contains(&moved), "{name}");
        }
    }

    #[test]
    fn turning_the_top_should_not_change_what_a_case_needs() {
        let case = &PLL[15];
        let mut cube = case.setup(1, 2);
        // A positive quarter is anticlockwise from above, so `before` of 1 is a U' to undo.
        cube.apply_all(parse_sequence("U").unwrap());
        cube.apply_all(case.turns());
        cube.apply_all(parse_sequence("U2").unwrap());
        assert!(cube.is_solved());
    }
}
