//! Waveform overview: peak amplitudes computed once from the audio file and cached in
//! `tracks.waveform_data` (one byte per bar, 0–100).

use std::path::Path;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Number of bars stored per track.
pub const WAVEFORM_BARS: usize = 400;

/// Decodes the whole file and returns `bins` peak values (0–100), or None if it cannot be decoded.
pub fn compute_peaks(path: &Path, bins: usize) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;
    let mut format = probed.format;
    let track = format.default_track()?;
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .ok()?;

    // Peak (absolute max) of every decoded packet, then folded into `bins` buckets.
    let mut packet_peaks: Vec<f32> = Vec::new();
    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                break
            }
            Err(_) => break,
        };
        if packet.track_id() != track_id {
            continue;
        }
        let Ok(decoded) = decoder.decode(&packet) else {
            continue;
        };
        let buf = sample_buf
            .get_or_insert_with(|| SampleBuffer::new(decoded.capacity() as u64, *decoded.spec()));
        if buf.capacity() < decoded.capacity() {
            *buf = SampleBuffer::new(decoded.capacity() as u64, *decoded.spec());
        }
        buf.copy_interleaved_ref(decoded);
        let peak = buf.samples().iter().fold(0.0f32, |max, s| max.max(s.abs()));
        packet_peaks.push(peak);
    }

    if packet_peaks.is_empty() || bins == 0 {
        return None;
    }
    Some(fold_peaks(&packet_peaks, bins))
}

/// Folds per-packet peaks into `bins` values scaled to 0–100 (max within each bucket).
fn fold_peaks(peaks: &[f32], bins: usize) -> Vec<u8> {
    (0..bins)
        .map(|bin| {
            let start = bin * peaks.len() / bins;
            let end = ((bin + 1) * peaks.len() / bins)
                .max(start + 1)
                .min(peaks.len());
            let peak = peaks[start.min(peaks.len() - 1)..end]
                .iter()
                .fold(0.0f32, |m, p| m.max(*p));
            (peak.clamp(0.0, 1.0) * 100.0).round() as u8
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 16-bit stereo PCM WAV: first half silent, second half a half-scale square wave.
    fn write_test_wav(path: &Path) {
        let rate = 44_100u32;
        let frames = rate as usize; // 1 s
        let mut data = Vec::with_capacity(frames * 4);
        for i in 0..frames {
            let v: i16 = if i < frames / 2 {
                0
            } else if (i / 50) % 2 == 0 {
                16_384
            } else {
                -16_384
            };
            data.extend(v.to_le_bytes());
            data.extend(v.to_le_bytes());
        }
        let mut wav = Vec::new();
        wav.extend(b"RIFF");
        wav.extend((36 + data.len() as u32).to_le_bytes());
        wav.extend(b"WAVEfmt ");
        wav.extend(16u32.to_le_bytes());
        wav.extend(1u16.to_le_bytes()); // PCM
        wav.extend(2u16.to_le_bytes()); // stereo
        wav.extend(rate.to_le_bytes());
        wav.extend((rate * 4).to_le_bytes());
        wav.extend(4u16.to_le_bytes());
        wav.extend(16u16.to_le_bytes());
        wav.extend(b"data");
        wav.extend((data.len() as u32).to_le_bytes());
        wav.extend(data);
        std::fs::write(path, wav).unwrap();
    }

    #[test]
    fn test_peaks_follow_the_audio() {
        let path = std::env::temp_dir().join(format!("crate_waveform_{}.wav", std::process::id()));
        write_test_wav(&path);
        let peaks = compute_peaks(&path, 10).unwrap();
        let _ = std::fs::remove_file(&path);
        assert_eq!(peaks.len(), 10);
        assert!(peaks[..4].iter().all(|p| *p == 0), "silent half: {peaks:?}");
        assert!(
            peaks[6..].iter().all(|p| (45..=55).contains(p)),
            "half-scale half: {peaks:?}"
        );
    }

    #[test]
    fn test_undecodable_file_has_no_waveform() {
        let path =
            std::env::temp_dir().join(format!("crate_waveform_bad_{}.mp3", std::process::id()));
        std::fs::write(&path, b"not audio at all").unwrap();
        assert!(compute_peaks(&path, 10).is_none());
        let _ = std::fs::remove_file(&path);
    }
}
