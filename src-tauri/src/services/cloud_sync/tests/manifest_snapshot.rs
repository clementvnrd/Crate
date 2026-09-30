//! The local manifest is recomputed on every push (and again by the pull-then-merge step),
//! so it has to stay cheap. These tests pin what must NOT change when it is made cheaper:
//! which shard a track lands in, what each bucket counts and reports as its watermark, and
//! that the bytes hashed for the manifest are the bytes that get uploaded.

use rusqlite::{params, Connection};

use crate::services::cloud_sync::hlc::Hlc;
use crate::services::cloud_sync::pipeline::buckets::{self, shard_for_track_id, Bucket};
use crate::services::cloud_sync::pipeline::manifest::{
    compute_local_manifest, compute_local_manifest_with,
};
use crate::services::cloud_sync::pipeline::{dirty, rows};

use super::new_device;

/// Ids covering every nibble in both cases, plus the ones the shard rule sends to shard 0
/// for another reason: not a hex digit, not ASCII, or empty.
const IDS: &[&str] = &[
    "0a", "1b", "2c", "3d", "4e", "5f", "6a", "7b", "8c", "9d", "aa", "Ab", "bb", "Bc", "cc", "Cd",
    "dd", "De", "ee", "Ee", "ff", "Fa", "g1", "zz", "_x", "é1", "ß2", "日本", "", " 3",
];

/// Ids that only exist as tombstones (deleted tracks), again across shards and cases.
const GONE: &[&str] = &["a-gone", "A-gone", "7-gone", "f-gone", "x-gone"];

fn hlc(wall: u64) -> String {
    Hlc::new(wall, 0, 1).format()
}

fn seeded_device() -> Connection {
    let conn = new_device(1);
    for (i, id) in IDS.iter().enumerate() {
        conn.execute(
            "INSERT INTO tracks (id, file_path, duration_ms, title, date_added, date_modified, _hlc) \
             VALUES (?1, ?2, 1000, ?3, '2020-01-01T00:00:00Z', '2020-01-01T00:00:00Z', ?4)",
            params![id, format!("/music/{i}.mp3"), format!("Track {i}"), hlc(1000 + i as u64)],
        )
        .unwrap();
    }
    for (i, id) in GONE.iter().enumerate() {
        // Newer than every live track, so a tombstone sets the watermark of its shard.
        dirty::record_tombstone(&conn, buckets::TRACKS_ENTITY, id, &hlc(5000 + i as u64)).unwrap();
    }
    conn
}

fn ids_of(blob: &[u8]) -> Vec<String> {
    let mut ids: Vec<String> = blob
        .split(|&b| b == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            let value: serde_json::Value = serde_json::from_slice(line).unwrap();
            value["id"].as_str().unwrap().to_string()
        })
        .collect();
    ids.sort();
    ids
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

#[test]
fn every_track_lands_in_exactly_the_shard_the_rule_names() {
    let conn = seeded_device();
    let mut seen = 0;
    for n in 0u8..16 {
        let blob = rows::serialize_bucket(&conn, &Bucket::Tracks(n)).unwrap();
        let expected = sorted(
            IDS.iter()
                .chain(GONE.iter())
                .filter(|id| shard_for_track_id(id) == n)
                .map(|id| id.to_string())
                .collect(),
        );
        assert_eq!(ids_of(&blob), expected, "shard {n:x}");
        seen += expected.len();
    }
    assert_eq!(
        seen,
        IDS.len() + GONE.len(),
        "no track or tombstone is lost or doubled"
    );
}

#[test]
fn counts_and_watermarks_follow_the_shard_rule() {
    let conn = seeded_device();
    let manifest = compute_local_manifest(&conn, "dev").unwrap();
    for n in 0u8..16 {
        let entry = &manifest.buckets[&Bucket::Tracks(n).as_str()];

        let live: Vec<usize> = IDS
            .iter()
            .enumerate()
            .filter(|(_, id)| shard_for_track_id(id) == n)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(entry.count, live.len() as u64, "live count of shard {n:x}");

        let newest_live = live.iter().map(|i| hlc(1000 + *i as u64)).max();
        let newest_tomb = GONE
            .iter()
            .enumerate()
            .filter(|(_, id)| shard_for_track_id(id) == n)
            .map(|(i, _)| hlc(5000 + i as u64))
            .max();
        let expected = [newest_live, newest_tomb]
            .into_iter()
            .flatten()
            .max()
            .unwrap_or_default();
        assert_eq!(entry.hlc, expected, "watermark of shard {n:x}");
    }
}

#[test]
fn the_blob_callback_sees_every_bucket_once_with_the_bytes_that_were_hashed() {
    let conn = seeded_device();
    let mut seen: Vec<(String, String, Vec<u8>)> = Vec::new();
    let manifest = compute_local_manifest_with(&conn, "dev", |name, hash, bytes| {
        seen.push((name.to_string(), hash.to_string(), bytes));
    })
    .unwrap();

    assert_eq!(seen.len(), Bucket::all().len(), "one callback per bucket");
    for (name, hash, bytes) in &seen {
        let bucket = Bucket::parse(name).expect("a real bucket name");
        assert_eq!(
            &rows::bucket_hash(bytes),
            hash,
            "the hash is of these very bytes"
        );
        assert_eq!(
            &manifest.buckets[name].blob_hash, hash,
            "and it is the manifest's hash"
        );
        assert_eq!(
            bytes,
            &rows::serialize_bucket(&conn, &bucket).unwrap(),
            "a fresh serialization gives the same bytes, so reusing them uploads the same blob"
        );
    }
    assert_eq!(
        manifest,
        compute_local_manifest(&conn, "dev").unwrap(),
        "the plain manifest is the callback one with the bytes thrown away"
    );
}
