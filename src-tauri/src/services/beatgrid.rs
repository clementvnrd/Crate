//! Beat grid of a track: where its beats fall, for drawing grid lines on the waveform.
//!
//! The analysis pass (stratum-dsp) estimates the tempo, but its own beat list is laid at that
//! rounded estimate and is not usable as a grid. The grid is therefore found here, in the same
//! pass and on the same decoded samples: an onset envelope, a comb search for where the beats fall
//! near the analysed tempo, then each beat snapped to its onset. The beats are condensed into a
//! Rekordbox-style description instead of a per-beat list: the first beat and a BPM with decimals
//! for the whole track, plus a short list of tempo changes only when the tempo actually moves. A
//! constant-tempo track costs two numbers.
//!
//! The grid lives in three local columns of `tracks` (migration 19). It is an analysis artefact
//! like `waveform_data`: it is neither synced nor backed up, and it never touches the BPM that is
//! displayed and exported (`tracks.bpm`).

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::{CrateError, Result};

/// A track's beat grid. Grid lines are drawn every `60000 / bpm` ms from `first_beat_ms`, and from
/// each tempo change onwards at that change's BPM.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrackBeatGrid {
    /// First grid line at or after the start of the file, in ms (`0 <= first_beat_ms < 60000 / bpm`).
    pub first_beat_ms: f64,
    /// Tempo of the grid from `first_beat_ms`, with decimals. Not the displayed BPM.
    pub bpm: f64,
    /// `None` for a constant tempo. Otherwise each later section of the grid, in time order.
    pub tempo_changes: Option<Vec<TempoChange>>,
}

/// A grid line where the tempo changes: the grid restarts from `position_ms` at `bpm`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TempoChange {
    pub position_ms: f64,
    pub bpm: f64,
}

/// Fewer beats than this (two bars) give no grid: the track has no usable pulse.
const MIN_BEATS: usize = 8;
/// A beat this far from the grid is off it.
const OFF_GRID_MS: f64 = 25.0;
/// This many consecutive off-grid beats mean the tempo or the phase really moved, not a stray
/// onset.
const DRIFT_RUN: usize = 4;
/// A tempo section is at least four bars long, so a section never follows a fill or a break.
const MIN_SECTION_BEATS: usize = 16;
/// A grid more than 10% away from the analysed tempo locked onto something else: no grid.
const MAX_TEMPO_GAP: f64 = 0.10;
/// A beat whose distance to the previous one is further than this from a whole number of beats
/// (in beats) is off the grid: an off-beat or a stray onset.
const MAX_BEAT_FRACTION: f64 = 0.25;

/// At least this share of the beats must sit on the grid, or the "grid" is fitted to noise.
const MIN_ON_GRID_SHARE: f64 = 0.65;

/// Resolution of the onset envelope (s).
const ENVELOPE_HOP_S: f64 = 0.0025;
/// Energy floor of the envelope (mean square), about -60 dBFS, so that fades and dither in quiet
/// passages do not read as onsets.
const ENERGY_FLOOR: f32 = 1e-6;
/// Beats per search window: the phase and the local tempo are found every four bars.
const WINDOW_BEATS: f64 = 16.0;
/// Local tempos tried around the analysed one in each window: +/-4% in 0.2% steps.
const TEMPO_STEPS: i32 = 20;
const TEMPO_STEP: f64 = 0.002;
/// A beat is snapped to the strongest onset within this fraction of a beat of its predicted place.
const SNAP_FRACTION: f64 = 0.125;
/// A window keeps the phase of the previous one when it scores at least this share of its best:
/// the grid does not jump to the off-beats of a bar where the hi-hats outweigh the kick.
const CONTINUITY_SHARE: f32 = 0.6;
/// A snapped onset weaker than this share of the median beat onset is not a beat (breakdowns).
const MIN_ONSET_SHARE: f32 = 0.25;

/// Beat grid of decoded mono samples, given the tempo the analysis found (`bpm_hint`). `None`
/// when there is no usable pulse. Runs on the samples already decoded for the analysis.
pub fn detect_beat_grid(samples: &[f32], sample_rate: u32, bpm_hint: f64) -> Option<TrackBeatGrid> {
    if sample_rate == 0 || !(bpm_hint.is_finite() && bpm_hint > 0.0) {
        return None;
    }
    let (envelope, hop_s) = onset_envelope(samples, sample_rate);
    let beats = track_beats(&envelope, hop_s, bpm_hint);
    fit_beat_grid(&beats, bpm_hint)
}

/// Positive change of log energy between consecutive 2.5 ms frames, and the frame length (s).
/// Value `k` is the onset strength at the start of frame `k`.
fn onset_envelope(samples: &[f32], sample_rate: u32) -> (Vec<f32>, f64) {
    let hop = ((sample_rate as f64 * ENVELOPE_HOP_S).round() as usize).max(1);
    let energies: Vec<f32> = samples
        .chunks(hop)
        .map(|frame| {
            let mean_square = frame.iter().map(|s| s * s).sum::<f32>() / frame.len() as f32;
            (mean_square + ENERGY_FLOOR).ln()
        })
        .collect();
    let mut envelope = vec![0.0f32; energies.len()];
    for k in 1..energies.len() {
        envelope[k] = (energies[k] - energies[k - 1]).max(0.0);
    }
    (envelope, hop as f64 / sample_rate as f64)
}

/// Strongest envelope value within one frame of `frame`.
fn strength_near(envelope: &[f32], frame: f64) -> f32 {
    let center = frame.round() as i64;
    (center - 1..=center + 1)
        .filter_map(|k| usize::try_from(k).ok().and_then(|k| envelope.get(k)))
        .fold(0.0f32, |best, v| best.max(*v))
}

/// Comb score of a grid of `period` frames from `first` up to `end` (frames).
fn comb_score(envelope: &[f32], first: f64, period: f64, end: f64) -> f32 {
    let mut score = 0.0;
    let mut t = first;
    while t < end {
        score += strength_near(envelope, t);
        t += period;
    }
    score
}

/// Beat times (s) in the file. Every four bars, the local tempo (within 4% of the hint) and the
/// phase that best line up with the onsets are found, then each predicted beat is snapped to the
/// strongest nearby onset. Beats with no real onset (breakdowns, silence) are left out.
fn track_beats(envelope: &[f32], hop_s: f64, bpm_hint: f64) -> Vec<f64> {
    let base_period = 60.0 / bpm_hint / hop_s;
    if !base_period.is_finite() || base_period < 2.0 || envelope.is_empty() {
        return Vec::new();
    }
    let window = (WINDOW_BEATS * base_period).ceil() as usize;
    let periods: Vec<f64> = (-TEMPO_STEPS..=TEMPO_STEPS)
        .map(|i| base_period * (1.0 + i as f64 * TEMPO_STEP))
        .collect();

    let mut onsets: Vec<(f64, f32)> = Vec::new();
    let mut previous: Option<(f64, f64)> = None; // (last predicted beat, period) of the last window
    for start in (0..envelope.len()).step_by(window) {
        let (start, end) = (start as f64, (start + window).min(envelope.len()) as f64);
        let mut best = (0.0f32, base_period, start);
        for &period in &periods {
            for phase in 0..period.ceil() as usize {
                let first = start + phase as f64;
                let score = comb_score(envelope, first, period, end);
                if score > best.0 {
                    best = (score, period, first);
                }
            }
        }
        if best.0 <= 0.0 {
            previous = None;
            continue;
        }
        if let Some((last, period)) = previous {
            let first = last + ((start - last) / period).ceil() * period;
            let score = comb_score(envelope, first, period, end);
            if score >= CONTINUITY_SHARE * best.0 {
                best = (score, period, first);
            }
        }
        let (_, period, mut predicted) = best;
        let reach = (period * SNAP_FRACTION).round() as i64;
        let mut last = predicted;
        while predicted < end {
            let center = predicted.round() as i64;
            let peak = (center - reach..=center + reach)
                .filter_map(|k| usize::try_from(k).ok())
                .filter_map(|k| envelope.get(k).map(|v| (k, *v)))
                .max_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((frame, strength)) = peak.filter(|(_, v)| *v > 0.0) {
                onsets.push((frame as f64 * hop_s, strength));
            }
            last = predicted;
            predicted += period;
        }
        previous = Some((last, period));
    }
    if onsets.is_empty() {
        return Vec::new();
    }

    let mut strengths: Vec<f32> = onsets.iter().map(|(_, v)| *v).collect();
    strengths.sort_by(f32::total_cmp);
    let floor = strengths[strengths.len() / 2] * MIN_ONSET_SHARE;
    let mut beats: Vec<f64> = Vec::with_capacity(onsets.len());
    for (time, strength) in onsets {
        if strength < floor {
            continue;
        }
        // Two windows can snap to the same onset at their border.
        if beats
            .last()
            .is_some_and(|last| time - last < 0.5 * base_period * hop_s)
        {
            continue;
        }
        beats.push(time);
    }
    beats
}

/// Condenses beat times (seconds in the file) into a grid. `bpm_hint` is the analysed tempo: it
/// sets the beat spacing used to number the beats, so a missed or doubled beat does not bend the
/// grid. `None` when there is no usable pulse.
pub fn fit_beat_grid(beats_s: &[f64], bpm_hint: f64) -> Option<TrackBeatGrid> {
    if beats_s.len() < MIN_BEATS || !(bpm_hint.is_finite() && bpm_hint > 0.0) {
        return None;
    }
    let mut beats: Vec<f64> = beats_s.iter().map(|t| t * 1000.0).collect();
    beats.sort_by(f64::total_cmp);
    let points = number_beats(&beats, 60_000.0 / bpm_hint);
    if points.len() < MIN_BEATS {
        return None;
    }
    let global = fit_section(&points)?;
    if (60_000.0 / global.period - bpm_hint).abs() / bpm_hint > MAX_TEMPO_GAP {
        return None;
    }

    let mut sections = Vec::new();
    split_sections(&points, 0, &mut sections);
    let off: usize = sections
        .iter()
        .enumerate()
        .map(|(i, (start, fit))| {
            let end = sections.get(i + 1).map_or(points.len(), |(next, _)| *next);
            off_grid(&points[*start..end], fit)
        })
        .sum();
    if ((points.len() - off) as f64) < MIN_ON_GRID_SHARE * points.len() as f64 {
        return None;
    }
    let (_, first) = *sections.first()?;
    let first_beat_ms = first.origin.rem_euclid(first.period);
    let tempo_changes = (sections.len() > 1).then(|| {
        sections[1..]
            .iter()
            .map(|(start, fit)| TempoChange {
                position_ms: round_to(fit.at(points[*start].1), 10.0),
                bpm: round_to(60_000.0 / fit.period, 10_000.0),
            })
            .collect()
    });
    Some(TrackBeatGrid {
        first_beat_ms: round_to(first_beat_ms, 10.0),
        bpm: round_to(60_000.0 / first.period, 10_000.0),
        tempo_changes,
    })
}

/// Numbers each beat (ms) from the previous kept one, so a tempo hint slightly off never
/// accumulates into a wrong number. A beat that falls between two grid lines is dropped, unless
/// `DRIFT_RUN` of them in a row agree with each other: the phase really moved (an edit), and the
/// numbering follows it from there. Returns `(beat number, time ms)` pairs.
fn number_beats(beats: &[f64], period: f64) -> Vec<(f64, f64)> {
    let mut points: Vec<(f64, f64)> = Vec::with_capacity(beats.len());
    let mut pending: Vec<f64> = Vec::new();
    for &t in beats {
        let Some(&(n_last, t_last)) = points.last() else {
            points.push((0.0, t));
            continue;
        };
        let steps = (t - t_last) / period;
        let whole = steps.round();
        if whole >= 1.0 && (steps - whole).abs() <= MAX_BEAT_FRACTION {
            points.push((n_last + whole, t));
            pending.clear();
            continue;
        }
        if whole < 1.0 && (steps - whole).abs() <= MAX_BEAT_FRACTION {
            continue; // the same beat twice
        }
        let follows = pending.last().is_none_or(|p| {
            let gap = (t - p) / period;
            gap.round() >= 1.0 && (gap - gap.round()).abs() <= MAX_BEAT_FRACTION
        });
        if !follows {
            pending.clear();
        }
        pending.push(t);
        if pending.len() >= DRIFT_RUN {
            let mut n = n_last + ((pending[0] - t_last) / period).ceil();
            let mut previous = pending[0];
            for &p in &pending {
                n += ((p - previous) / period).round();
                points.push((n, p));
                previous = p;
            }
            pending.clear();
        }
    }
    points
}

/// A straight grid: line `n` falls at `origin + n * period` (ms).
#[derive(Debug, Clone, Copy)]
struct LineFit {
    origin: f64,
    period: f64,
}

impl LineFit {
    /// The grid line nearest to `t`.
    fn at(&self, t: f64) -> f64 {
        self.origin + ((t - self.origin) / self.period).round() * self.period
    }

    /// Distance of a numbered beat to its own grid line.
    fn residual(&self, (n, t): (f64, f64)) -> f64 {
        t - (self.origin + n * self.period)
    }
}

/// Least-squares grid through numbered beats, refitted once without the beats it leaves off the
/// grid so that a few stray onsets do not tilt it.
fn fit_section(points: &[(f64, f64)]) -> Option<LineFit> {
    let fit = least_squares(points.iter().copied())?;
    let fit = least_squares(
        points
            .iter()
            .copied()
            .filter(|p| fit.residual(*p).abs() <= OFF_GRID_MS),
    )
    .unwrap_or(fit);
    (fit.period.is_finite() && fit.period > 0.0).then_some(fit)
}

fn least_squares(points: impl Iterator<Item = (f64, f64)> + Clone) -> Option<LineFit> {
    let count = points.clone().count();
    if count < 2 {
        return None;
    }
    let count = count as f64;
    let mean_n = points.clone().map(|(n, _)| n).sum::<f64>() / count;
    let mean_t = points.clone().map(|(_, t)| t).sum::<f64>() / count;
    let (mut cov, mut var) = (0.0, 0.0);
    for (n, t) in points {
        cov += (n - mean_n) * (t - mean_t);
        var += (n - mean_n) * (n - mean_n);
    }
    if var <= 0.0 {
        return None;
    }
    let period = cov / var;
    Some(LineFit {
        origin: mean_t - period * mean_n,
        period,
    })
}

fn off_grid(points: &[(f64, f64)], fit: &LineFit) -> usize {
    points
        .iter()
        .filter(|p| fit.residual(**p).abs() > OFF_GRID_MS)
        .count()
}

/// True when `DRIFT_RUN` consecutive beats sit off the grid.
fn drifts(points: &[(f64, f64)], fit: &LineFit) -> bool {
    let mut run = 0;
    for p in points {
        if fit.residual(*p).abs() > OFF_GRID_MS {
            run += 1;
            if run >= DRIFT_RUN {
                return true;
            }
        } else {
            run = 0;
        }
    }
    false
}

/// Binary segmentation: a section whose beats drift off its own grid is cut where two grids fit
/// best, as long as the cut at least halves the beats left off the grid (a real tempo or phase
/// change, not a noisy passage). Only drifting tracks pay for the search. `offset` is the index
/// of `points[0]` in the whole track.
fn split_sections(points: &[(f64, f64)], offset: usize, out: &mut Vec<(usize, LineFit)>) {
    let Some(fit) = fit_section(points) else {
        return;
    };
    if points.len() < 2 * MIN_SECTION_BEATS || !drifts(points, &fit) {
        out.push((offset, fit));
        return;
    }
    // Truncated squares: a stray onset weighs no more than any other off-grid beat.
    let cost = |range: &[(f64, f64)]| -> f64 {
        fit_section(range)
            .map(|f| {
                range
                    .iter()
                    .map(|p| f.residual(*p).abs().min(OFF_GRID_MS).powi(2))
                    .sum()
            })
            .unwrap_or(f64::INFINITY)
    };
    let best = (MIN_SECTION_BEATS..=points.len() - MIN_SECTION_BEATS)
        .map(|cut| (cut, cost(&points[..cut]) + cost(&points[cut..])))
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(cut, _)| cut);
    let improves = best.is_some_and(|cut| {
        let left = fit_section(&points[..cut]).map(|f| off_grid(&points[..cut], &f));
        let right = fit_section(&points[cut..]).map(|f| off_grid(&points[cut..], &f));
        matches!((left, right), (Some(l), Some(r)) if 2 * (l + r) <= off_grid(points, &fit))
    });
    match best.filter(|_| improves) {
        Some(cut) => {
            split_sections(&points[..cut], offset, out);
            split_sections(&points[cut..], offset + cut, out);
        }
        None => out.push((offset, fit)),
    }
}

fn round_to(value: f64, scale: f64) -> f64 {
    (value * scale).round() / scale
}

/// Stores a track's grid, or clears it when the analysis found no pulse. A local column: no HLC
/// and no dirty marking, the grid does not travel through cloud sync.
pub fn store_beat_grid(
    conn: &Connection,
    track_id: &str,
    grid: Option<&TrackBeatGrid>,
) -> Result<()> {
    let (first_beat_ms, bpm, changes) = grid_columns(grid)?;
    conn.execute(
        "UPDATE tracks SET beatgrid_first_beat_ms = ?1, beatgrid_bpm = ?2, \
         beatgrid_tempo_changes = ?3 WHERE id = ?4",
        rusqlite::params![first_beat_ms, bpm, changes, track_id],
    )?;
    Ok(())
}

/// The three column values for a grid (all `NULL` for no grid). Tempo changes are a compact JSON
/// array of `[position_ms, bpm]` pairs.
pub fn grid_columns(
    grid: Option<&TrackBeatGrid>,
) -> Result<(Option<f64>, Option<f64>, Option<String>)> {
    let Some(grid) = grid else {
        return Ok((None, None, None));
    };
    let changes = match &grid.tempo_changes {
        Some(changes) => Some(
            serde_json::to_string(
                &changes
                    .iter()
                    .map(|c| [c.position_ms, c.bpm])
                    .collect::<Vec<_>>(),
            )
            .map_err(|e| CrateError::Analysis(format!("beat grid: {e}")))?,
        ),
        None => None,
    };
    Ok((Some(grid.first_beat_ms), Some(grid.bpm), changes))
}

/// A track's stored grid; `None` when it was never analysed by Crate, had no pulse, or is unknown.
pub fn load_beat_grid(conn: &Connection, track_id: &str) -> Result<Option<TrackBeatGrid>> {
    let row: Option<(Option<f64>, Option<f64>, Option<String>)> = conn
        .query_row(
            "SELECT beatgrid_first_beat_ms, beatgrid_bpm, beatgrid_tempo_changes \
             FROM tracks WHERE id = ?1",
            [track_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    let Some((Some(first_beat_ms), Some(bpm), changes)) = row else {
        return Ok(None);
    };
    let tempo_changes = match changes {
        Some(json) => Some(
            serde_json::from_str::<Vec<[f64; 2]>>(&json)
                .map_err(|e| CrateError::Analysis(format!("stored beat grid: {e}")))?
                .into_iter()
                .map(|[position_ms, bpm]| TempoChange { position_ms, bpm })
                .collect(),
        ),
        None => None,
    };
    Ok(Some(TrackBeatGrid {
        first_beat_ms,
        bpm,
        tempo_changes,
    }))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::f32::consts::PI;

    /// Mono audio of `seconds` with a short decaying 2 kHz click of `amplitude` at each time (s).
    pub(crate) fn clicks(rate: u32, times_s: &[f64], amplitude: f32, seconds: f64) -> Vec<f32> {
        let mut samples = vec![0.0f32; (seconds * rate as f64) as usize];
        add_clicks(&mut samples, rate, times_s, amplitude);
        samples
    }

    fn add_clicks(samples: &mut [f32], rate: u32, times_s: &[f64], amplitude: f32) {
        let click_len = (0.02 * rate as f64) as usize;
        for t in times_s {
            let start = (t * rate as f64).round() as usize;
            for i in 0..click_len {
                if let Some(s) = samples.get_mut(start + i) {
                    let x = i as f32 / rate as f32;
                    *s += amplitude * (-x * 250.0).exp() * (2.0 * PI * 2000.0 * x).sin();
                }
            }
        }
    }

    /// Click track: silence until `first_click_s`, then a click on every beat at `bpm`.
    pub(crate) fn click_track(rate: u32, bpm: f64, first_click_s: f64, seconds: f64) -> Vec<f32> {
        let beats = (0..)
            .map(|i| first_click_s + i as f64 * 60.0 / bpm)
            .take_while(|t| *t < seconds)
            .collect::<Vec<_>>();
        clicks(rate, &beats, 0.8, seconds)
    }

    /// Beat times (s) of a steady grid: `count` beats from `first_s` at `bpm`.
    fn steady(first_s: f64, bpm: f64, count: usize) -> Vec<f64> {
        (0..count)
            .map(|i| first_s + i as f64 * 60.0 / bpm)
            .collect()
    }

    #[test]
    fn steady_beats_give_the_first_beat_and_a_decimal_bpm() {
        let grid = fit_beat_grid(&steady(0.137, 123.45, 200), 123.0).unwrap();
        assert!((grid.bpm - 123.45).abs() < 0.001, "{grid:?}");
        assert!((grid.first_beat_ms - 137.0).abs() < 0.2, "{grid:?}");
        assert_eq!(grid.tempo_changes, None);
    }

    #[test]
    fn the_first_beat_is_folded_back_to_the_start_of_the_file() {
        // The music starts after 30 s: the grid still starts within one beat of 0.
        let grid = fit_beat_grid(&steady(30.25, 120.0, 100), 120.0).unwrap();
        assert!((grid.first_beat_ms - 250.0).abs() < 0.2, "{grid:?}");
    }

    #[test]
    fn missed_doubled_and_jittered_beats_do_not_bend_the_grid() {
        let mut beats: Vec<f64> = steady(0.5, 128.0, 300)
            .into_iter()
            .enumerate()
            .filter(|(i, _)| i % 17 != 3) // missed beats
            .map(|(i, t)| t + if i % 2 == 0 { 0.008 } else { -0.008 }) // onset jitter
            .collect();
        beats.push(10.0 + 0.17); // a stray onset between two beats
        beats.sort_by(f64::total_cmp);
        let grid = fit_beat_grid(&beats, 128.0).unwrap();
        assert!((grid.bpm - 128.0).abs() < 0.01, "{grid:?}");
        // 500 ms folded back by one 468.75 ms beat.
        assert!((grid.first_beat_ms - 31.25).abs() < 2.0, "{grid:?}");
        assert_eq!(grid.tempo_changes, None);
    }

    /// The new section starts at the first beat of the new tempo, or at the last beat of the old
    /// one (that line belongs to both grids); either way its grid lands on `first_new_beat_ms`.
    fn assert_change_lands_on(position_ms: f64, first_new_beat_ms: f64, bpm: f64) {
        let beat = 60_000.0 / bpm;
        let beats = (first_new_beat_ms - position_ms) / beat;
        assert!(
            (0.0..=1.0 + 1e-6).contains(&beats.round())
                && (beats - beats.round()).abs() * beat < 5.0,
            "change at {position_ms} ms, first beat of the new tempo at {first_new_beat_ms} ms"
        );
    }

    #[test]
    fn a_tempo_change_is_recorded_as_a_second_section() {
        let mut beats = steady(0.1, 120.0, 64);
        // One 126 BPM beat after the last 120 BPM one: the first beat off the old grid.
        let change_at = beats.last().unwrap() + 60.0 / 126.0;
        beats.extend(steady(change_at, 126.0, 64));
        let grid = fit_beat_grid(&beats, 123.0).unwrap();
        assert!((grid.bpm - 120.0).abs() < 0.01, "{grid:?}");
        assert!((grid.first_beat_ms - 100.0).abs() < 1.0, "{grid:?}");
        let changes = grid.tempo_changes.expect("a tempo change");
        assert_eq!(changes.len(), 1, "{changes:?}");
        assert!((changes[0].bpm - 126.0).abs() < 0.01, "{changes:?}");
        assert_change_lands_on(changes[0].position_ms, change_at * 1000.0, 126.0);
    }

    #[test]
    fn too_few_beats_give_no_grid() {
        assert_eq!(fit_beat_grid(&steady(0.0, 120.0, 4), 120.0), None);
        assert_eq!(fit_beat_grid(&[], 120.0), None);
        assert_eq!(fit_beat_grid(&steady(0.0, 120.0, 50), 0.0), None);
    }

    const RATE: u32 = 44_100;

    #[test]
    fn a_click_track_gives_its_decimal_tempo_and_first_beat() {
        // The analysed tempo is rounded and 0.4 BPM off: the grid still finds 127.6.
        let samples = click_track(RATE, 127.6, 0.2134, 60.0);
        let grid = detect_beat_grid(&samples, RATE, 128.0).unwrap();
        assert!((grid.bpm - 127.6).abs() < 0.01, "{grid:?}");
        assert!((grid.first_beat_ms - 213.4).abs() < 3.0, "{grid:?}");
        assert_eq!(grid.tempo_changes, None);
    }

    #[test]
    fn louder_off_beats_in_a_bar_do_not_move_the_grid() {
        let bpm = 124.0;
        let beat = 60.0 / bpm;
        let on: Vec<f64> = (0..240).map(|i| 1.0 + i as f64 * beat).collect();
        // Off-beat hats everywhere, louder than the kick for bars 20 to 24.
        let mut samples = clicks(RATE, &on, 0.5, 120.0);
        let quiet: Vec<f64> = on.iter().map(|t| t + beat / 2.0).collect();
        add_clicks(&mut samples, RATE, &quiet, 0.2);
        let loud: Vec<f64> = quiet[80..96].to_vec();
        add_clicks(&mut samples, RATE, &loud, 0.6);
        let grid = detect_beat_grid(&samples, RATE, bpm).unwrap();
        assert!((grid.bpm - bpm).abs() < 0.01, "{grid:?}");
        let first = (1000.0_f64).rem_euclid(beat * 1000.0);
        assert!((grid.first_beat_ms - first).abs() < 3.0, "{grid:?}");
        assert_eq!(grid.tempo_changes, None);
    }

    #[test]
    fn a_click_track_that_speeds_up_records_the_change() {
        let mut beats: Vec<f64> = (0..64).map(|i| 0.5 + i as f64 * 0.5).collect();
        let change_at = beats.last().unwrap() + 60.0 / 126.0;
        beats.extend((0..64).map(|i| change_at + i as f64 * 60.0 / 126.0));
        let samples = clicks(RATE, &beats, 0.8, 70.0);
        let grid = detect_beat_grid(&samples, RATE, 123.0).unwrap();
        assert!((grid.bpm - 120.0).abs() < 0.02, "{grid:?}");
        assert!((grid.first_beat_ms - 500.0).abs() < 3.0, "{grid:?}");
        let changes = grid.tempo_changes.expect("a tempo change");
        assert_eq!(changes.len(), 1, "{changes:?}");
        assert!((changes[0].bpm - 126.0).abs() < 0.05, "{changes:?}");
        assert_change_lands_on(changes[0].position_ms, change_at * 1000.0, 126.0);
    }

    #[test]
    fn silence_and_noise_give_no_grid() {
        assert_eq!(
            detect_beat_grid(&vec![0.0; RATE as usize * 20], RATE, 120.0),
            None
        );
        assert_eq!(detect_beat_grid(&[], RATE, 120.0), None);
        for mut seed in [7u32, 42, 2026] {
            let noise: Vec<f32> = (0..RATE * 30)
                .map(|_| {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    (seed >> 8) as f32 / (1u32 << 24) as f32 * 0.2 - 0.1
                })
                .collect();
            assert_eq!(detect_beat_grid(&noise, RATE, 120.0), None, "seed {seed}");
            assert_eq!(detect_beat_grid(&noise, 0, 120.0), None);
        }
    }

    fn db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO tracks (id, file_path, duration_ms, date_added, date_modified) \
             VALUES ('t1', '/music/a.wav', 1000, '2020-01-01', '2020-01-01')",
            [],
        )
        .unwrap();
        conn
    }

    #[test]
    fn a_track_without_a_grid_reads_as_none() {
        let conn = db();
        assert_eq!(load_beat_grid(&conn, "t1").unwrap(), None);
        assert_eq!(load_beat_grid(&conn, "unknown").unwrap(), None);
    }

    #[test]
    fn a_stored_grid_reads_back_and_can_be_cleared() {
        let conn = db();
        let grid = TrackBeatGrid {
            first_beat_ms: 12.5,
            bpm: 127.98,
            tempo_changes: Some(vec![TempoChange {
                position_ms: 61_000.0,
                bpm: 130.0,
            }]),
        };
        store_beat_grid(&conn, "t1", Some(&grid)).unwrap();
        assert_eq!(load_beat_grid(&conn, "t1").unwrap(), Some(grid));
        let stored: String = conn
            .query_row(
                "SELECT beatgrid_tempo_changes FROM tracks WHERE id = 't1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored, "[[61000.0,130.0]]");

        store_beat_grid(&conn, "t1", None).unwrap();
        assert_eq!(load_beat_grid(&conn, "t1").unwrap(), None);
    }
}
