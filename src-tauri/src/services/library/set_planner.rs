//! Preparing a DJ set: check the order of a list of tracks (key, tempo and energy of every
//! transition, with bridge tracks for the ones that clash) and propose an order that mixes well.
//!
//! The cost of a transition adds three things a DJ feels: how far the keys are on the Camelot
//! wheel, how far the tempos are, and how big the energy jump is. The proposed order starts from
//! the calmest track (or the one chosen), goes to the closest track each time, then removes the
//! crossings a nearest-neighbour walk leaves (2-opt). It is a starting point, not an oracle.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use super::*;
use crate::services::harmonic::{
    bpm_delta_percent, relation, HarmonicRelation, PITCH_RANGE_PERCENT,
};

/// An energy change of at least this many levels is reported as a jump.
const ENERGY_JUMP: i32 = 3;
/// Bridge tracks offered per difficult transition.
const MAX_BRIDGES: usize = 3;
/// Most tracks in a set (and so the most the 2-opt pass works on).
const MAX_SET_TRACKS: usize = 300;
/// Library tracks considered as bridges.
const MAX_BRIDGE_POOL: usize = 5_000;

/// What the planner knows about a track.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct SetItem {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub key: Option<String>,
    pub bpm: Option<f64>,
    pub energy: Option<i32>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetBridge {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub key: Option<String>,
    pub bpm: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetTransition {
    pub harmonic: HarmonicRelation,
    pub bpm_delta_percent: Option<f64>,
    pub energy_delta: Option<i32>,
    /// The energy changes by 3 levels or more.
    pub energy_jump: bool,
    /// Library tracks that mix with both sides, offered when the keys clash or the tempo jumps
    /// beyond a deck's pitch range.
    pub bridges: Vec<SetBridge>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetEntry {
    /// 1-based.
    pub position: usize,
    pub track_id: String,
    pub title: String,
    pub artist: String,
    pub key: Option<String>,
    pub bpm: Option<f64>,
    pub energy: Option<i32>,
    pub duration_ms: u64,
    /// `None` for the first track.
    pub from_previous: Option<SetTransition>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SetAnalysis {
    pub entries: Vec<SetEntry>,
    pub total_duration_ms: u64,
    pub harmonic_transitions: usize,
    pub clashing_transitions: usize,
    pub unknown_transitions: usize,
    pub energy_jumps: usize,
    pub bpm_min: Option<f64>,
    pub bpm_max: Option<f64>,
}

fn is_compatible(relation: HarmonicRelation) -> bool {
    matches!(
        relation,
        HarmonicRelation::Same | HarmonicRelation::Adjacent | HarmonicRelation::Relative
    )
}

fn harmonic_cost(relation: HarmonicRelation) -> f64 {
    match relation {
        HarmonicRelation::Same => 0.0,
        HarmonicRelation::Adjacent | HarmonicRelation::Relative => 1.0,
        HarmonicRelation::Unknown => 3.0,
        HarmonicRelation::Clash => 6.0,
    }
}

/// How awkward the transition from `a` to `b` is (0 is perfect). Symmetric.
pub(super) fn transition_cost(a: &SetItem, b: &SetItem) -> f64 {
    let mut cost = harmonic_cost(relation(a.key.as_deref(), b.key.as_deref()));
    if let Some(delta) = bpm_delta_percent(a.bpm, b.bpm) {
        let gap = delta.abs();
        cost += gap * 0.5;
        if gap > PITCH_RANGE_PERCENT {
            cost += 4.0;
        }
    }
    if let (Some(x), Some(y)) = (a.energy, b.energy) {
        let jump = (x - y).abs();
        cost += f64::from(jump) * 0.5;
        if jump >= ENERGY_JUMP {
            cost += 1.0;
        }
    }
    cost
}

/// The tempo of `candidate` is within a deck's pitch range of `other` (or unknown).
fn tempo_fits(candidate: &SetItem, other: &SetItem) -> bool {
    bpm_delta_percent(other.bpm, candidate.bpm)
        .is_none_or(|delta| delta.abs() <= PITCH_RANGE_PERCENT)
}

/// Library tracks from `pool` that mix with both `a` and `b`, best first.
fn bridges_between(a: &SetItem, b: &SetItem, pool: &[SetItem]) -> Vec<SetBridge> {
    let mut found: Vec<(f64, &SetItem)> = pool
        .iter()
        .filter(|c| c.id != a.id && c.id != b.id)
        .filter(|c| {
            is_compatible(relation(a.key.as_deref(), c.key.as_deref()))
                && is_compatible(relation(c.key.as_deref(), b.key.as_deref()))
        })
        .filter(|c| tempo_fits(c, a) && tempo_fits(c, b))
        .map(|c| (transition_cost(a, c) + transition_cost(c, b), c))
        .collect();
    found.sort_by(|x, y| x.0.total_cmp(&y.0).then_with(|| x.1.id.cmp(&y.1.id)));
    found
        .into_iter()
        .take(MAX_BRIDGES)
        .map(|(_, c)| SetBridge {
            id: c.id.clone(),
            title: c.title.clone(),
            artist: c.artist.clone(),
            key: c.key.clone(),
            bpm: c.bpm,
        })
        .collect()
}

/// The set in the order given, with every transition described. `pool` is where bridge tracks
/// are looked for (the library outside the set).
pub(super) fn analyze(items: &[SetItem], pool: &[SetItem]) -> SetAnalysis {
    let mut entries: Vec<SetEntry> = Vec::with_capacity(items.len());
    let (mut harmonic, mut clashing, mut unknown, mut jumps) = (0, 0, 0, 0);

    for (i, item) in items.iter().enumerate() {
        let from_previous = i.checked_sub(1).map(|p| {
            let previous = &items[p];
            let harmonic_relation = relation(previous.key.as_deref(), item.key.as_deref());
            match harmonic_relation {
                HarmonicRelation::Clash => clashing += 1,
                HarmonicRelation::Unknown => unknown += 1,
                _ => harmonic += 1,
            }
            let bpm_delta = bpm_delta_percent(previous.bpm, item.bpm);
            let energy_delta = previous.energy.zip(item.energy).map(|(x, y)| y - x);
            let energy_jump = energy_delta.is_some_and(|d| d.abs() >= ENERGY_JUMP);
            if energy_jump {
                jumps += 1;
            }
            let tempo_jump = bpm_delta.is_some_and(|d| d.abs() > PITCH_RANGE_PERCENT);
            let bridges = if harmonic_relation == HarmonicRelation::Clash || tempo_jump {
                bridges_between(previous, item, pool)
            } else {
                Vec::new()
            };
            SetTransition {
                harmonic: harmonic_relation,
                bpm_delta_percent: bpm_delta,
                energy_delta,
                energy_jump,
                bridges,
            }
        });
        entries.push(SetEntry {
            position: i + 1,
            track_id: item.id.clone(),
            title: item.title.clone(),
            artist: item.artist.clone(),
            key: item.key.clone(),
            bpm: item.bpm,
            energy: item.energy,
            duration_ms: item.duration_ms,
            from_previous,
        });
    }

    let tempos = items.iter().filter_map(|i| i.bpm).filter(|b| *b > 0.0);
    SetAnalysis {
        total_duration_ms: items.iter().map(|i| i.duration_ms).sum(),
        harmonic_transitions: harmonic,
        clashing_transitions: clashing,
        unknown_transitions: unknown,
        energy_jumps: jumps,
        bpm_min: tempos.clone().min_by(f64::total_cmp),
        bpm_max: tempos.max_by(f64::total_cmp),
        entries,
    }
}

fn path_cost(items: &[SetItem], order: &[usize]) -> f64 {
    order
        .windows(2)
        .map(|w| transition_cost(&items[w[0]], &items[w[1]]))
        .sum()
}

/// An order (as indices into `items`) that mixes well, beginning at `start` or, by default, at the
/// calmest track (lowest energy, then lowest tempo). Deterministic.
pub(super) fn propose_order(items: &[SetItem], start: Option<usize>) -> Vec<usize> {
    if items.len() <= 2 {
        return (0..items.len()).collect();
    }
    let first = start.unwrap_or_else(|| {
        (0..items.len())
            .min_by(|&a, &b| {
                let key = |i: usize| {
                    (
                        items[i].energy.unwrap_or(i32::MAX),
                        items[i].bpm.unwrap_or(f64::MAX),
                        items[i].id.clone(),
                    )
                };
                let (ka, kb) = (key(a), key(b));
                ka.0.cmp(&kb.0)
                    .then(ka.1.total_cmp(&kb.1))
                    .then_with(|| ka.2.cmp(&kb.2))
            })
            .unwrap_or(0)
    });

    // Nearest neighbour.
    let mut order = vec![first];
    let mut left: Vec<usize> = (0..items.len()).filter(|&i| i != first).collect();
    while !left.is_empty() {
        let current = *order.last().expect("not empty");
        let (at, _) = left
            .iter()
            .enumerate()
            .min_by(|(_, &a), (_, &b)| {
                transition_cost(&items[current], &items[a])
                    .total_cmp(&transition_cost(&items[current], &items[b]))
                    .then_with(|| items[a].id.cmp(&items[b].id))
            })
            .expect("not empty");
        order.push(left.remove(at));
    }

    // 2-opt on the open path, first track fixed: reverse a stretch when it lowers the total cost.
    let mut improved = true;
    let mut passes = 0;
    while improved && passes < 50 {
        improved = false;
        passes += 1;
        for i in 1..order.len() - 1 {
            for j in i + 1..order.len() {
                let before = path_cost(items, &order);
                order[i..=j].reverse();
                if path_cost(items, &order) + 1e-9 < before {
                    improved = true;
                } else {
                    order[i..=j].reverse();
                }
            }
        }
    }
    order
}

impl LibraryService {
    /// The set in the given order with every transition described. Duplicated ids are kept once.
    pub fn analyze_set(&self, track_ids: &[String]) -> Result<SetAnalysis> {
        let items = self.set_items(track_ids)?;
        let in_set: HashSet<&str> = items.iter().map(|i| i.id.as_str()).collect();
        let pool: Vec<SetItem> = self
            .bridge_pool()?
            .into_iter()
            .filter(|c| !in_set.contains(c.id.as_str()))
            .collect();
        Ok(analyze(&items, &pool))
    }

    /// The same tracks in an order that mixes well, as track ids. `start_track_id` (which must be
    /// one of them) opens the set; by default it is the calmest track.
    pub fn suggest_set_order(
        &self,
        track_ids: &[String],
        start_track_id: Option<&str>,
    ) -> Result<Vec<String>> {
        let items = self.set_items(track_ids)?;
        let start = match start_track_id {
            Some(id) => Some(items.iter().position(|i| i.id == id).ok_or_else(|| {
                CrateError::InvalidOperation("the opening track is not in the set".to_string())
            })?),
            None => None,
        };
        Ok(propose_order(&items, start)
            .into_iter()
            .map(|i| items[i].id.clone())
            .collect())
    }

    fn set_items(&self, track_ids: &[String]) -> Result<Vec<SetItem>> {
        let mut seen = HashSet::new();
        let ids: Vec<&String> = track_ids
            .iter()
            .filter(|id| seen.insert(id.as_str()))
            .collect();
        if ids.len() > MAX_SET_TRACKS {
            return Err(CrateError::InvalidOperation(format!(
                "a set has at most {MAX_SET_TRACKS} tracks"
            )));
        }
        ids.into_iter()
            .map(|id| {
                let t = self.get_track(id)?;
                Ok(SetItem {
                    title: t.title.clone().unwrap_or_default(),
                    artist: t.artist.clone().unwrap_or_default(),
                    key: t.key.clone(),
                    bpm: t.bpm,
                    energy: t.energy,
                    duration_ms: t.duration_ms.max(0) as u64,
                    id: t.id,
                })
            })
            .collect()
    }

    /// Library tracks that could bridge two tracks: those with a key and a tempo.
    fn bridge_pool(&self) -> Result<Vec<SetItem>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;
        let mut stmt = conn.prepare(
            "SELECT id, title, artist, key, bpm, energy, duration_ms FROM tracks
             WHERE key IS NOT NULL AND key != '' AND bpm IS NOT NULL
             ORDER BY id LIMIT ?1",
        )?;
        let rows = stmt
            .query_map([MAX_BRIDGE_POOL as i64], |r| {
                Ok(SetItem {
                    id: r.get(0)?,
                    title: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    artist: r.get::<_, Option<String>>(2)?.unwrap_or_default(),
                    key: r.get(3)?,
                    bpm: r.get(4)?,
                    energy: r.get(5)?,
                    duration_ms: r.get::<_, i64>(6)?.max(0) as u64,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn item(id: &str, key: Option<&str>, bpm: Option<f64>, energy: Option<i32>) -> SetItem {
        SetItem {
            id: id.into(),
            title: format!("Title {id}"),
            artist: "Artist".into(),
            key: key.map(String::from),
            bpm,
            energy,
            duration_ms: 200_000,
        }
    }

    fn ids(items: &[SetItem], order: &[usize]) -> Vec<String> {
        order.iter().map(|&i| items[i].id.clone()).collect()
    }

    #[test]
    fn the_cost_grows_with_key_distance_tempo_gap_and_energy_jump() {
        let base = item("a", Some("8A"), Some(124.0), Some(5));
        let same = item("b", Some("8A"), Some(124.0), Some(5));
        let next_key = item("c", Some("9A"), Some(124.0), Some(5));
        let clash = item("d", Some("3B"), Some(124.0), Some(5));
        let fast = item("e", Some("8A"), Some(140.0), Some(5));
        let jump = item("f", Some("8A"), Some(124.0), Some(9));
        let cost = |x: &SetItem| transition_cost(&base, x);

        assert_eq!(cost(&same), 0.0);
        assert!(cost(&same) < cost(&next_key) && cost(&next_key) < cost(&clash));
        assert!(
            cost(&fast) > cost(&next_key),
            "a 13 % tempo jump is worse than a neighbouring key"
        );
        assert!(cost(&jump) > 2.0, "an energy jump of 4 costs");
        assert_eq!(
            transition_cost(&base, &clash),
            transition_cost(&clash, &base),
            "symmetric"
        );
    }

    #[test]
    fn a_scrambled_harmonic_chain_is_put_back_in_order() {
        // 1A to 5A, one step apart each, same tempo and energy: the best path is the chain.
        let items = vec![
            item("3A", Some("3A"), Some(124.0), Some(5)),
            item("1A", Some("1A"), Some(124.0), Some(5)),
            item("5A", Some("5A"), Some(124.0), Some(5)),
            item("2A", Some("2A"), Some(124.0), Some(5)),
            item("4A", Some("4A"), Some(124.0), Some(5)),
        ];
        let order = propose_order(&items, Some(1));
        assert_eq!(ids(&items, &order), ["1A", "2A", "3A", "4A", "5A"]);
    }

    #[test]
    fn by_default_the_set_opens_with_the_calmest_track_and_builds_up() {
        let items = vec![
            item("e9", Some("8A"), Some(124.0), Some(9)),
            item("e3", Some("8A"), Some(124.0), Some(3)),
            item("e7", Some("8A"), Some(124.0), Some(7)),
            item("e5", Some("8A"), Some(124.0), Some(5)),
        ];
        assert_eq!(
            ids(&items, &propose_order(&items, None)),
            ["e3", "e5", "e7", "e9"]
        );
    }

    #[test]
    fn the_order_is_the_same_whatever_the_input_order() {
        let a = item("a", Some("8A"), Some(124.0), Some(4));
        let b = item("b", Some("9A"), Some(125.0), Some(5));
        let c = item("c", Some("10A"), Some(126.0), Some(6));
        let d = item("d", Some("8B"), Some(123.0), Some(3));
        let one = [a.clone(), b.clone(), c.clone(), d.clone()];
        let two = [d, c, b, a];
        assert_eq!(
            ids(&one, &propose_order(&one, None)),
            ids(&two, &propose_order(&two, None))
        );
    }

    #[test]
    fn small_sets_and_a_chosen_opener_are_respected() {
        assert!(propose_order(&[], None).is_empty());
        let one = vec![item("a", None, None, None)];
        assert_eq!(propose_order(&one, None), [0]);
        let items = vec![
            item("a", Some("8A"), Some(124.0), Some(3)),
            item("b", Some("8A"), Some(124.0), Some(5)),
            item("c", Some("8A"), Some(124.0), Some(7)),
        ];
        assert_eq!(
            propose_order(&items, Some(2))[0],
            2,
            "the chosen track opens the set"
        );
    }

    #[test]
    fn every_transition_of_a_set_is_described() {
        let items = vec![
            item("a", Some("8A"), Some(124.0), Some(4)),
            item("b", Some("9A"), Some(126.0), Some(5)), // next key, +1.6 %
            item("c", Some("3B"), Some(126.0), Some(9)), // clash, energy jump of 4
            item("d", None, Some(126.0), Some(9)),       // unknown key
        ];
        let analysis = analyze(&items, &[]);

        assert_eq!(analysis.entries.len(), 4);
        assert!(analysis.entries[0].from_previous.is_none());
        let t = |i: usize| analysis.entries[i].from_previous.clone().unwrap();
        assert_eq!(t(1).harmonic, HarmonicRelation::Adjacent);
        assert_eq!(t(2).harmonic, HarmonicRelation::Clash);
        assert_eq!((t(2).energy_delta, t(2).energy_jump), (Some(4), true));
        assert_eq!(t(3).harmonic, HarmonicRelation::Unknown);
        assert_eq!(
            (
                analysis.harmonic_transitions,
                analysis.clashing_transitions,
                analysis.unknown_transitions,
                analysis.energy_jumps
            ),
            (1, 1, 1, 1)
        );
        assert_eq!(analysis.total_duration_ms, 800_000);
        assert_eq!(
            (analysis.bpm_min, analysis.bpm_max),
            (Some(124.0), Some(126.0))
        );
    }

    #[test]
    fn a_clashing_transition_offers_bridge_tracks_that_mix_with_both_sides() {
        let items = vec![
            item("a", Some("8A"), Some(124.0), Some(5)),
            item("b", Some("10A"), Some(125.0), Some(5)), // two steps away: clash
        ];
        let pool = vec![
            item("bridge", Some("9A"), Some(124.5), Some(5)), // next to both
            item("only_a", Some("8B"), Some(124.0), Some(5)), // mixes with a only
            item("too_fast", Some("9A"), Some(140.0), Some(5)), // right keys, wrong tempo
            item("unkeyed", None, Some(124.0), Some(5)),
        ];
        let analysis = analyze(&items, &pool);
        let bridges = &analysis.entries[1].from_previous.as_ref().unwrap().bridges;
        assert_eq!(
            bridges.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(),
            ["bridge"]
        );
    }

    #[test]
    fn a_smooth_transition_offers_no_bridge() {
        let items = vec![
            item("a", Some("8A"), Some(124.0), Some(5)),
            item("b", Some("9A"), Some(125.0), Some(5)),
        ];
        let pool = vec![item("x", Some("8A"), Some(124.0), Some(5))];
        let analysis = analyze(&items, &pool);
        assert!(analysis.entries[1]
            .from_previous
            .as_ref()
            .unwrap()
            .bridges
            .is_empty());
    }

    #[test]
    fn bridges_are_limited_and_best_first() {
        let items = vec![
            item("a", Some("8A"), Some(124.0), Some(5)),
            item("b", Some("10A"), Some(124.0), Some(5)),
        ];
        let pool: Vec<SetItem> = (0..6)
            .map(|i| {
                item(
                    &format!("p{i}"),
                    Some("9A"),
                    Some(124.0 + f64::from(i) * 0.3),
                    Some(5),
                )
            })
            .collect();
        let analysis = analyze(&items, &pool);
        let bridges = &analysis.entries[1].from_previous.as_ref().unwrap().bridges;
        assert_eq!(bridges.len(), MAX_BRIDGES);
        assert_eq!(bridges[0].id, "p0", "the closest tempo first");
    }

    // ----- with the library -----------------------------------------------------------------

    fn library(name: &str) -> (LibraryService, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("crate_set_{name}_{}", std::process::id()));
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        for (id, key, bpm, energy) in [
            ("a", "8A", 124.0, 4),
            ("b", "10A", 125.0, 6),
            ("bridge", "9A", 124.5, 5),
            ("far", "3B", 90.0, 5),
        ] {
            conn.execute(
                "INSERT INTO tracks (id, file_path, format, title, artist, key, bpm, energy, duration_ms, date_added, date_modified)
                 VALUES (?1, ?2, 'mp3', ?1, 'Artist', ?3, ?4, ?5, 200000, '2026-01-01', '2026-01-01')",
                rusqlite::params![id, format!("/m/{id}.mp3"), key, bpm, energy],
            )
            .unwrap();
        }
        (
            LibraryService::new(Arc::new(Mutex::new(conn)), dir.clone()),
            dir,
        )
    }

    #[test]
    fn the_library_service_analyses_a_set_and_finds_bridges_outside_it() {
        let (service, dir) = library("analyse");
        let set = vec!["a".to_string(), "b".to_string(), "a".to_string()]; // the repeat is ignored
        let analysis = service.analyze_set(&set).unwrap();

        assert_eq!(analysis.entries.len(), 2);
        let bridges = &analysis.entries[1].from_previous.as_ref().unwrap().bridges;
        assert_eq!(
            bridges.iter().map(|b| b.id.as_str()).collect::<Vec<_>>(),
            ["bridge"]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_library_service_proposes_an_order_and_rejects_bad_input() {
        let (service, dir) = library("order");
        let set: Vec<String> = ["b", "bridge", "a"].iter().map(|s| s.to_string()).collect();
        let order = service.suggest_set_order(&set, None).unwrap();
        assert_eq!(
            order,
            ["a", "bridge", "b"],
            "calmest first, then the harmonic chain"
        );

        assert_eq!(service.suggest_set_order(&set, Some("b")).unwrap()[0], "b");
        assert!(
            service.suggest_set_order(&set, Some("far")).is_err(),
            "the opener must be in the set"
        );
        assert!(service.analyze_set(&["nope".to_string()]).is_err());
        assert!(service.analyze_set(&[]).unwrap().entries.is_empty());
        let huge: Vec<String> = (0..=MAX_SET_TRACKS).map(|i| format!("t{i}")).collect();
        assert!(service.analyze_set(&huge).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
