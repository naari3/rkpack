use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::core;

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

pub fn run_cli() -> Result<()> {
    let cli = Cli::parse();

    let db_path = match cli.db_path {
        Some(p) => PathBuf::from(p),
        None => core::default_db_path()?,
    };
    let key = cli.key.as_deref().unwrap_or(core::DEFAULT_KEY);

    tracing::info!("DB: {}", db_path.display());

    let read_only = matches!(
        cli.command,
        Command::ListTables | Command::ListPlaylists | Command::Pack { .. }
    );
    let conn = core::open_rekordbox_db(&db_path, key, read_only)?;

    match cli.command {
        Command::Export { output } => {
            core::export_decrypted(&conn, &output)?;
        }
        Command::ListTables => {
            core::list_tables(&conn)?;
        }
        Command::ListPlaylists => {
            core::list_playlists(&conn)?;
        }
        Command::Pack {
            output,
            playlist,
            keep_structure,
        } => {
            core::pack_playlist(&conn, &output, &playlist, keep_structure, &|msg| tracing::info!("{}", msg))?;
        }
        Command::Unpack {
            pack_path,
            dest_dir,
        } => {
            let confirm = |info: &core::DuplicateInfo| -> bool {
                eprintln!("重複トラックが見つかりました:");
                eprintln!(
                    "  元のトラック名: {} メモリーキュー: {}個 ホットキュー: {}個",
                    info.existing_title, info.existing_memory_cue_count, info.existing_hot_cue_count
                );
                eprintln!(
                    "  新しいトラック名: {} メモリーキュー: {}個 ホットキュー: {}個",
                    info.new_title, info.new_memory_cue_count, info.new_hot_cue_count
                );
                eprint!("更新しますか？ [y/N] ");
                let _ = std::io::Write::flush(&mut std::io::stderr());
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap_or(0);
                input.trim().eq_ignore_ascii_case("y")
            };
            core::unpack_playlist(&conn, &pack_path, &dest_dir, &|msg| tracing::info!("{}", msg), &confirm)?;
        }
    }

    Ok(())
}
