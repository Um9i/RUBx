//! Saved progress: completed lessons and times for everything the player can practise.
//!
//! Saved as a small text file with one tab-separated record per line. Saving is best effort
//! because losing a statistic should never interrupt the game, and unreadable lines are skipped so
//! that an edited or older file still loads.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use bevy::prelude::*;

/// Results for one thing the player practises.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stats {
    /// How many times it has been finished.
    pub attempts: u32,
    /// How many of those were done unaided.
    pub clean: u32,
    /// The fastest time, in seconds.
    pub best_secs: Option<f32>,
    /// The sum of all times, in seconds.
    pub total_secs: f32,
}

impl Stats {
    /// Returns the average time in seconds, if there are any results.
    pub fn average_secs(&self) -> Option<f32> {
        (self.attempts > 0).then(|| self.total_secs / self.attempts as f32)
    }

    /// Returns the share of attempts done unaided, from 0 to 1.
    pub fn clean_rate(&self) -> Option<f32> {
        (self.attempts > 0).then(|| self.clean as f32 / self.attempts as f32)
    }
}

/// Everything saved between sessions.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct Progress {
    lessons_done: BTreeSet<String>,
    stats: BTreeMap<String, Stats>,
    path: Option<PathBuf>,
}

impl Progress {
    /// Loads progress from the player's data directory.
    pub fn load_default() -> Self {
        let Some(path) = default_path() else {
            return Self::default();
        };
        let mut progress = match fs::read_to_string(&path) {
            Ok(text) => Self::parse(&text),
            Err(error) => {
                if error.kind() != io::ErrorKind::NotFound {
                    warn!("could not read {}: {error}", path.display());
                }
                Self::default()
            }
        };
        progress.path = Some(path);
        progress
    }

    /// Returns the results recorded under `key`.
    pub fn stats(&self, key: &str) -> Stats {
        self.stats.get(key).copied().unwrap_or_default()
    }

    /// Returns whether a lesson has been completed.
    pub fn lesson_done(&self, id: &str) -> bool {
        self.lessons_done.contains(id)
    }

    /// Marks a lesson as completed and saves.
    pub fn complete_lesson(&mut self, id: &str) {
        self.lessons_done.insert(id.to_owned());
        self.save();
    }

    /// Adds a result under `key` and saves.
    pub fn record(&mut self, key: &str, secs: f32, clean: bool) {
        let stats = self.stats.entry(key.to_owned()).or_default();
        stats.attempts += 1;
        stats.clean += u32::from(clean);
        stats.total_secs += secs;
        stats.best_secs = Some(stats.best_secs.map_or(secs, |best| best.min(secs)));
        self.save();
    }

    fn save(&self) {
        let Some(path) = &self.path else { return };
        if let Err(error) = write_atomically(path, &self.serialise()) {
            warn!("could not save progress to {}: {error}", path.display());
        }
    }

    fn serialise(&self) -> String {
        let mut text = String::new();
        for id in &self.lessons_done {
            text.push_str(&format!("lesson\t{id}\n"));
        }
        for (key, stats) in &self.stats {
            let best = stats
                .best_secs
                .map_or("-".to_owned(), |best| best.to_string());
            text.push_str(&format!(
                "stats\t{key}\t{}\t{}\t{best}\t{}\n",
                stats.attempts, stats.clean, stats.total_secs
            ));
        }
        text
    }

    fn parse(text: &str) -> Self {
        let mut progress = Self::default();
        for line in text.lines() {
            let fields: Vec<&str> = line.split('\t').collect();
            match fields.as_slice() {
                ["lesson", id] => {
                    progress.lessons_done.insert((*id).to_owned());
                }
                ["stats", key, attempts, clean, best, total] => {
                    let parsed = (|| {
                        Some(Stats {
                            attempts: attempts.parse().ok()?,
                            clean: clean.parse().ok()?,
                            best_secs: match *best {
                                "-" => None,
                                best => Some(best.parse().ok()?),
                            },
                            total_secs: total.parse().ok()?,
                        })
                    })();
                    if let Some(stats) = parsed {
                        progress.stats.insert((*key).to_owned(), stats);
                    }
                }
                _ => {}
            }
        }
        progress
    }
}

/// Writes through a temporary file, so that a crash can't leave a half-written save behind.
fn write_atomically(path: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, text)?;
    fs::rename(&temporary, path)
}

/// Finds where to save. `RUBX_DATA_DIR` lets scripted runs avoid touching real progress.
fn default_path() -> Option<PathBuf> {
    let directory = std::env::var_os("RUBX_DATA_DIR")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(|data| PathBuf::from(data).join("rubx")))
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share/rubx"))
        })?;
    Some(directory.join("progress.tsv"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Progress {
        let mut progress = Progress::default();
        progress.complete_lesson("cross");
        progress.record("drill:T perm", 4.5, true);
        progress.record("drill:T perm", 6.5, false);
        progress.record("play", 62.25, true);
        progress
    }

    #[test]
    fn progress_should_survive_a_round_trip() {
        let progress = sample();
        assert_eq!(Progress::parse(&progress.serialise()), progress);
    }

    #[test]
    fn record_should_keep_the_fastest_time() {
        assert_eq!(sample().stats("drill:T perm").best_secs, Some(4.5));
    }

    #[test]
    fn stats_should_average_times() {
        assert_eq!(sample().stats("drill:T perm").average_secs(), Some(5.5));
    }

    #[test]
    fn stats_should_report_the_clean_rate() {
        assert_eq!(sample().stats("drill:T perm").clean_rate(), Some(0.5));
    }

    #[test]
    fn unknown_stats_should_be_empty() {
        assert_eq!(sample().stats("nothing"), Stats::default());
    }

    #[test]
    fn parse_should_skip_unreadable_lines() {
        let progress = Progress::parse(
            "garbage\nstats\tbad\tnot-a-number\t0\t-\t0\nlesson\tcross\nstats\tplay\t1\t1\t9.5\t9.5\n",
        );
        assert_eq!(
            (
                progress.lesson_done("cross"),
                progress.stats("play").attempts,
                progress.stats("bad").attempts,
            ),
            (true, 1, 0),
        );
    }

    #[test]
    fn progress_should_load_what_it_saved() {
        let directory =
            std::env::temp_dir().join(format!("rubx-progress-test-{}", std::process::id()));
        let path = directory.join("nested").join("progress.tsv");

        let mut progress = Progress {
            path: Some(path.clone()),
            ..Progress::default()
        };
        progress.complete_lesson("middle");
        progress.record("coach", 90.0, false);

        let loaded = Progress::parse(&fs::read_to_string(&path).unwrap());
        fs::remove_dir_all(&directory).unwrap();
        assert_eq!(
            (loaded.lesson_done("middle"), loaded.stats("coach").attempts),
            (true, 1),
        );
    }
}
