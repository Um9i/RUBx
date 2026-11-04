//! The on-screen display: timer, move count, buttons, and control hints.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui::Pressed;
use bevy::ui_widgets::{Activate, Button, observe};

use crate::game::{GameCommand, GameSet, Session, SolvePhase};

const TEXT: Color = Color::srgb(0.92, 0.93, 0.95);
const DIM_TEXT: Color = Color::srgb(0.56, 0.59, 0.65);
const ACCENT: Color = Color::srgb(0.36, 0.86, 0.52);
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

/// Whether the cursor is over a HUD button, in which case clicks shouldn't reach the cube.
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

/// Adds the HUD.
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PointerOverHud>()
            .add_systems(Startup, spawn_hud)
            .add_systems(Update, track_pointer_over_hud.in_set(GameSet::Input))
            .add_systems(
                Update,
                (update_status, style_buttons).in_set(GameSet::Present),
            );
    }
}

fn spawn_hud(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(24),
            left: px(28),
            flex_direction: FlexDirection::Column,
            row_gap: px(2),
            ..default()
        },
        children![
            (
                TimerText,
                Text::new("0:00.00"),
                TextFont::from_font_size(44.0),
                TextColor(TEXT),
            ),
            (
                DetailText,
                Text::new(""),
                TextFont::from_font_size(16.0),
                TextColor(DIM_TEXT),
            ),
        ],
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(24),
            left: px(28),
            ..default()
        },
        children![(
            Text::new(CONTROLS_HELP),
            TextFont::from_font_size(13.0),
            TextColor(DIM_TEXT),
        )],
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(24),
            right: px(28),
            column_gap: px(8),
            ..default()
        },
        children![
            command_button("Scramble", "Space", GameCommand::Scramble),
            command_button("Undo", "Ctrl+Z", GameCommand::Undo),
            command_button("Redo", "Ctrl+Y", GameCommand::Redo),
            command_button("Reset", "Ctrl+R", GameCommand::Reset),
        ],
    ));
}

fn command_button(
    label: &'static str,
    shortcut: &'static str,
    command: GameCommand,
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
            move |_activate: On<Activate>, mut commands: MessageWriter<GameCommand>| {
                commands.write(command);
            },
        ),
        children![
            (
                Text::new(label),
                TextFont::from_font_size(15.0),
                TextColor(TEXT),
            ),
            (
                Text::new(shortcut),
                TextFont::from_font_size(11.0),
                TextColor(DIM_TEXT),
            ),
        ],
    )
}

fn track_pointer_over_hud(
    buttons: Query<&Hovered, With<Button>>,
    mut over_hud: ResMut<PointerOverHud>,
) {
    let hovered = buttons.iter().any(|hovered| hovered.0);
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
    timer: TimerQuery,
    mut detail: Single<&mut Text, (With<DetailText>, Without<TimerText>)>,
) {
    let (mut timer_text, mut timer_colour) = timer.into_inner();
    let moves = session.move_count();
    let (time, colour, status) = match session.phase {
        SolvePhase::Free => (0.0, DIM_TEXT, format!("Free play - {moves} moves")),
        SolvePhase::Scrambling => (0.0, DIM_TEXT, "Scrambling...".to_owned()),
        SolvePhase::Ready => (
            0.0,
            TEXT,
            "Scrambled. Your first move starts the clock".to_owned(),
        ),
        SolvePhase::Solving { secs } => (secs, TEXT, format!("{moves} moves")),
        SolvePhase::Solved { secs, moves } => (secs, ACCENT, format!("Solved in {moves} moves!")),
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

/// Formats seconds as `m:ss.cc`.
fn format_time(secs: f32) -> String {
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
    use super::format_time;

    #[test]
    fn format_time_should_show_minutes_seconds_and_centiseconds() {
        assert_eq!(format_time(83.456), "1:23.45");
    }

    #[test]
    fn format_time_should_pad_seconds() {
        assert_eq!(format_time(5.0), "0:05.00");
    }
}
