//! Local manifest computation and diffing — backend-agnostic (operates on the
//! plain [`Manifest`] type, never on Firestore).
//!
//! **INCLUDE-ALL policy:** every `Bucket::all()` gets an entry, even when empty.
//! An empty bucket serializes to zero bytes, whose blake3 is a fixed value, so two
//! devices' empty buckets share a hash and never spuriously diff. Diffing keys on
//! `blob_hash` only (`count`/`hlc` are diagnostics).

use std::collections::{BTreeMap, BTreeSet};

use rusqlite::Connection;

use crate::error::Result;

use super::super::backend::types::{BucketEntry, Manifest, MANIFEST_FORMAT_VERSION};
use super::buckets::{self, Bucket};
use super::rows;

/// The Cloud Storage object key a bucket blob lives under. Phase 1 mock stores raw
/// JSONL under the same key (the `.jsonl.gz` suffix is kept so it matches Phase 2).
pub fn object_key(uid: &str, bucket: &Bucket, blob_hash: &str) -> String {
    format!(
        "users/{uid}/vault/{}-{}.jsonl.gz",
        bucket.as_str(),
        blob_hash
    )
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ManifestDiff {
    /// Bucket names whose local blob differs from (or is missing on) the remote.
    pub to_upload: Vec<String>,
    /// Bucket names whose remote blob differs from (or is missing on) local.
    pub to_download: Vec<String>,
}

/// Compute this device's manifest over every bucket.
pub fn compute_local_manifest(conn: &Connection, device_id: &str) -> Result<Manifest> {
    compute_local_manifest_with(conn, device_id, |_, _, _| {})
}

/// Like [`compute_local_manifest`], but hands each bucket's serialized bytes to
/// `on_blob(bucket name, blob hash, bytes)` right after hashing them. Serializing a bucket is
/// the expensive part of the manifest, so a caller that is going to upload the changed buckets
/// keeps the bytes it needs here instead of serializing them a second time. The callback owns
/// the bytes and decides what to keep; the rest are dropped bucket by bucket, so the whole
/// library is never held in memory at once.
pub fn compute_local_manifest_with(
    conn: &Connection,
    device_id: &str,
    mut on_blob: impl FnMut(&str, &str, Vec<u8>),
) -> Result<Manifest> {
    let mut buckets_map = BTreeMap::new();
    let mut manifest_hlc = String::new();
    for bucket in Bucket::all() {
        let name = bucket.as_str();
        let bytes = rows::serialize_bucket(conn, &bucket)?;
        let blob_hash = rows::bucket_hash(&bytes);
        let count = count_live_rows(conn, &bucket)?;
        let hlc = rows::bucket_max_hlc(conn, &bucket)?;
        if hlc > manifest_hlc {
            manifest_hlc = hlc.clone();
        }
        on_blob(&name, &blob_hash, bytes);
        buckets_map.insert(
            name.clone(),
            BucketEntry {
                object_key: format!("{name}-{blob_hash}.jsonl.gz"),
                blob_hash,
                count,
                hlc,
            },
        );
    }
    Ok(Manifest {
        format_version: MANIFEST_FORMAT_VERSION,
        last_writer_device: device_id.to_string(),
        manifest_hlc,
        buckets: buckets_map,
    })
}

/// Which buckets must be uploaded (local differs) / downloaded (remote differs).
pub fn diff_manifest(local: &Manifest, remote: &Manifest) -> ManifestDiff {
    let mut diff = ManifestDiff::default();
    let mut names: BTreeSet<&String> = local.buckets.keys().collect();
    names.extend(remote.buckets.keys());
    for name in names {
        let l = local.buckets.get(name).map(|e| &e.blob_hash);
        let r = remote.buckets.get(name).map(|e| &e.blob_hash);
        match (l, r) {
            (Some(lh), Some(rh)) if lh == rh => {}
            (Some(_), Some(_)) => {
                diff.to_upload.push(name.clone());
                diff.to_download.push(name.clone());
            }
            (Some(_), None) => diff.to_upload.push(name.clone()),
            (None, Some(_)) => diff.to_download.push(name.clone()),
            (None, None) => {}
        }
    }
    diff
}

fn count_live_rows(conn: &Connection, bucket: &Bucket) -> Result<u64> {
    match bucket {
        Bucket::Settings => {
            let mut n = 0u64;
            for key in crate::services::cloud_sync::SYNCED_SETTING_KEYS {
                let present: bool = conn.query_row(
                    "SELECT EXISTS(SELECT 1 FROM settings WHERE key = ?1)",
                    [key],
                    |r| r.get(0),
                )?;
                if present {
                    n += 1;
                }
            }
            Ok(n)
        }
        Bucket::Tracks(shard) => {
            let c: i64 = conn.query_row(
                &format!(
                    "SELECT COUNT(*) FROM tracks WHERE {}",
                    buckets::shard_sql_condition("id", *shard)
                ),
                [],
                |r| r.get(0),
            )?;
            Ok(c as u64)
        }
        _ => {
            let table = bucket.table();
            let c: i64 =
                conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?;
            Ok(c as u64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::cloud_sync::backend::types::BucketEntry;

    fn manifest_with(entries: &[(&str, &str)]) -> Manifest {
        let mut m = Manifest::empty("dev");
        for (name, hash) in entries {
            m.buckets.insert(
                name.to_string(),
                BucketEntry {
                    blob_hash: hash.to_string(),
                    object_key: format!("{name}-{hash}.jsonl.gz"),
                    count: 0,
                    hlc: String::new(),
                },
            );
        }
        m
    }

    #[test]
    fn identical_manifests_have_no_diff() {
        let a = manifest_with(&[("playlists", "h1"), ("tags", "h2")]);
        let b = manifest_with(&[("playlists", "h1"), ("tags", "h2")]);
        assert_eq!(diff_manifest(&a, &b), ManifestDiff::default());
    }

    #[test]
    fn differing_hash_uploads_and_downloads() {
        let local = manifest_with(&[("playlists", "h1")]);
        let remote = manifest_with(&[("playlists", "h2")]);
        let d = diff_manifest(&local, &remote);
        assert_eq!(d.to_upload, vec!["playlists".to_string()]);
        assert_eq!(d.to_download, vec!["playlists".to_string()]);
    }

    #[test]
    fn one_sided_buckets_go_the_right_way() {
        let local = manifest_with(&[("playlists", "h1")]);
        let remote = manifest_with(&[("tags", "h2")]);
        let d = diff_manifest(&local, &remote);
        assert_eq!(d.to_upload, vec!["playlists".to_string()]);
        assert_eq!(d.to_download, vec!["tags".to_string()]);
    }
}
