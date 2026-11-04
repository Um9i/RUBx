//! Shared UI: the look of panels and buttons, the timer, and Play mode's controls.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::Pressed;
use bevy::ui_widgets::{Activate, Button, observe};

use crate::game::{GameCommand, GameSet, Session, SolvePhase};
use crate::screens::{Screen, UiAction, cube_controls_active};

pub(crate) const TEXT: Color = Color::srgb(0.92, 0.93, 0.95);
pub(crate) const DIM_TEXT: Color = Color::srgb(0.6, 0.63, 0.69);
pub(crate) const ACCENT: Color = Color::srgb(0.36, 0.86, 0.52);
pub(crate) const HINT: Color = Color::srgb(0.25, 0.9, 1.0);
const BUTTON: Color = Color::srgb(0.145, 0.155, 0.185);
const BUTTON_HOVER: Color = Color::srgb(0.2, 0.215, 0.255);
const BUTTON_PRESS: Color = Color::srgb(0.27, 0.29, 0.34);

const CONTROLS_HELP: &str = "\
Drag cube: turn a layer
Drag background or right-drag: orbit
Scroll: zoom
R L U D F B M E S: turn (Shift reverses)
X Y Z: rotate whole cube
Arrows: orbit";

/// Whether the cursor is over the UI, in which case clicks shouldn't reach the cube.
#[derive(Resource, Default)]
pub struct PointerOverHud(pub bool);

#[derive(Component)]
struct TimerText;

#[derive(Component)]
struct DetailText;

type TimerQuery<'w, 's> = Single<
    'w,
    's,
    (&'static mut Text, &'static mut TextColor),
    (With<TimerText>, Without<DetailText>),
>;

/// Adds the shared UI and Play mode's controls.
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PointerOverHud>()
            .add_systems(OnEnter(Screen::Play), spawn_play_controls)
            .add_systems(Update, track_pointer_over_hud.in_set(GameSet::Input))
            .add_systems(
                Update,
                (update_status.run_if(cube_controls_active), style_buttons)
                    .in_set(GameSet::Present),
            );
    }
}

/// A plain text node.
pub(crate) fn label(text: impl Into<String>, size: f32, colour: Color) -> impl Bundle {
    (
        Text::new(text),
        TextFont::from_font_size(size),
        TextColor(colour),
    )
}

/// A button that performs an action, with a small note below its label.
pub(crate) fn action_button(
    text: impl Into<String>,
    note: impl Into<String>,
    action: UiAction,
) -> impl Bundle {
    (
        Button,
        Hovered::default(),
        Node {
            min_width: px(86),
            padding: UiRect::axes(px(14), px(9)),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            border_radius: BorderRadius::all(px(7)),
            ..default()
        },
        BackgroundColor(BUTTON),
        observe(
            move |_activate: On<Activate>, mut actions: MessageWriter<UiAction>| {
                actions.write(action);
            },
        ),
        children![label(text, 15.0, TEXT), label(note, 11.0, DIM_TEXT)],
    )
}

/// The timer and status line.
pub(crate) fn status_display() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            row_gap: px(2),
            ..default()
        },
        children![
            (TimerText, label("0:00.00", 44.0, TEXT)),
            (DetailText, label("", 16.0, DIM_TEXT)),
        ],
    )
}

fn spawn_play_controls(mut commands: Commands) {
    let scope = DespawnOnExit(Screen::Play);
    commands.spawn((
        scope.clone(),
        Node {
            position_type: PositionType::Absolute,
            top: px(24),
            left: px(28),
            ..default()
        },
        children![status_display()],
    ));
    commands.spawn((
        scope.clone(),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(24),
            left: px(28),
            ..default()
        },
        children![label(CONTROLS_HELP, 13.0, DIM_TEXT)],
    ));
    commands.spawn((
        scope,
        Node {
            position_type: PositionType::Absolute,
            bottom: px(24),
            right: px(28),
            column_gap: px(8),
            ..default()
        },
        children![
            action_button("Scramble", "Space", UiAction::Game(GameCommand::Scramble)),
            action_button("Undo", "Ctrl+Z", UiAction::Game(GameCommand::Undo)),
            action_button("Redo", "Ctrl+Y", UiAction::Game(GameCommand::Redo)),
            action_button("Reset", "Ctrl+R", UiAction::Game(GameCommand::Reset)),
            action_button("Menu", "Esc", UiAction::Back),
        ],
    ));
}

fn track_pointer_over_hud(hovered: Query<&Hovered>, mut over_hud: ResMut<PointerOverHud>) {
    let hovered = hovered.iter().any(|hovered| hovered.0);
    if over_hud.0 != hovered {
        over_hud.0 = hovered;
    }
}

fn style_buttons(mut buttons: Query<(&Hovered, Has<Pressed>, &mut BackgroundColor), With<Button>>) {
    for (hovered, pressed, mut background) in &mut buttons {
        let colour = if pressed {
            BUTTON_PRESS
        } else if hovered.0 {
            BUTTON_HOVER
        } else {
            BUTTON
        };
        if background.0 != colour {
            background.0 = colour;
        }
    }
}

fn update_status(
    session: Res<Session>,
    screen: Res<State<Screen>>,
    timer: TimerQuery,
    mut detail: Single<&mut Text, (With<DetailText>, Without<TimerText>)>,
) {
    let (mut timer_text, mut timer_colour) = timer.into_inner();
    let moves = moves_text(session.move_count());
    let (time, colour, status) = match session.phase {
        SolvePhase::Free if *screen.get() == Screen::Play => {
            (0.0, DIM_TEXT, format!("Free play - {moves}"))
        }
        SolvePhase::Free => (0.0, DIM_TEXT, moves),
        SolvePhase::Scrambling => (0.0, DIM_TEXT, "Scrambling...".to_owned()),
        SolvePhase::Ready => (0.0, TEXT, "Your first move starts the clock".to_owned()),
        SolvePhase::Solving { secs } => (secs, TEXT, moves),
        SolvePhase::Solved { secs, moves } => {
            (secs, ACCENT, format!("Done in {}!", moves_text(moves)))
        }
    };
    let status = match session.best_secs {
        Some(best) => format!("{status} - best {}", format_time(best)),
        None => status,
    };
    let time = format_time(time);
    if timer_text.0 != time {
        timer_text.0 = time;
    }
    if timer_colour.0 != colour {
        timer_colour.0 = colour;
    }
    if detail.0 != status {
        detail.0 = status;
    }
}

/// Formats a number of moves, such as `1 move` or `5 moves`.
pub(crate) fn moves_text(moves: usize) -> String {
    if moves == 1 {
        "1 move".to_owned()
    } else {
        format!("{moves} moves")
    }
}

/// Formats seconds as `m:ss.cc`.
pub(crate) fn format_time(secs: f32) -> String {
    let centis = (secs * 100.0).floor() as u32;
    format!(
        "{}:{:02}.{:02}",
        centis / 6000,
        centis / 100 % 60,
        centis % 100
    )
}

#[cfg(test)]
mod tests {
    use super::{format_time, moves_text};

    #[test]
    fn moves_text_should_use_the_singular_for_one_move() {
        assert_eq!(moves_text(1), "1 move");
    }

    #[test]
    fn moves_text_should_use_the_plural_otherwise() {
        assert_eq!([moves_text(0), moves_text(2)], ["0 moves", "2 moves"]);
    }

    #[test]
    fn format_time_should_show_minutes_seconds_and_centiseconds() {
        assert_eq!(format_time(83.456), "1:23.45");
    }

    #[test]
    fn format_time_should_pad_seconds() {
        assert_eq!(format_time(5.0), "0:05.00");
    }
}
