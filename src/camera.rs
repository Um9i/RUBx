//! An orbit camera that always looks at the cube.

use std::f32::consts::{FRAC_PI_2, FRAC_PI_8, PI, TAU};

use bevy::core_pipeline::tonemapping::{DebandDither, Tonemapping};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

use crate::screens::Screen;

const ORBIT_RADIANS_PER_PIXEL: f32 = 0.008;
const KEY_ORBIT_RADIANS_PER_SEC: f32 = 1.8;
/// Stopping just short of straight up or down avoids the view flipping at the pole.
const MAX_PITCH: f32 = FRAC_PI_2 - 0.05;
const MIN_DISTANCE: f32 = 5.5;
const MAX_DISTANCE: f32 = 16.0;
/// How fast the camera closes on its target, as a decay rate per second.
const SMOOTHING: f32 = 18.0;
/// How fast the cube slowly turns behind the menus, in radians per second.
const MENU_SPIN_RADIANS_PER_SEC: f32 = 0.25;
/// The width the cube needs on screen to stay clear of a side panel, in world units.
const CUBE_CLEARANCE: f32 = 5.8;

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

    /// Returns to the standard view, taking the short way round.
    fn reset_view(&mut self) {
        let standard = Self::default();
        self.yaw = standard.yaw;
        self.pitch = standard.pitch;
        self.distance = standard.distance;
        let offset = self.current.0 - standard.yaw;
        self.current.0 = standard.yaw + (offset + PI).rem_euclid(TAU) - PI;
    }

    /// Builds the camera transform. `shift` moves the camera sideways in its own frame.
    fn transform(yaw: f32, pitch: f32, distance: f32, shift: f32) -> Transform {
        let rotation = Quat::from_euler(EulerRot::YXZ, yaw, -pitch, 0.0);
        Transform {
            translation: rotation * Vec3::new(shift, 0.0, distance),
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
            (
                reset_on_screen_change,
                menu_spin,
                orbit_with_keys,
                follow_target,
            )
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
        OrbitCamera::transform(orbit.yaw, orbit.pitch, orbit.distance, 0.0),
        orbit,
    ));
}

fn reset_on_screen_change(screen: Res<State<Screen>>, mut camera: Single<&mut OrbitCamera>) {
    if !screen.is_changed() {
        return;
    }
    if screen.get().has_cube_controls() {
        camera.reset_view();
    } else {
        // Keep the slow spin going, but from a good angle.
        let standard = OrbitCamera::default();
        camera.pitch = standard.pitch;
        camera.distance = standard.distance;
    }
}

fn menu_spin(time: Res<Time>, screen: Res<State<Screen>>, mut camera: Single<&mut OrbitCamera>) {
    if !screen.get().has_cube_controls() {
        camera.yaw += MENU_SPIN_RADIANS_PER_SEC * time.delta_secs();
    }
}

fn orbit_with_keys(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    screen: Res<State<Screen>>,
    mut camera: Single<&mut OrbitCamera>,
) {
    if !screen.get().has_cube_controls() {
        return;
    }
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

fn follow_target(
    time: Res<Time>,
    screen: Res<State<Screen>>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&mut OrbitCamera, &mut Transform)>,
) {
    let (mut orbit, mut transform) = camera.into_inner();
    let blend = 1.0 - (-SMOOTHING * time.delta_secs()).exp();
    let (yaw, pitch, distance) = orbit.current;
    let current = (
        yaw + (orbit.yaw - yaw) * blend,
        pitch + (orbit.pitch - pitch) * blend,
        distance + (orbit.distance - distance) * blend,
    );
    orbit.current = current;
    let (distance, shift) = fit_beside_panel(
        current.2,
        window.width(),
        window.height(),
        screen.get().panel_width(),
    );
    *transform = OrbitCamera::transform(current.0, current.1, distance, shift);
}

/// Keeps the cube in the part of the window that a side panel leaves free.
///
/// Returns the camera distance to use, backing off if the cube wouldn't fit, and how far to move
/// the camera sideways so that the cube is centred in the free space.
fn fit_beside_panel(distance: f32, width: f32, height: f32, panel: f32) -> (f32, f32) {
    if height <= 0.0 {
        return (distance, 0.0);
    }
    let free_width = (width - panel).max(1.0);
    // The default vertical field of view is 45 degrees, so this is half its tangent doubled.
    let world_per_pixel_at_one = 2.0 * FRAC_PI_8.tan() / height;
    let needed = CUBE_CLEARANCE / (free_width * world_per_pixel_at_one);
    let distance = distance.max(needed);
    let shift = -(panel / 2.0) * world_per_pixel_at_one * distance;
    (distance, shift)
}
