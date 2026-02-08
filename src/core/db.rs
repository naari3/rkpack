use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use unicode_normalization::UnicodeNormalization;

pub const DEFAULT_KEY: &str =
    "402fd482c38817c35ffa8ffb8c7d93143b749e7d315df7a81732a1ff43608497";

pub fn default_db_path() -> Result<PathBuf> {
    let candidates = if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").context("$HOME が設定されていません")?;
        vec![
            PathBuf::from(&home).join("Library/Application Support/Pioneer/rekordbox/master.db"),
            PathBuf::from(&home).join("Library/Pioneer/rekordbox/master.db"),
        ]
    } else if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").context("%APPDATA% が設定されていません")?;
        vec![PathBuf::from(&appdata)
            .join("Pioneer")
            .join("rekordbox")
            .join("master.db")]
    } else {
        let home = std::env::var("HOME").context("$HOME が設定されていません")?;
        vec![PathBuf::from(&home).join(".Pioneer/rekordbox/master.db")]
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

pub fn is_plain_sqlite(path: &PathBuf) -> bool {
    fs::read(path)
        .map(|buf| buf.len() >= 16 && &buf[..16] == b"SQLite format 3\0")
        .unwrap_or(false)
}

pub fn open_rekordbox_db(db_path: &PathBuf, key: &str, read_only: bool) -> Result<Connection> {
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

pub fn export_decrypted(conn: &Connection, export_path: &str) -> Result<()> {
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

    tracing::info!("エクスポート完了: {}", export_path.display());
    Ok(())
}

pub(crate) fn to_nfc(s: &str) -> String {
    s.nfc().collect()
}

pub(crate) fn get_actual_path_on_disk(expected: &std::path::Path) -> PathBuf {
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
