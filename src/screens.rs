//! The screens of the game and the actions that move between them.

use bevy::prelude::*;

use crate::drills::DrillSet;
use crate::game::{CubeState, GameCommand, GameSet};
use crate::setups::PracticeStage;

/// Where the player is in the game.
#[derive(States, Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Screen {
    /// Choose between Training and Play.
    #[default]
    Menu,
    /// Play mode: scramble and solve against the clock.
    Play,
    /// Training mode's four activities.
    Hub,
    /// Choose a lesson.
    LessonList,
    /// Work through a lesson.
    Lesson,
    /// Solve a scramble with hints on request.
    Coach,
    /// Choose an algorithm set.
    DrillList,
    /// Drill algorithm cases.
    Drill,
    /// Choose a solve stage to practise.
    PracticeList,
    /// Practise one stage of a solve.
    Practice,
}

/// The width of the panel beside the cube on training and menu screens, in logical pixels.
pub const PANEL_WIDTH: f32 = 400.0;

impl Screen {
    /// Returns whether the player can turn the cube and move the camera on this screen.
    pub const fn has_cube_controls(self) -> bool {
        matches!(
            self,
            Screen::Play | Screen::Lesson | Screen::Coach | Screen::Drill | Screen::Practice
        )
    }

    /// Returns the width of this screen's side panel, in logical pixels.
    pub const fn panel_width(self) -> f32 {
        match self {
            Screen::Play => 0.0,
            _ => PANEL_WIDTH,
        }
    }

    /// Returns the screen that Back goes to.
    pub const fn parent(self) -> Option<Screen> {
        match self {
            Screen::Menu => None,
            Screen::Play | Screen::Hub => Some(Screen::Menu),
            Screen::LessonList | Screen::Coach | Screen::DrillList | Screen::PracticeList => {
                Some(Screen::Hub)
            }
            Screen::Lesson => Some(Screen::LessonList),
            Screen::Drill => Some(Screen::DrillList),
            Screen::Practice => Some(Screen::PracticeList),
        }
    }
}

/// Runs a system only on screens where the player can use the cube.
pub fn cube_controls_active(screen: Res<State<Screen>>) -> bool {
    screen.get().has_cube_controls()
}

/// Something the player asked for through a button or a shortcut.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub enum UiAction {
    /// Go directly to a screen.
    Go(Screen),
    /// Go to the screen above this one.
    Back,
    /// Begin a lesson.
    StartLesson(usize),
    /// Begin solving a scramble with the coach.
    StartCoach,
    /// Begin drilling a set of algorithms.
    StartDrill(DrillSet),
    /// Begin practising a stage of a solve.
    StartPractice(PracticeStage),
    /// Show or hide the next move.
    Hint,
    /// Make the next moves on the player's behalf.
    ShowMe,
    /// Move on to a new position.
    Next,
    /// Try the current position again.
    Restart,
    /// Show the algorithm for the current case.
    Reveal,
    /// Send a command to the game.
    Game(GameCommand),
    /// Leave the game.
    Quit,
}

/// Contains the system that moves between screens.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NavigateSet;

/// Adds the screens and their navigation.
pub struct ScreenPlugin;

impl Plugin for ScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<Screen>()
            .add_message::<UiAction>()
            .add_systems(Update, navigate.in_set(GameSet::Input).in_set(NavigateSet));
    }
}

fn navigate(
    mut actions: MessageReader<UiAction>,
    screen: Res<State<Screen>>,
    state: Res<CubeState>,
    mut next: ResMut<NextState<Screen>>,
    mut commands: MessageWriter<GameCommand>,
    mut exit: MessageWriter<AppExit>,
) {
    for action in actions.read() {
        match *action {
            UiAction::Go(target) => next.set(target),
            UiAction::Back => {
                if let Some(parent) = screen.get().parent() {
                    next.set(parent);
                }
            }
            UiAction::Game(command) if !state.locked => {
                commands.write(command);
            }
            UiAction::Quit => {
                exit.write(AppExit::Success);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_screen_except_the_menu_should_lead_back_to_the_menu() {
        let screens = [
            Screen::Play,
            Screen::Hub,
            Screen::LessonList,
            Screen::Lesson,
            Screen::Coach,
            Screen::DrillList,
            Screen::Drill,
            Screen::PracticeList,
            Screen::Practice,
        ];
        for start in screens {
            let mut at = start;
            for _ in 0..4 {
                if let Some(parent) = at.parent() {
                    at = parent;
                }
            }
            assert_eq!(at, Screen::Menu, "{start:?}");
        }
    }

    #[test]
    fn play_should_not_have_a_side_panel() {
        assert_eq!(Screen::Play.panel_width(), 0.0);
    }
}
