use std::borrow::Cow;

use rusqlite::types::Value;

use crate::error::{CrateError, Result};
use crate::models::{
    DateOperator, EnumOperator, MatchMode, NumericOperator, SmartCondition, SmartLimit, SmartRules,
    SortDirection, TagOperator, TextOperator,
};

/// SQL of a listening statistic about library track `t`, computed from `listen_events`.
///
/// Listens are matched to the track by artist and title, case and surrounding spaces ignored:
/// only Crate-local listens carry a track id, while the Rekordbox sets and the Spotify history
/// carry just the two names. `select` and `extra` are constants written in this file, never user
/// input. The result is a scalar subquery, so it can stand wherever a column can.
fn listening_stat(select: &str, extra: &str) -> String {
    format!(
        "(SELECT {select} FROM listen_events le \
         WHERE lower(trim(le.artist)) = lower(trim(t.artist)) \
         AND lower(trim(le.title)) = lower(trim(t.title)){extra})"
    )
}

/// The listening statistics a library smart playlist can filter and sort on.
///
/// Numeric: `listens_total`, `listens_7d`, `listens_30d`, `listens_365d`, `set_plays` (times the
/// track was played in a Rekordbox set) and `minutes_listened`. Date: `last_listened` and
/// `last_set_play`. A track never listened to counts 0 and has no date.
fn listening_stat_expression(field: &str) -> Option<String> {
    let since =
        |days: u32| format!(" AND datetime(le.played_at) >= datetime('now', '-{days} days')");
    const IN_A_SET: &str = " AND le.source = 'rekordbox'";
    Some(match field {
        "listens_total" => listening_stat("COUNT(*)", ""),
        "listens_7d" => listening_stat("COUNT(*)", &since(7)),
        "listens_30d" => listening_stat("COUNT(*)", &since(30)),
        "listens_365d" => listening_stat("COUNT(*)", &since(365)),
        "set_plays" => listening_stat("COUNT(*)", IN_A_SET),
        "minutes_listened" => listening_stat("COALESCE(SUM(le.played_ms), 0) / 60000.0", ""),
        // `datetime()` normalises the `T...Z` and offset forms so the comparison with
        // `datetime('now', …)` is between values of the same shape (see [B14]).
        "last_listened" => listening_stat("MAX(datetime(le.played_at))", ""),
        "last_set_play" => listening_stat("MAX(datetime(le.played_at))", IN_A_SET),
        _ => return None,
    })
}

/// Map a field name to its SQL column for the library (tracks) context.
fn library_field_column(field: &str) -> Result<Cow<'static, str>> {
    if let Some(expression) = listening_stat_expression(field) {
        return Ok(Cow::Owned(expression));
    }
    plain_library_column(field).map(Cow::Borrowed)
}

fn plain_library_column(field: &str) -> Result<&'static str> {
    match field {
        "title" => Ok("t.title"),
        "artist" => Ok("t.artist"),
        "album" => Ok("t.album"),
        "genre" => Ok("t.genre"),
        "label" => Ok("t.label"),
        "catalog_number" => Ok("t.catalog_number"),
        "bpm" => Ok("t.bpm"),
        "rating" => Ok("t.rating"),
        "play_count" => Ok("t.play_count"),
        "year" => Ok("t.year"),
        "duration_ms" => Ok("t.duration_ms"),
        "bitrate" => Ok("t.bitrate"),
        "sample_rate" => Ok("t.sample_rate"),
        "date_added" => Ok("t.date_added"),
        "last_played" => Ok("t.last_played"),
        "date_modified" => Ok("t.date_modified"),
        "color" => Ok("t.color"),
        "format" => Ok("t.format"),
        "key" => Ok("t.key"),
        "energy" => Ok("t.energy"),
        _ => Err(CrateError::InvalidOperation(format!(
            "Invalid library field: {field}"
        ))),
    }
}

/// Map a field name to its SQL column for the discovery (releases) context.
fn discovery_field_column(field: &str) -> Result<&'static str> {
    match field {
        "title" => Ok("dr.title"),
        "artist" => Ok("dr.artist"),
        "label" => Ok("dr.label"),
        "source_type" => Ok("dr.source_type"),
        "release_date" => Ok("dr.release_date"),
        "notes" => Ok("dr.notes"),
        "date_added" => Ok("dr.date_added"),
        "date_modified" => Ok("dr.date_modified"),
        _ => Err(CrateError::InvalidOperation(format!(
            "Invalid discovery field: {field}"
        ))),
    }
}

/// Map a sort field to its SQL column for the library context.
fn library_sort_column(field: &str) -> Result<Cow<'static, str>> {
    // Sorting on a listening statistic ("most listened in the last 30 days").
    if matches!(
        field,
        "listens_total"
            | "listens_7d"
            | "listens_30d"
            | "listens_365d"
            | "set_plays"
            | "minutes_listened"
            | "last_listened"
            | "last_set_play"
    ) {
        if let Some(expression) = listening_stat_expression(field) {
            return Ok(Cow::Owned(expression));
        }
    }
    plain_library_sort_column(field).map(Cow::Borrowed)
}

fn plain_library_sort_column(field: &str) -> Result<&'static str> {
    match field {
        "date_added" => Ok("t.date_added"),
        "rating" => Ok("t.rating"),
        "play_count" => Ok("t.play_count"),
        "bpm" => Ok("t.bpm"),
        "energy" => Ok("t.energy"),
        "title" => Ok("t.title"),
        "artist" => Ok("t.artist"),
        "random" => Ok("RANDOM()"),
        _ => Err(CrateError::InvalidOperation(format!(
            "Invalid library sort field: {field}"
        ))),
    }
}

/// Map a sort field to its SQL column for the discovery context.
fn discovery_sort_column(field: &str) -> Result<&'static str> {
    match field {
        "date_added" => Ok("dr.date_added"),
        "title" => Ok("dr.title"),
        "artist" => Ok("dr.artist"),
        "release_date" => Ok("dr.release_date"),
        "random" => Ok("RANDOM()"),
        _ => Err(CrateError::InvalidOperation(format!(
            "Invalid discovery sort field: {field}"
        ))),
    }
}

/// Build a text condition SQL fragment.
fn build_text_condition(
    column: &str,
    operator: &TextOperator,
    value: &Option<String>,
    params: &mut Vec<Value>,
) -> String {
    match operator {
        TextOperator::Contains => {
            let val = value.as_deref().unwrap_or("");
            params.push(Value::Text(format!("%{val}%")));
            format!("COALESCE({column}, '') LIKE ?{}", params.len())
        }
        TextOperator::NotContains => {
            let val = value.as_deref().unwrap_or("");
            params.push(Value::Text(format!("%{val}%")));
            format!("COALESCE({column}, '') NOT LIKE ?{}", params.len())
        }
        TextOperator::Equals => {
            let val = value.as_deref().unwrap_or("");
            params.push(Value::Text(val.to_string()));
            format!("COALESCE({column}, '') = ?{}", params.len())
        }
        TextOperator::NotEquals => {
            let val = value.as_deref().unwrap_or("");
            params.push(Value::Text(val.to_string()));
            format!("COALESCE({column}, '') != ?{}", params.len())
        }
        TextOperator::StartsWith => {
            let val = value.as_deref().unwrap_or("");
            params.push(Value::Text(format!("{val}%")));
            format!("COALESCE({column}, '') LIKE ?{}", params.len())
        }
        TextOperator::EndsWith => {
            let val = value.as_deref().unwrap_or("");
            params.push(Value::Text(format!("%{val}")));
            format!("COALESCE({column}, '') LIKE ?{}", params.len())
        }
        TextOperator::IsEmpty => {
            format!("({column} IS NULL OR {column} = '')")
        }
        TextOperator::IsNotEmpty => {
            format!("({column} IS NOT NULL AND {column} != '')")
        }
    }
}

/// Build a numeric condition SQL fragment.
fn build_numeric_condition(
    column: &str,
    operator: &NumericOperator,
    value: &Option<f64>,
    value2: &Option<f64>,
    params: &mut Vec<Value>,
) -> String {
    match operator {
        NumericOperator::Equals => {
            let val = value.unwrap_or(0.0);
            params.push(Value::Real(val));
            format!("{column} = ?{}", params.len())
        }
        NumericOperator::NotEquals => {
            let val = value.unwrap_or(0.0);
            params.push(Value::Real(val));
            format!("{column} != ?{}", params.len())
        }
        NumericOperator::GreaterThan => {
            let val = value.unwrap_or(0.0);
            params.push(Value::Real(val));
            format!("{column} > ?{}", params.len())
        }
        NumericOperator::LessThan => {
            let val = value.unwrap_or(0.0);
            params.push(Value::Real(val));
            format!("{column} < ?{}", params.len())
        }
        NumericOperator::InRange => {
            let v1 = value.unwrap_or(0.0);
            let v2 = value2.unwrap_or(0.0);
            params.push(Value::Real(v1));
            params.push(Value::Real(v2));
            format!(
                "{column} BETWEEN ?{} AND ?{}",
                params.len() - 1,
                params.len()
            )
        }
    }
}

/// Build a date condition SQL fragment.
fn build_date_condition(
    column: &str,
    operator: &DateOperator,
    value: &Option<String>,
    params: &mut Vec<Value>,
) -> String {
    match operator {
        DateOperator::InLastDays => {
            let val = value.as_deref().unwrap_or("30");
            params.push(Value::Text(format!("-{val} days")));
            format!("{column} > datetime('now', ?{})", params.len())
        }
        DateOperator::NotInLastDays => {
            let val = value.as_deref().unwrap_or("30");
            params.push(Value::Text(format!("-{val} days")));
            format!(
                "({column} IS NULL OR {column} <= datetime('now', ?{}))",
                params.len()
            )
        }
        DateOperator::Before => {
            let val = value.as_deref().unwrap_or("2000-01-01");
            params.push(Value::Text(val.to_string()));
            format!("{column} < ?{}", params.len())
        }
        DateOperator::After => {
            let val = value.as_deref().unwrap_or("2000-01-01");
            params.push(Value::Text(val.to_string()));
            format!("{column} > ?{}", params.len())
        }
        DateOperator::IsEmpty => {
            format!("{column} IS NULL")
        }
        DateOperator::IsNotEmpty => {
            format!("{column} IS NOT NULL")
        }
    }
}

/// Build an enum condition SQL fragment.
fn build_enum_condition(
    column: &str,
    operator: &EnumOperator,
    value: &Option<String>,
    params: &mut Vec<Value>,
) -> String {
    match operator {
        EnumOperator::Equals => {
            let val = value.as_deref().unwrap_or("");
            params.push(Value::Text(val.to_string()));
            format!("{column} = ?{}", params.len())
        }
        EnumOperator::NotEquals => {
            let val = value.as_deref().unwrap_or("");
            params.push(Value::Text(val.to_string()));
            format!("({column} IS NULL OR {column} != ?{})", params.len())
        }
        EnumOperator::IsEmpty => {
            format!("({column} IS NULL OR {column} = '')")
        }
        EnumOperator::IsNotEmpty => {
            format!("({column} IS NOT NULL AND {column} != '')")
        }
    }
}

/// Build a tag condition SQL fragment for the library context.
fn build_library_tag_condition(
    operator: &TagOperator,
    tag_ids: &[String],
    params: &mut Vec<Value>,
) -> String {
    if tag_ids.is_empty() {
        return "1=1".to_string();
    }

    let placeholders: Vec<String> = tag_ids
        .iter()
        .map(|id| {
            params.push(Value::Text(id.clone()));
            format!("?{}", params.len())
        })
        .collect();
    let ph = placeholders.join(", ");

    match operator {
        TagOperator::HasAny => {
            format!("t.id IN (SELECT track_id FROM track_tags WHERE tag_id IN ({ph}))")
        }
        TagOperator::HasAll => {
            format!(
                "t.id IN (SELECT track_id FROM track_tags WHERE tag_id IN ({ph}) GROUP BY track_id HAVING COUNT(DISTINCT tag_id) = {})",
                tag_ids.len()
            )
        }
        TagOperator::HasNone => {
            format!("t.id NOT IN (SELECT track_id FROM track_tags WHERE tag_id IN ({ph}))")
        }
    }
}

/// Build a tag condition SQL fragment for the discovery context.
fn build_discovery_tag_condition(
    operator: &TagOperator,
    tag_ids: &[String],
    params: &mut Vec<Value>,
) -> String {
    if tag_ids.is_empty() {
        return "1=1".to_string();
    }

    let placeholders: Vec<String> = tag_ids
        .iter()
        .map(|id| {
            params.push(Value::Text(id.clone()));
            format!("?{}", params.len())
        })
        .collect();
    let ph = placeholders.join(", ");

    match operator {
        TagOperator::HasAny => {
            format!(
                "dr.id IN (SELECT release_id FROM discovery_release_tags WHERE tag_id IN ({ph}))"
            )
        }
        TagOperator::HasAll => {
            format!(
                "dr.id IN (SELECT release_id FROM discovery_release_tags WHERE tag_id IN ({ph}) GROUP BY release_id HAVING COUNT(DISTINCT tag_id) = {})",
                tag_ids.len()
            )
        }
        TagOperator::HasNone => {
            format!(
                "dr.id NOT IN (SELECT release_id FROM discovery_release_tags WHERE tag_id IN ({ph}))"
            )
        }
    }
}

/// The SQL column (or expression) of `field` in the given context.
fn field_column(field: &str, context: &str) -> Result<Cow<'static, str>> {
    if context == "discovery" {
        discovery_field_column(field).map(Cow::Borrowed)
    } else {
        library_field_column(field)
    }
}

/// Build a single condition SQL fragment.
fn build_condition_sql(
    condition: &SmartCondition,
    context: &str,
    params: &mut Vec<Value>,
) -> Result<String> {
    match condition {
        SmartCondition::Text {
            field,
            operator,
            value,
        } => {
            let column = field_column(field, context)?;
            Ok(build_text_condition(&column, operator, value, params))
        }
        SmartCondition::Numeric {
            field,
            operator,
            value,
            value2,
        } => {
            let column = field_column(field, context)?;
            Ok(build_numeric_condition(
                &column, operator, value, value2, params,
            ))
        }
        SmartCondition::Date {
            field,
            operator,
            value,
        } => {
            let column = field_column(field, context)?;
            Ok(build_date_condition(&column, operator, value, params))
        }
        SmartCondition::Enum {
            field,
            operator,
            value,
        } => {
            let column = field_column(field, context)?;
            Ok(build_enum_condition(&column, operator, value, params))
        }
        SmartCondition::Tags {
            operator, tag_ids, ..
        } => {
            if context == "discovery" {
                Ok(build_discovery_tag_condition(operator, tag_ids, params))
            } else {
                Ok(build_library_tag_condition(operator, tag_ids, params))
            }
        }
    }
}

/// Build the ORDER BY + LIMIT clause from a SmartLimit.
fn build_limit_sql(limit: &SmartLimit, context: &str, params: &mut Vec<Value>) -> Result<String> {
    let sort_col = if context == "discovery" {
        Cow::Borrowed(discovery_sort_column(&limit.sort_field)?)
    } else {
        library_sort_column(&limit.sort_field)?
    };

    if limit.sort_field == "random" {
        params.push(Value::Integer(limit.count as i64));
        return Ok(format!("ORDER BY RANDOM() LIMIT ?{}", params.len()));
    }

    let dir = match limit.sort_direction {
        SortDirection::Ascending => "ASC",
        SortDirection::Descending => "DESC",
    };

    params.push(Value::Integer(limit.count as i64));
    Ok(format!("ORDER BY {sort_col} {dir} LIMIT ?{}", params.len()))
}

/// Build the full WHERE clause (and optional ORDER BY + LIMIT) for a library smart playlist.
/// Returns `(sql_fragment, params)` where `sql_fragment` is everything after `FROM tracks t WHERE`.
pub fn build_smart_query_library(rules: &SmartRules) -> Result<(String, Vec<Value>)> {
    let mut params: Vec<Value> = Vec::new();

    let condition_sqls: Vec<String> = rules
        .conditions
        .iter()
        .map(|c| build_condition_sql(c, "library", &mut params))
        .collect::<Result<Vec<_>>>()?;

    let where_clause = if condition_sqls.is_empty() {
        "1=1".to_string()
    } else {
        let joiner = match rules.match_mode {
            MatchMode::All => " AND ",
            MatchMode::Any => " OR ",
        };
        condition_sqls.join(joiner)
    };

    let mut sql = where_clause;

    if let Some(ref limit) = rules.limit {
        let limit_sql = build_limit_sql(limit, "library", &mut params)?;
        sql = format!("{sql} {limit_sql}");
    }

    Ok((sql, params))
}

/// Build the full WHERE clause (and optional ORDER BY + LIMIT) for a discovery smart playlist.
/// Returns `(sql_fragment, params)` where `sql_fragment` is everything after `FROM discovery_releases dr WHERE`.
pub fn build_smart_query_discovery(rules: &SmartRules) -> Result<(String, Vec<Value>)> {
    let mut params: Vec<Value> = Vec::new();

    let condition_sqls: Vec<String> = rules
        .conditions
        .iter()
        .map(|c| build_condition_sql(c, "discovery", &mut params))
        .collect::<Result<Vec<_>>>()?;

    let where_clause = if condition_sqls.is_empty() {
        "1=1".to_string()
    } else {
        let joiner = match rules.match_mode {
            MatchMode::All => " AND ",
            MatchMode::Any => " OR ",
        };
        condition_sqls.join(joiner)
    };

    let mut sql = where_clause;

    if let Some(ref limit) = rules.limit {
        let limit_sql = build_limit_sql(limit, "discovery", &mut params)?;
        sql = format!("{sql} {limit_sql}");
    }

    Ok((sql, params))
}

/// Validate that smart rules are well-formed for the given context.
pub fn validate_smart_rules(rules: &SmartRules, context: &str) -> Result<()> {
    for condition in &rules.conditions {
        match condition {
            SmartCondition::Text { field, .. }
            | SmartCondition::Numeric { field, .. }
            | SmartCondition::Date { field, .. }
            | SmartCondition::Enum { field, .. } => {
                // Validate field exists for context
                if context == "discovery" {
                    discovery_field_column(field)?;
                } else {
                    library_field_column(field)?;
                }
            }
            SmartCondition::Tags { .. } => {
                // Tags are valid in both contexts
            }
        }
    }

    if let Some(ref limit) = rules.limit {
        if context == "discovery" {
            discovery_sort_column(&limit.sort_field)?;
        } else {
            library_sort_column(&limit.sort_field)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod listening_stats_tests {
    use super::*;
    use chrono::{Duration, Utc};
    use rusqlite::{params_from_iter, Connection};

    const NOW: fn() -> chrono::DateTime<Utc> = Utc::now;

    fn library() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::run_migrations(&conn).unwrap();
        for (id, artist, title) in [
            ("A", "Artist A", "Song One"),
            ("B", "Artist B", "Song Two"),
            ("C", "Artist C", "Song Three"),
            ("D", "Artist D", "Never Heard"),
        ] {
            conn.execute(
                "INSERT INTO tracks (id, file_path, format, title, artist, duration_ms, date_added, date_modified)
                 VALUES (?1, ?2, 'mp3', ?3, ?4, 200000, '2026-01-01', '2026-01-01')",
                rusqlite::params![id, format!("/m/{id}.mp3"), title, artist],
            )
            .unwrap();
        }
        conn
    }

    /// One listen, matched to a track by the names it carries.
    fn listen(
        conn: &Connection,
        n: u32,
        source: &str,
        artist: &str,
        title: &str,
        at: &str,
        played_ms: i64,
    ) {
        conn.execute(
            "INSERT INTO listen_events (id, source, title, artist, duration_ms, played_ms, played_at)
             VALUES (?1, ?2, ?3, ?4, 200000, ?5, ?6)",
            rusqlite::params![format!("e{n}"), source, title, artist, played_ms, at],
        )
        .unwrap();
    }

    fn days_ago(days: i64) -> String {
        (NOW() - Duration::days(days)).to_rfc3339()
    }

    fn numeric(field: &str, operator: NumericOperator, value: f64) -> SmartCondition {
        SmartCondition::Numeric {
            field: field.to_string(),
            operator,
            value: Some(value),
            value2: None,
        }
    }

    fn rules(conditions: Vec<SmartCondition>, limit: Option<SmartLimit>) -> SmartRules {
        SmartRules {
            match_mode: MatchMode::All,
            conditions,
            limit,
        }
    }

    fn ids(conn: &Connection, rules: &SmartRules) -> Vec<String> {
        let (clause, params) = build_smart_query_library(rules).unwrap();
        let sql = format!("SELECT t.id FROM tracks t WHERE {clause}");
        let mut stmt = conn.prepare(&sql).unwrap();
        let rows = stmt
            .query_map(params_from_iter(params), |r| r.get::<_, String>(0))
            .unwrap();
        rows.map(|row| row.unwrap()).collect()
    }

    fn sorted(mut v: Vec<String>) -> Vec<String> {
        v.sort();
        v
    }

    #[test]
    fn never_played_in_a_set_finds_tracks_absent_from_every_rekordbox_session() {
        let conn = library();
        // A was only listened to on Spotify; B was played in a set, written with other casing
        // and stray spaces (Rekordbox does not copy the tags byte for byte).
        listen(
            &conn,
            1,
            "spotify",
            "Artist A",
            "Song One",
            &days_ago(3),
            200000,
        );
        listen(
            &conn,
            2,
            "rekordbox",
            "  artist b ",
            "SONG TWO",
            &days_ago(40),
            200000,
        );

        let never_in_a_set = rules(
            vec![numeric("set_plays", NumericOperator::Equals, 0.0)],
            None,
        );
        assert_eq!(sorted(ids(&conn, &never_in_a_set)), ["A", "C", "D"]);
    }

    #[test]
    fn the_thirty_day_window_uses_real_dates_whatever_the_stored_format() {
        let conn = library();
        // Inside the window, in three different stored shapes: `Z`, an offset, and a bare date-time.
        listen(
            &conn,
            1,
            "spotify",
            "Artist A",
            "Song One",
            &days_ago(29),
            1000,
        );
        let offset = (NOW() - Duration::days(10))
            .with_timezone(&chrono::FixedOffset::east_opt(2 * 3600).unwrap())
            .to_rfc3339();
        listen(&conn, 2, "spotify", "Artist B", "Song Two", &offset, 1000);
        let bare = (NOW() - Duration::days(1))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        listen(&conn, 3, "spotify", "Artist C", "Song Three", &bare, 1000);
        // Outside the window.
        listen(
            &conn,
            4,
            "spotify",
            "Artist D",
            "Never Heard",
            &days_ago(31),
            1000,
        );

        let recent = rules(
            vec![numeric("listens_30d", NumericOperator::GreaterThan, 0.0)],
            None,
        );
        assert_eq!(sorted(ids(&conn, &recent)), ["A", "B", "C"]);
    }

    #[test]
    fn top_tracks_of_the_last_thirty_days_sorts_by_listens_and_keeps_the_limit() {
        let conn = library();
        for n in 0..3 {
            listen(
                &conn,
                n,
                "spotify",
                "Artist B",
                "Song Two",
                &days_ago(2),
                1000,
            );
        }
        for n in 3..5 {
            listen(
                &conn,
                n,
                "crate_local",
                "Artist A",
                "Song One",
                &days_ago(5),
                1000,
            );
        }
        listen(
            &conn,
            5,
            "spotify",
            "Artist C",
            "Song Three",
            &days_ago(9),
            1000,
        );

        let top = rules(
            vec![],
            Some(SmartLimit {
                count: 2,
                sort_field: "listens_30d".to_string(),
                sort_direction: SortDirection::Descending,
            }),
        );
        assert_eq!(
            ids(&conn, &top),
            ["B", "A"],
            "most listened first, only two kept"
        );
    }

    #[test]
    fn not_listened_to_for_ninety_days_includes_never_listened_tracks() {
        let conn = library();
        listen(
            &conn,
            1,
            "spotify",
            "Artist A",
            "Song One",
            &days_ago(10),
            1000,
        );
        listen(
            &conn,
            2,
            "spotify",
            "Artist C",
            "Song Three",
            &days_ago(200),
            1000,
        );

        let forgotten = rules(
            vec![SmartCondition::Date {
                field: "last_listened".to_string(),
                operator: DateOperator::NotInLastDays,
                value: Some("90".to_string()),
            }],
            None,
        );
        // A was heard 10 days ago; B, D never; C long ago.
        assert_eq!(sorted(ids(&conn, &forgotten)), ["B", "C", "D"]);
    }

    #[test]
    fn minutes_listened_adds_up_the_played_time() {
        let conn = library();
        listen(
            &conn,
            1,
            "spotify",
            "Artist A",
            "Song One",
            &days_ago(1),
            120_000,
        );
        listen(
            &conn,
            2,
            "crate_local",
            "Artist A",
            "Song One",
            &days_ago(2),
            90_000,
        );
        listen(
            &conn,
            3,
            "spotify",
            "Artist B",
            "Song Two",
            &days_ago(1),
            60_000,
        );

        let more_than_three = rules(
            vec![numeric(
                "minutes_listened",
                NumericOperator::GreaterThan,
                3.0,
            )],
            None,
        );
        assert_eq!(ids(&conn, &more_than_three), ["A"], "3.5 min vs 1 min");
    }

    #[test]
    fn listening_criteria_are_library_only_and_unknown_fields_are_still_rejected() {
        let ok = rules(
            vec![numeric("listens_total", NumericOperator::GreaterThan, 0.0)],
            None,
        );
        assert!(validate_smart_rules(&ok, "library").is_ok());
        assert!(
            validate_smart_rules(&ok, "discovery").is_err(),
            "releases have no listening statistics"
        );

        let unknown = rules(
            vec![numeric(
                "listens_forever",
                NumericOperator::GreaterThan,
                0.0,
            )],
            None,
        );
        assert!(validate_smart_rules(&unknown, "library").is_err());

        let bad_sort = rules(
            vec![],
            Some(SmartLimit {
                count: 5,
                sort_field: "listens_forever".to_string(),
                sort_direction: SortDirection::Descending,
            }),
        );
        assert!(validate_smart_rules(&bad_sort, "library").is_err());
    }
}
