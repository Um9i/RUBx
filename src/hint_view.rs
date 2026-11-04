//! Draws hints directly on the cube: an arrow for the next turn and an outline for the target.

use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use bevy::prelude::*;

use crate::cube::{Layer, Turn};
use crate::drag::axis_vector;
use crate::game::{CubeState, GameSet};
use crate::training::Run;
use crate::view::cubie_transform;

const ARROW_COLOUR: Color = Color::srgb(0.25, 0.9, 1.0);
const TARGET_COLOUR: Color = Color::srgb(1.0, 0.3, 0.85);
/// Just outside the corners of the cube, whose distance from any axis is about 2.12.
const ARROW_RADIUS: f32 = 2.4;
const ARROW_HEAD_LENGTH: f32 = 0.3;
const ARC_SEGMENTS: u32 = 24;

/// Adds the hint drawings.
pub struct HintViewPlugin;

impl Plugin for HintViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, configure_gizmos).add_systems(
            Update,
            draw_hints
                .in_set(GameSet::Present)
                .run_if(crate::screens::cube_controls_active),
        );
    }
}

fn configure_gizmos(mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<DefaultGizmoConfigGroup>();
    config.line.width = 5.0;
    // Draw over the cube, so that a target on the far side can still be found.
    config.depth_bias = -1.0;
}

fn draw_hints(
    mut gizmos: Gizmos,
    run: Res<Run>,
    state: Res<CubeState>,
    camera: Single<&GlobalTransform, With<Camera3d>>,
) {
    if let Some(turn) = run.arrow_turn() {
        draw_turn_arrow(&mut gizmos, turn, camera.translation());
    }
    if let Some(index) = run.target_piece()
        && let Some(&cubie) = state.cube.cubies().get(index)
    {
        let transform = cubie_transform(cubie, state.rotating_layer());
        gizmos.cube(
            Transform {
                scale: Vec3::splat(1.04),
                ..transform
            },
            TARGET_COLOUR,
        );
    }
}

/// Draws an arrow around a layer, on the side nearest the camera, in the direction it turns.
fn draw_turn_arrow(gizmos: &mut Gizmos, turn: Turn, camera: Vec3) {
    let axis = axis_vector(turn.axis);
    let coords: &[f32] = match turn.layer {
        Layer::Pos => &[1.0],
        Layer::Neg => &[-1.0],
        Layer::Mid => &[0.0],
        Layer::All => &[-1.0, 0.0, 1.0],
    };

    // Measure angles from the direction facing the camera, so that the arrow is on the near side.
    let towards_camera =
        (camera - axis * camera.dot(axis)).normalize_or(axis.any_orthonormal_vector());
    let along = axis.cross(towards_camera);
    let direction = f32::from(turn.quarters.signum());
    let half_span = if turn.quarters.abs() == 2 {
        FRAC_PI_2 * 0.9
    } else {
        FRAC_PI_4
    };

    for &coord in coords {
        let centre = axis * coord;
        let point = |angle: f32| {
            centre + (towards_camera * angle.cos() + along * angle.sin() * direction) * ARROW_RADIUS
        };
        gizmos.linestrip(
            (0..=ARC_SEGMENTS).map(|i| {
                let t = i as f32 / ARC_SEGMENTS as f32;
                point(-half_span + 2.0 * half_span * t)
            }),
            ARROW_COLOUR,
        );
        let tip = point(half_span);
        let back = point(half_span - ARROW_HEAD_LENGTH / ARROW_RADIUS);
        let shaft = (back - tip).normalize_or_zero();
        for side in [axis, -axis] {
            gizmos.line(
                tip,
                tip + (shaft + side * 0.7) * ARROW_HEAD_LENGTH,
                ARROW_COLOUR,
            );
        }
    }
}
