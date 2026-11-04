//! The 3D cube: procedural meshes, materials, lighting, and placing cubies each frame.

use bevy::asset::RenderAssetUsages;
use bevy::light::GlobalAmbientLight;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

use crate::cube::{Cubie, Rotation};
use crate::drag::{CUBIE_SIZE, axis_vector};
use crate::game::{CubeState, GameSet, LayerAngle};

const BODY_HALF_SIZE: f32 = 0.495 * CUBIE_SIZE;
const BODY_CORNER_RADIUS: f32 = 0.07 * CUBIE_SIZE;
const STICKER_HALF_SIZE: f32 = 0.42 * CUBIE_SIZE;
const STICKER_CORNER_RADIUS: f32 = 0.09 * CUBIE_SIZE;
/// Raises stickers just enough above the body to avoid depth fighting.
const STICKER_LIFT: f32 = 0.0015 * CUBIE_SIZE;

const BACKGROUND: Color = Color::srgb(0.075, 0.08, 0.095);
const BODY_COLOUR: Color = Color::srgb(0.012, 0.012, 0.014);

/// The standard colour scheme, keyed by the outward normal of each solved face.
const FACE_COLOURS: [([i8; 3], Color); 6] = [
    ([1, 0, 0], Color::srgb(0.78, 0.07, 0.10)),  // R: red
    ([-1, 0, 0], Color::srgb(0.98, 0.42, 0.04)), // L: orange
    ([0, 1, 0], Color::srgb(0.84, 0.84, 0.82)),  // U: white
    ([0, -1, 0], Color::srgb(0.98, 0.82, 0.08)), // D: yellow
    ([0, 0, 1], Color::srgb(0.04, 0.60, 0.26)),  // F: green
    ([0, 0, -1], Color::srgb(0.05, 0.27, 0.72)), // B: blue
];

/// Marks the entity that displays the cubie at this index of [`Cube::cubies`].
#[derive(Component, Clone, Copy, Debug)]
struct CubieView(usize);

/// Adds the visible cube and its lighting.
pub struct ViewPlugin;

impl Plugin for ViewPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ClearColor(BACKGROUND))
            .insert_resource(GlobalAmbientLight {
                color: Color::WHITE,
                brightness: 650.0,
                ..default()
            })
            .add_systems(Startup, (spawn_lights, spawn_cube))
            .add_systems(Update, place_cubies.in_set(GameSet::Present));
    }
}

fn spawn_lights(mut commands: Commands) {
    commands.spawn((
        DirectionalLight {
            illuminance: 5200.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 9.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // A dim fill from below and behind keeps faces turned away from the key light readable.
    commands.spawn((
        DirectionalLight {
            illuminance: 1800.0,
            ..default()
        },
        Transform::from_xyz(-5.0, -6.0, -4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn spawn_cube(
    mut commands: Commands,
    state: Res<CubeState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let body_mesh = meshes.add(rounded_box(BODY_HALF_SIZE, BODY_CORNER_RADIUS, 4));
    let sticker_mesh = meshes.add(rounded_square(STICKER_HALF_SIZE, STICKER_CORNER_RADIUS, 6));
    let body_material = materials.add(StandardMaterial {
        base_color: BODY_COLOUR,
        // Mostly matte, so that the rounded corners don't catch the lights as bright dots.
        perceptual_roughness: 0.85,
        reflectance: 0.12,
        ..default()
    });
    let sticker_materials = FACE_COLOURS.map(|(normal, colour)| {
        let material = materials.add(StandardMaterial {
            base_color: colour,
            perceptual_roughness: 0.6,
            reflectance: 0.15,
            ..default()
        });
        (normal, material)
    });

    for (index, cubie) in state.cube.cubies().iter().enumerate() {
        commands
            .spawn((
                CubieView(index),
                Mesh3d(body_mesh.clone()),
                MeshMaterial3d(body_material.clone()),
                cubie_transform(*cubie, None),
                Visibility::default(),
            ))
            .with_children(|parent| {
                for normal in cubie.home_normals() {
                    let material = sticker_materials
                        .iter()
                        .find(|(face, _)| *face == normal)
                        .map(|(_, material)| material.clone());
                    let Some(material) = material else { continue };
                    parent.spawn((
                        Mesh3d(sticker_mesh.clone()),
                        MeshMaterial3d(material),
                        sticker_transform(normal),
                    ));
                }
            });
    }
}

/// Places a sticker, whose mesh lies in the XY plane facing `+Z`, on one side of its cubie.
fn sticker_transform(normal: [i8; 3]) -> Transform {
    let normal = int_vec(normal);
    Transform {
        translation: normal * (BODY_HALF_SIZE + STICKER_LIFT),
        rotation: Quat::from_rotation_arc(Vec3::Z, normal),
        ..default()
    }
}

fn place_cubies(state: Res<CubeState>, mut cubies: Query<(&CubieView, &mut Transform)>) {
    if !state.is_changed() {
        return;
    }
    let rotating = state.rotating_layer();
    for (view, mut transform) in &mut cubies {
        let cubie = state.cube.cubies()[view.0];
        *transform = cubie_transform(cubie, rotating);
    }
}

/// Computes a cubie's transform from its exact state, adding any in-progress layer rotation.
///
/// Rebuilding from the exact state each frame means animation errors can never accumulate.
pub(crate) fn cubie_transform(cubie: Cubie, rotating: Option<LayerAngle>) -> Transform {
    let position = cubie.position();
    let base = Transform {
        translation: int_vec(position) * CUBIE_SIZE,
        rotation: rotation_quat(cubie.rotation()),
        ..default()
    };
    match rotating {
        Some(LayerAngle { axis, layer, angle }) if layer.contains(position[axis.index()]) => {
            let spin = Quat::from_axis_angle(axis_vector(axis), angle);
            Transform {
                translation: spin * base.translation,
                rotation: spin * base.rotation,
                ..default()
            }
        }
        _ => base,
    }
}

fn rotation_quat(rotation: Rotation) -> Quat {
    let [x, y, z] = rotation.rows();
    let row = |row: [i8; 3]| int_vec(row);
    Quat::from_mat3(&Mat3::from_cols(row(x), row(y), row(z)).transpose()).normalize()
}

fn int_vec(v: [i8; 3]) -> Vec3 {
    Vec3::new(f32::from(v[0]), f32::from(v[1]), f32::from(v[2]))
}

/// Builds a cube of half-width `half` with rounded edges of radius `radius`.
///
/// Each face is a grid whose lines are concentrated near its border. Grid points are then pushed
/// out from the inner box by `radius`, which rounds the border and leaves the interior flat.
fn rounded_box(half: f32, radius: f32, segments: u32) -> Mesh {
    let inner = half - radius;
    let line = |i: u32| -> f32 {
        let step = radius / segments as f32;
        if i <= segments {
            -half + step * i as f32
        } else {
            inner + step * (i - segments - 1) as f32
        }
    };
    let lines = 2 * segments + 2;

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut indices = Vec::new();
    // Each face as (normal, u direction, v direction), chosen so that u × v = normal.
    let faces = [
        (Vec3::X, Vec3::NEG_Z, Vec3::Y),
        (Vec3::NEG_X, Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::X, Vec3::NEG_Z),
        (Vec3::NEG_Y, Vec3::X, Vec3::Z),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y),
    ];
    for (normal, u_dir, v_dir) in faces {
        let first = positions.len() as u32;
        for v in 0..lines {
            for u in 0..lines {
                let on_box = normal * half + u_dir * line(u) + v_dir * line(v);
                let core = on_box.clamp(Vec3::splat(-inner), Vec3::splat(inner));
                let outward = (on_box - core).normalize_or(normal);
                positions.push((core + outward * radius).to_array());
                normals.push(outward.to_array());
            }
        }
        for v in 0..lines - 1 {
            for u in 0..lines - 1 {
                let a = first + v * lines + u;
                let (b, c, d) = (a + 1, a + lines, a + lines + 1);
                indices.extend([a, b, d, a, d, c]);
            }
        }
    }
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
}

/// Builds a flat rounded square in the XY plane facing `+Z`, as a triangle fan.
fn rounded_square(half: f32, radius: f32, corner_segments: u32) -> Mesh {
    let inner = half - radius;
    let mut positions = vec![[0.0, 0.0, 0.0]];
    let corners = [(1.0, 1.0), (-1.0, 1.0), (-1.0, -1.0), (1.0, -1.0)];
    for (quadrant, (sign_x, sign_y)) in corners.into_iter().enumerate() {
        for step in 0..=corner_segments {
            let angle = (quadrant as f32 + step as f32 / corner_segments as f32)
                * std::f32::consts::FRAC_PI_2;
            positions.push([
                sign_x * inner + radius * angle.cos(),
                sign_y * inner + radius * angle.sin(),
                0.0,
            ]);
        }
    }
    let rim = positions.len() as u32 - 1;
    let indices = (0..rim)
        .flat_map(|i| [0, i + 1, (i + 1) % rim + 1])
        .collect();
    let normals = vec![[0.0, 0.0, 1.0]; positions.len()];
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_indices(Indices::U32(indices))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cube::{Cube, Turn, parse_sequence};

    /// Asserts that the floating point transform agrees with the integer model for a sticker.
    fn assert_sticker_matches_model(cube: &Cube) {
        for cubie in cube.cubies() {
            let transform = cubie_transform(*cubie, None);
            assert!(transform.translation.distance(int_vec(cubie.position())) < 1e-5);
            for normal in cubie.home_normals() {
                let shown = transform.rotation * int_vec(normal);
                let exact = int_vec(cubie.rotation().apply(normal));
                assert!(shown.distance(exact) < 1e-5, "{shown} != {exact}");
            }
        }
    }

    #[test]
    fn rotation_quat_should_match_integer_rotations_after_any_sequence() {
        let mut cube = Cube::solved();
        cube.apply_all(parse_sequence("R U2 F' L D B2 M E' S x y' z2").unwrap());
        assert_sticker_matches_model(&cube);
    }

    #[test]
    fn finished_animation_angle_should_land_exactly_on_the_turned_state() {
        let turn: Turn = "R".parse().unwrap();
        let before = Cube::solved();
        let mut after = before.clone();
        after.apply(turn);
        let mid_flight = LayerAngle {
            axis: turn.axis,
            layer: turn.layer,
            angle: f32::from(turn.quarters) * std::f32::consts::FRAC_PI_2,
        };
        for (old, new) in before.cubies().iter().zip(after.cubies()) {
            let animated = cubie_transform(*old, Some(mid_flight));
            let resting = cubie_transform(*new, None);
            assert!(animated.translation.distance(resting.translation) < 1e-5);
            assert!(animated.rotation.angle_between(resting.rotation) < 1e-3);
        }
    }

    /// Returns the geometric normal of each triangle dotted with its first vertex normal.
    fn triangle_facing(mesh: &Mesh) -> Vec<f32> {
        let Some(positions) = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|values| values.as_float3())
        else {
            panic!("mesh has no positions");
        };
        let Some(bevy::mesh::VertexAttributeValues::Float32x3(normals)) =
            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
        else {
            panic!("mesh has no normals");
        };
        let indices: Vec<usize> = mesh.indices().expect("mesh has indices").iter().collect();
        indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|tri| {
                let [a, b, c] = tri.map(|i| Vec3::from(positions[i]));
                (b - a).cross(c - a).dot(Vec3::from(normals[tri[0]]))
            })
            .collect()
    }

    // Wrongly wound triangles get culled, which would leave holes in the cube.
    #[test]
    fn rounded_box_triangles_should_face_outwards() {
        let facing = triangle_facing(&rounded_box(0.5, 0.1, 3));
        assert!(facing.iter().all(|&dot| dot >= 0.0));
    }

    #[test]
    fn rounded_square_triangles_should_face_outwards() {
        let facing = triangle_facing(&rounded_square(0.4, 0.1, 4));
        assert!(facing.iter().all(|&dot| dot >= 0.0));
    }
}
