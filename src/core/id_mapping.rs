use std::collections::HashMap;

use anyhow::{Context, Result};
use rusqlite::{Connection, params};

pub(crate) type IdMap = HashMap<String, HashMap<String, String>>;

pub(crate) fn get_max_numeric_id(conn: &Connection, table: &str) -> Result<i64> {
    let sql = format!("SELECT MAX(CAST(ID AS INTEGER)) FROM `{}`", table);
    let max_id: Option<i64> = conn.query_row(&sql, [], |row| row.get(0)).unwrap_or(None);
    Ok(max_id.unwrap_or(0))
}

pub(crate) fn find_existing_master_id(
    conn: &Connection,
    table: &str,
    name_column: &str,
    name_value: &str,
) -> Result<Option<String>> {
    let sql = format!(
        "SELECT ID FROM `{}` WHERE `{}` = ? AND rb_local_deleted = 0 LIMIT 1",
        table, name_column
    );
    let result: Option<String> = conn
        .query_row(&sql, params![name_value], |row| row.get(0))
        .ok();
    Ok(result)
}

pub(crate) fn master_table_name_column(table: &str) -> Option<&'static str> {
    match table {
        "djmdArtist" => Some("Name"),
        "djmdAlbum" => Some("Name"),
        "djmdGenre" => Some("Name"),
        "djmdKey" => Some("ScaleName"),
        "djmdLabel" => Some("Name"),
        "djmdColor" => Some("ColorCode"),
        "djmdMyTag" => Some("Name"),
        "djmdHotCueBanklist" => Some("Name"),
        _ => None,
    }
}

pub(crate) fn fk_columns_for_table(table: &str) -> Vec<(&'static str, &'static str)> {
    match table {
        "djmdContent" => vec![
            ("ArtistID", "djmdArtist"),
            ("AlbumID", "djmdAlbum"),
            ("GenreID", "djmdGenre"),
            ("KeyID", "djmdKey"),
            ("LabelID", "djmdLabel"),
            ("ColorID", "djmdColor"),
            ("RemixerID", "djmdArtist"),
            ("OrgArtistID", "djmdArtist"),
            ("ComposerID", "djmdArtist"),
            ("MasterSongID", "djmdContent"),
        ],
        "djmdAlbum" => vec![("AlbumArtistID", "djmdArtist")],
        "djmdSongPlaylist" => vec![
            ("PlaylistID", "djmdPlaylist"),
            ("ContentID", "djmdContent"),
        ],
        "djmdCue" => vec![("ContentID", "djmdContent")],
        "djmdActiveCensor" => vec![("ContentID", "djmdContent")],
        "djmdMixerParam" => vec![("ContentID", "djmdContent")],
        "djmdSongMyTag" => vec![
            ("MyTagID", "djmdMyTag"),
            ("ContentID", "djmdContent"),
        ],
        "djmdSongTagList" => vec![("ContentID", "djmdContent")],
        "djmdSongHotCueBanklist" => vec![
            ("HotCueBanklistID", "djmdHotCueBanklist"),
            ("ContentID", "djmdContent"),
        ],
        "hotCueBanklistCue" => vec![("HotCueBanklistID", "djmdHotCueBanklist")],
        "contentCue" => vec![("ContentID", "djmdContent")],
        "contentActiveCensor" => vec![("ContentID", "djmdContent")],
        "contentFile" => vec![("ContentID", "djmdContent")],
        _ => vec![],
    }
}

pub(crate) fn insert_row(conn: &Connection, table: &str, row: &serde_json::Value) -> Result<()> {
    let obj = row
        .as_object()
        .context("行データがオブジェクトではありません")?;

    let columns: Vec<&String> = obj.keys().collect();
    let col_names: Vec<String> = columns.iter().map(|c| format!("`{}`", c)).collect();
    let placeholders: Vec<String> = columns.iter().map(|_| "?".to_string()).collect();

    let sql = format!(
        "INSERT INTO `{}` ({}) VALUES ({})",
        table,
        col_names.join(", "),
        placeholders.join(", ")
    );

    let values: Vec<Box<dyn rusqlite::types::ToSql>> = columns
        .iter()
        .map(|col| -> Box<dyn rusqlite::types::ToSql> {
            match &obj[col.as_str()] {
                serde_json::Value::Null => Box::new(Option::<String>::None),
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Box::new(i)
                    } else if let Some(f) = n.as_f64() {
                        Box::new(f)
                    } else {
                        Box::new(n.to_string())
                    }
                }
                serde_json::Value::String(s) => Box::new(s.clone()),
                serde_json::Value::Bool(b) => Box::new(*b as i32),
                other => Box::new(other.to_string()),
            }
        })
        .collect();

    let params: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    conn.execute(&sql, params.as_slice())?;
    Ok(())
}

pub(crate) fn apply_mapping(
    row: &serde_json::Value,
    table: &str,
    id_map: &IdMap,
) -> serde_json::Value {
    let mut row = row.clone();
    if let Some(obj) = row.as_object_mut() {
        if let Some(old_id) = obj
            .get("ID")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            && let Some(table_map) = id_map.get(table)
                && let Some(new_id) = table_map.get(&old_id) {
                    obj.insert(
                        "ID".to_string(),
                        serde_json::Value::String(new_id.clone()),
                    );
                }

        for (fk_col, ref_table) in fk_columns_for_table(table) {
            if let Some(old_fk) = obj
                .get(fk_col)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                && let Some(ref_map) = id_map.get(ref_table)
                    && let Some(new_fk) = ref_map.get(&old_fk) {
                        obj.insert(
                            fk_col.to_string(),
                            serde_json::Value::String(new_fk.clone()),
                        );
                    }
        }

        for &sync_field in &[
            "rb_data_status",
            "rb_local_data_status",
            "rb_local_file_status",
        ] {
            if obj.contains_key(sync_field) {
                obj.insert(
                    sync_field.to_string(),
                    serde_json::Value::Number(0.into()),
                );
            }
        }
        for &sync_field in &["rb_local_synced"] {
            if obj.contains_key(sync_field) {
                obj.insert(
                    sync_field.to_string(),
                    serde_json::Value::Number(0.into()),
                );
            }
        }
        for &sync_field in &["usn", "rb_local_usn", "rb_insync_local_usn"] {
            if obj.contains_key(sync_field) {
                obj.insert(sync_field.to_string(), serde_json::Value::Null);
            }
        }
    }
    row
}

fn remap_json_blob_field(
    obj: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    map: &HashMap<String, String>,
) {
    let old_val = obj.get(key).and_then(|v| v.as_str()).map(|s| s.to_string());
    if let Some(old) = old_val {
        if let Some(new) = map.get(&old) {
            obj.insert(key.to_string(), serde_json::Value::String(new.clone()));
        }
    }
}

pub(crate) fn remap_json_blob(
    row: &mut serde_json::Value,
    table: &str,
    id_map: &IdMap,
) {
    let (blob_field, ref_table) = match table {
        "contentCue" => ("Cues", "djmdCue"),
        "contentActiveCensor" => ("ActiveCensors", "djmdActiveCensor"),
        "hotCueBanklistCue" => ("Cues", "djmdSongHotCueBanklist"),
        _ => return,
    };

    let Some(obj) = row.as_object_mut() else {
        return;
    };
    let Some(blob_str) = obj.get(blob_field).and_then(|v| v.as_str()).map(|s| s.to_string())
    else {
        return;
    };
    let Ok(mut arr) = serde_json::from_str::<Vec<serde_json::Value>>(&blob_str) else {
        return;
    };

    let content_map = id_map.get("djmdContent");
    let detail_map = id_map.get(ref_table);

    for item in arr.iter_mut() {
        if let Some(item_obj) = item.as_object_mut() {
            if let Some(d_map) = detail_map {
                remap_json_blob_field(item_obj, "ID", d_map);
            }
            if let Some(c_map) = content_map {
                remap_json_blob_field(item_obj, "ContentID", c_map);
            }
        }
    }

    if let Ok(new_blob) = serde_json::to_string(&arr) {
        obj.insert(
            blob_field.to_string(),
            serde_json::Value::String(new_blob),
        );
    }
}
