use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::error::{CrateError, Result};
use crate::models::{Tag, TagCategory};
use crate::services::cloud_sync::pipeline::{buckets, dirty};

pub struct TagService {
    conn: Arc<Mutex<Connection>>,
}

impl TagService {
    pub fn new(conn: Arc<Mutex<Connection>>) -> Self {
        Self { conn }
    }

    pub fn get_categories(&self) -> Result<Vec<TagCategory>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        // Get categories
        let mut stmt = conn.prepare(
            "SELECT id, name, color, sort_order FROM tag_categories ORDER BY sort_order, name",
        )?;

        let categories: Vec<TagCategory> = stmt
            .query_map([], |row| {
                Ok(TagCategory {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    sort_order: row.get(3)?,
                    tags: Vec::new(),
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        // Get all tags
        let mut stmt = conn.prepare(
            "SELECT id, category_id, name, color, sort_order FROM tags ORDER BY sort_order, name",
        )?;

        let tags: Vec<Tag> = stmt
            .query_map([], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    category_id: row.get(1)?,
                    name: row.get(2)?,
                    color: row.get(3)?,
                    sort_order: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        // Group tags by category
        let mut result = categories;
        for tag in tags {
            if let Some(cat) = result.iter_mut().find(|c| c.id == tag.category_id) {
                cat.tags.push(tag);
            }
        }

        Ok(result)
    }

    pub fn create_category(&self, name: String, color: Option<String>) -> Result<TagCategory> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        // Check category count (max 4)
        let count: i32 =
            conn.query_row("SELECT COUNT(*) FROM tag_categories", [], |row| row.get(0))?;

        if count >= 4 {
            return Err(CrateError::InvalidOperation(
                "Maximum of 4 tag categories allowed".to_string(),
            ));
        }

        // Get next sort order
        let max_order: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), -1) FROM tag_categories",
                [],
                |row| row.get(0),
            )
            .unwrap_or(-1);

        // Default color if not provided
        let category_color = color.or_else(|| Some("#6366f1".to_string()));

        let category = TagCategory {
            id: uuid::Uuid::new_v4().to_string(),
            name,
            color: category_color,
            sort_order: max_order + 1,
            tags: Vec::new(),
        };

        let hlc = dirty::next_hlc(&conn)?;
        conn.execute(
            "INSERT INTO tag_categories (id, name, color, sort_order, _hlc) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                category.id,
                category.name,
                category.color,
                category.sort_order,
                hlc
            ],
        )?;
        dirty::mark_dirty(&conn, buckets::TAG_CATEGORIES)?;

        Ok(category)
    }

    pub fn update_category(
        &self,
        id: &str,
        name: Option<String>,
        color: Option<String>,
    ) -> Result<TagCategory> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let hlc = dirty::next_hlc(&conn)?;
        if let Some(ref n) = name {
            conn.execute(
                "UPDATE tag_categories SET name = ?1, _hlc = ?2 WHERE id = ?3",
                rusqlite::params![n, hlc, id],
            )?;
        }

        if let Some(ref c) = color {
            conn.execute(
                "UPDATE tag_categories SET color = ?1, _hlc = ?2 WHERE id = ?3",
                rusqlite::params![c, hlc, id],
            )?;
        }
        dirty::mark_dirty(&conn, buckets::TAG_CATEGORIES)?;

        drop(conn);
        self.get_category(id)
    }

    pub fn delete_category(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let hlc = dirty::next_hlc(&conn)?;
        dirty::record_tombstone(&conn, buckets::TAG_CATEGORIES, id, &hlc)?;
        conn.execute("DELETE FROM tag_categories WHERE id = ?1", [id])?;
        // Cascade removes this category's tags + their track/discovery links;
        // re-serialize those buckets so peers don't re-insert orphaned rows.
        dirty::mark_dirty(&conn, buckets::TAG_CATEGORIES)?;
        dirty::mark_dirty(&conn, buckets::TAGS)?;
        dirty::mark_dirty(&conn, buckets::TRACK_TAGS)?;
        dirty::mark_dirty(&conn, buckets::DISCOVERY_RELEASE_TAGS)?;
        Ok(())
    }

    fn get_category(&self, id: &str) -> Result<TagCategory> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let mut category = conn.query_row(
            "SELECT id, name, color, sort_order FROM tag_categories WHERE id = ?1",
            [id],
            |row| {
                Ok(TagCategory {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    color: row.get(2)?,
                    sort_order: row.get(3)?,
                    tags: Vec::new(),
                })
            },
        )?;

        // Get tags for this category
        let mut stmt = conn.prepare(
            "SELECT id, category_id, name, color, sort_order FROM tags WHERE category_id = ?1 ORDER BY sort_order, name",
        )?;

        category.tags = stmt
            .query_map([id], |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    category_id: row.get(1)?,
                    name: row.get(2)?,
                    color: row.get(3)?,
                    sort_order: row.get(4)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;

        Ok(category)
    }

    pub fn create_tag(
        &self,
        category_id: String,
        name: String,
        color: Option<String>,
    ) -> Result<Tag> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        // Get next sort order
        let max_order: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), -1) FROM tags WHERE category_id = ?1",
                [&category_id],
                |row| row.get(0),
            )
            .unwrap_or(-1);

        let tag = Tag {
            id: uuid::Uuid::new_v4().to_string(),
            category_id,
            name,
            color,
            sort_order: max_order + 1,
        };

        let hlc = dirty::next_hlc(&conn)?;
        conn.execute(
            "INSERT INTO tags (id, category_id, name, color, sort_order, _hlc) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![tag.id, tag.category_id, tag.name, tag.color, tag.sort_order, hlc],
        )?;
        dirty::mark_dirty(&conn, buckets::TAGS)?;

        Ok(tag)
    }

    pub fn update_tag(&self, id: &str, name: Option<String>, color: Option<String>) -> Result<Tag> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let hlc = dirty::next_hlc(&conn)?;
        if let Some(ref n) = name {
            conn.execute(
                "UPDATE tags SET name = ?1, _hlc = ?2 WHERE id = ?3",
                rusqlite::params![n, hlc, id],
            )?;
        }

        if let Some(ref c) = color {
            conn.execute(
                "UPDATE tags SET color = ?1, _hlc = ?2 WHERE id = ?3",
                rusqlite::params![c, hlc, id],
            )?;
        }
        dirty::mark_dirty(&conn, buckets::TAGS)?;

        conn.query_row(
            "SELECT id, category_id, name, color, sort_order FROM tags WHERE id = ?1",
            [id],
            |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    category_id: row.get(1)?,
                    name: row.get(2)?,
                    color: row.get(3)?,
                    sort_order: row.get(4)?,
                })
            },
        )
        .map_err(|e| e.into())
    }

    pub fn move_tag(&self, tag_id: &str, target_category_id: &str) -> Result<Tag> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        // Get current tag
        let (current_category_id, tag_name): (String, String) = conn.query_row(
            "SELECT category_id, name FROM tags WHERE id = ?1",
            [tag_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        // No-op if same category
        if current_category_id == target_category_id {
            return conn
                .query_row(
                    "SELECT id, category_id, name, color, sort_order FROM tags WHERE id = ?1",
                    [tag_id],
                    |row| {
                        Ok(Tag {
                            id: row.get(0)?,
                            category_id: row.get(1)?,
                            name: row.get(2)?,
                            color: row.get(3)?,
                            sort_order: row.get(4)?,
                        })
                    },
                )
                .map_err(|e| e.into());
        }

        // Check for name collision in target category
        let collision: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM tags WHERE category_id = ?1 AND name = ?2)",
            rusqlite::params![target_category_id, tag_name],
            |row| row.get(0),
        )?;

        if collision {
            return Err(CrateError::InvalidOperation(
                "A tag with this name already exists in the target category".to_string(),
            ));
        }

        // Compute next sort_order in target category
        let max_order: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), -1) FROM tags WHERE category_id = ?1",
                [target_category_id],
                |row| row.get(0),
            )
            .unwrap_or(-1);

        // Update the tag
        let hlc = dirty::next_hlc(&conn)?;
        conn.execute(
            "UPDATE tags SET category_id = ?1, sort_order = ?2, _hlc = ?3 WHERE id = ?4",
            rusqlite::params![target_category_id, max_order + 1, hlc, tag_id],
        )?;
        dirty::mark_dirty(&conn, buckets::TAGS)?;

        // Return updated tag
        conn.query_row(
            "SELECT id, category_id, name, color, sort_order FROM tags WHERE id = ?1",
            [tag_id],
            |row| {
                Ok(Tag {
                    id: row.get(0)?,
                    category_id: row.get(1)?,
                    name: row.get(2)?,
                    color: row.get(3)?,
                    sort_order: row.get(4)?,
                })
            },
        )
        .map_err(|e| e.into())
    }

    pub fn delete_tag(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        // One transaction: the tombstone and the delete stand or fall together.
        let tx = conn.unchecked_transaction()?;
        let hlc = dirty::next_hlc(&tx)?;
        dirty::record_tombstone(&tx, buckets::TAGS, id, &hlc)?;
        tx.execute("DELETE FROM tags WHERE id = ?1", [id])?;
        // Cascade removes this tag's track/discovery links; re-serialize them.
        dirty::mark_dirty(&tx, buckets::TAGS)?;
        dirty::mark_dirty(&tx, buckets::TRACK_TAGS)?;
        dirty::mark_dirty(&tx, buckets::DISCOVERY_RELEASE_TAGS)?;
        tx.commit()?;
        Ok(())
    }

    pub fn assign_tags(&self, track_ids: Vec<String>, tag_ids: Vec<String>) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        // One transaction: tagging 500 tracks is all-or-nothing, and one disk sync instead of 500.
        let tx = conn.unchecked_transaction()?;
        let hlc = dirty::next_hlc(&tx)?;
        for track_id in &track_ids {
            for tag_id in &tag_ids {
                // OR IGNORE preserves an existing link's _hlc; new links are stamped.
                tx.execute(
                    "INSERT OR IGNORE INTO track_tags (track_id, tag_id, _hlc) VALUES (?1, ?2, ?3)",
                    rusqlite::params![track_id, tag_id, hlc],
                )?;
            }
        }
        dirty::mark_dirty(&tx, buckets::TRACK_TAGS)?;
        tx.commit()?;

        Ok(())
    }

    pub fn remove_tags(&self, track_ids: Vec<String>, tag_ids: Vec<String>) -> Result<()> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        // One transaction: a link is never deleted without its tombstone.
        let tx = conn.unchecked_transaction()?;
        let hlc = dirty::next_hlc(&tx)?;
        for track_id in &track_ids {
            for tag_id in &tag_ids {
                let deleted = tx.execute(
                    "DELETE FROM track_tags WHERE track_id = ?1 AND tag_id = ?2",
                    rusqlite::params![track_id, tag_id],
                )?;
                if deleted > 0 {
                    dirty::record_tombstone(
                        &tx,
                        buckets::TRACK_TAGS,
                        &dirty::junction_entity_id(track_id, tag_id),
                        &hlc,
                    )?;
                }
            }
        }
        dirty::mark_dirty(&tx, buckets::TRACK_TAGS)?;
        tx.commit()?;

        Ok(())
    }

    #[allow(dead_code)]
    pub fn get_tracks_by_tag(&self, tag_id: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().map_err(|_| CrateError::LockPoisoned)?;

        let mut stmt = conn.prepare("SELECT track_id FROM track_tags WHERE tag_id = ?1")?;

        let track_ids = stmt
            .query_map([tag_id], |row| row.get(0))?
            .collect::<std::result::Result<Vec<String>, _>>()?;

        Ok(track_ids)
    }
}

#[cfg(test)]
mod bulk_tests {
    use super::*;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    fn service() -> (TagService, Arc<Mutex<Connection>>) {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO tag_categories (id, name) VALUES ('c', 'Energy');
             INSERT INTO tags (id, category_id, name) VALUES ('g1', 'c', 'Peak'), ('g2', 'c', 'Warm');
             INSERT INTO tracks (id, file_path, format, title, artist, duration_ms, date_added, date_modified) VALUES
               ('t1', '/m/1.mp3', 'mp3', 'One', 'A', 1000, '2026-01-01', '2026-01-01'),
               ('t2', '/m/2.mp3', 'mp3', 'Two', 'A', 1000, '2026-01-01', '2026-01-01'),
               ('t3', '/m/3.mp3', 'mp3', 'Three', 'A', 1000, '2026-01-01', '2026-01-01');",
        )
        .unwrap();
        let conn = Arc::new(Mutex::new(conn));
        (TagService::new(conn.clone()), conn)
    }

    fn count(conn: &Arc<Mutex<Connection>>, sql: &str) -> i64 {
        conn.lock()
            .unwrap()
            .query_row(sql, [], |r| r.get(0))
            .unwrap()
    }

    fn ids(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn assigning_tags_to_many_tracks_is_all_or_nothing() {
        let (tags, conn) = service();
        // The write for the third track fails.
        conn.lock()
            .unwrap()
            .execute_batch(
                "CREATE TRIGGER refuse_t3 BEFORE INSERT ON track_tags WHEN NEW.track_id = 't3'
                 BEGIN SELECT RAISE(ABORT, 'simulated failure'); END;",
            )
            .unwrap();

        assert!(tags
            .assign_tags(ids(&["t1", "t2", "t3"]), ids(&["g1"]))
            .is_err());
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM track_tags"),
            0,
            "the two writes that worked are rolled back with the failed one"
        );
    }

    #[test]
    fn removing_tags_never_deletes_a_link_without_its_tombstone() {
        let (tags, conn) = service();
        conn.lock()
            .unwrap()
            .execute_batch(
                "INSERT INTO track_tags (track_id, tag_id) VALUES ('t1', 'g1'), ('t2', 'g1'), ('t3', 'g1');
                 CREATE TRIGGER refuse_t3 BEFORE DELETE ON track_tags WHEN OLD.track_id = 't3'
                 BEGIN SELECT RAISE(ABORT, 'simulated failure'); END;",
            )
            .unwrap();

        assert!(tags
            .remove_tags(ids(&["t1", "t2", "t3"]), ids(&["g1"]))
            .is_err());
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM track_tags"),
            3,
            "nothing was removed"
        );
        assert_eq!(
            count(&conn, "SELECT COUNT(*) FROM sync_tombstones"),
            0,
            "and no tombstone says otherwise to the cloud sync"
        );
    }

    #[test]
    fn a_removal_leaves_one_tombstone_per_link_that_really_went() {
        let (tags, conn) = service();
        conn.lock()
            .unwrap()
            .execute_batch(
                "INSERT INTO track_tags (track_id, tag_id) VALUES ('t1', 'g1'), ('t2', 'g1');",
            )
            .unwrap();

        // t3 never had the tag: no tombstone for it.
        tags.remove_tags(ids(&["t1", "t2", "t3"]), ids(&["g1"]))
            .unwrap();

        assert_eq!(count(&conn, "SELECT COUNT(*) FROM track_tags"), 0);
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM sync_tombstones"), 2);
    }

    #[test]
    fn assigning_twice_keeps_one_link() {
        let (tags, conn) = service();
        tags.assign_tags(ids(&["t1", "t2"]), ids(&["g1", "g2"]))
            .unwrap();
        tags.assign_tags(ids(&["t1"]), ids(&["g1"])).unwrap();
        assert_eq!(count(&conn, "SELECT COUNT(*) FROM track_tags"), 4);
    }
}
