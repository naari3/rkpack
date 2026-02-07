use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write as _};
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rusqlite::types::Value;
use rusqlite::{Connection, OpenFlags, params};
use serde_json::json;
use unicode_normalization::UnicodeNormalization;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

const DEFAULT_KEY: &str = "402fd482c38817c35ffa8ffb8c7d93143b749e7d315df7a81732a1ff43608497";

fn default_db_path() -> Result<PathBuf> {
    let candidates = if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").context("$HOME が設定されていません")?;
        vec![
            PathBuf::from(&home)
                .join("Library/Application Support/Pioneer/rekordbox/master.db"),
            PathBuf::from(&home)
                .join("Library/Pioneer/rekordbox/master.db"),
        ]
    } else if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").context("%APPDATA% が設定されていません")?;
        vec![
            PathBuf::from(&appdata)
                .join("Pioneer")
                .join("rekordbox")
                .join("master.db"),
        ]
    } else {
        // Linux/Wine
        let home = std::env::var("HOME").context("$HOME が設定されていません")?;
        vec![
            PathBuf::from(&home).join(".Pioneer/rekordbox/master.db"),
        ]
    };

    for path in &candidates {
        if path.exists() {
            return Ok(path.clone());
        }
    }

    let paths_str = candidates
        .iter()
        .map(|p| format!("  {}", p.display()))
        .collect::<Vec<_>>()
        .join("\n");
    anyhow::bail!("master.db が見つかりません。探索パス:\n{}", paths_str);
}

/// rekordbox の master.db を操作するCLIツール
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// master.db のパス (省略時は自動検出)
    #[arg(long)]
    db_path: Option<String>,

    /// SQLCipher の復号キー (省略時はデフォルトキー)
    #[arg(long)]
    key: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// テーブル一覧とスキーマを表示
    ListTables,

    /// 暗号化DBを平文SQLiteにエクスポート
    Export {
        /// 出力先ファイルパス
        output: String,
    },

    /// プレイリスト一覧を表示
    ListPlaylists,

    /// プレイリストの全関連データと音声ファイルを .rkp にパック
    Pack {
        /// 出力先 .rkp ファイルパス
        output: String,

        /// パックするプレイリスト名
        #[arg(long)]
        playlist: String,

        /// 音声ファイルのディレクトリ構造を維持する
        #[arg(long)]
        keep_structure: bool,
    },

    /// パックされた .rkp を別DBにインポート
    Unpack {
        /// パック .rkp ファイルのパス
        pack_path: String,

        /// 音声ファイルの配置先ディレクトリ
        #[arg(long)]
        dest_dir: String,
    },
}

fn is_plain_sqlite(path: &PathBuf) -> bool {
    fs::read(path)
        .map(|buf| buf.len() >= 16 && &buf[..16] == b"SQLite format 3\0")
        .unwrap_or(false)
}

fn open_rekordbox_db(db_path: &PathBuf, key: &str, read_only: bool) -> Result<Connection> {
    let flags = if read_only {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    } else {
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
    };
    let conn = Connection::open_with_flags(db_path, flags)
        .with_context(|| format!("DB を開けません: {}", db_path.display()))?;

    if !is_plain_sqlite(db_path) {
        conn.pragma_update(None, "cipher_compatibility", 4)?;
        conn.pragma_update(None, "key", key)?;
    }

    Ok(conn)
}

fn export_decrypted(conn: &Connection, export_path: &str) -> Result<()> {
    let export_path = std::path::absolute(export_path)
        .with_context(|| format!("パスの解決に失敗: {}", export_path))?;
    if export_path.exists() {
        anyhow::bail!("既にファイルが存在します: {}", export_path.display());
    }

    let export_path_str = export_path.to_string_lossy().replace('\\', "/");
    conn.execute_batch(&format!(
        "ATTACH DATABASE '{}' AS plaintext KEY '';",
        export_path_str
    ))?;
    conn.query_row("SELECT sqlcipher_export('plaintext')", [], |_| Ok(()))?;
    conn.execute_batch("DETACH DATABASE plaintext;")?;

    println!("エクスポート完了: {}", export_path.display());
    Ok(())
}

fn format_create_table(sql: &str) -> String {
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

fn list_tables(conn: &Connection) -> Result<()> {
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

    println!("\n-- {} テーブル, {} インデックス", tables.len(), indexes.len());
    Ok(())
}

fn query_table_rows(
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
                Value::Blob(b) => {
                    serde_json::Value::String(base64_encode(&b))
                }
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

fn base64_encode(data: &[u8]) -> String {
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

fn to_nfc(s: &str) -> String {
    s.nfc().collect()
}

/// macOS (APFS) ではファイル名が NFD に変換されるため、
/// コピー先の親ディレクトリを読んでディスク上の実際のファイル名を返す。
fn get_actual_path_on_disk(expected: &std::path::Path) -> PathBuf {
    let Some(parent) = expected.parent() else {
        return expected.to_path_buf();
    };
    let Some(expected_name) = expected.file_name() else {
        return expected.to_path_buf();
    };
    let expected_nfc: String = expected_name.to_string_lossy().nfc().collect();

    if let Ok(entries) = fs::read_dir(parent) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_nfc: String = name.to_string_lossy().nfc().collect();
            if name_nfc == expected_nfc {
                return entry.path();
            }
        }
    }
    expected.to_path_buf()
}

fn list_playlists(conn: &Connection) -> Result<()> {
    let playlists = query_table_rows(
        conn,
        "SELECT p.ID, p.Name, p.Attribute, \
         (SELECT COUNT(*) FROM djmdSongPlaylist sp WHERE sp.PlaylistID = p.ID AND sp.rb_local_deleted = 0) as TrackCount \
         FROM djmdPlaylist p WHERE p.rb_local_deleted = 0 ORDER BY p.Seq",
        &[],
    )?;

    println!("{:<8} {:<6} {:<6} {}", "ID", "種別", "曲数", "名前");
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

fn collect_ids_from_column(rows: &[serde_json::Value], column: &str) -> HashSet<String> {
    let mut ids = HashSet::new();
    for row in rows {
        if let Some(serde_json::Value::String(id)) = row.get(column) {
            if !id.is_empty() {
                ids.insert(id.clone());
            }
        }
    }
    ids
}

fn query_by_ids(
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
    let params: Vec<&dyn rusqlite::types::ToSql> =
        id_vec.iter().map(|s| s as &dyn rusqlite::types::ToSql).collect();
    query_table_rows(conn, &sql, &params)
}

fn query_by_content_ids(
    conn: &Connection,
    table: &str,
    content_ids: &HashSet<String>,
) -> Result<Vec<serde_json::Value>> {
    query_by_ids(conn, table, "ContentID", content_ids)
}

fn add_file_to_rkp<W: io::Write + io::Seek>(
    writer: &mut ZipWriter<W>,
    entry_name: &str,
    source_path: &std::path::Path,
) -> Result<()> {
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    writer.start_file(entry_name, options)?;
    let mut f = fs::File::open(source_path)
        .with_context(|| format!("ファイルを開けません: {}", source_path.display()))?;
    io::copy(&mut f, writer)?;
    Ok(())
}

fn pack_playlist(conn: &Connection, output: &str, playlist_name: &str, keep_structure: bool) -> Result<()> {
    let playlists = query_table_rows(
        conn,
        "SELECT * FROM djmdPlaylist WHERE Name = ? AND rb_local_deleted = 0",
        &[&playlist_name],
    )?;

    if playlists.is_empty() {
        anyhow::bail!("プレイリスト '{}' が見つかりません", playlist_name);
    }
    if playlists.len() > 1 {
        println!("同名のプレイリストが {} 件見つかりました:", playlists.len());
        for p in &playlists {
            println!("  ID: {}", p["ID"].as_str().unwrap_or("?"));
        }
        anyhow::bail!("プレイリスト名が一意ではありません。IDで指定してください。");
    }
    let playlist = &playlists[0];
    let playlist_id = playlist["ID"].as_str().context("プレイリストのIDが取得できません")?;
    println!("プレイリスト: {} (ID: {})", playlist_name, playlist_id);

    let song_playlists = query_table_rows(
        conn,
        "SELECT * FROM djmdSongPlaylist WHERE PlaylistID = ? AND rb_local_deleted = 0",
        &[&playlist_id],
    )?;
    let content_ids = collect_ids_from_column(&song_playlists, "ContentID");
    println!("トラック数: {}", content_ids.len());

    let contents = query_by_ids(conn, "djmdContent", "ID", &content_ids)?;

    let mut artist_ids = HashSet::new();
    artist_ids.extend(collect_ids_from_column(&contents, "ArtistID"));
    artist_ids.extend(collect_ids_from_column(&contents, "OrgArtistID"));
    artist_ids.extend(collect_ids_from_column(&contents, "RemixerID"));
    artist_ids.extend(collect_ids_from_column(&contents, "ComposerID"));

    let album_ids = collect_ids_from_column(&contents, "AlbumID");
    let genre_ids = collect_ids_from_column(&contents, "GenreID");
    let key_ids = collect_ids_from_column(&contents, "KeyID");
    let label_ids = collect_ids_from_column(&contents, "LabelID");
    let color_ids = collect_ids_from_column(&contents, "ColorID");

    let albums = query_by_ids(conn, "djmdAlbum", "ID", &album_ids)?;
    artist_ids.extend(collect_ids_from_column(&albums, "AlbumArtistID"));

    let artists = query_by_ids(conn, "djmdArtist", "ID", &artist_ids)?;
    let genres = query_by_ids(conn, "djmdGenre", "ID", &genre_ids)?;
    let keys = query_by_ids(conn, "djmdKey", "ID", &key_ids)?;
    let labels = query_by_ids(conn, "djmdLabel", "ID", &label_ids)?;
    let colors = query_by_ids(conn, "djmdColor", "ID", &color_ids)?;

    let cues = query_by_content_ids(conn, "djmdCue", &content_ids)?;
    let active_censors = query_by_content_ids(conn, "djmdActiveCensor", &content_ids)?;
    let mixer_params = query_by_content_ids(conn, "djmdMixerParam", &content_ids)?;
    let song_my_tags = query_by_content_ids(conn, "djmdSongMyTag", &content_ids)?;
    let song_tag_lists = query_by_content_ids(conn, "djmdSongTagList", &content_ids)?;
    let song_hot_cue_banklists =
        query_by_content_ids(conn, "djmdSongHotCueBanklist", &content_ids)?;
    let content_cues = query_by_content_ids(conn, "contentCue", &content_ids)?;
    let content_active_censors =
        query_by_content_ids(conn, "contentActiveCensor", &content_ids)?;
    let content_files = query_by_content_ids(conn, "contentFile", &content_ids)?;

    let my_tag_ids = collect_ids_from_column(&song_my_tags, "MyTagID");
    let my_tags = query_by_ids(conn, "djmdMyTag", "ID", &my_tag_ids)?;

    let hot_cue_banklist_ids =
        collect_ids_from_column(&song_hot_cue_banklists, "HotCueBanklistID");
    let hot_cue_banklists =
        query_by_ids(conn, "djmdHotCueBanklist", "ID", &hot_cue_banklist_ids)?;
    let hot_cue_banklist_cues =
        query_by_ids(conn, "hotCueBanklistCue", "HotCueBanklistID", &hot_cue_banklist_ids)?;

    let output_path = PathBuf::from(output);
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("出力ディレクトリの作成に失敗: {}", parent.display()))?;
        }
    }
    let rkp_file = fs::File::create(&output_path)
        .with_context(|| format!(".rkp ファイルの作成に失敗: {}", output_path.display()))?;
    let mut writer = ZipWriter::new(rkp_file);

    let mut audio_files: Vec<serde_json::Value> = Vec::new();
    let mut copy_success = 0u32;
    let mut copy_skip = 0u32;
    let mut copy_fail = 0u32;

    for content in &contents {
        let content_id = match content["ID"].as_str() {
            Some(id) => id,
            None => continue,
        };
        // FolderPath はフォルダではなくファイルのフルパス
        let folder_path = match content["FolderPath"].as_str() {
            Some(p) => p,
            None => {
                copy_skip += 1;
                continue;
            }
        };

        let source_path = PathBuf::from(folder_path);

        let file_name = source_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        let relative = if keep_structure {
            // ドライブレター除去 (例: C:/foo/bar.mp4 → foo/bar.mp4)
            let full_str = folder_path.replace('\\', "/");
            if full_str.len() > 3 && full_str.chars().nth(1) == Some(':') {
                full_str[3..].to_string()
            } else {
                full_str
            }
        } else {
            file_name.clone()
        };

        if source_path.exists() {
            let entry_name = format!("files/{}", relative.replace('\\', "/"));
            match add_file_to_rkp(&mut writer, &entry_name, &source_path) {
                Ok(_) => {
                    copy_success += 1;
                    audio_files.push(json!({
                        "content_id": content_id,
                        "relative_path": to_nfc(&relative),
                    }));
                }
                Err(e) => {
                    eprintln!(
                        "警告: ファイル追加失敗: {}: {}",
                        source_path.display(),
                        e
                    );
                    copy_fail += 1;
                }
            }
        } else {
            eprintln!(
                "警告: 音声ファイルが見つかりません: {}",
                source_path.display()
            );
            copy_skip += 1;
        }
    }

    let mut content_data_files: Vec<serde_json::Value> = Vec::new();
    let mut data_copy_success = 0u32;
    let mut data_copy_skip = 0u32;
    let mut data_copy_fail = 0u32;

    for cf in &content_files {
        let cf_id = cf.get("ID").and_then(|v| v.as_str()).unwrap_or("");
        let local_path = match cf.get("rb_local_path").and_then(|v| v.as_str()) {
            Some(p) if !p.is_empty() => p,
            _ => {
                data_copy_skip += 1;
                continue;
            }
        };
        // Path は /PIONEER/... の相対パス
        let pioneer_rel = match cf.get("Path").and_then(|v| v.as_str()) {
            Some(p) if !p.is_empty() => p.trim_start_matches('/').to_string(),
            _ => {
                data_copy_skip += 1;
                continue;
            }
        };

        let source = PathBuf::from(local_path);

        if source.exists() {
            let entry_name = format!("content_data/{}", pioneer_rel.replace('\\', "/"));
            match add_file_to_rkp(&mut writer, &entry_name, &source) {
                Ok(_) => {
                    data_copy_success += 1;
                    content_data_files.push(json!({
                        "content_file_id": cf_id,
                        "relative_path": to_nfc(&pioneer_rel),
                    }));
                }
                Err(e) => {
                    eprintln!("警告: データファイル追加失敗: {}: {}", source.display(), e);
                    data_copy_fail += 1;
                }
            }
        } else {
            eprintln!("警告: データファイルが見つかりません: {}", source.display());
            data_copy_skip += 1;
        }
    }

    let pack_data = json!({
        "version": 1,
        "playlist": playlist,
        "tables": {
            "djmdSongPlaylist": song_playlists,
            "djmdContent": contents,
            "djmdArtist": artists,
            "djmdAlbum": albums,
            "djmdGenre": genres,
            "djmdKey": keys,
            "djmdLabel": labels,
            "djmdColor": colors,
            "djmdCue": cues,
            "djmdActiveCensor": active_censors,
            "djmdMixerParam": mixer_params,
            "djmdMyTag": my_tags,
            "djmdSongMyTag": song_my_tags,
            "djmdSongTagList": song_tag_lists,
            "djmdHotCueBanklist": hot_cue_banklists,
            "djmdSongHotCueBanklist": song_hot_cue_banklists,
            "hotCueBanklistCue": hot_cue_banklist_cues,
            "contentCue": content_cues,
            "contentActiveCensor": content_active_censors,
            "contentFile": content_files,
        },
        "audio_files": audio_files,
        "content_data_files": content_data_files,
    });

    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    writer.start_file("pack.json", options)?;
    let json_bytes = serde_json::to_vec_pretty(&pack_data)
        .context("pack.json のシリアライズに失敗")?;
    writer.write_all(&json_bytes)?;

    writer.finish()?;

    println!("\nパック完了: {}", output_path.display());
    println!(
        "音声ファイル: 成功={}, スキップ={}, 失敗={}",
        copy_success, copy_skip, copy_fail
    );
    println!(
        "データファイル(artwork/分析): 成功={}, スキップ={}, 失敗={}",
        data_copy_success, data_copy_skip, data_copy_fail
    );
    println!("テーブルデータ:");
    if let Some(tables) = pack_data["tables"].as_object() {
        for (name, rows) in tables {
            if let Some(arr) = rows.as_array() {
                if !arr.is_empty() {
                    println!("  {}: {} 行", name, arr.len());
                }
            }
        }
    }

    Ok(())
}

fn get_max_numeric_id(conn: &Connection, table: &str) -> Result<i64> {
    let sql = format!(
        "SELECT MAX(CAST(ID AS INTEGER)) FROM `{}`",
        table
    );
    let max_id: Option<i64> = conn.query_row(&sql, [], |row| row.get(0)).unwrap_or(None);
    Ok(max_id.unwrap_or(0))
}

fn find_existing_master_id(
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

/// マスタテーブルの重複検出に使う名前比較カラム
fn master_table_name_column(table: &str) -> Option<&'static str> {
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

fn fk_columns_for_table(table: &str) -> Vec<(&'static str, &'static str)> {
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

fn insert_row(
    conn: &Connection,
    table: &str,
    row: &serde_json::Value,
) -> Result<()> {
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

fn extract_rkp_entry(archive: &mut ZipArchive<fs::File>, name: &str, dest: &std::path::Path) -> Result<()> {
    let mut entry = archive.by_name(name)
        .with_context(|| format!(".rkp 内にエントリが見つかりません: {}", name))?;
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut out = fs::File::create(dest)
        .with_context(|| format!("ファイルの作成に失敗: {}", dest.display()))?;
    io::copy(&mut entry, &mut out)?;
    Ok(())
}

fn unpack_playlist(conn: &Connection, pack_path: &str, dest_dir: &str) -> Result<()> {
    let rkp_path = PathBuf::from(pack_path);
    let rkp_file = fs::File::open(&rkp_path)
        .with_context(|| format!(".rkp ファイルを開けません: {}", rkp_path.display()))?;
    let mut archive = ZipArchive::new(rkp_file)
        .with_context(|| format!(".rkp ファイルの解析に失敗: {}", rkp_path.display()))?;

    let pack_data: serde_json::Value = {
        let entry = archive.by_name("pack.json")
            .context(".rkp 内に pack.json が見つかりません")?;
        serde_json::from_reader(entry).context("pack.json の解析に失敗")?
    };

    let version = pack_data["version"].as_i64().unwrap_or(0);
    if version != 1 {
        anyhow::bail!("未対応のパックバージョン: {}", version);
    }

    let tables = pack_data["tables"]
        .as_object()
        .context("tables が見つかりません")?;

    let mut id_map: HashMap<String, HashMap<String, String>> = HashMap::new();

    // 重複トラック検出 (contentFile.Hash)
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
                    println!(
                        "重複トラック検出: ContentID {} (Hash: {}) → 既存 ContentID {}",
                        pack_cid, hash, existing_cid
                    );
                    skipped_content_ids.insert(pack_cid.to_string());
                    existing_content_map.insert(pack_cid.to_string(), existing_cid);
                }
            }
        }
    }

    let master_tables = [
        "djmdArtist",
        "djmdAlbum",
        "djmdGenre",
        "djmdKey",
        "djmdLabel",
        "djmdColor",
        "djmdMyTag",
        "djmdHotCueBanklist",
    ];
    let content_table = "djmdContent";
    let related_tables = [
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

    for &table in &master_tables {
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
                let name_val = row.get(name_col).and_then(|v| match v {
                    serde_json::Value::String(s) => Some(s.as_str()),
                    _ => None,
                });
                // djmdColor は ColorCode (数値) で既存検索
                let existing_id = if table == "djmdColor" {
                    if let Some(code) = row.get("ColorCode").and_then(|v| v.as_i64()) {
                        let sql = format!(
                            "SELECT ID FROM `{}` WHERE ColorCode = ? AND rb_local_deleted = 0 LIMIT 1",
                            table
                        );
                        conn.query_row(&sql, params![code], |row| row.get::<_, String>(0)).ok()
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

    {
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
    }

    {
        let mut max_id = get_max_numeric_id(conn, "djmdPlaylist")?;
        let mut table_map = HashMap::new();
        if let Some(playlist) = pack_data.get("playlist") {
            if let Some(old_id) = playlist.get("ID").and_then(|v| v.as_str()) {
                max_id += 1;
                table_map.insert(old_id.to_string(), max_id.to_string());
            }
        }
        id_map.insert("djmdPlaylist".to_string(), table_map);
    }

    let all_id_tables: Vec<&str> = related_tables
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

    let share_dir = if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").unwrap_or_default();
        let candidates = [
            PathBuf::from(&home).join("Library/Application Support/Pioneer/rekordbox/share"),
            PathBuf::from(&home).join("Library/Pioneer/rekordbox/share"),
        ];
        candidates.into_iter().find(|p| p.exists())
            .unwrap_or_else(|| PathBuf::from(&home).join("Library/Application Support/Pioneer/rekordbox/share"))
    } else if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").unwrap_or_default();
        PathBuf::from(&appdata).join("Pioneer").join("rekordbox").join("share")
    } else {
        let home = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(&home).join(".Pioneer/rekordbox/share")
    };

    let mut audio_actual_paths: HashMap<String, String> = HashMap::new();
    let mut file_copy_success = 0u32;
    let mut file_copy_skip = 0u32;
    let mut file_copy_fail = 0u32;

    if let Some(audio_files) = pack_data.get("audio_files").and_then(|v| v.as_array()) {
        let dest_path = PathBuf::from(dest_dir);
        let _ = fs::create_dir_all(&dest_path);
        for af in audio_files {
            let content_id = af.get("content_id").and_then(|v| v.as_str()).unwrap_or("");
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
            let target = dest_path.join(&file_name);

            match extract_rkp_entry(&mut archive, &entry_name, &target) {
                Ok(_) => {
                    file_copy_success += 1;
                    let actual = get_actual_path_on_disk(&target);
                    let actual_str = actual.to_string_lossy().replace('\\', "/");
                    audio_actual_paths.insert(content_id.to_string(), actual_str);
                }
                Err(e) => {
                    eprintln!("警告: 音声ファイル展開失敗: {}: {}", entry_name, e);
                    file_copy_fail += 1;
                }
            }
        }
    }
    println!("音声ファイル配置: 成功={}, スキップ={}, 失敗={}",
        file_copy_success, file_copy_skip, file_copy_fail);

    let mut data_actual_paths: HashMap<String, String> = HashMap::new();
    let mut data_file_success = 0u32;
    let data_file_skip = 0u32;
    let mut data_file_fail = 0u32;

    if let Some(data_files) = pack_data.get("content_data_files").and_then(|v| v.as_array()) {
        for df in data_files {
            let cf_id = df.get("content_file_id").and_then(|v| v.as_str()).unwrap_or("");
            let relative_path = match df.get("relative_path").and_then(|v| v.as_str()) {
                Some(p) => p,
                None => continue,
            };

            let entry_name = format!("content_data/{}", relative_path.replace('\\', "/"));
            let target = share_dir.join(relative_path);

            match extract_rkp_entry(&mut archive, &entry_name, &target) {
                Ok(_) => {
                    data_file_success += 1;
                    let actual = get_actual_path_on_disk(&target);
                    let actual_str = actual.to_string_lossy().to_string();
                    data_actual_paths.insert(cf_id.to_string(), actual_str);
                }
                Err(e) => {
                    eprintln!("警告: データファイル展開失敗: {}: {}", entry_name, e);
                    data_file_fail += 1;
                }
            }
        }
        println!("データファイル配置: 成功={}, スキップ={}, 失敗={}",
            data_file_success, data_file_skip, data_file_fail);
    }

    let target_dbid: Option<String> = conn
        .query_row("SELECT DBID FROM djmdProperty LIMIT 1", [], |row| row.get(0))
        .ok();
    let target_device_id: Option<String> = conn
        .query_row("SELECT ID FROM djmdDevice WHERE rb_local_deleted = 0 LIMIT 1", [], |row| row.get(0))
        .ok();

    let tx = conn.unchecked_transaction()?;

    let mut inserted_count = 0u32;
    let mut skipped_count = 0u32;

    let apply_mapping = |row: &serde_json::Value, table: &str| -> serde_json::Value {
        let mut row = row.clone();
        if let Some(obj) = row.as_object_mut() {
            if let Some(old_id) = obj.get("ID").and_then(|v| v.as_str()).map(|s| s.to_string()) {
                if let Some(table_map) = id_map.get(table) {
                    if let Some(new_id) = table_map.get(&old_id) {
                        obj.insert("ID".to_string(), serde_json::Value::String(new_id.clone()));
                    }
                }
            }

            for (fk_col, ref_table) in fk_columns_for_table(table) {
                if let Some(old_fk) = obj.get(fk_col).and_then(|v| v.as_str()).map(|s| s.to_string()) {
                    if let Some(ref_map) = id_map.get(ref_table) {
                        if let Some(new_fk) = ref_map.get(&old_fk) {
                            obj.insert(fk_col.to_string(), serde_json::Value::String(new_fk.clone()));
                        }
                    }
                }
            }

            // クラウド同期フィールドをリセット
            for &sync_field in &["rb_data_status", "rb_local_data_status", "rb_local_file_status"] {
                if obj.contains_key(sync_field) {
                    obj.insert(sync_field.to_string(), serde_json::Value::Number(0.into()));
                }
            }
            for &sync_field in &["rb_local_synced"] {
                if obj.contains_key(sync_field) {
                    obj.insert(sync_field.to_string(), serde_json::Value::Number(0.into()));
                }
            }
            for &sync_field in &["usn", "rb_local_usn", "rb_insync_local_usn"] {
                if obj.contains_key(sync_field) {
                    obj.insert(sync_field.to_string(), serde_json::Value::Null);
                }
            }
        }
        row
    };

    for &table in &master_tables {
        let rows = match tables.get(table).and_then(|v| v.as_array()) {
            Some(r) => r,
            None => continue,
        };
        for row in rows {
            let old_id = match row.get("ID").and_then(|v| v.as_str()) {
                Some(id) => id,
                None => continue,
            };
            let mapped_row = apply_mapping(row, table);
            let new_id = mapped_row.get("ID").and_then(|v| v.as_str()).unwrap_or("");
            let old_id_str = old_id.to_string();
            if let Some(table_map) = id_map.get(table) {
                if let Some(mapped_id) = table_map.get(&old_id_str) {
                    let exists: bool = conn
                        .query_row(
                            &format!("SELECT 1 FROM `{}` WHERE ID = ?", table),
                            params![mapped_id],
                            |_| Ok(true),
                        )
                        .unwrap_or(false);
                    if exists {
                        skipped_count += 1;
                        continue;
                    }
                }
            }
            insert_row(&tx, table, &mapped_row)
                .with_context(|| format!("{} への挿入に失敗 (ID: {})", table, new_id))?;
            inserted_count += 1;
        }
    }

    if let Some(rows) = tables.get(content_table).and_then(|v| v.as_array()) {
        for row in rows {
            let old_id = match row.get("ID").and_then(|v| v.as_str()) {
                Some(id) => id.to_string(),
                None => continue,
            };
            if skipped_content_ids.contains(&old_id) {
                skipped_count += 1;
                continue;
            }

            let mut mapped_row = apply_mapping(row, content_table);

            if let Some(obj) = mapped_row.as_object_mut() {
                if let Some(ref dbid) = target_dbid {
                    obj.insert("MasterDBID".to_string(), serde_json::Value::String(dbid.clone()));
                }
                if let Some(ref dev_id) = target_device_id {
                    obj.insert("DeviceID".to_string(), serde_json::Value::String(dev_id.clone()));
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

            insert_row(&tx, content_table, &mapped_row)
                .with_context(|| format!("djmdContent への挿入に失敗 (old ID: {})", old_id))?;
            inserted_count += 1;
        }
    }

    for &table in &related_tables {
        let rows = match tables.get(table).and_then(|v| v.as_array()) {
            Some(r) => r,
            None => continue,
        };
        for row in rows {
            if let Some(cid) = row.get("ContentID").and_then(|v| v.as_str()) {
                if skipped_content_ids.contains(cid) {
                    skipped_count += 1;
                    continue;
                }
            }
            let mut mapped_row = apply_mapping(row, table);

            if table == "contentFile" {
                let cf_id = row.get("ID").and_then(|v| v.as_str()).unwrap_or("");
                if let Some(actual_path) = data_actual_paths.get(cf_id) {
                    if let Some(obj) = mapped_row.as_object_mut() {
                        obj.insert("rb_local_path".to_string(), serde_json::Value::String(actual_path.clone()));
                    }
                } else if let Some(obj) = mapped_row.as_object_mut() {
                    if let Some(rel_path) = obj.get("Path").and_then(|v| v.as_str()).map(|s| s.to_string()) {
                        let rel_trimmed = rel_path.trim_start_matches('/');
                        let new_local = share_dir.join(rel_trimmed);
                        let new_local_str = new_local.to_string_lossy().to_string();
                        obj.insert("rb_local_path".to_string(), serde_json::Value::String(new_local_str));
                    }
                }
            }

            // JSON blob 内の ID/ContentID をリマップ
            let json_blob_field = match table {
                "contentCue" => Some("Cues"),
                "contentActiveCensor" => Some("ActiveCensors"),
                "hotCueBanklistCue" => Some("Cues"),
                _ => None,
            };
            let json_id_ref_table = match table {
                "contentCue" => Some("djmdCue"),
                "contentActiveCensor" => Some("djmdActiveCensor"),
                "hotCueBanklistCue" => Some("djmdSongHotCueBanklist"),
                _ => None,
            };
            if let (Some(blob_field), Some(ref_table)) = (json_blob_field, json_id_ref_table) {
                if let Some(obj) = mapped_row.as_object_mut() {
                    if let Some(blob_str) = obj.get(blob_field).and_then(|v| v.as_str()).map(|s| s.to_string()) {
                        if let Ok(mut arr) = serde_json::from_str::<Vec<serde_json::Value>>(&blob_str) {
                            let content_map = id_map.get("djmdContent");
                            let detail_map = id_map.get(ref_table);
                            for item in arr.iter_mut() {
                                if let Some(item_obj) = item.as_object_mut() {
                                    if let Some(d_map) = detail_map {
                                        if let Some(old_id) = item_obj.get("ID").and_then(|v| v.as_str()).map(|s| s.to_string()) {
                                            if let Some(new_id) = d_map.get(&old_id) {
                                                item_obj.insert("ID".to_string(), serde_json::Value::String(new_id.clone()));
                                            }
                                        }
                                    }
                                    if let Some(c_map) = content_map {
                                        if let Some(old_cid) = item_obj.get("ContentID").and_then(|v| v.as_str()).map(|s| s.to_string()) {
                                            if let Some(new_cid) = c_map.get(&old_cid) {
                                                item_obj.insert("ContentID".to_string(), serde_json::Value::String(new_cid.clone()));
                                            }
                                        }
                                    }
                                }
                            }
                            if let Ok(new_blob) = serde_json::to_string(&arr) {
                                obj.insert(blob_field.to_string(), serde_json::Value::String(new_blob));
                            }
                        }
                    }
                }
            }

            let new_id = mapped_row.get("ID").and_then(|v| v.as_str()).unwrap_or("?");
            insert_row(&tx, table, &mapped_row)
                .with_context(|| format!("{} への挿入に失敗 (ID: {})", table, new_id))?;
            inserted_count += 1;
        }
    }

    if let Some(playlist) = pack_data.get("playlist") {
        let mapped_playlist = apply_mapping(playlist, "djmdPlaylist");
        let mut mapped = mapped_playlist.clone();
        if let Some(obj) = mapped.as_object_mut() {
            obj.insert("ParentID".to_string(), serde_json::Value::String("root".to_string()));
        }
        insert_row(&tx, "djmdPlaylist", &mapped)
            .context("djmdPlaylist への挿入に失敗")?;
        inserted_count += 1;
    }

    if let Some(rows) = tables.get("djmdSongPlaylist").and_then(|v| v.as_array()) {
        for row in rows {
            let mapped_row = apply_mapping(row, "djmdSongPlaylist");
            let new_id = mapped_row.get("ID").and_then(|v| v.as_str()).unwrap_or("?");
            insert_row(&tx, "djmdSongPlaylist", &mapped_row)
                .with_context(|| {
                    format!("djmdSongPlaylist への挿入に失敗 (ID: {})", new_id)
                })?;
            inserted_count += 1;
        }
    }

    tx.commit()?;

    println!("\nアンパック完了!");
    println!(
        "挿入: {} 行, スキップ(重複等): {} 行",
        inserted_count, skipped_count
    );
    if !skipped_content_ids.is_empty() {
        println!(
            "重複トラック: {} 件 (contentFile.Hash一致)",
            skipped_content_ids.len()
        );
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let db_path = match cli.db_path {
        Some(p) => PathBuf::from(p),
        None => default_db_path()?,
    };
    let key = cli.key.as_deref().unwrap_or(DEFAULT_KEY);

    println!("DB: {}", db_path.display());

    let read_only = matches!(cli.command, Command::ListTables | Command::ListPlaylists | Command::Pack { .. });
    let conn = open_rekordbox_db(&db_path, key, read_only)?;

    match cli.command {
        Command::Export { output } => {
            export_decrypted(&conn, &output)?;
        }
        Command::ListTables => {
            list_tables(&conn)?;
        }
        Command::ListPlaylists => {
            list_playlists(&conn)?;
        }
        Command::Pack {
            output,
            playlist,
            keep_structure,
        } => {
            pack_playlist(&conn, &output, &playlist, keep_structure)?;
        }
        Command::Unpack { pack_path, dest_dir } => {
            unpack_playlist(&conn, &pack_path, &dest_dir)?;
        }
    }

    Ok(())
}
