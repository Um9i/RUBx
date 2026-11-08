//! The menus, and the side panel beside the cube during Training.

use bevy::picking::hover::Hovered;
use bevy::prelude::*;

use crate::drills::{DrillSet, stats_key};
use crate::game::CubeState;
use crate::game::{GameCommand, GameSet};
use crate::hud::{
    ACCENT, DIM_TEXT, HINT, TEXT, action_button, format_time, label, moves_text, status_display,
};
use crate::lessons::{Exercise, LESSONS};
use crate::method::{MethodView, Stage};
use crate::progress::{Progress, Stats};
use crate::screens::{PANEL_WIDTH, Screen, UiAction};
use crate::setups::PracticeStage;
use crate::training::{Activity, Run, practice_key};

const PANEL_BACKGROUND: Color = Color::srgba(0.05, 0.055, 0.07, 0.92);

/// Adds the menus and training panels.
pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(Screen::Menu), spawn_main_menu)
            .add_systems(OnEnter(Screen::Hub), spawn_hub)
            .add_systems(OnEnter(Screen::LessonList), spawn_lesson_list)
            .add_systems(OnEnter(Screen::DrillList), spawn_drill_list)
            .add_systems(OnEnter(Screen::PracticeList), spawn_practice_list)
            .add_systems(OnEnter(Screen::Lesson), spawn_lesson_panel)
            .add_systems(OnEnter(Screen::Coach), spawn_coach_panel)
            .add_systems(OnEnter(Screen::Drill), spawn_drill_panel)
            .add_systems(OnEnter(Screen::Practice), spawn_practice_panel)
            .add_systems(
                Update,
                (
                    ui_shortcuts.in_set(GameSet::Input),
                    update_panel_text.in_set(GameSet::Present),
                ),
            );
    }
}

fn ui_shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    screen: Res<State<Screen>>,
    mut actions: MessageWriter<UiAction>,
) {
    if keys.just_pressed(KeyCode::Escape) && screen.get().parent().is_some() {
        actions.write(UiAction::Back);
    }
    if !screen.get().has_cube_controls() || *screen.get() == Screen::Play {
        return;
    }
    let shortcuts = [
        (KeyCode::KeyH, UiAction::Hint),
        (KeyCode::KeyJ, UiAction::ShowMe),
        (KeyCode::KeyN, UiAction::Next),
        (KeyCode::KeyG, UiAction::Restart),
        (KeyCode::KeyV, UiAction::Reveal),
    ];
    for (key, action) in shortcuts {
        if keys.just_pressed(key) {
            actions.write(action);
        }
    }
}

/// The column every menu and panel is laid out in.
fn panel(screen: Screen) -> impl Bundle {
    (
        DespawnOnExit(screen),
        // Marks the panel as UI, so that clicks on it don't reach the cube behind.
        Hovered::default(),
        Node {
            position_type: PositionType::Absolute,
            top: px(0),
            bottom: px(0),
            left: px(0),
            width: px(PANEL_WIDTH),
            padding: UiRect::all(px(24)),
            flex_direction: FlexDirection::Column,
            row_gap: px(12),
            overflow: Overflow::clip(),
            ..default()
        },
        BackgroundColor(PANEL_BACKGROUND),
    )
}

/// A wide menu entry with a description.
fn menu_entry(text: impl Into<String>, note: impl Into<String>, action: UiAction) -> impl Bundle {
    action_button(text, note, action)
}

fn spawn_menu(
    commands: &mut Commands,
    screen: Screen,
    title: &str,
    subtitle: &str,
    entries: Vec<(String, String, UiAction)>,
) {
    commands.spawn(panel(screen)).with_children(|parent| {
        parent.spawn(label(title, 36.0, TEXT));
        parent.spawn((
            label(subtitle, 15.0, DIM_TEXT),
            Node {
                margin: UiRect::bottom(px(12)),
                ..default()
            },
        ));
        for (text, note, action) in entries {
            parent.spawn(menu_entry(text, note, action));
        }
    });
}

fn spawn_main_menu(mut commands: Commands) {
    spawn_menu(
        &mut commands,
        Screen::Menu,
        "RUBx",
        "Learn it. Drill it. Race it.",
        vec![
            (
                "Training".to_owned(),
                "Learn to solve and build your skills".to_owned(),
                UiAction::Go(Screen::Hub),
            ),
            (
                "Play".to_owned(),
                "Scramble and solve against the clock".to_owned(),
                UiAction::Go(Screen::Play),
            ),
            ("Quit".to_owned(), String::new(), UiAction::Quit),
        ],
    );
}

fn spawn_hub(mut commands: Commands, progress: Res<Progress>) {
    let lessons_done = LESSONS
        .iter()
        .filter(|lesson| progress.lesson_done(lesson.id))
        .count();
    let coach = progress.stats("coach");
    spawn_menu(
        &mut commands,
        Screen::Hub,
        "Training",
        "Choose what to work on.",
        vec![
            (
                "Learn to solve".to_owned(),
                format!(
                    "Step-by-step lessons - {lessons_done} of {} done",
                    LESSONS.len()
                ),
                UiAction::Go(Screen::LessonList),
            ),
            (
                "Coach".to_owned(),
                format!("Solve a scramble with hints - {} solves", coach.attempts),
                UiAction::StartCoach,
            ),
            (
                "Algorithm drills".to_owned(),
                "Master OLL and PLL cases".to_owned(),
                UiAction::Go(Screen::DrillList),
            ),
            (
                "Stage practice".to_owned(),
                "Time one part of a solve".to_owned(),
                UiAction::Go(Screen::PracticeList),
            ),
            ("Back".to_owned(), "Esc".to_owned(), UiAction::Back),
        ],
    );
}

fn spawn_lesson_list(mut commands: Commands, progress: Res<Progress>) {
    let mut entries: Vec<_> = LESSONS
        .iter()
        .enumerate()
        .map(|(index, lesson)| {
            let note = if progress.lesson_done(lesson.id) {
                "Done"
            } else {
                "Not done yet"
            };
            (
                format!("{}. {}", index + 1, lesson.title),
                note.to_owned(),
                UiAction::StartLesson(index),
            )
        })
        .collect();
    entries.push(("Back".to_owned(), "Esc".to_owned(), UiAction::Back));
    spawn_menu(
        &mut commands,
        Screen::LessonList,
        "Learn to solve",
        "The beginner method, one stage at a time.",
        entries,
    );
}

fn spawn_drill_list(mut commands: Commands, progress: Res<Progress>) {
    let mut entries: Vec<_> = DrillSet::ALL
        .iter()
        .map(|&set| {
            let cases = set.cases();
            let seen = cases
                .iter()
                .filter(|case| progress.stats(&stats_key(case)).attempts > 0)
                .count();
            (
                set.name().to_owned(),
                format!("{} - tried {seen} of {}", set.description(), cases.len()),
                UiAction::StartDrill(set),
            )
        })
        .collect();
    entries.push(("Back".to_owned(), "Esc".to_owned(), UiAction::Back));
    spawn_menu(
        &mut commands,
        Screen::DrillList,
        "Algorithm drills",
        "Weak cases come up more often. Your results are saved.",
        entries,
    );
}

fn spawn_practice_list(mut commands: Commands, progress: Res<Progress>) {
    let mut entries: Vec<_> = PracticeStage::ALL
        .iter()
        .map(|&stage| {
            let stats = progress.stats(&practice_key(stage));
            (
                stage.name().to_owned(),
                stats_line(stats),
                UiAction::StartPractice(stage),
            )
        })
        .collect();
    entries.push(("Back".to_owned(), "Esc".to_owned(), UiAction::Back));
    spawn_menu(
        &mut commands,
        Screen::PracticeList,
        "Stage practice",
        "Set up one part of a solve and time just that part.",
        entries,
    );
}

/// Summarises results in one line.
fn stats_line(stats: Stats) -> String {
    match (stats.best_secs, stats.average_secs()) {
        (Some(best), Some(average)) => format!(
            "{} done - best {} - average {}",
            stats.attempts,
            format_time(best),
            format_time(average)
        ),
        _ if stats.attempts > 0 => format!("{} done - no unaided times yet", stats.attempts),
        _ => "Not tried yet".to_owned(),
    }
}

/// The parts of a training panel that change as the player works.
#[derive(Component, Clone, Copy, Debug)]
enum PanelText {
    Title,
    Body,
    Hint,
    Result,
}

fn spawn_training_panel(
    commands: &mut Commands,
    screen: Screen,
    buttons: Vec<(&str, &str, UiAction)>,
) {
    commands.spawn(panel(screen)).with_children(|parent| {
        parent.spawn(status_display());
        parent.spawn((PanelText::Title, label("", 22.0, TEXT)));
        parent.spawn((PanelText::Body, label("", 14.0, TEXT)));
        parent.spawn((PanelText::Hint, label("", 15.0, HINT)));
        parent.spawn((PanelText::Result, label("", 15.0, ACCENT)));
        parent
            .spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(6),
                row_gap: px(6),
                margin: UiRect::top(px(4)),
                ..default()
            })
            .with_children(|row| {
                for (text, note, action) in buttons {
                    row.spawn(action_button(text, note, action));
                }
            });
    });
}

const UNDO: (&str, &str, UiAction) = ("Undo", "Backspace", UiAction::Game(GameCommand::Undo));
const BACK: (&str, &str, UiAction) = ("Back", "Esc", UiAction::Back);

fn spawn_lesson_panel(mut commands: Commands) {
    spawn_training_panel(
        &mut commands,
        Screen::Lesson,
        vec![
            ("Hint", "H", UiAction::Hint),
            ("Show me", "J", UiAction::ShowMe),
            UNDO,
            ("Restart", "G", UiAction::Restart),
            ("Next", "N", UiAction::Next),
            BACK,
        ],
    );
}

fn spawn_coach_panel(mut commands: Commands) {
    spawn_training_panel(
        &mut commands,
        Screen::Coach,
        vec![
            ("Hint", "H", UiAction::Hint),
            ("Show me", "J", UiAction::ShowMe),
            UNDO,
            ("New scramble", "N", UiAction::Next),
            BACK,
        ],
    );
}

fn spawn_drill_panel(mut commands: Commands) {
    spawn_training_panel(
        &mut commands,
        Screen::Drill,
        vec![
            ("Reveal", "V", UiAction::Reveal),
            ("Show me", "J", UiAction::ShowMe),
            UNDO,
            ("Retry", "G", UiAction::Restart),
            ("Next", "N", UiAction::Next),
            BACK,
        ],
    );
}

fn spawn_practice_panel(mut commands: Commands) {
    spawn_training_panel(
        &mut commands,
        Screen::Practice,
        vec![
            ("Hint", "H", UiAction::Hint),
            ("Show me", "J", UiAction::ShowMe),
            UNDO,
            ("Retry", "G", UiAction::Restart),
            ("Next", "N", UiAction::Next),
            BACK,
        ],
    );
}

fn update_panel_text(
    run: Res<Run>,
    state: Res<CubeState>,
    progress: Res<Progress>,
    mut texts: Query<(&PanelText, &mut Text)>,
) {
    for (part, mut text) in &mut texts {
        let wanted = match part {
            PanelText::Title => panel_title(&run),
            PanelText::Body => panel_body(&run, &state, &progress),
            PanelText::Hint => panel_hint(&run),
            PanelText::Result => panel_result(&run),
        };
        if text.0 != wanted {
            text.0 = wanted;
        }
    }
}

fn panel_title(run: &Run) -> String {
    match run.activity {
        Activity::None => String::new(),
        Activity::Lesson { index, .. } => {
            let title = run.lesson().map_or("", |lesson| lesson.title);
            format!("{}. {title}", index + 1)
        }
        Activity::Coach => "Coach".to_owned(),
        // Recognising the case is half the skill, so its name stays hidden until it's earned.
        Activity::Drill { case, .. } if run.revealed || run.is_finished() => case.name.to_owned(),
        Activity::Drill { set, .. } => {
            let family = if set.is_pll() { "PLL" } else { "OLL" };
            format!("Which {family} case is this?")
        }
        Activity::Practice(stage) => stage.name().to_owned(),
    }
}

fn panel_body(run: &Run, state: &CubeState, progress: &Progress) -> String {
    match run.activity {
        Activity::None => String::new(),
        Activity::Lesson { finished, .. } => {
            let Some(lesson) = run.lesson() else {
                return String::new();
            };
            let mut body = lesson.body.to_owned();
            if let Some(algorithm) = lesson.algorithm {
                body.push_str(&format!("\n\nThe move: {algorithm}"));
            }
            let noun = match lesson.exercise {
                Exercise::Follow(_) => "Exercise",
                Exercise::Stage(_) => "Position",
            };
            let shown = (finished + 1).min(lesson.positions());
            body.push_str(&format!("\n\n{noun} {shown} of {}", lesson.positions()));
            body
        }
        Activity::Coach => {
            let current = MethodView::of(&state.cube).stage();
            let checklist = Stage::STEPS
                .iter()
                .map(|&stage| {
                    let mark = match stage.cmp(&current) {
                        std::cmp::Ordering::Less => "[x]",
                        std::cmp::Ordering::Equal => "[>]",
                        std::cmp::Ordering::Greater => "[ ]",
                    };
                    format!("{mark} {}", stage.name())
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "Solve it your own way, with white on the bottom. Press Hint whenever you're \
                 stuck.\n\n{checklist}"
            )
        }
        Activity::Drill { set, case, .. } => {
            let stats = progress.stats(&stats_key(case));
            // Results for a specific case would give away which case it is.
            let known = run.revealed || run.is_finished();
            let goal = if set.is_pll() {
                "Recognise the case and solve the cube from memory. Reveal shows the answer, \
                 but won't count as a clean attempt."
            } else {
                "Recognise the case and make the top all yellow from memory. Reveal shows the \
                 answer, but won't count as a clean attempt."
            };
            let clean = stats.clean_rate().map_or(String::new(), |rate| {
                format!(" - {:.0}% clean", rate * 100.0)
            });
            if known {
                format!("{goal}\n\n{}{clean}", stats_line(stats))
            } else {
                goal.to_owned()
            }
        }
        Activity::Practice(stage) => {
            let par = run.cross_par.map_or(String::new(), |par| {
                format!("\n\nCan be done in {par} turns.")
            });
            format!("{}{par}", stage.instructions())
        }
    }
}

fn panel_hint(run: &Run) -> String {
    if run.is_finished() {
        return String::new();
    }
    if let Some(follow) = &run.follow {
        if follow.is_off_track() {
            return "That wasn't the next move. Press Backspace to undo it.".to_owned();
        }
        let (_, left) = follow.progress();
        return format!("Next: {}", turns_text(left));
    }
    if let Activity::Drill { case, .. } = run.activity {
        return if run.revealed {
            format!("Algorithm: {}", case.alg)
        } else {
            String::new()
        };
    }
    if !run.hint_visible {
        return String::new();
    }
    let Some((step, left, rest)) = run.tracker.remaining() else {
        return String::new();
    };
    let mut hint = format!("Next: {}\n{}", turns_text(left), step.why);
    if let Some(after) = rest.first() {
        hint.push_str(&format!("\nThen: {}", turns_text(&after.turns)));
        if rest.len() > 1 {
            hint.push_str(" ...");
        }
    }
    hint
}

fn panel_result(run: &Run) -> String {
    let Some(result) = run.result else {
        return String::new();
    };
    let time = format_time(result.secs);
    match run.activity {
        Activity::None => String::new(),
        Activity::Lesson { .. } => {
            if run.lesson_complete() {
                "Lesson complete! Press Next to carry on.".to_owned()
            } else if result.clean {
                "Nice! Press Next for another.".to_owned()
            } else {
                "That one doesn't count towards the lesson. Press Next to do a new one yourself."
                    .to_owned()
            }
        }
        Activity::Coach => {
            if result.clean {
                format!("Solved with no help in {time}!")
            } else if run.retried {
                format!("Solved in {time}. A retry doesn't count as unaided.")
            } else {
                format!("Solved in {time}. Next time, try it with fewer hints.")
            }
        }
        Activity::Drill { .. } => {
            if result.clean {
                format!("Clean! {time}, {}.", moves_text(result.moves))
            } else if run.revealed || run.assisted {
                format!("{time}. You had help, so this case will come up again soon.")
            } else if run.retried {
                format!("{time}. A retry doesn't count as a clean attempt.")
            } else {
                format!("{time}, but that's more turning than this algorithm needs.")
            }
        }
        Activity::Practice(_) => match run.cross_par {
            Some(par) => format!(
                "{time} in {}. The best possible was {par}.",
                moves_text(result.moves)
            ),
            None => format!("{time} in {}.", moves_text(result.moves)),
        },
    }
}

fn turns_text(turns: &[crate::cube::Turn]) -> String {
    turns
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(" ")
}
