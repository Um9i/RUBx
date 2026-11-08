//! Algorithm drills: the sets on offer, and choosing which case to practise next.

use crate::algs::{Case, OLL, PLL};
use crate::progress::{Progress, Stats};

/// A group of algorithm cases to drill.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DrillSet {
    /// A small set of the most common and easiest PLLs.
    StarterPll,
    /// All 21 PLLs.
    AllPll,
    /// The seven OLLs where the yellow cross is already done.
    CrossOll,
    /// All 57 OLLs.
    AllOll,
}

impl DrillSet {
    /// All sets, in the order they should be learned.
    pub const ALL: [DrillSet; 4] = [
        DrillSet::StarterPll,
        DrillSet::CrossOll,
        DrillSet::AllPll,
        DrillSet::AllOll,
    ];

    /// The set's name.
    pub const fn name(self) -> &'static str {
        match self {
            DrillSet::StarterPll => "Starter PLL",
            DrillSet::AllPll => "All PLL",
            DrillSet::CrossOll => "Yellow cross OLL",
            DrillSet::AllOll => "All OLL",
        }
    }

    /// A one-line description of the set.
    pub const fn description(self) -> &'static str {
        match self {
            DrillSet::StarterPll => "The 7 easiest cases to learn first",
            DrillSet::AllPll => "All 21 ways to finish the last layer",
            DrillSet::CrossOll => "The 7 cases with the yellow cross already made",
            DrillSet::AllOll => "All 57 ways to make the top yellow",
        }
    }

    /// Returns whether finishing a case means solving the whole cube.
    pub const fn is_pll(self) -> bool {
        matches!(self, DrillSet::StarterPll | DrillSet::AllPll)
    }

    /// Returns the cases in the set.
    pub fn cases(self) -> Vec<&'static Case> {
        const STARTER: [&str; 7] = [
            "Aa perm", "Ab perm", "H perm", "T perm", "Ua perm", "Ub perm", "Z perm",
        ];
        match self {
            DrillSet::StarterPll => PLL
                .iter()
                .filter(|case| STARTER.contains(&case.name))
                .collect(),
            DrillSet::AllPll => PLL.iter().collect(),
            // In the standard numbering, these are OLL 21 to OLL 27.
            DrillSet::CrossOll => OLL[20..27].iter().collect(),
            DrillSet::AllOll => OLL.iter().collect(),
        }
    }
}

/// Returns the key that a case's results are saved under.
pub fn stats_key(case: &Case) -> String {
    format!("drill:{}", case.name)
}

/// How much more often than normal to offer a case that has never been tried.
const UNSEEN_WEIGHT: f32 = 4.0;

/// Returns how strongly to favour a case, given its results and the set's average time.
///
/// Cases that the player keeps needing help with, or that are slower than their others, come up
/// more often. Every case keeps a weight of at least 1, so nothing drops out of rotation.
pub fn case_weight(stats: Stats, set_average_secs: Option<f32>) -> f32 {
    let Some(clean_rate) = stats.clean_rate() else {
        return UNSEEN_WEIGHT;
    };
    let slowness = match (stats.average_secs(), set_average_secs) {
        (Some(average), Some(set_average)) if set_average > 0.0 => {
            (average / set_average - 1.0).clamp(0.0, 2.0)
        }
        _ => 0.0,
    };
    1.0 + 3.0 * (1.0 - clean_rate) + slowness
}

/// Chooses the next case, favouring weak ones and avoiding repeating `last`.
pub fn pick_case(
    set: DrillSet,
    progress: &Progress,
    last: Option<&Case>,
    rng: &mut fastrand::Rng,
) -> &'static Case {
    let cases = set.cases();
    let stats: Vec<Stats> = cases
        .iter()
        .map(|case| progress.stats(&stats_key(case)))
        .collect();
    let timed: u32 = stats.iter().map(|stats| stats.timed).sum();
    let total: f32 = stats.iter().map(|stats| stats.total_secs).sum();
    let set_average = (timed > 0).then(|| total / timed as f32);

    let weights: Vec<f32> = cases
        .iter()
        .zip(&stats)
        .map(|(case, &stats)| {
            if Some(*case) == last {
                0.0
            } else {
                case_weight(stats, set_average)
            }
        })
        .collect();
    let mut remaining = rng.f32() * weights.iter().sum::<f32>();
    for (case, weight) in cases.iter().zip(&weights) {
        if *weight > 0.0 && remaining < *weight {
            return case;
        }
        remaining -= weight;
    }
    // Rounding can run past the end, in which case the last eligible case is the right answer.
    cases
        .iter()
        .zip(&weights)
        .rev()
        .find(|(_, weight)| **weight > 0.0)
        .map_or(cases[0], |(case, _)| case)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(attempts: u32, clean: u32, total_secs: f32) -> Stats {
        Stats {
            attempts,
            clean,
            timed: attempts,
            total_secs,
            best_secs: Some(1.0),
        }
    }

    #[test]
    fn set_sizes_should_match_their_descriptions() {
        let sizes = DrillSet::ALL.map(|set| set.cases().len());
        assert_eq!(sizes, [7, 7, 21, 57]);
    }

    #[test]
    fn unseen_case_should_outweigh_a_mastered_one() {
        assert!(
            case_weight(Stats::default(), Some(5.0)) > case_weight(stats(10, 10, 50.0), Some(5.0))
        );
    }

    #[test]
    fn case_needing_help_should_outweigh_a_clean_one() {
        assert!(
            case_weight(stats(10, 2, 50.0), Some(5.0))
                > case_weight(stats(10, 10, 50.0), Some(5.0))
        );
    }

    #[test]
    fn slow_case_should_outweigh_a_fast_one() {
        assert!(
            case_weight(stats(10, 10, 100.0), Some(5.0))
                > case_weight(stats(10, 10, 50.0), Some(5.0))
        );
    }

    #[test]
    fn mastered_case_should_still_come_up() {
        assert!(case_weight(stats(100, 100, 1.0), Some(5.0)) >= 1.0);
    }

    #[test]
    fn pick_should_not_repeat_the_last_case() {
        let mut rng = fastrand::Rng::with_seed(1);
        let progress = Progress::default();
        let last = DrillSet::StarterPll.cases()[0];
        for _ in 0..200 {
            let picked = pick_case(DrillSet::StarterPll, &progress, Some(last), &mut rng);
            assert_ne!(picked, last);
        }
    }

    #[test]
    fn pick_should_favour_a_weak_case() {
        let mut progress = Progress::default();
        let cases = DrillSet::StarterPll.cases();
        let weak = cases[0];
        for case in &cases {
            let key = stats_key(case);
            for _ in 0..5 {
                progress.record(&key, 5.0, *case != weak);
            }
        }
        let mut rng = fastrand::Rng::with_seed(2);
        let picks = (0..700)
            .filter(|_| pick_case(DrillSet::StarterPll, &progress, None, &mut rng) == weak)
            .count();
        // Evenly spread would be 100 of 700. The weak case has weight 4 against 1 for the others.
        assert!(picks > 200, "{picks}");
    }

    #[test]
    fn every_case_should_eventually_come_up() {
        let progress = Progress::default();
        let mut rng = fastrand::Rng::with_seed(3);
        let mut seen = std::collections::HashSet::new();
        for _ in 0..2000 {
            seen.insert(pick_case(DrillSet::AllOll, &progress, None, &mut rng).name);
        }
        assert_eq!(seen.len(), 57);
    }
}
