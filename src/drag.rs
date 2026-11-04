//! Geometry for turning a layer with the mouse. Free of Bevy systems, so it can be unit tested.
//!
//! The cube always sits axis-aligned at the origin because only the camera orbits. That reduces
//! picking to a ray-box test, and a drag to choosing which edge direction of the grabbed face the
//! cursor is moving along.

use bevy::math::{Vec2, Vec3};

use crate::cube::{Axis, Layer};

/// The distance from the centre of the cube to each face, in world units.
pub const CUBE_HALF_EXTENT: f32 = 1.5;
/// The width of one cubie, in world units.
pub const CUBIE_SIZE: f32 = 1.0;

/// A point on the surface of the cube under the cursor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FaceHit {
    /// The point on the surface, in world space.
    pub point: Vec3,
    /// The axis perpendicular to the grabbed face.
    pub normal_axis: Axis,
    /// The side of the cube the grabbed face is on: `1.0` or `-1.0`.
    pub normal_sign: f32,
}

/// Casts a ray at the cube, returning the first face it enters.
pub fn pick_face(origin: Vec3, direction: Vec3) -> Option<FaceHit> {
    let mut entry = f32::NEG_INFINITY;
    let mut exit = f32::INFINITY;
    let mut hit_axis = None;
    for axis in Axis::ALL {
        let i = axis.index();
        let (o, d) = (origin[i], direction[i]);
        if d.abs() < 1e-6 {
            if o.abs() > CUBE_HALF_EXTENT {
                return None;
            }
            continue;
        }
        let near = (-d.signum() * CUBE_HALF_EXTENT - o) / d;
        let far = (d.signum() * CUBE_HALF_EXTENT - o) / d;
        if near > entry {
            entry = near;
            hit_axis = Some((axis, -d.signum()));
        }
        exit = exit.min(far);
    }
    if entry > exit || entry < 0.0 {
        return None;
    }
    let (normal_axis, normal_sign) = hit_axis?;
    Some(FaceHit {
        point: origin + direction * entry,
        normal_axis,
        normal_sign,
    })
}

/// A drag resolved to a specific layer, along with how to convert cursor motion to an angle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerDrag {
    /// The axis the layer rotates about.
    pub axis: Axis,
    /// The layer being turned.
    pub layer: Layer,
    /// Radians of rotation about `+axis` per pixel of cursor motion along screen X and Y.
    pub radians_per_pixel: Vec2,
}

impl LayerDrag {
    /// Returns the layer's angle after the cursor has moved by `drag` pixels.
    pub fn angle(&self, drag: Vec2) -> f32 {
        self.radians_per_pixel.dot(drag)
    }
}

/// Decides which layer a drag turns.
///
/// `drag` is the cursor motion in pixels. `project` maps a world point to viewport pixels and
/// returns `None` if the point isn't visible.
pub fn resolve_drag(
    hit: FaceHit,
    drag: Vec2,
    project: impl Fn(Vec3) -> Option<Vec2>,
) -> Option<LayerDrag> {
    let hit_on_screen = project(hit.point)?;
    let normal = axis_vector(hit.normal_axis) * hit.normal_sign;

    // Pick the edge direction of the face that the cursor's motion lines up with best.
    let (tangent, screen_per_world) = Axis::ALL
        .into_iter()
        .filter(|&axis| axis != hit.normal_axis)
        .filter_map(|axis| {
            let tangent = axis_vector(axis);
            let screen_per_world = project(hit.point + tangent * 0.1)? - hit_on_screen;
            Some((tangent, screen_per_world * 10.0))
        })
        .max_by(|(_, a), (_, b)| {
            alignment(*a, drag)
                .partial_cmp(&alignment(*b, drag))
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;

    let pixels_per_world = screen_per_world.length();
    if pixels_per_world < 1e-3 {
        return None;
    }

    // A positive rotation about `normal × tangent` carries a point on the face towards `+tangent`.
    let rotation = normal.cross(tangent);
    let axis = Axis::ALL
        .into_iter()
        .find(|axis| rotation[axis.index()].abs() > 0.5)?;
    let rotation_sign = rotation[axis.index()];

    let coord = (hit.point[axis.index()] / CUBIE_SIZE).round() as i8;
    let direction = screen_per_world / pixels_per_world;
    let radians_per_world = rotation_sign / CUBE_HALF_EXTENT;

    Some(LayerDrag {
        axis,
        layer: Layer::at(coord),
        radians_per_pixel: direction * radians_per_world / pixels_per_world,
    })
}

fn alignment(screen_direction: Vec2, drag: Vec2) -> f32 {
    screen_direction.normalize_or_zero().dot(drag).abs()
}

/// Returns the unit vector along `axis`.
pub fn axis_vector(axis: Axis) -> Vec3 {
    match axis {
        Axis::X => Vec3::X,
        Axis::Y => Vec3::Y,
        Axis::Z => Vec3::Z,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A simple view straight down `-Z`, with screen Y pointing down as in a real viewport.
    fn front_view(point: Vec3) -> Option<Vec2> {
        Some(Vec2::new(point.x, -point.y) * 100.0)
    }

    fn front_face_hit(x: f32, y: f32) -> FaceHit {
        pick_face(Vec3::new(x, y, 10.0), Vec3::NEG_Z).unwrap()
    }

    #[test]
    fn ray_at_the_front_face_should_hit_it() {
        let hit = front_face_hit(0.4, -0.4);
        assert_eq!(
            (hit.normal_axis, hit.normal_sign, hit.point),
            (Axis::Z, 1.0, Vec3::new(0.4, -0.4, 1.5)),
        );
    }

    #[test]
    fn ray_from_behind_should_hit_the_back_face() {
        let hit = pick_face(Vec3::new(0.0, 0.0, -10.0), Vec3::Z).unwrap();
        assert_eq!((hit.normal_axis, hit.normal_sign), (Axis::Z, -1.0));
    }

    #[test]
    fn angled_ray_should_hit_the_face_it_enters_first() {
        let hit = pick_face(Vec3::new(5.0, 0.0, 1.0), Vec3::NEG_X).unwrap();
        assert_eq!((hit.normal_axis, hit.normal_sign), (Axis::X, 1.0));
    }

    #[test]
    fn ray_beside_the_cube_should_miss() {
        assert_eq!(pick_face(Vec3::new(2.0, 0.0, 10.0), Vec3::NEG_Z), None);
    }

    #[test]
    fn ray_pointing_away_should_miss() {
        assert_eq!(pick_face(Vec3::new(0.0, 0.0, 10.0), Vec3::Z), None);
    }

    #[test]
    fn ray_starting_inside_the_cube_should_miss() {
        assert_eq!(pick_face(Vec3::ZERO, Vec3::Z), None);
    }

    /// Returns the drag made by grabbing the front face at `(x, y)` and moving by `drag` pixels.
    fn front_drag(x: f32, y: f32, drag: Vec2) -> LayerDrag {
        resolve_drag(front_face_hit(x, y), drag, front_view).unwrap()
    }

    // The expected layers and signs below are the real moves a person's hand would make.
    #[test]
    fn dragging_top_row_left_should_be_a_clockwise_u() {
        let drag = Vec2::new(-30.0, 0.0);
        let turn = front_drag(0.0, 1.0, drag);
        assert_eq!((turn.axis, turn.layer), (Axis::Y, Layer::Pos));
        // U is clockwise from above, which is a negative rotation about +Y.
        assert!(turn.angle(drag) < 0.0);
    }

    #[test]
    fn dragging_right_column_up_should_be_a_clockwise_r() {
        let drag = Vec2::new(0.0, -30.0);
        let turn = front_drag(1.0, 0.0, drag);
        assert_eq!((turn.axis, turn.layer), (Axis::X, Layer::Pos));
        // R is clockwise from the right, which is a negative rotation about +X.
        assert!(turn.angle(drag) < 0.0);
    }

    #[test]
    fn dragging_left_column_down_should_be_a_clockwise_l() {
        let drag = Vec2::new(0.0, 30.0);
        let turn = front_drag(-1.0, 0.0, drag);
        assert_eq!((turn.axis, turn.layer), (Axis::X, Layer::Neg));
        assert!(turn.angle(drag) > 0.0);
    }

    #[test]
    fn dragging_middle_column_should_turn_the_middle_slice() {
        let turn = front_drag(0.0, 0.0, Vec2::new(0.0, 30.0));
        assert_eq!((turn.axis, turn.layer), (Axis::X, Layer::Mid));
    }

    #[test]
    fn dragging_the_right_face_vertically_should_be_an_f_or_b_turn() {
        let hit = FaceHit {
            point: Vec3::new(1.5, 0.0, 1.0),
            normal_axis: Axis::X,
            normal_sign: 1.0,
        };
        // Look at the +X face straight on, with +Z to the left of the screen.
        let view = |p: Vec3| Some(Vec2::new(-p.z, -p.y) * 100.0);
        let drag = Vec2::new(0.0, 30.0);
        let turn = resolve_drag(hit, drag, view).unwrap();
        assert_eq!((turn.axis, turn.layer), (Axis::Z, Layer::Pos));
        // Moving the right side of the front layer downwards is a clockwise F.
        assert!(turn.angle(drag) < 0.0);
    }

    #[test]
    fn dragging_across_half_a_cube_should_turn_about_a_third_of_a_circle() {
        // Moving one radius along the surface is one radian of rotation at that radius.
        let drag = Vec2::new(0.0, -150.0);
        let angle = front_drag(1.0, 0.0, drag).angle(drag);
        assert!((angle.abs() - 1.0).abs() < 1e-3, "{angle}");
    }

    #[test]
    fn drag_should_be_rejected_when_the_point_is_not_visible() {
        assert_eq!(
            resolve_drag(front_face_hit(0.0, 0.0), Vec2::X, |_| None),
            None
        );
    }
}
