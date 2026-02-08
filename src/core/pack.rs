use std::collections::HashSet;
use std::fs;
use std::io::{self, Write as _};
use std::path::PathBuf;

use anyhow::{Context, Result};
use rusqlite::Connection;
use serde_json::json;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

use super::db::to_nfc;
use super::query::{collect_ids_from_column, query_by_content_ids, query_by_ids, query_table_rows};

pub(crate) fn add_file_to_rkp<W: io::Write + io::Seek>(
    writer: &mut ZipWriter<W>,
    entry_name: &str,
    source_path: &std::path::Path,
) -> Result<()> {
    let options =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    writer.start_file(entry_name, options)?;
    let mut f = fs::File::open(source_path)
        .with_context(|| format!("ファイルを開けません: {}", source_path.display()))?;
    io::copy(&mut f, writer)?;
    Ok(())
}

fn find_playlist(
    conn: &Connection,
    playlist_name: &str,
    progress: &dyn Fn(&str),
) -> Result<serde_json::Value> {
    let playlists = query_table_rows(
        conn,
        "SELECT * FROM djmdPlaylist WHERE Name = ? AND rb_local_deleted = 0",
        &[&playlist_name],
    )?;

    if playlists.is_empty() {
        anyhow::bail!("プレイリスト '{}' が見つかりません", playlist_name);
    }
    if playlists.len() > 1 {
        progress(&format!(
            "同名のプレイリストが {} 件見つかりました:",
            playlists.len()
        ));
        for p in &playlists {
            progress(&format!("  ID: {}", p["ID"].as_str().unwrap_or("?")));
        }
        anyhow::bail!("プレイリスト名が一意ではありません。IDで指定してください。");
    }
    Ok(playlists.into_iter().next().unwrap())
}

struct PackData {
    playlist: serde_json::Value,
    song_playlists: Vec<serde_json::Value>,
    contents: Vec<serde_json::Value>,
    artists: Vec<serde_json::Value>,
    albums: Vec<serde_json::Value>,
    genres: Vec<serde_json::Value>,
    keys: Vec<serde_json::Value>,
    labels: Vec<serde_json::Value>,
    colors: Vec<serde_json::Value>,
    cues: Vec<serde_json::Value>,
    active_censors: Vec<serde_json::Value>,
    mixer_params: Vec<serde_json::Value>,
    my_tags: Vec<serde_json::Value>,
    song_my_tags: Vec<serde_json::Value>,
    song_tag_lists: Vec<serde_json::Value>,
    hot_cue_banklists: Vec<serde_json::Value>,
    song_hot_cue_banklists: Vec<serde_json::Value>,
    hot_cue_banklist_cues: Vec<serde_json::Value>,
    content_cues: Vec<serde_json::Value>,
    content_active_censors: Vec<serde_json::Value>,
    content_files: Vec<serde_json::Value>,
}

fn collect_pack_data(
    conn: &Connection,
    playlist: serde_json::Value,
    progress: &dyn Fn(&str),
) -> Result<PackData> {
    let playlist_id = playlist["ID"]
        .as_str()
        .context("プレイリストのIDが取得できません")?;
    let playlist_name = playlist["Name"].as_str().unwrap_or("?");
    progress(&format!("プレイリスト: {} (ID: {})", playlist_name, playlist_id));

    let song_playlists = query_table_rows(
        conn,
        "SELECT * FROM djmdSongPlaylist WHERE PlaylistID = ? AND rb_local_deleted = 0",
        &[&playlist_id],
    )?;
    let content_ids = collect_ids_from_column(&song_playlists, "ContentID");
    progress(&format!("トラック数: {}", content_ids.len()));

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
    let hot_cue_banklist_cues = query_by_ids(
        conn,
        "hotCueBanklistCue",
        "HotCueBanklistID",
        &hot_cue_banklist_ids,
    )?;

    Ok(PackData {
        playlist,
        song_playlists,
        contents,
        artists,
        albums,
        genres,
        keys,
        labels,
        colors,
        cues,
        active_censors,
        mixer_params,
        my_tags,
        song_my_tags,
        song_tag_lists,
        hot_cue_banklists,
        song_hot_cue_banklists,
        hot_cue_banklist_cues,
        content_cues,
        content_active_censors,
        content_files,
    })
}

struct FileCopyStats {
    success: u32,
    skip: u32,
    fail: u32,
}

fn pack_audio_files<W: io::Write + io::Seek>(
    writer: &mut ZipWriter<W>,
    contents: &[serde_json::Value],
    keep_structure: bool,
    progress: &dyn Fn(&str),
) -> Result<(Vec<serde_json::Value>, FileCopyStats)> {
    let mut audio_files: Vec<serde_json::Value> = Vec::new();
    let mut stats = FileCopyStats { success: 0, skip: 0, fail: 0 };
    let total_contents = contents.len();

    for (idx, content) in contents.iter().enumerate() {
        let content_id = match content["ID"].as_str() {
            Some(id) => id,
            None => continue,
        };
        let folder_path = match content["FolderPath"].as_str() {
            Some(p) => p,
            None => {
                stats.skip += 1;
                continue;
            }
        };

        let source_path = PathBuf::from(folder_path);

        let file_name = source_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        progress(&format!(
            "音声ファイル ({}/{}) {}",
            idx + 1,
            total_contents,
            file_name
        ));

        let relative = if keep_structure {
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
            match add_file_to_rkp(writer, &entry_name, &source_path) {
                Ok(_) => {
                    stats.success += 1;
                    audio_files.push(json!({
                        "content_id": content_id,
                        "relative_path": to_nfc(&relative),
                    }));
                }
                Err(e) => {
                    progress(&format!(
                        "警告: ファイル追加失敗: {}: {}",
                        source_path.display(),
                        e
                    ));
                    stats.fail += 1;
                }
            }
        } else {
            progress(&format!(
                "警告: 音声ファイルが見つかりません: {}",
                source_path.display()
            ));
            stats.skip += 1;
        }
    }

    Ok((audio_files, stats))
}

fn pack_content_data_files<W: io::Write + io::Seek>(
    writer: &mut ZipWriter<W>,
    content_files: &[serde_json::Value],
    progress: &dyn Fn(&str),
) -> Result<(Vec<serde_json::Value>, FileCopyStats)> {
    let mut data_files: Vec<serde_json::Value> = Vec::new();
    let mut stats = FileCopyStats { success: 0, skip: 0, fail: 0 };
    let total_data_files = content_files.len();

    for (idx, cf) in content_files.iter().enumerate() {
        let cf_id = cf.get("ID").and_then(|v| v.as_str()).unwrap_or("");
        let local_path = match cf.get("rb_local_path").and_then(|v| v.as_str()) {
            Some(p) if !p.is_empty() => p,
            _ => {
                stats.skip += 1;
                continue;
            }
        };
        let pioneer_rel = match cf.get("Path").and_then(|v| v.as_str()) {
            Some(p) if !p.is_empty() => p.trim_start_matches('/').to_string(),
            _ => {
                stats.skip += 1;
                continue;
            }
        };

        progress(&format!(
            "データファイル ({}/{}) {}",
            idx + 1,
            total_data_files,
            pioneer_rel
        ));

        let source = PathBuf::from(local_path);

        if source.exists() {
            let entry_name = format!("content_data/{}", pioneer_rel.replace('\\', "/"));
            match add_file_to_rkp(writer, &entry_name, &source) {
                Ok(_) => {
                    stats.success += 1;
                    data_files.push(json!({
                        "content_file_id": cf_id,
                        "relative_path": to_nfc(&pioneer_rel),
                    }));
                }
                Err(e) => {
                    progress(&format!(
                        "警告: データファイル追加失敗: {}: {}",
                        source.display(),
                        e
                    ));
                    stats.fail += 1;
                }
            }
        } else {
            progress(&format!(
                "警告: データファイルが見つかりません: {}",
                source.display()
            ));
            stats.skip += 1;
        }
    }

    Ok((data_files, stats))
}

pub fn pack_playlist(
    conn: &Connection,
    output: &str,
    playlist_name: &str,
    keep_structure: bool,
    progress: &dyn Fn(&str),
) -> Result<()> {
    let playlist = find_playlist(conn, playlist_name, progress)?;
    let data = collect_pack_data(conn, playlist, progress)?;

    let output_path = PathBuf::from(output);
    if let Some(parent) = output_path.parent()
        && !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| {
                format!("出力ディレクトリの作成に失敗: {}", parent.display())
            })?;
        }
    let rkp_file = fs::File::create(&output_path)
        .with_context(|| format!(".rkp ファイルの作成に失敗: {}", output_path.display()))?;
    let mut writer = ZipWriter::new(rkp_file);

    let (audio_files, audio_stats) =
        pack_audio_files(&mut writer, &data.contents, keep_structure, progress)?;
    let (content_data_files, data_stats) =
        pack_content_data_files(&mut writer, &data.content_files, progress)?;

    let pack_data = json!({
        "version": 1,
        "playlist": data.playlist,
        "tables": {
            "djmdSongPlaylist": data.song_playlists,
            "djmdContent": data.contents,
            "djmdArtist": data.artists,
            "djmdAlbum": data.albums,
            "djmdGenre": data.genres,
            "djmdKey": data.keys,
            "djmdLabel": data.labels,
            "djmdColor": data.colors,
            "djmdCue": data.cues,
            "djmdActiveCensor": data.active_censors,
            "djmdMixerParam": data.mixer_params,
            "djmdMyTag": data.my_tags,
            "djmdSongMyTag": data.song_my_tags,
            "djmdSongTagList": data.song_tag_lists,
            "djmdHotCueBanklist": data.hot_cue_banklists,
            "djmdSongHotCueBanklist": data.song_hot_cue_banklists,
            "hotCueBanklistCue": data.hot_cue_banklist_cues,
            "contentCue": data.content_cues,
            "contentActiveCensor": data.content_active_censors,
            "contentFile": data.content_files,
        },
        "audio_files": audio_files,
        "content_data_files": content_data_files,
    });

    let options =
        SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    writer.start_file("pack.json", options)?;
    let json_bytes = serde_json::to_vec_pretty(&pack_data)
        .context("pack.json のシリアライズに失敗")?;
    writer.write_all(&json_bytes)?;

    writer.finish()?;

    progress(&format!("パック完了: {}", output_path.display()));
    progress(&format!(
        "音声ファイル: 成功={}, スキップ={}, 失敗={}",
        audio_stats.success, audio_stats.skip, audio_stats.fail
    ));
    progress(&format!(
        "データファイル(artwork/分析): 成功={}, スキップ={}, 失敗={}",
        data_stats.success, data_stats.skip, data_stats.fail
    ));
    if let Some(tables) = pack_data["tables"].as_object() {
        let mut table_summary = String::from("テーブルデータ:");
        for (name, rows) in tables {
            if let Some(arr) = rows.as_array()
                && !arr.is_empty() {
                    table_summary.push_str(&format!(" {}={}行", name, arr.len()));
                }
        }
        progress(&table_summary);
    }

    Ok(())
}
