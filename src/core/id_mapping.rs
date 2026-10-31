use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};

pub(crate) type IdMap = HashMap<String, HashMap<String, String>>;

/// rekordbox の ID 形式 (実 DB の観察に基づく。notes/id-allocation.md 参照)
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum IdKind {
    /// [100, 2^28) の乱数 (djmdContent)
    Random28,
    /// [100, 2^32) の乱数 (マスタ系、djmdPlaylist、djmdCue など)
    Random32,
    /// UUID v4 文字列 (djmdSongPlaylist、djmdMixerParam、contentCue など)
    Uuid,
    /// `<djmdContent.UUID>_<パーセントエンコードした Path>` (contentFile)
    ContentFile,
}

pub(crate) const CONTENT_ID_LIMIT: u64 = 1 << 28;
const MIN_RANDOM_ID: u64 = 100;

pub(crate) fn id_kind_for_table(table: &str) -> IdKind {
    match table {
        "djmdContent" => IdKind::Random28,
        "contentFile" => IdKind::ContentFile,
        // 観察で UUID だったテーブル
        "djmdSongPlaylist" | "djmdMixerParam" | "contentCue" | "djmdSongHistory"
        // データがなく推定 (notes/id-allocation.md 調査 3)
        | "contentActiveCensor" | "djmdSongMyTag" | "djmdSongTagList"
        | "djmdSongHotCueBanklist" | "hotCueBanklistCue" => IdKind::Uuid,
        _ => IdKind::Random32,
    }
}

/// rekordbox の採番方式に合わせて未使用の ID を払い出す。
/// DB 上の既存行 (論理削除済み含む) と、このセッションで払い出し済みの ID の両方を避ける。
#[derive(Default)]
pub(crate) struct IdAllocator {
    allocated: HashMap<String, HashSet<String>>,
}

impl IdAllocator {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// 呼び出し側で決めた ID (ユーザー指定の ContentID など) を払い出し済みとして登録する
    pub(crate) fn reserve(&mut self, table: &str, id: &str) {
        self.allocated
            .entry(table.to_string())
            .or_default()
            .insert(id.to_string());
    }

    fn is_free(&self, conn: &Connection, table: &str, id: &str) -> Result<bool> {
        if self
            .allocated
            .get(table)
            .is_some_and(|set| set.contains(id))
        {
            return Ok(false);
        }
        id_exists(conn, table, id).map(|exists| !exists)
    }

    /// Random28 / Random32 / Uuid のテーブルの新しい ID を払い出す
    pub(crate) fn allocate(&mut self, conn: &Connection, table: &str) -> Result<String> {
        let kind = id_kind_for_table(table);
        for _ in 0..1_000_000 {
            let candidate = match kind {
                IdKind::Random28 => random_id(CONTENT_ID_LIMIT),
                IdKind::Random32 => random_id(1 << 32),
                IdKind::Uuid => uuid::Uuid::new_v4().to_string(),
                IdKind::ContentFile => {
                    anyhow::bail!("contentFile の ID は allocate_content_file で払い出す")
                }
            };
            if self.is_free(conn, table, &candidate)? {
                self.reserve(table, &candidate);
                return Ok(candidate);
            }
        }
        anyhow::bail!("{} の未使用 ID を生成できませんでした", table)
    }

    /// contentFile の ID を rekordbox と同じ形式で払い出す。
    /// 同じ ID が既にある場合 (同じパックを新規として二重に取り込んだ場合など) は None を返す。
    pub(crate) fn allocate_content_file(
        &mut self,
        conn: &Connection,
        content_uuid: &str,
        path: &str,
    ) -> Result<Option<String>> {
        let candidate = content_file_id(content_uuid, path);
        if self.is_free(conn, "contentFile", &candidate)? {
            self.reserve("contentFile", &candidate);
            Ok(Some(candidate))
        } else {
            Ok(None)
        }
    }
}

fn random_id(limit: u64) -> String {
    loop {
        let v = rand::random::<u64>() % limit;
        if v >= MIN_RANDOM_ID {
            return v.to_string();
        }
    }
}

pub(crate) fn id_exists(conn: &Connection, table: &str, id: &str) -> Result<bool> {
    let sql = format!("SELECT 1 FROM `{}` WHERE ID = ? LIMIT 1", table);
    let exists = conn
        .query_row(&sql, params![id], |_| Ok(()))
        .optional()?
        .is_some();
    Ok(exists)
}

/// `<content UUID>_<Path>` で、Path は英数字と `_.-~` 以外をすべて %XX (大文字) にする
pub(crate) fn content_file_id(content_uuid: &str, path: &str) -> String {
    let mut encoded = String::with_capacity(path.len() * 3);
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-' | b'~') {
            encoded.push(b as char);
        } else {
            encoded.push_str(&format!("%{:02X}", b));
        }
    }
    format!("{}_{}", content_uuid, encoded)
}

/// localUpdateCount を基に、挿入する行の rb_local_usn を 1 ずつ払い出す
pub(crate) struct UsnCounter {
    current: i64,
}

impl UsnCounter {
    pub(crate) fn load(conn: &Connection) -> Result<Self> {
        let current: i64 = conn
            .query_row(
                "SELECT int_1 FROM agentRegistry WHERE registry_id = 'localUpdateCount'",
                [],
                |row| row.get::<_, Option<i64>>(0),
            )
            .optional()?
            .flatten()
            .context("agentRegistry の localUpdateCount が見つかりません")?;
        Ok(Self { current })
    }

    /// 行に新しい rb_local_usn を設定する (1 行ごとに +1)
    pub(crate) fn assign(&mut self, table: &str, row: &mut serde_json::Value) {
        if !Self::is_tracked(table, row) {
            return;
        }
        self.current += 1;
        Self::set(row, self.current);
    }

    /// 複数行に同じ rb_local_usn を設定する。rekordbox は djmdSongPlaylist のまとめ追加で
    /// 1 回の操作に 1 つの USN を使う (notes/id-allocation-ghidra.md、実 DB でも同じ傾向)
    pub(crate) fn assign_batch(&mut self, table: &str, rows: &mut [serde_json::Value]) {
        let tracked: Vec<usize> = (0..rows.len())
            .filter(|&i| Self::is_tracked(table, &rows[i]))
            .collect();
        if tracked.is_empty() {
            return;
        }
        self.current += 1;
        for i in tracked {
            Self::set(&mut rows[i], self.current);
        }
    }

    /// rekordbox がライブラリ同期の対象外 (NoLibrarySyncSql) として USN を付けないテーブルと、
    /// rb_local_usn 列のない行は対象外
    fn is_tracked(table: &str, row: &serde_json::Value) -> bool {
        const UNTRACKED: &[&str] = &[
            "djmdCue",
            "djmdActiveCensor",
            "djmdSongHotCueBanklist",
            "djmdSongTagList",
        ];
        !UNTRACKED.contains(&table)
            && row
                .as_object()
                .is_some_and(|obj| obj.contains_key("rb_local_usn"))
    }

    fn set(row: &mut serde_json::Value, usn: i64) {
        if let Some(obj) = row.as_object_mut() {
            obj.insert(
                "rb_local_usn".to_string(),
                serde_json::Value::Number(usn.into()),
            );
        }
    }

    pub(crate) fn save(&self, conn: &Connection) -> Result<()> {
        conn.execute(
            "UPDATE agentRegistry SET int_1 = ? WHERE registry_id = 'localUpdateCount'",
            params![self.current],
        )?;
        Ok(())
    }
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
