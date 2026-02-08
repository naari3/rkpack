use std::collections::HashSet;

use anyhow::Result;
use rusqlite::Connection;
use rusqlite::types::Value;

pub fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let chunks = data.chunks(3);
    for chunk in chunks {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((n >> 18) & 63) as usize] as char);
        result.push(CHARS[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((n >> 6) & 63) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(n & 63) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}

pub fn query_table_rows(
    conn: &Connection,
    sql: &str,
    params: &[&dyn rusqlite::types::ToSql],
) -> Result<Vec<serde_json::Value>> {
    let mut stmt = conn.prepare(sql)?;
    let column_count = stmt.column_count();
    let column_names: Vec<String> = (0..column_count)
        .map(|i| stmt.column_name(i).unwrap().to_string())
        .collect();

    let rows = stmt.query_map(params, |row| {
        let mut map = serde_json::Map::new();
        for (i, name) in column_names.iter().enumerate() {
            let val: Value = row.get(i)?;
            let json_val = match val {
                Value::Null => serde_json::Value::Null,
                Value::Integer(n) => serde_json::Value::Number(n.into()),
                Value::Real(f) => serde_json::Value::Number(
                    serde_json::Number::from_f64(f).unwrap_or_else(|| 0.into()),
                ),
                Value::Text(s) => serde_json::Value::String(s),
                Value::Blob(b) => serde_json::Value::String(base64_encode(&b)),
            };
            map.insert(name.clone(), json_val);
        }
        Ok(serde_json::Value::Object(map))
    })?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r?);
    }
    Ok(result)
}

pub fn format_create_table(sql: &str) -> String {
    let Some(paren_start) = sql.find('(') else {
        return sql.to_string();
    };
    let Some(paren_end) = sql.rfind(')') else {
        return sql.to_string();
    };

    let prefix = &sql[..=paren_start];
    let inner = &sql[paren_start + 1..paren_end];

    let mut columns = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;
    for ch in inner.chars() {
        match ch {
            '`' => {
                in_quote = !in_quote;
                current.push(ch);
            }
            ',' if !in_quote => {
                columns.push(current.trim().to_string());
                current = String::new();
            }
            _ => current.push(ch),
        }
    }
    let last = current.trim().to_string();
    if !last.is_empty() {
        columns.push(last);
    }

    let mut result = String::new();
    result.push_str(prefix);
    result.push('\n');
    for (i, col) in columns.iter().enumerate() {
        result.push_str("  ");
        result.push_str(col);
        if i < columns.len() - 1 {
            result.push(',');
        }
        result.push('\n');
    }
    result.push(')');
    result
}

pub fn list_tables(conn: &Connection) -> Result<()> {
    let mut stmt = conn
        .prepare("SELECT name, sql FROM sqlite_master WHERE type='table' ORDER BY name")?;
    let tables: Vec<(String, String)> = stmt
        .query_map([], |row| {
            let name: String = row.get(0)?;
            let sql: String = row.get(1)?;
            Ok((name, sql))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    let mut idx_stmt = conn.prepare(
        "SELECT tbl_name, name, sql FROM sqlite_master WHERE type='index' AND sql IS NOT NULL ORDER BY tbl_name, name",
    )?;
    let indexes: Vec<(String, String, String)> = idx_stmt
        .query_map([], |row| {
            let tbl: String = row.get(0)?;
            let name: String = row.get(1)?;
            let sql: String = row.get(2)?;
            Ok((tbl, name, sql))
        })?
        .collect::<Result<Vec<_>, _>>()?;

    for (i, (name, sql)) in tables.iter().enumerate() {
        if i > 0 {
            println!();
        }
        println!("-- {}", name);
        println!("{};", format_create_table(sql));

        for (_, idx_name, idx_sql) in indexes.iter().filter(|(tbl, _, _)| tbl == name) {
            println!("-- index: {}", idx_name);
            println!("{};", idx_sql);
        }
    }

    println!(
        "\n-- {} テーブル, {} インデックス",
        tables.len(),
        indexes.len()
    );
    Ok(())
}

pub fn list_playlists(conn: &Connection) -> Result<()> {
    let playlists = query_table_rows(
        conn,
        "SELECT p.ID, p.Name, p.Attribute, \
         (SELECT COUNT(*) FROM djmdSongPlaylist sp WHERE sp.PlaylistID = p.ID AND sp.rb_local_deleted = 0) as TrackCount \
         FROM djmdPlaylist p WHERE p.rb_local_deleted = 0 ORDER BY p.Seq",
        &[],
    )?;

    println!("{:<8} {:<6} {:<6} 名前", "ID", "種別", "曲数");
    println!("{}", "-".repeat(60));
    for p in &playlists {
        let id = p["ID"].as_str().unwrap_or("");
        let name = p["Name"].as_str().unwrap_or("(no name)");
        let attr = p["Attribute"].as_i64().unwrap_or(0);
        let track_count = p["TrackCount"].as_i64().unwrap_or(0);
        let kind = if attr == 0 { "フォルダ" } else { "リスト" };
        println!("{:<8} {:<6} {:<6} {}", id, kind, track_count, name);
    }
    println!("\n合計 {} プレイリスト", playlists.len());
    Ok(())
}

pub(crate) fn collect_ids_from_column(rows: &[serde_json::Value], column: &str) -> HashSet<String> {
    let mut ids = HashSet::new();
    for row in rows {
        if let Some(serde_json::Value::String(id)) = row.get(column)
            && !id.is_empty() {
                ids.insert(id.clone());
            }
    }
    ids
}

pub(crate) fn query_by_ids(
    conn: &Connection,
    table: &str,
    id_column: &str,
    ids: &HashSet<String>,
) -> Result<Vec<serde_json::Value>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let placeholders: Vec<String> = ids.iter().map(|_| "?".to_string()).collect();
    let sql = format!(
        "SELECT * FROM `{}` WHERE `{}` IN ({})",
        table,
        id_column,
        placeholders.join(",")
    );
    let id_vec: Vec<String> = ids.iter().cloned().collect();
    let params: Vec<&dyn rusqlite::types::ToSql> = id_vec
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();
    query_table_rows(conn, &sql, &params)
}

pub(crate) fn query_by_content_ids(
    conn: &Connection,
    table: &str,
    content_ids: &HashSet<String>,
) -> Result<Vec<serde_json::Value>> {
    query_by_ids(conn, table, "ContentID", content_ids)
}

pub struct PlaylistInfo {
    pub id: String,
    pub name: String,
    pub attribute: i64,
    pub track_count: i64,
}

pub struct TrackInfo {
    #[allow(dead_code)]
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
}

pub fn get_playlists(conn: &Connection) -> Result<Vec<PlaylistInfo>> {
    let rows = query_table_rows(
        conn,
        "SELECT p.ID, p.Name, p.Attribute, \
         (SELECT COUNT(*) FROM djmdSongPlaylist sp WHERE sp.PlaylistID = p.ID AND sp.rb_local_deleted = 0) as TrackCount \
         FROM djmdPlaylist p WHERE p.rb_local_deleted = 0 ORDER BY p.Seq",
        &[],
    )?;

    let mut playlists = Vec::new();
    for row in &rows {
        playlists.push(PlaylistInfo {
            id: row["ID"].as_str().unwrap_or("").to_string(),
            name: row["Name"].as_str().unwrap_or("(no name)").to_string(),
            attribute: row["Attribute"].as_i64().unwrap_or(0),
            track_count: row["TrackCount"].as_i64().unwrap_or(0),
        });
    }
    Ok(playlists)
}

pub fn get_playlist_tracks(conn: &Connection, playlist_id: &str) -> Result<Vec<TrackInfo>> {
    let rows = query_table_rows(
        conn,
        "SELECT c.ID, c.Title, \
         COALESCE(a.Name, '') as ArtistName, \
         COALESCE(al.Name, '') as AlbumName \
         FROM djmdSongPlaylist sp \
         JOIN djmdContent c ON c.ID = sp.ContentID \
         LEFT JOIN djmdArtist a ON a.ID = c.ArtistID \
         LEFT JOIN djmdAlbum al ON al.ID = c.AlbumID \
         WHERE sp.PlaylistID = ? AND sp.rb_local_deleted = 0 \
         ORDER BY sp.TrackNo",
        &[&playlist_id as &dyn rusqlite::types::ToSql],
    )?;

    let mut tracks = Vec::new();
    for row in &rows {
        tracks.push(TrackInfo {
            id: row["ID"].as_str().unwrap_or("").to_string(),
            title: row["Title"].as_str().unwrap_or("").to_string(),
            artist: row["ArtistName"].as_str().unwrap_or("").to_string(),
            album: row["AlbumName"].as_str().unwrap_or("").to_string(),
        });
    }
    Ok(tracks)
}
