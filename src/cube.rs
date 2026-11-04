//! Pure Rubik's cube model, with no dependency on Bevy.
//!
//! Each cubie stores only an integer rotation from its solved ("home") placement. Its current
//! position is derived from that rotation, so the state cannot drift the way accumulated floating
//! point transforms would.
//!
//! The coordinate system matches Bevy's: right-handed and Y-up. In the solved state, `+X` is the
//! R face, `+Y` is U, and `+Z` is F.

use std::fmt;
use std::str::FromStr;

/// One of the cube's three rotation axes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    /// All axes, in index order.
    pub const ALL: [Axis; 3] = [Axis::X, Axis::Y, Axis::Z];

    /// The index of this axis within an `[x, y, z]` array.
    pub const fn index(self) -> usize {
        match self {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        }
    }
}

/// Which cubies along an axis take part in a turn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Layer {
    /// The outer layer on the negative side of the axis.
    Neg,
    /// The middle slice.
    Mid,
    /// The outer layer on the positive side of the axis.
    Pos,
    /// Every layer, which rotates the whole cube.
    All,
}

impl Layer {
    /// Returns whether a cubie at `coord` along the turn axis belongs to this layer.
    pub const fn contains(self, coord: i8) -> bool {
        match self {
            Layer::Neg => coord < 0,
            Layer::Mid => coord == 0,
            Layer::Pos => coord > 0,
            Layer::All => true,
        }
    }

    /// Returns the single layer at `coord`, treating any negative or positive value as outer.
    pub const fn at(coord: i8) -> Layer {
        if coord < 0 {
            Layer::Neg
        } else if coord > 0 {
            Layer::Pos
        } else {
            Layer::Mid
        }
    }
}

/// A rotation of some layers about an axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Turn {
    /// The axis being rotated about.
    pub axis: Axis,
    /// The layers that move.
    pub layer: Layer,
    /// Quarter turns, positive by the right-hand rule about `+axis`. Always in `-1..=2`.
    pub quarters: i8,
}

impl Turn {
    /// Creates a turn, reducing `quarters` to the shortest equivalent in `-1..=2`.
    pub fn new(axis: Axis, layer: Layer, quarters: i32) -> Self {
        let quarters = match quarters.rem_euclid(4) {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => -1,
        };
        Self {
            axis,
            layer,
            quarters,
        }
    }

    /// Returns the turn that undoes this one.
    pub fn inverse(self) -> Self {
        Self::new(self.axis, self.layer, -i32::from(self.quarters))
    }

    /// Returns whether this turn leaves the cube unchanged.
    pub fn is_identity(self) -> bool {
        self.quarters == 0
    }

    /// Returns whether this rotates the whole cube instead of scrambling it.
    pub fn is_whole_cube(self) -> bool {
        self.layer == Layer::All
    }
}

/// Maps notation letters to the turn they name: axis, layer, and the sign of a clockwise turn.
///
/// Clockwise means as seen looking at that face from outside the cube. M follows L, E follows D,
/// S follows F, and the whole-cube rotations x, y, and z follow R, U, and F.
const NOTATION: [(char, Axis, Layer, i8); 12] = [
    ('R', Axis::X, Layer::Pos, -1),
    ('L', Axis::X, Layer::Neg, 1),
    ('M', Axis::X, Layer::Mid, 1),
    ('x', Axis::X, Layer::All, -1),
    ('U', Axis::Y, Layer::Pos, -1),
    ('D', Axis::Y, Layer::Neg, 1),
    ('E', Axis::Y, Layer::Mid, 1),
    ('y', Axis::Y, Layer::All, -1),
    ('F', Axis::Z, Layer::Pos, -1),
    ('B', Axis::Z, Layer::Neg, 1),
    ('S', Axis::Z, Layer::Mid, -1),
    ('z', Axis::Z, Layer::All, -1),
];

impl fmt::Display for Turn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let entry = NOTATION
            .iter()
            .find(|(_, axis, layer, _)| *axis == self.axis && *layer == self.layer);
        let Some((letter, _, _, clockwise)) = entry else {
            return Err(fmt::Error);
        };
        match self.quarters {
            0 => write!(f, "{letter}0"),
            2 => write!(f, "{letter}2"),
            q if q == *clockwise => write!(f, "{letter}"),
            _ => write!(f, "{letter}'"),
        }
    }
}

/// The error returned when a string is not valid cube notation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseTurnError(String);

impl fmt::Display for ParseTurnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid cube notation: {:?}", self.0)
    }
}

impl std::error::Error for ParseTurnError {}

impl FromStr for Turn {
    type Err = ParseTurnError;

    /// Parses standard notation such as `R`, `U'`, `F2`, `M`, or `x'`.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let error = || ParseTurnError(text.to_owned());
        let mut chars = text.chars();
        let letter = chars.next().ok_or_else(error)?;
        let &(_, axis, layer, clockwise) = NOTATION
            .iter()
            .find(|(candidate, ..)| *candidate == letter)
            .ok_or_else(error)?;
        let clockwise = i32::from(clockwise);
        let quarters = match chars.as_str() {
            "" => clockwise,
            "'" => -clockwise,
            "2" | "2'" => 2,
            _ => return Err(error()),
        };
        Ok(Turn::new(axis, layer, quarters))
    }
}

/// Parses a whitespace-separated sequence of turns such as `R U R' U'`.
pub fn parse_sequence(text: &str) -> Result<Vec<Turn>, ParseTurnError> {
    text.split_whitespace().map(str::parse).collect()
}

/// Parses an algorithm, which may also use wide turns such as `r` or `f'`.
///
/// A wide turn moves an outer layer and the slice next to it, so it expands to those two turns.
pub fn parse_algorithm(text: &str) -> Result<Vec<Turn>, ParseTurnError> {
    let mut turns = Vec::new();
    for token in text.split_whitespace() {
        let mut chars = token.chars();
        let wide = match chars.next() {
            Some('r') => Some(('R', 'M', true)),
            Some('l') => Some(('L', 'M', false)),
            Some('u') => Some(('U', 'E', true)),
            Some('d') => Some(('D', 'E', false)),
            Some('f') => Some(('F', 'S', false)),
            Some('b') => Some(('B', 'S', true)),
            _ => None,
        };
        let Some((face, slice, slice_reversed)) = wide else {
            turns.push(token.parse()?);
            continue;
        };
        let suffix = chars.as_str();
        let slice_turn: Turn = format!("{slice}{suffix}").parse()?;
        turns.push(format!("{face}{suffix}").parse()?);
        turns.push(if slice_reversed {
            slice_turn.inverse()
        } else {
            slice_turn
        });
    }
    Ok(turns)
}

/// Returns the turns that undo `turns`.
pub fn invert_sequence(turns: &[Turn]) -> Vec<Turn> {
    turns.iter().rev().map(|turn| turn.inverse()).collect()
}

/// An exact rotation of a cubie, stored as a signed permutation matrix in row order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rotation([[i8; 3]; 3]);

impl Rotation {
    /// The rotation that leaves everything in place.
    pub const IDENTITY: Rotation = Rotation([[1, 0, 0], [0, 1, 0], [0, 0, 1]]);

    /// Returns a rotation of `quarters` quarter turns about `axis`, by the right-hand rule.
    pub fn quarter_turns(axis: Axis, quarters: i8) -> Self {
        let step = match axis {
            Axis::X => Rotation([[1, 0, 0], [0, 0, -1], [0, 1, 0]]),
            Axis::Y => Rotation([[0, 0, 1], [0, 1, 0], [-1, 0, 0]]),
            Axis::Z => Rotation([[0, -1, 0], [1, 0, 0], [0, 0, 1]]),
        };
        (0..quarters.rem_euclid(4)).fold(Self::IDENTITY, |acc, _| step.after(acc))
    }

    /// Returns the rotation that applies `first` and then `self`.
    pub fn after(self, first: Rotation) -> Rotation {
        let mut out = [[0; 3]; 3];
        for (row, out_row) in out.iter_mut().enumerate() {
            for (col, cell) in out_row.iter_mut().enumerate() {
                *cell = (0..3).map(|k| self.0[row][k] * first.0[k][col]).sum();
            }
        }
        Rotation(out)
    }

    /// Returns the rotation that undoes this one.
    pub fn inverse(self) -> Rotation {
        let m = self.0;
        Rotation([
            [m[0][0], m[1][0], m[2][0]],
            [m[0][1], m[1][1], m[2][1]],
            [m[0][2], m[1][2], m[2][2]],
        ])
    }

    /// Returns the turn that acts in the rotated frame the way `turn` acts in the original one.
    pub fn rotate_turn(self, turn: Turn) -> Turn {
        let mut unit = [0; 3];
        unit[turn.axis.index()] = 1;
        let image = self.apply(unit);
        let axis = Axis::ALL
            .into_iter()
            .find(|axis| image[axis.index()] != 0)
            .unwrap_or(turn.axis);
        let sign = image[axis.index()];
        let layer = match (sign < 0, turn.layer) {
            (true, Layer::Pos) => Layer::Neg,
            (true, Layer::Neg) => Layer::Pos,
            (_, layer) => layer,
        };
        Turn::new(axis, layer, i32::from(turn.quarters) * i32::from(sign))
    }

    /// Applies this rotation to a vector.
    pub fn apply(self, v: [i8; 3]) -> [i8; 3] {
        let dot = |row: [i8; 3]| row[0] * v[0] + row[1] * v[1] + row[2] * v[2];
        [dot(self.0[0]), dot(self.0[1]), dot(self.0[2])]
    }

    /// Returns the matrix rows, for converting to a floating point rotation.
    pub const fn rows(self) -> [[i8; 3]; 3] {
        self.0
    }
}

/// One of the 27 pieces of the cube, including the hidden core.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cubie {
    home: [i8; 3],
    rotation: Rotation,
}

impl Cubie {
    /// The position this cubie occupies when the cube is solved.
    pub const fn home(self) -> [i8; 3] {
        self.home
    }

    /// The current rotation from the cubie's home placement.
    pub const fn rotation(self) -> Rotation {
        self.rotation
    }

    /// The current position, with each coordinate in `-1..=1`.
    pub fn position(self) -> [i8; 3] {
        self.rotation.apply(self.home)
    }

    /// The outward normals of this cubie's stickers in its home placement.
    pub fn home_normals(self) -> impl Iterator<Item = [i8; 3]> {
        Axis::ALL.into_iter().filter_map(move |axis| {
            let coord = self.home[axis.index()];
            (coord != 0).then(|| {
                let mut normal = [0; 3];
                normal[axis.index()] = coord;
                normal
            })
        })
    }
}

/// The number of cubies in a 3×3×3 cube, including the core.
pub const CUBIE_COUNT: usize = 27;

/// A 3×3×3 cube.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cube {
    cubies: [Cubie; CUBIE_COUNT],
}

impl Default for Cube {
    fn default() -> Self {
        Self::solved()
    }
}

impl Cube {
    /// Creates a solved cube.
    pub fn solved() -> Self {
        let cubies = std::array::from_fn(|index| {
            // `index` is below 27, so each base-3 digit fits and the casts cannot truncate.
            let digit = |place: usize| ((index / place) % 3) as i8 - 1;
            Cubie {
                home: [digit(1), digit(3), digit(9)],
                rotation: Rotation::IDENTITY,
            }
        });
        Self { cubies }
    }

    /// All cubies. Their order never changes, so an index identifies a physical piece.
    pub fn cubies(&self) -> &[Cubie; CUBIE_COUNT] {
        &self.cubies
    }

    /// Returns the cubie that belongs at `home`.
    pub fn cubie_at_home(&self, home: [i8; 3]) -> Cubie {
        self.cubies[index_of_home(home)]
    }

    /// Returns the rotation of the cube as a whole, as shown by where its centres have gone.
    ///
    /// The core turns with every slice and whole-cube rotation and with nothing else, so it always
    /// has the same rotation as the set of centres.
    pub fn frame(&self) -> Rotation {
        self.cubies[index_of_home([0, 0, 0])].rotation
    }

    /// Re-expresses the cube so that centres sit where `colours` puts them when solved.
    ///
    /// The result doesn't depend on how the cube is being held. In it, a piece is in its solved
    /// place relative to the centres exactly when its rotation is the identity. Also returns the
    /// rotation that maps the new frame back to the world, for converting turns.
    pub fn reframed(&self, colours: Rotation) -> (Cube, Rotation) {
        let to_world = self.frame().after(colours);
        let from_world = to_world.inverse();
        let from_colours = colours.inverse();
        let mut cubies = self.cubies;
        for cubie in &self.cubies {
            let home = from_colours.apply(cubie.home);
            cubies[index_of_home(home)] = Cubie {
                home,
                rotation: from_world.after(cubie.rotation.after(colours)),
            };
        }
        (Self { cubies }, to_world)
    }

    /// Applies a turn to the cube.
    pub fn apply(&mut self, turn: Turn) {
        let rotation = Rotation::quarter_turns(turn.axis, turn.quarters);
        for cubie in &mut self.cubies {
            if turn.layer.contains(cubie.position()[turn.axis.index()]) {
                cubie.rotation = rotation.after(cubie.rotation);
            }
        }
    }

    /// Applies a sequence of turns in order.
    pub fn apply_all(&mut self, turns: impl IntoIterator<Item = Turn>) {
        for turn in turns {
            self.apply(turn);
        }
    }

    /// Returns whether every face shows a single colour.
    ///
    /// This holds in any overall orientation, so whole-cube rotations and a matched set of slice
    /// turns don't make a solved cube count as unsolved.
    pub fn is_solved(&self) -> bool {
        let mut face_colours: [Option<[i8; 3]>; 6] = [None; 6];
        self.cubies.iter().all(|cubie| {
            cubie.home_normals().all(|home_normal| {
                let colour = face_colours[face_slot(cubie.rotation.apply(home_normal))]
                    .get_or_insert(home_normal);
                *colour == home_normal
            })
        })
    }
}

/// Returns the index in [`Cube::cubies`] of the piece whose solved position is `home`.
pub fn index_of_home(home: [i8; 3]) -> usize {
    let digit = |coord: i8| (coord.clamp(-1, 1) + 1) as usize;
    digit(home[0]) + 3 * digit(home[1]) + 9 * digit(home[2])
}

/// Returns a distinct index in `0..6` for an axis-aligned unit normal.
fn face_slot(normal: [i8; 3]) -> usize {
    match normal {
        [1, _, _] => 0,
        [-1, _, _] => 1,
        [_, 1, _] => 2,
        [_, -1, _] => 3,
        [_, _, 1] => 4,
        _ => 5,
    }
}

/// Generates a random scramble of outer-layer turns.
///
/// Consecutive turns never share an axis, so no turn can partly cancel the previous one.
pub fn scramble(rng: &mut fastrand::Rng, length: usize) -> Vec<Turn> {
    let mut turns: Vec<Turn> = Vec::with_capacity(length);
    while turns.len() < length {
        let axis = Axis::ALL[rng.usize(..Axis::ALL.len())];
        if turns.last().is_some_and(|last| last.axis == axis) {
            continue;
        }
        let layer = if rng.bool() { Layer::Pos } else { Layer::Neg };
        const QUARTERS: [i32; 3] = [-1, 1, 2];
        turns.push(Turn::new(
            axis,
            layer,
            QUARTERS[rng.usize(..QUARTERS.len())],
        ));
    }
    turns
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seq(text: &str) -> Vec<Turn> {
        parse_sequence(text).unwrap()
    }

    fn after(text: &str) -> Cube {
        let mut cube = Cube::solved();
        cube.apply_all(seq(text));
        cube
    }

    /// Returns where every sticker is, which ignores the unseen spin of centre pieces.
    fn appearance(cube: &Cube) -> Vec<([i8; 3], [i8; 3])> {
        cube.cubies()
            .iter()
            .flat_map(|cubie| {
                cubie
                    .home_normals()
                    .map(|normal| (cubie.position(), cubie.rotation().apply(normal)))
            })
            .collect()
    }

    /// Returns how many times `text` must repeat before a solved cube looks solved again.
    fn order_of(text: &str) -> usize {
        let turns = seq(text);
        let solved = appearance(&Cube::solved());
        let mut cube = Cube::solved();
        for count in 1..=2000 {
            cube.apply_all(turns.iter().copied());
            if appearance(&cube) == solved {
                return count;
            }
        }
        panic!("{text} did not return to solved");
    }

    #[test]
    fn solved_cube_should_report_solved() {
        assert!(Cube::solved().is_solved());
    }

    #[test]
    fn solved_cube_should_have_one_cubie_per_position() {
        let mut positions: Vec<_> = Cube::solved().cubies().map(Cubie::position).to_vec();
        positions.sort_unstable();
        positions.dedup();
        assert_eq!(positions.len(), CUBIE_COUNT);
    }

    #[test]
    fn solved_cube_should_have_54_stickers() {
        let stickers: usize = Cube::solved()
            .cubies()
            .iter()
            .map(|cubie| cubie.home_normals().count())
            .sum();
        assert_eq!(stickers, 54);
    }

    #[test]
    fn single_turn_should_unsolve_the_cube() {
        assert!(!after("R").is_solved());
    }

    #[test]
    fn four_quarter_turns_should_restore_every_face_and_slice() {
        for letter in ["R", "L", "U", "D", "F", "B", "M", "E", "S", "x", "y", "z"] {
            assert_eq!(order_of(letter), 4, "{letter}");
        }
    }

    #[test]
    fn turn_followed_by_inverse_should_restore_the_cube() {
        for text in ["R", "U'", "F2", "M", "S'", "E2", "y", "z'"] {
            let turn: Turn = text.parse().unwrap();
            let mut cube = Cube::solved();
            cube.apply(turn);
            cube.apply(turn.inverse());
            assert_eq!(cube, Cube::solved(), "{text}");
        }
    }

    // Published group orders for well-known sequences catch a wrong rotation direction or axis
    // that simpler round trips cannot.
    #[test]
    fn sexy_move_should_have_order_six() {
        assert_eq!(order_of("R U R' U'"), 6);
    }

    #[test]
    fn r_u_should_have_order_105() {
        assert_eq!(order_of("R U"), 105);
    }

    #[test]
    fn r_u2_d_prime_b_d_prime_should_have_order_1260() {
        assert_eq!(order_of("R U2 D' B D'"), 1260);
    }

    #[test]
    fn sune_should_have_order_six() {
        assert_eq!(order_of("R U R' U R U2 R'"), 6);
    }

    #[test]
    fn t_perm_should_have_order_two() {
        assert_eq!(order_of("R U R' U' R' F R2 U' R' U' R U R' F'"), 2);
    }

    // Checks each direction against a physical cube rather than against the code's own tables:
    // the corner at U-F-R moves where a real clockwise turn of that face sends it.
    #[test]
    fn clockwise_face_turns_should_move_the_ufr_corner_correctly() {
        let cases = [
            ("R", [1, 1, -1]),
            ("R'", [1, -1, 1]),
            ("U", [-1, 1, 1]),
            ("U'", [1, 1, -1]),
            ("F", [1, -1, 1]),
            ("F'", [-1, 1, 1]),
        ];
        for (text, expected) in cases {
            let cube = after(text);
            let corner = cube
                .cubies()
                .iter()
                .find(|c| c.home() == [1, 1, 1])
                .unwrap();
            assert_eq!(corner.position(), expected, "{text}");
        }
    }

    #[test]
    fn opposite_clockwise_turns_should_move_their_corners_correctly() {
        let cases = [
            ("L", [-1, 1, 1], [-1, -1, 1]),
            ("D", [1, -1, 1], [1, -1, -1]),
            ("B", [1, 1, -1], [-1, 1, -1]),
        ];
        for (text, home, expected) in cases {
            let cube = after(text);
            let corner = cube.cubies().iter().find(|c| c.home() == home).unwrap();
            assert_eq!(corner.position(), expected, "{text}");
        }
    }

    #[test]
    fn whole_cube_rotation_should_still_count_as_solved() {
        assert!(after("x y2 z'").is_solved());
    }

    #[test]
    fn matched_slice_turns_should_count_as_solved() {
        assert!(after("M M'").is_solved());
    }

    #[test]
    fn slice_turn_should_unsolve_the_cube() {
        assert!(!after("M").is_solved());
    }

    #[test]
    fn y_rotation_should_equal_u_e_prime_d_prime() {
        assert_eq!(after("y"), after("U E' D'"));
    }

    #[test]
    fn notation_should_round_trip_through_display() {
        for text in [
            "R", "R'", "R2", "L", "L'", "U", "U'", "D", "D'", "F", "F'", "B", "B'", "M", "M'", "E",
            "E'", "S", "S'", "x", "x'", "y", "y'", "z", "z'",
        ] {
            let turn: Turn = text.parse().unwrap();
            assert_eq!(turn.to_string(), text);
        }
    }

    #[test]
    fn parsing_should_reject_invalid_notation() {
        for text in ["", "Q", "R3", "R''", "r"] {
            assert!(text.parse::<Turn>().is_err(), "{text}");
        }
    }

    #[test]
    fn new_should_normalise_quarters_to_the_shortest_turn() {
        let quarters = |q| Turn::new(Axis::X, Layer::Pos, q).quarters;
        assert_eq!(
            [
                quarters(0),
                quarters(3),
                quarters(-3),
                quarters(4),
                quarters(-2)
            ],
            [0, -1, 1, 0, 2],
        );
    }

    #[test]
    fn wide_turns_should_move_the_face_and_the_slice_beside_it() {
        let cases = [
            ("r", "R M'"),
            ("r'", "R' M"),
            ("r2", "R2 M2"),
            ("l", "L M"),
            ("u", "U E'"),
            ("d", "D E"),
            ("f", "F S"),
            ("b", "B S'"),
        ];
        for (wide, expanded) in cases {
            assert_eq!(parse_algorithm(wide).unwrap(), seq(expanded), "{wide}");
        }
    }

    #[test]
    fn every_cubie_should_be_stored_at_the_index_of_its_home() {
        for (index, cubie) in Cube::solved().cubies().iter().enumerate() {
            assert_eq!(index_of_home(cubie.home()), index);
        }
    }

    #[test]
    fn invert_sequence_should_undo_a_sequence() {
        let turns = seq("R U F2 L' M");
        let mut cube = Cube::solved();
        cube.apply_all(turns.iter().copied());
        cube.apply_all(invert_sequence(&turns));
        assert_eq!(appearance(&cube), appearance(&Cube::solved()));
    }

    /// The whole-cube rotations and slices that can change how a cube is being held.
    const HOLDS: [&str; 6] = ["", "z2", "x y", "y' z", "M", "x2 E S'"];

    #[test]
    fn reframed_should_ignore_how_the_cube_is_held() {
        let colours = Rotation::quarter_turns(Axis::Z, 2);
        let base = after("R U2 F'").reframed(colours).0;
        for hold in ["", "z2", "x y", "y' z", "x2 y"] {
            let mut cube = after(hold);
            // Make the same physical turns in the new hold.
            let (_, to_world) = cube.reframed(colours);
            let colour_frame = Rotation::quarter_turns(Axis::Z, 2).inverse();
            cube.apply_all(
                seq("R U2 F'")
                    .into_iter()
                    .map(|turn| to_world.after(colour_frame).rotate_turn(turn)),
            );
            assert_eq!(
                appearance(&cube.reframed(colours).0),
                appearance(&base),
                "{hold}"
            );
        }
    }

    #[test]
    fn rotate_turn_should_match_the_same_turn_in_the_reframed_cube() {
        let colours = Rotation::quarter_turns(Axis::Z, 2);
        for hold in HOLDS {
            let start = {
                let mut cube = after(hold);
                cube.apply_all(seq("R U2 F' L D2 B"));
                cube
            };
            for name in ["R", "L", "U", "D", "F", "B", "R'", "U'", "F2"] {
                let method_turn: Turn = name.parse().unwrap();
                let (view, to_world) = start.reframed(colours);

                let mut moved_in_view = view;
                moved_in_view.apply(method_turn);

                let mut moved_in_world = start.clone();
                moved_in_world.apply(to_world.rotate_turn(method_turn));

                assert_eq!(
                    appearance(&moved_in_world.reframed(colours).0),
                    appearance(&moved_in_view),
                    "{hold}: {name}",
                );
            }
        }
    }

    #[test]
    fn scramble_should_have_the_requested_length() {
        assert_eq!(scramble(&mut fastrand::Rng::with_seed(1), 25).len(), 25);
    }

    #[test]
    fn scramble_should_not_repeat_an_axis_consecutively() {
        let turns = scramble(&mut fastrand::Rng::with_seed(2), 500);
        assert!(turns.windows(2).all(|pair| pair[0].axis != pair[1].axis));
    }

    #[test]
    fn scramble_should_not_contain_identity_turns() {
        let turns = scramble(&mut fastrand::Rng::with_seed(3), 500);
        assert!(!turns.iter().any(|turn| turn.is_identity()));
    }

    #[test]
    fn scramble_should_leave_the_cube_unsolved() {
        let mut cube = Cube::solved();
        cube.apply_all(scramble(&mut fastrand::Rng::with_seed(4), 25));
        assert!(!cube.is_solved());
    }
}
