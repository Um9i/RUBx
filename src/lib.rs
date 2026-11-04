//! RUBx: a Rubik's cube simulator built with Bevy.
//!
//! - [`cube`] is the pure puzzle model and has no Bevy dependency.
//! - [`game`] holds the rules: the turn queue, undo history, and solve timer. It runs headless.
//! - [`drag`] is the geometry behind turning a layer with the mouse.
//! - [`view`], [`camera`], [`input`], and [`hud`] connect the game to the window.

pub mod camera;
pub mod cube;
pub mod drag;
pub mod game;
pub mod hud;
pub mod input;
pub mod view;

use bevy::prelude::*;

/// Adds the complete interactive game to an app that already has [`DefaultPlugins`].
pub struct RubxPlugin;

impl Plugin for RubxPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            game::GamePlugin,
            view::ViewPlugin,
            camera::CameraPlugin,
            input::InputPlugin,
            hud::HudPlugin,
        ));
    }
}
