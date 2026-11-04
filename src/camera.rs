//! An orbit camera that always looks at the cube.

use std::f32::consts::FRAC_PI_2;

use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::prelude::*;

const ORBIT_RADIANS_PER_PIXEL: f32 = 0.008;
const KEY_ORBIT_RADIANS_PER_SEC: f32 = 1.8;
/// Stopping just short of straight up or down avoids the view flipping at the pole.
const MAX_PITCH: f32 = FRAC_PI_2 - 0.05;
const MIN_DISTANCE: f32 = 5.5;
const MAX_DISTANCE: f32 = 16.0;
/// How fast the camera closes on its target, as a decay rate per second.
const SMOOTHING: f32 = 18.0;

/// Where the camera is heading, and where it is on the way there.
#[derive(Component, Clone, Copy, Debug)]
pub struct OrbitCamera {
    /// The target rotation about the world Y axis.
    pub yaw: f32,
    /// The target height angle, positive when looking down from above.
    pub pitch: f32,
    /// The target distance from the cube.
    pub distance: f32,
    current: (f32, f32, f32),
}

impl Default for OrbitCamera {
    fn default() -> Self {
        // A classic three-quarter view showing the U, F, and R faces.
        let (yaw, pitch, distance) = (0.62, 0.55, 9.0);
        Self {
            yaw,
            pitch,
            distance,
            current: (yaw, pitch, distance),
        }
    }
}

impl OrbitCamera {
    /// Orbits by a cursor movement in pixels.
    pub fn orbit_pixels(&mut self, delta: Vec2) {
        self.yaw -= delta.x * ORBIT_RADIANS_PER_PIXEL;
        self.pitch = (self.pitch + delta.y * ORBIT_RADIANS_PER_PIXEL).clamp(-MAX_PITCH, MAX_PITCH);
    }

    /// Moves closer by a number of scroll steps.
    pub fn zoom_steps(&mut self, steps: f32) {
        self.distance = (self.distance * 0.92_f32.powf(steps)).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }

    fn transform(yaw: f32, pitch: f32, distance: f32) -> Transform {
        let rotation = Quat::from_euler(EulerRot::YXZ, yaw, -pitch, 0.0);
        Transform {
            translation: rotation * Vec3::new(0.0, 0.0, distance),
            rotation,
            ..default()
        }
    }
}

/// Adds the camera and its movement.
pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera).add_systems(
            Update,
            (orbit_with_keys, follow_target)
                .chain()
                .in_set(crate::game::GameSet::Present),
        );
    }
}

fn spawn_camera(mut commands: Commands) {
    let orbit = OrbitCamera::default();
    commands.spawn((
        Camera3d::default(),
        // Sticker colours should look like the real thing, not a filmic rendering of it.
        Tonemapping::None,
        DebandDither::Disabled,
        OrbitCamera::transform(orbit.yaw, orbit.pitch, orbit.distance),
        orbit,
    ));
}

fn orbit_with_keys(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut camera: Single<&mut OrbitCamera>,
) {
    let axis = |positive, negative| {
        f32::from(u8::from(keys.pressed(positive))) - f32::from(u8::from(keys.pressed(negative)))
    };
    let step = KEY_ORBIT_RADIANS_PER_SEC * time.delta_secs();
    let yaw = axis(KeyCode::ArrowRight, KeyCode::ArrowLeft);
    let pitch = axis(KeyCode::ArrowUp, KeyCode::ArrowDown);
    if yaw != 0.0 || pitch != 0.0 {
        camera.orbit_pixels(Vec2::new(yaw, pitch) * step / ORBIT_RADIANS_PER_PIXEL);
    }
}

fn follow_target(time: Res<Time>, camera: Single<(&mut OrbitCamera, &mut Transform)>) {
    let (mut orbit, mut transform) = camera.into_inner();
    let blend = 1.0 - (-SMOOTHING * time.delta_secs()).exp();
    let (yaw, pitch, distance) = orbit.current;
    let current = (
        yaw + (orbit.yaw - yaw) * blend,
        pitch + (orbit.pitch - pitch) * blend,
        distance + (orbit.distance - distance) * blend,
    );
    orbit.current = current;
    *transform = OrbitCamera::transform(current.0, current.1, current.2);
}
