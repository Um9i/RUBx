//! RUBx: a Rubik's cube simulator built with Bevy.
//!
//! - [`cube`] is the pure puzzle model and has no Bevy dependency.
//! - [`method`], [`solver`], [`algs`], [`setups`], [`lessons`], [`drills`], and [`progress`] are
//!   the knowledge behind Training, also free of rendering.
//! - [`game`] holds the rules: the turn queue, undo history, and solve timer. It runs headless.
//! - [`screens`] and [`training`] run the two modes. They also run headless.
//! - [`drag`] is the geometry behind turning a layer with the mouse.
//! - [`view`], [`camera`], [`input`], [`hud`], [`menus`], and [`hint_view`] connect it all to the
//!   window.

pub mod algs;
pub mod camera;
pub mod cube;
pub mod drag;
pub mod drills;
pub mod game;
pub mod hint_view;
pub mod hud;
pub mod input;
pub mod lessons;
pub mod menus;
pub mod method;
pub mod progress;
pub mod screens;
pub mod setups;
pub mod solver;
pub mod training;
pub mod view;

use bevy::prelude::*;

/// Adds the complete interactive game to an app that already has [`DefaultPlugins`].
pub struct RubxPlugin;

impl Plugin for RubxPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            game::GamePlugin,
            screens::ScreenPlugin,
            training::TrainingPlugin,
            view::ViewPlugin,
            camera::CameraPlugin,
            input::InputPlugin,
            hud::HudPlugin,
            menus::MenuPlugin,
            hint_view::HintViewPlugin,
        ));
    }
}
