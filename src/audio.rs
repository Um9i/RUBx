//! Plays a sound for each cube turn and a shuffled playlist of background music.

use std::time::Duration;

use bevy::audio::Volume;
use bevy::prelude::*;

use crate::game::{GameSet, TurnStarted};

const TURN_VOLUME: f32 = 0.7;
const SCRAMBLE_VOLUME: f32 = 0.3;
const MUSIC_VOLUME: f32 = 0.25;

/// The shortest gap between turn sounds, in seconds, so fast undos don't blur together.
const MIN_TURN_GAP_SECS: f32 = 0.05;
/// The shortest gap during a scramble, whose turns are far quicker than the ear wants.
const MIN_SCRAMBLE_GAP_SECS: f32 = 0.13;
/// How long before the measured onset each clip starts, so the attack isn't cut off.
const ONSET_LEAD_SECS: f32 = 0.01;

/// Each turn sample and when its sound begins, in seconds.
///
/// The recordings open with 0.2 to 0.5 seconds of silence, which is longer than a whole turn
/// animation, so playing them from the start would put the sound after the move.
const TURN_CLIPS: [(&str, f32); 25] = [
    ("486561__spacejoe__rubik-cube-turn-1.ogg", 0.498),
    ("486562__spacejoe__rubik-cube-turn-10.ogg", 0.387),
    ("486563__spacejoe__rubik-cube-turn-11.ogg", 0.423),
    ("486564__spacejoe__rubik-cube-turn-12.ogg", 0.436),
    ("486565__spacejoe__rubik-cube-turn-13.ogg", 0.411),
    ("486566__spacejoe__rubik-cube-turn-14.ogg", 0.416),
    ("486567__spacejoe__rubik-cube-turn-15.ogg", 0.491),
    ("486568__spacejoe__rubik-cube-turn-16.ogg", 0.381),
    ("486569__spacejoe__rubik-cube-turn-17.ogg", 0.386),
    ("486570__spacejoe__rubik-cube-turn-18.ogg", 0.438),
    ("486571__spacejoe__rubik-cube-turn-19.ogg", 0.268),
    ("486572__spacejoe__rubik-cube-turn-2.ogg", 0.436),
    ("486573__spacejoe__rubik-cube-turn-20.ogg", 0.356),
    ("486574__spacejoe__rubik-cube-turn-21.ogg", 0.360),
    ("486575__spacejoe__rubik-cube-turn-22.ogg", 0.240),
    ("486576__spacejoe__rubik-cube-turn-23.ogg", 0.430),
    ("486577__spacejoe__rubik-cube-turn-24.ogg", 0.421),
    ("486578__spacejoe__rubik-cube-turn-25.ogg", 0.488),
    ("486579__spacejoe__rubik-cube-turn-3.ogg", 0.469),
    ("486580__spacejoe__rubik-cube-turn-4.ogg", 0.426),
    ("486581__spacejoe__rubik-cube-turn-5.ogg", 0.407),
    ("486582__spacejoe__rubik-cube-turn-6.ogg", 0.330),
    ("486583__spacejoe__rubik-cube-turn-7.ogg", 0.370),
    ("486584__spacejoe__rubik-cube-turn-8.ogg", 0.460),
    ("486585__spacejoe__rubik-cube-turn-9.ogg", 0.342),
];

const MUSIC_TRACKS: [&str; 8] = [
    "audio/music/002_Synthwave_15k.ogg",
    "audio/music/sunset_plains.ogg",
    "audio/music/holizna/01 HoliznaCC0 - Dreams.ogg",
    "audio/music/holizna/02 HoliznaCC0 - Day Jobs.ogg",
    "audio/music/holizna/02 HoliznaCC0 - Deja Vu.ogg",
    "audio/music/holizna/03 HoliznaCC0 - Childhood.ogg",
    "audio/music/holizna/04 HoliznaCC0 - Dancing.ogg",
    "audio/music/holizna/06 HoliznaCC0 - Advertisements.ogg",
];

/// The loaded turn samples, kept alive so a sound can start the moment a turn does.
#[derive(Resource)]
struct TurnSounds {
    clips: Vec<(Handle<AudioSource>, Duration)>,
    last_clip: usize,
    last_played_secs: f32,
}

/// The tracks still to play before the playlist is shuffled again.
#[derive(Resource, Default)]
struct Playlist {
    remaining: Vec<&'static str>,
}

/// Marks the music track that is currently playing.
#[derive(Component)]
struct MusicTrack;

/// Adds turn sounds and music. Needs [`DefaultPlugins`] for the asset server and audio output.
pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Playlist>()
            .add_systems(Startup, load_turn_sounds)
            .add_systems(
                Update,
                (play_turn_sounds.in_set(GameSet::Present), play_next_track),
            );
    }
}

fn load_turn_sounds(mut commands: Commands, assets: Res<AssetServer>) {
    let clips = TURN_CLIPS
        .iter()
        .map(|&(file, onset)| {
            let start = (onset - ONSET_LEAD_SECS).max(0.0);
            (
                assets.load(format!("audio/cube/{file}")),
                Duration::from_secs_f32(start),
            )
        })
        .collect();
    commands.insert_resource(TurnSounds {
        clips,
        last_clip: 0,
        last_played_secs: f32::NEG_INFINITY,
    });
}

fn play_turn_sounds(
    mut commands: Commands,
    mut started: MessageReader<TurnStarted>,
    mut sounds: ResMut<TurnSounds>,
    time: Res<Time>,
) {
    let now = time.elapsed_secs();
    for turn in started.read() {
        let (min_gap, volume) = if turn.scramble {
            (MIN_SCRAMBLE_GAP_SECS, SCRAMBLE_VOLUME)
        } else {
            (MIN_TURN_GAP_SECS, TURN_VOLUME)
        };
        if now - sounds.last_played_secs < min_gap {
            continue;
        }
        sounds.last_played_secs = now;

        let index = next_clip(sounds.last_clip, sounds.clips.len());
        sounds.last_clip = index;
        let (clip, start) = sounds.clips[index].clone();
        commands.spawn((
            AudioPlayer::new(clip),
            PlaybackSettings::DESPAWN
                .with_volume(Volume::Linear(volume))
                .with_start_position(start),
        ));
    }
}

/// Picks a random clip other than the one just played.
fn next_clip(last: usize, count: usize) -> usize {
    let pick = fastrand::usize(..count - 1);
    if pick >= last { pick + 1 } else { pick }
}

fn play_next_track(
    mut commands: Commands,
    mut playlist: ResMut<Playlist>,
    playing: Query<(), With<MusicTrack>>,
    assets: Res<AssetServer>,
) {
    if !playing.is_empty() {
        return;
    }
    if playlist.remaining.is_empty() {
        playlist.remaining = MUSIC_TRACKS.to_vec();
        fastrand::shuffle(&mut playlist.remaining);
    }
    let Some(track) = playlist.remaining.pop() else {
        return;
    };
    commands.spawn((
        MusicTrack,
        AudioPlayer::new(assets.load(track)),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(MUSIC_VOLUME)),
    ));
}

#[cfg(test)]
mod tests {
    use bevy::audio::Decodable;

    use super::*;

    #[test]
    fn next_clip_should_never_repeat_and_stay_in_range() {
        for last in 0..TURN_CLIPS.len() {
            for _ in 0..200 {
                let clip = next_clip(last, TURN_CLIPS.len());
                assert_ne!(clip, last);
                assert!(clip < TURN_CLIPS.len());
            }
        }
    }

    #[test]
    fn every_audio_file_should_decode() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        let turns = TURN_CLIPS
            .iter()
            .map(|(file, _)| format!("audio/cube/{file}"));
        for path in turns.chain(MUSIC_TRACKS.iter().map(ToString::to_string)) {
            let bytes = std::fs::read(root.join(&path)).expect(&path);
            let source = AudioSource {
                bytes: bytes.into(),
            };
            assert!(source.decoder().count() > 0, "no audio in {path}");
        }
    }
}
