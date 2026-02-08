use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::PathBuf;

use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use zip::ZipArchive;

use super::db::get_actual_path_on_disk;
use super::id_mapping::{
    IdMap, apply_mapping, find_existing_master_id, get_max_numeric_id, insert_row,
    master_table_name_column, remap_json_blob,
};

pub(crate) fn extract_rkp_entry(
    archive: &mut ZipArchive<fs::File>,
    name: &str,
    dest: &std::path::Path,
) -> Result<()> {
    let mut entry = archive
        .by_name(name)
        .with_context(|| format!(".rkp 内にエントリが見つかりません: {}", name))?;
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut out = fs::File::create(dest)
        .with_context(|| format!("ファイルの作成に失敗: {}", dest.display()))?;
    io::copy(&mut entry, &mut out)?;
    Ok(())
}

fn load_pack_data(archive: &mut ZipArchive<fs::File>) -> Result<serde_json::Value> {
    let entry = archive
        .by_name("pack.json")
        .context(".rkp 内に pack.json が見つかりません")?;
    let pack_data: serde_json::Value =
        serde_json::from_reader(entry).context("pack.json の解析に失敗")?;

    let version = pack_data["version"].as_i64().unwrap_or(0);
    if version != 1 {
        anyhow::bail!("未対応のパックバージョン: {}", version);
    }

    Ok(pack_data)
}

fn detect_duplicate_contents(
    conn: &Connection,
    tables: &serde_json::Map<String, serde_json::Value>,
    progress: &dyn Fn(&str),
) -> Result<(HashSet<String>, HashMap<String, String>)> {
    let mut skipped_content_ids: HashSet<String> = HashSet::new();
    let mut existing_content_map: HashMap<String, String> = HashMap::new();

    if let Some(content_files) = tables.get("contentFile").and_then(|v| v.as_array()) {
        for cf in content_files {
            let hash = cf.get("Hash").and_then(|h| h.as_str());
            let pack_content_id = cf.get("ContentID").and_then(|c| c.as_str());
            if let (Some(hash), Some(pack_cid)) = (hash, pack_content_id) {
                if hash.is_empty() {
                    continue;
                }
                let existing: Option<String> = conn
                    .query_row(
                        "SELECT ContentID FROM contentFile WHERE Hash = ? AND rb_local_deleted = 0 LIMIT 1",
                        params![hash],
                        |row| row.get(0),
                    )
                    .ok();
                if let Some(existing_cid) = existing {
                    progress(&format!(
                        "重複トラック検出: ContentID {} (Hash: {}) → 既存 ContentID {}",
                        pack_cid, hash, existing_cid
                    ));
                    skipped_content_ids.insert(pack_cid.to_string());
                    existing_content_map.insert(pack_cid.to_string(), existing_cid);
                }
            }
        }
    }

    Ok((skipped_content_ids, existing_content_map))
}

const MASTER_TABLES: &[&str] = &[
    "djmdArtist",
    "djmdAlbum",
    "djmdGenre",
    "djmdKey",
    "djmdLabel",
    "djmdColor",
    "djmdMyTag",
    "djmdHotCueBanklist",
];

const RELATED_TABLES: &[&str] = &[
    "djmdCue",
    "djmdActiveCensor",
    "djmdMixerParam",
    "djmdSongMyTag",
    "djmdSongTagList",
    "djmdSongHotCueBanklist",
    "hotCueBanklistCue",
    "contentCue",
    "contentActiveCensor",
    "contentFile",
];

fn build_master_id_map(
    conn: &Connection,
    tables: &serde_json::Map<String, serde_json::Value>,
    id_map: &mut IdMap,
) -> Result<()> {
    for &table in MASTER_TABLES {
        let rows = match tables.get(table).and_then(|v| v.as_array()) {
            Some(r) => r,
            None => continue,
        };
        let name_col = master_table_name_column(table);
        let mut max_id = get_max_numeric_id(conn, table)?;
        let mut table_map = HashMap::new();

        for row in rows {
            let old_id = match row.get("ID").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => continue,
            };

            if let Some(name_col) = name_col {
                let name_val = row
                    .get(name_col)
                    .and_then(|v| match v {
                        serde_json::Value::String(s) => Some(s.as_str()),
                        _ => None,
                    });
                let existing_id = if table == "djmdColor" {
                    if let Some(code) = row.get("ColorCode").and_then(|v| v.as_i64()) {
                        let sql = format!(
                            "SELECT ID FROM `{}` WHERE ColorCode = ? AND rb_local_deleted = 0 LIMIT 1",
                            table
                        );
                        conn.query_row(&sql, params![code], |row| row.get::<_, String>(0))
                            .ok()
                    } else {
                        None
                    }
                } else if let Some(name_val) = name_val {
                    find_existing_master_id(conn, table, name_col, name_val)?
                } else {
                    None
                };

                if let Some(eid) = existing_id {
                    table_map.insert(old_id, eid);
                    continue;
                }
            }

            max_id += 1;
            table_map.insert(old_id, max_id.to_string());
        }

        id_map.insert(table.to_string(), table_map);
    }

    Ok(())
}

fn build_content_id_map(
    conn: &Connection,
    tables: &serde_json::Map<String, serde_json::Value>,
    existing_content_map: &HashMap<String, String>,
    id_map: &mut IdMap,
) -> Result<()> {
    let content_table = "djmdContent";
    let rows = tables
        .get(content_table)
        .and_then(|v| v.as_array())
        .unwrap_or(&Vec::new())
        .clone();
    let mut max_id = get_max_numeric_id(conn, content_table)?;
    let mut table_map = HashMap::new();

    for row in &rows {
        let old_id = match row.get("ID").and_then(|v| v.as_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };

        if let Some(existing_cid) = existing_content_map.get(&old_id) {
            table_map.insert(old_id, existing_cid.clone());
        } else {
            max_id += 1;
            table_map.insert(old_id, max_id.to_string());
        }
    }

    id_map.insert(content_table.to_string(), table_map);
    Ok(())
}

fn build_related_id_maps(
    conn: &Connection,
    tables: &serde_json::Map<String, serde_json::Value>,
    pack_data: &serde_json::Value,
    id_map: &mut IdMap,
) -> Result<()> {
    // Playlist ID map
    {
        let mut max_id = get_max_numeric_id(conn, "djmdPlaylist")?;
        let mut table_map = HashMap::new();
        if let Some(playlist) = pack_data.get("playlist")
            && let Some(old_id) = playlist.get("ID").and_then(|v| v.as_str()) {
                max_id += 1;
                table_map.insert(old_id.to_string(), max_id.to_string());
            }
        id_map.insert("djmdPlaylist".to_string(), table_map);
    }

    // Related tables + djmdSongPlaylist
    let all_id_tables: Vec<&str> = RELATED_TABLES
        .iter()
        .copied()
        .chain(std::iter::once("djmdSongPlaylist"))
        .collect();
    for &table in &all_id_tables {
        let rows = match tables.get(table).and_then(|v| v.as_array()) {
            Some(r) => r,
            None => continue,
        };
        let mut max_id = get_max_numeric_id(conn, table)?;
        let mut table_map = HashMap::new();

        for row in rows {
            let old_id = match row.get("ID").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => continue,
            };
            max_id += 1;
            table_map.insert(old_id, max_id.to_string());
        }

        id_map.insert(table.to_string(), table_map);
    }

    Ok(())
}

fn extract_audio_files(
    archive: &mut ZipArchive<fs::File>,
    pack_data: &serde_json::Value,
    dest_dir: &str,
    skipped_content_ids: &HashSet<String>,
    progress: &dyn Fn(&str),
) -> Result<HashMap<String, String>> {
    let mut audio_actual_paths: HashMap<String, String> = HashMap::new();
    let mut file_copy_success = 0u32;
    let mut file_copy_skip = 0u32;
    let mut file_copy_fail = 0u32;

    if let Some(audio_files) = pack_data.get("audio_files").and_then(|v| v.as_array()) {
        let dest_path = PathBuf::from(dest_dir);
        let _ = fs::create_dir_all(&dest_path);
        let total_audio = audio_files.len();
        for (idx, af) in audio_files.iter().enumerate() {
            let content_id = af
                .get("content_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let relative_path = match af.get("relative_path").and_then(|v| v.as_str()) {
                Some(p) => p,
                None => continue,
            };

            if skipped_content_ids.contains(content_id) {
                file_copy_skip += 1;
                continue;
            }

            let entry_name = format!("files/{}", relative_path.replace('\\', "/"));
            let file_name = std::path::Path::new(relative_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            progress(&format!(
                "音声ファイル展開 ({}/{}) {}",
                idx + 1,
                total_audio,
                file_name
            ));

            let target = dest_path.join(&file_name);

            match extract_rkp_entry(archive, &entry_name, &target) {
                Ok(_) => {
                    file_copy_success += 1;
                    let actual = get_actual_path_on_disk(&target);
                    let actual_str = actual.to_string_lossy().replace('\\', "/");
                    audio_actual_paths.insert(content_id.to_string(), actual_str);
                }
                Err(e) => {
                    progress(&format!("警告: 音声ファイル展開失敗: {}: {}", entry_name, e));
                    file_copy_fail += 1;
                }
            }
        }
    }
    progress(&format!(
        "音声ファイル配置: 成功={}, スキップ={}, 失敗={}",
        file_copy_success, file_copy_skip, file_copy_fail
    ));

    Ok(audio_actual_paths)
}

fn extract_data_files(
    archive: &mut ZipArchive<fs::File>,
    pack_data: &serde_json::Value,
    share_dir: &std::path::Path,
    progress: &dyn Fn(&str),
) -> Result<HashMap<String, String>> {
    let mut data_actual_paths: HashMap<String, String> = HashMap::new();
    let mut data_file_success = 0u32;
    let data_file_skip = 0u32;
    let mut data_file_fail = 0u32;

    if let Some(data_files) = pack_data
        .get("content_data_files")
        .and_then(|v| v.as_array())
    {
        let total_data = data_files.len();
        for (idx, df) in data_files.iter().enumerate() {
            let cf_id = df
                .get("content_file_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let relative_path = match df.get("relative_path").and_then(|v| v.as_str()) {
                Some(p) => p,
                None => continue,
            };

            progress(&format!(
                "データファイル展開 ({}/{}) {}",
                idx + 1,
                total_data,
                relative_path
            ));

            let entry_name = format!("content_data/{}", relative_path.replace('\\', "/"));
            let target = share_dir.join(relative_path);

            match extract_rkp_entry(archive, &entry_name, &target) {
                Ok(_) => {
                    data_file_success += 1;
                    let actual = get_actual_path_on_disk(&target);
                    let actual_str = actual.to_string_lossy().to_string();
                    data_actual_paths.insert(cf_id.to_string(), actual_str);
                }
                Err(e) => {
                    progress(&format!(
                        "警告: データファイル展開失敗: {}: {}",
                        entry_name, e
                    ));
                    data_file_fail += 1;
                }
            }
        }
        progress(&format!(
            "データファイル配置: 成功={}, スキップ={}, 失敗={}",
            data_file_success, data_file_skip, data_file_fail
        ));
    }

    Ok(data_actual_paths)
}

fn get_target_db_info(conn: &Connection) -> (Option<String>, Option<String>) {
    let target_dbid: Option<String> = conn
        .query_row("SELECT DBID FROM djmdProperty LIMIT 1", [], |row| {
            row.get(0)
        })
        .ok();
    let target_device_id: Option<String> = conn
        .query_row(
            "SELECT ID FROM djmdDevice WHERE rb_local_deleted = 0 LIMIT 1",
            [],
            |row| row.get(0),
        )
        .ok();
    (target_dbid, target_device_id)
}

fn get_share_dir() -> PathBuf {
    if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").unwrap_or_default();
        let candidates = [
            PathBuf::from(&home)
                .join("Library/Application Support/Pioneer/rekordbox/share"),
            PathBuf::from(&home).join("Library/Pioneer/rekordbox/share"),
        ];
        candidates
            .into_iter()
            .find(|p| p.exists())
            .unwrap_or_else(|| {
                PathBuf::from(&home)
                    .join("Library/Application Support/Pioneer/rekordbox/share")
            })
    } else if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        PathBuf::from(&appdata)
            .join("Pioneer")
            .join("rekordbox")
            .join("share")
    } else {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(&home).join(".Pioneer/rekordbox/share")
    }
}

fn insert_master_tables(
    tx: &Connection,
    conn: &Connection,
    tables: &serde_json::Map<String, serde_json::Value>,
    id_map: &IdMap,
    inserted_count: &mut u32,
    skipped_count: &mut u32,
) -> Result<()> {
    for &table in MASTER_TABLES {
        let rows = match tables.get(table).and_then(|v| v.as_array()) {
            Some(r) => r,
            None => continue,
        };
        for row in rows {
            let old_id = match row.get("ID").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => continue,
            };
            let mapped_row = apply_mapping(row, table, id_map);
            let new_id = mapped_row
                .get("ID")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let old_id_str = old_id.to_string();
            if let Some(table_map) = id_map.get(table)
                && let Some(mapped_id) = table_map.get(&old_id_str) {
                    let exists: bool = conn
                        .query_row(
                            &format!("SELECT 1 FROM `{}` WHERE ID = ?", table),
                            params![mapped_id],
                            |_| Ok(true),
                        )
                        .unwrap_or(false);
                    if exists {
                        *skipped_count += 1;
                        continue;
                    }
                }
            insert_row(tx, table, &mapped_row)
                .with_context(|| format!("{} への挿入に失敗 (ID: {})", table, new_id))?;
            *inserted_count += 1;
        }
    }
    Ok(())
}

fn insert_content_rows(
    tx: &Connection,
    tables: &serde_json::Map<String, serde_json::Value>,
    id_map: &IdMap,
    skipped_content_ids: &HashSet<String>,
    audio_actual_paths: &HashMap<String, String>,
    dest_dir: &str,
    target_dbid: &Option<String>,
    target_device_id: &Option<String>,
    inserted_count: &mut u32,
    skipped_count: &mut u32,
) -> Result<()> {
    let content_table = "djmdContent";
    if let Some(rows) = tables.get(content_table).and_then(|v| v.as_array()) {
        for row in rows {
            let old_id = match row.get("ID").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => continue,
            };
            if skipped_content_ids.contains(&old_id) {
                *skipped_count += 1;
                continue;
            }

            let mut mapped_row = apply_mapping(row, content_table, id_map);

            if let Some(obj) = mapped_row.as_object_mut() {
                if let Some(dbid) = target_dbid {
                    obj.insert(
                        "MasterDBID".to_string(),
                        serde_json::Value::String(dbid.clone()),
                    );
                }
                if let Some(dev_id) = target_device_id {
                    obj.insert(
                        "DeviceID".to_string(),
                        serde_json::Value::String(dev_id.clone()),
                    );
                }
            }

            if let Some(obj) = mapped_row.as_object_mut() {
                if let Some(actual_path) = audio_actual_paths.get(&old_id) {
                    obj.insert(
                        "FolderPath".to_string(),
                        serde_json::Value::String(actual_path.clone()),
                    );
                    obj.insert(
                        "rb_LocalFolderPath".to_string(),
                        serde_json::Value::String(actual_path.clone()),
                    );
                } else {
                    let dest_normalized = dest_dir.replace('\\', "/");
                    let dest_with_slash = if dest_normalized.ends_with('/') {
                        dest_normalized.clone()
                    } else {
                        format!("{}/", dest_normalized)
                    };
                    obj.insert(
                        "FolderPath".to_string(),
                        serde_json::Value::String(dest_with_slash.clone()),
                    );
                    obj.insert(
                        "rb_LocalFolderPath".to_string(),
                        serde_json::Value::String(dest_with_slash),
                    );
                }
            }

            insert_row(tx, content_table, &mapped_row)
                .with_context(|| {
                    format!("djmdContent への挿入に失敗 (old ID: {})", old_id)
                })?;
            *inserted_count += 1;
        }
    }
    Ok(())
}

fn insert_related_tables(
    tx: &Connection,
    tables: &serde_json::Map<String, serde_json::Value>,
    id_map: &IdMap,
    skipped_content_ids: &HashSet<String>,
    data_actual_paths: &HashMap<String, String>,
    share_dir: &std::path::Path,
    inserted_count: &mut u32,
    skipped_count: &mut u32,
) -> Result<()> {
    for &table in RELATED_TABLES {
        let rows = match tables.get(table).and_then(|v| v.as_array()) {
            Some(r) => r,
            None => continue,
        };
        for row in rows {
            if let Some(cid) = row.get("ContentID").and_then(|v| v.as_str())
                && skipped_content_ids.contains(cid) {
                    *skipped_count += 1;
                    continue;
                }
            let mut mapped_row = apply_mapping(row, table, id_map);

            if table == "contentFile" {
                let cf_id = row.get("ID").and_then(|v| v.as_str()).unwrap_or("");
                if let Some(actual_path) = data_actual_paths.get(cf_id) {
                    if let Some(obj) = mapped_row.as_object_mut() {
                        obj.insert(
                            "rb_local_path".to_string(),
                            serde_json::Value::String(actual_path.clone()),
                        );
                    }
                } else if let Some(obj) = mapped_row.as_object_mut()
                    && let Some(rel_path) = obj
                        .get("Path")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                    {
                        let rel_trimmed = rel_path.trim_start_matches('/');
                        let new_local = share_dir.join(rel_trimmed);
                        let new_local_str = new_local.to_string_lossy().to_string();
                        obj.insert(
                            "rb_local_path".to_string(),
                            serde_json::Value::String(new_local_str),
                        );
                    }
            }

            remap_json_blob(&mut mapped_row, table, id_map);

            let new_id = mapped_row
                .get("ID")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            insert_row(tx, table, &mapped_row)
                .with_context(|| format!("{} への挿入に失敗 (ID: {})", table, new_id))?;
            *inserted_count += 1;
        }
    }
    Ok(())
}

fn insert_playlist_and_songs(
    tx: &Connection,
    tables: &serde_json::Map<String, serde_json::Value>,
    pack_data: &serde_json::Value,
    id_map: &IdMap,
    inserted_count: &mut u32,
) -> Result<()> {
    if let Some(playlist) = pack_data.get("playlist") {
        let mapped_playlist = apply_mapping(playlist, "djmdPlaylist", id_map);
        let mut mapped = mapped_playlist.clone();
        if let Some(obj) = mapped.as_object_mut() {
            obj.insert(
                "ParentID".to_string(),
                serde_json::Value::String("root".to_string()),
            );
        }
        insert_row(tx, "djmdPlaylist", &mapped).context("djmdPlaylist への挿入に失敗")?;
        *inserted_count += 1;
    }

    if let Some(rows) = tables
        .get("djmdSongPlaylist")
        .and_then(|v| v.as_array())
    {
        for row in rows {
            let mapped_row = apply_mapping(row, "djmdSongPlaylist", id_map);
            let new_id = mapped_row
                .get("ID")
                .and_then(|v| v.as_str())
                .unwrap_or("?");
            insert_row(tx, "djmdSongPlaylist", &mapped_row).with_context(|| {
                format!("djmdSongPlaylist への挿入に失敗 (ID: {})", new_id)
            })?;
            *inserted_count += 1;
        }
    }

    Ok(())
}

pub fn unpack_playlist(
    conn: &Connection,
    pack_path: &str,
    dest_dir: &str,
    progress: &dyn Fn(&str),
) -> Result<()> {
    let rkp_path = PathBuf::from(pack_path);
    let rkp_file = fs::File::open(&rkp_path)
        .with_context(|| format!(".rkp ファイルを開けません: {}", rkp_path.display()))?;
    let mut archive = ZipArchive::new(rkp_file)
        .with_context(|| format!(".rkp ファイルの解析に失敗: {}", rkp_path.display()))?;

    let pack_data = load_pack_data(&mut archive)?;

    let tables = pack_data["tables"]
        .as_object()
        .context("tables が見つかりません")?;

    let (skipped_content_ids, existing_content_map) =
        detect_duplicate_contents(conn, tables, progress)?;

    let mut id_map: IdMap = HashMap::new();
    build_master_id_map(conn, tables, &mut id_map)?;
    build_content_id_map(conn, tables, &existing_content_map, &mut id_map)?;
    build_related_id_maps(conn, tables, &pack_data, &mut id_map)?;

    let share_dir = get_share_dir();

    let audio_actual_paths =
        extract_audio_files(&mut archive, &pack_data, dest_dir, &skipped_content_ids, progress)?;
    let data_actual_paths =
        extract_data_files(&mut archive, &pack_data, &share_dir, progress)?;

    let (target_dbid, target_device_id) = get_target_db_info(conn);

    progress("DBへの挿入を開始...");

    let tx = conn.unchecked_transaction()?;

    let mut inserted_count = 0u32;
    let mut skipped_count = 0u32;

    insert_master_tables(&tx, conn, tables, &id_map, &mut inserted_count, &mut skipped_count)?;

    insert_content_rows(
        &tx,
        tables,
        &id_map,
        &skipped_content_ids,
        &audio_actual_paths,
        dest_dir,
        &target_dbid,
        &target_device_id,
        &mut inserted_count,
        &mut skipped_count,
    )?;

    insert_related_tables(
        &tx,
        tables,
        &id_map,
        &skipped_content_ids,
        &data_actual_paths,
        &share_dir,
        &mut inserted_count,
        &mut skipped_count,
    )?;

    insert_playlist_and_songs(&tx, tables, &pack_data, &id_map, &mut inserted_count)?;

    tx.commit()?;

    progress("アンパック完了!");
    progress(&format!(
        "挿入: {} 行, スキップ(重複等): {} 行",
        inserted_count, skipped_count
    ));
    if !skipped_content_ids.is_empty() {
        progress(&format!(
            "重複トラック: {} 件 (contentFile.Hash一致)",
            skipped_content_ids.len()
        ));
    }

    Ok(())
}
