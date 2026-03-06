use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc;

use eframe::egui;

use crate::core::{self, PlaylistInfo, TrackInfo};

enum BgResult {
    Progress(String),
    PackDone(Result<String, String>),
    UnpackDone(Result<String, String>),
    PreviewLoaded(Result<core::UnpackPreviewData, String>),
}

#[derive(PartialEq)]
enum AppScreen {
    Main,
    UnpackPreview,
}

fn setup_japanese_fonts(ctx: &egui::Context) {
    let font_paths: &[&str] = if cfg!(target_os = "windows") {
        &[
            "C:\\Windows\\Fonts\\YuGothR.ttc",
            "C:\\Windows\\Fonts\\yugothic.ttf",
            "C:\\Windows\\Fonts\\meiryo.ttc",
            "C:\\Windows\\Fonts\\msgothic.ttc",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
            "/System/Library/Fonts/HiraginoSans-W3.otf",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/Library/Fonts/Arial Unicode.ttf",
        ]
    } else {
        &[
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
            "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
        ]
    };

    let mut font_data = None;
    for path in font_paths {
        if let Ok(data) = std::fs::read(path) {
            font_data = Some(data);
            break;
        }
    }

    let Some(data) = font_data else {
        return;
    };

    let mut fonts = egui::FontDefinitions::default();
    let mut font_data = egui::FontData::from_owned(data.clone());

    if let Ok(face) = ttf_parser::Face::parse(&data, 0) {
        let ascender = face.ascender() as f32;
        let descender = face.descender() as f32;
        let units_per_em = face.units_per_em() as f32;
        font_data.tweak.y_offset_factor = (ascender + descender) / (2.0 * units_per_em);
    }

    fonts.font_data.insert(
        "system_jp".to_owned(),
        font_data.into(),
    );
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "system_jp".to_owned());
    fonts
        .families
        .get_mut(&egui::FontFamily::Monospace)
        .unwrap()
        .insert(0, "system_jp".to_owned());
    ctx.set_fonts(fonts);
}

pub fn run_gui() -> anyhow::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "rkpack",
        options,
        Box::new(|cc| {
            setup_japanese_fonts(&cc.egui_ctx);
            Ok(Box::new(RkpackApp::new()))
        }),
    )
    .map_err(|e| anyhow::anyhow!("GUI エラー: {}", e))
}

struct RkpackApp {
    db_path: Option<PathBuf>,
    db_error: Option<String>,
    playlists: Vec<PlaylistInfo>,
    selected_playlist_idx: Option<usize>,
    tracks: Vec<TrackInfo>,
    keep_structure: bool,
    status: String,
    bg_rx: Option<mpsc::Receiver<BgResult>>,
    busy: bool,
    screen: AppScreen,
    preview_data: Option<core::UnpackPreviewData>,
    preview_detail_idx: Option<usize>,
    /// Previous content_id_input values to detect changes for duplicate check
    prev_content_id_inputs: Vec<String>,
}

impl RkpackApp {
    fn new() -> Self {
        let mut app = Self {
            db_path: None,
            db_error: None,
            playlists: Vec::new(),
            selected_playlist_idx: None,
            tracks: Vec::new(),
            keep_structure: false,
            status: "起動中...".to_string(),
            bg_rx: None,
            busy: false,
            screen: AppScreen::Main,
            preview_data: None,
            preview_detail_idx: None,
            prev_content_id_inputs: Vec::new(),
        };
        app.try_auto_connect();
        app
    }

    fn try_auto_connect(&mut self) {
        match core::default_db_path() {
            Ok(path) => {
                self.load_db(path);
            }
            Err(_) => {
                self.status = "master.db が見つかりません。[Select DB...] で選択してください。".to_string();
            }
        }
    }

    fn load_db(&mut self, path: PathBuf) {
        self.playlists.clear();
        self.tracks.clear();
        self.selected_playlist_idx = None;
        self.db_error = None;

        match core::open_rekordbox_db(&path, core::DEFAULT_KEY, true) {
            Ok(conn) => {
                match core::get_playlists(&conn) {
                    Ok(pl) => {
                        self.status = format!("DB: {} ({} プレイリスト)", path.display(), pl.len());
                        self.playlists = pl;
                    }
                    Err(e) => {
                        self.db_error = Some(format!("プレイリスト読み込みエラー: {}", e));
                        self.status = "エラー".to_string();
                    }
                }
                self.db_path = Some(path);
            }
            Err(e) => {
                self.db_error = Some(format!("DB接続エラー: {}", e));
                self.status = "エラー".to_string();
            }
        }
    }

    fn load_tracks(&mut self, playlist_id: &str) {
        let Some(ref db_path) = self.db_path else {
            return;
        };
        match core::open_rekordbox_db(db_path, core::DEFAULT_KEY, true) {
            Ok(conn) => match core::get_playlist_tracks(&conn, playlist_id) {
                Ok(tracks) => {
                    self.tracks = tracks;
                }
                Err(e) => {
                    self.db_error = Some(format!("トラック読み込みエラー: {}", e));
                }
            },
            Err(e) => {
                self.db_error = Some(format!("DB接続エラー: {}", e));
            }
        }
    }

    fn start_pack(&mut self, ctx: &egui::Context) {
        let Some(idx) = self.selected_playlist_idx else {
            self.status = "プレイリストを選択してください".to_string();
            return;
        };
        let playlist_name = self.playlists[idx].name.clone();
        let playlist_id = self.playlists[idx].id.clone();
        let Some(ref db_path) = self.db_path else {
            return;
        };

        let save_path = rfd::FileDialog::new()
            .set_dialog_id("rkpack-pack-save")
            .set_title("Pack 保存先")
            .set_file_name(format!("{}.rkp", playlist_name))
            .add_filter("rkp", &["rkp"])
            .save_file();

        let Some(save_path) = save_path else {
            return;
        };

        let (tx, rx) = mpsc::channel();
        self.bg_rx = Some(rx);
        self.busy = true;
        self.status = format!("パック中: {}...", playlist_name);

        let db_path = db_path.clone();
        let keep_structure = self.keep_structure;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let tx_progress = tx.clone();
            let ctx_progress = ctx.clone();
            let progress = move |msg: &str| {
                tracing::info!("{}", msg);
                let _ = tx_progress.send(BgResult::Progress(msg.to_string()));
                ctx_progress.request_repaint();
            };
            let result = (|| -> anyhow::Result<String> {
                let conn = core::open_rekordbox_db(&db_path, core::DEFAULT_KEY, true)?;
                let output = save_path.to_string_lossy().to_string();
                core::pack_playlist_by_id(&conn, &output, &playlist_id, keep_structure, &progress)?;
                Ok(output)
            })();
            let _ = tx.send(BgResult::PackDone(
                result.map_err(|e| e.to_string()),
            ));
            ctx.request_repaint();
        });
    }

    fn start_unpack_preview(&mut self, ctx: &egui::Context) {
        let Some(ref db_path) = self.db_path else {
            self.status = "DBが接続されていません".to_string();
            return;
        };

        let rkp_path = rfd::FileDialog::new()
            .set_dialog_id("rkpack-unpack-rkp")
            .set_title("Unpack する .rkp ファイル")
            .add_filter("rkp", &["rkp"])
            .pick_file();

        let Some(rkp_path) = rkp_path else {
            return;
        };

        let (tx, rx) = mpsc::channel();
        self.bg_rx = Some(rx);
        self.busy = true;
        self.status = format!("プレビュー読み込み中: {}...", rkp_path.display());

        let db_path = db_path.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<core::UnpackPreviewData> {
                let conn = core::open_rekordbox_db(&db_path, core::DEFAULT_KEY, true)?;
                let pack_str = rkp_path.to_string_lossy().to_string();
                core::load_unpack_preview(&conn, &pack_str)
            })();
            let _ = tx.send(BgResult::PreviewLoaded(
                result.map_err(|e| e.to_string()),
            ));
            ctx.request_repaint();
        });
    }

    fn start_unpack_execute(&mut self, ctx: &egui::Context) {
        let Some(ref db_path) = self.db_path else {
            return;
        };
        let Some(ref preview) = self.preview_data else {
            return;
        };

        // プレイリスト名の重複チェック
        if let Ok(conn) = core::open_rekordbox_db(db_path, core::DEFAULT_KEY, true) {
            let existing: Option<String> = conn
                .query_row(
                    "SELECT ID FROM djmdPlaylist WHERE Name = ? AND rb_local_deleted = 0 LIMIT 1",
                    rusqlite::params![&preview.playlist_name],
                    |row| row.get(0),
                )
                .ok();
            if existing.is_some() {
                self.status = format!(
                    "エラー: プレイリスト名 '{}' は既に存在します。名前を変更してください。",
                    preview.playlist_name
                );
                return;
            }
        }

        let dest_dir = rfd::FileDialog::new()
            .set_dialog_id("rkpack-unpack-dest")
            .set_title("音声ファイルの配置先")
            .pick_folder();

        let Some(dest_dir) = dest_dir else {
            return;
        };

        let mut skipped_content_ids = HashSet::new();
        let mut update_content_ids = HashSet::new();
        let mut existing_content_map = HashMap::new();

        for track in &preview.tracks {
            match track.decision {
                core::DuplicateDecision::Skip => {
                    skipped_content_ids.insert(track.pack_content_id.clone());
                    if let Some(ref dup) = track.duplicate {
                        existing_content_map
                            .insert(track.pack_content_id.clone(), dup.existing_content_id.clone());
                    }
                }
                core::DuplicateDecision::Update => {
                    update_content_ids.insert(track.pack_content_id.clone());
                    if let Some(ref dup) = track.duplicate {
                        existing_content_map
                            .insert(track.pack_content_id.clone(), dup.existing_content_id.clone());
                    }
                }
                core::DuplicateDecision::New => {}
            }
        }

        let decisions = core::UnpackDecisions {
            skipped_content_ids,
            update_content_ids,
            existing_content_map,
        };

        let pack_path = preview.rkp_path.clone();
        let playlist_name = preview.playlist_name.clone();
        let dest_dir = dest_dir.to_string_lossy().to_string();

        let (tx, rx) = mpsc::channel();
        self.bg_rx = Some(rx);
        self.busy = true;
        self.status = "アンパック実行中...".to_string();

        let db_path = db_path.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let tx_progress = tx.clone();
            let ctx_progress = ctx.clone();
            let progress = move |msg: &str| {
                tracing::info!("{}", msg);
                let _ = tx_progress.send(BgResult::Progress(msg.to_string()));
                ctx_progress.request_repaint();
            };
            let result = (|| -> anyhow::Result<String> {
                let conn = core::open_rekordbox_db(&db_path, core::DEFAULT_KEY, false)?;
                core::unpack_playlist_with_decisions(
                    &conn,
                    &pack_path,
                    &dest_dir,
                    &decisions,
                    Some(&playlist_name),
                    &progress,
                )?;
                Ok(pack_path)
            })();
            let _ = tx.send(BgResult::UnpackDone(
                result.map_err(|e| e.to_string()),
            ));
            ctx.request_repaint();
        });
    }

    fn poll_bg(&mut self) {
        let Some(ref rx) = self.bg_rx else {
            return;
        };
        while let Ok(result) = rx.try_recv() {
            match result {
                BgResult::Progress(msg) => {
                    self.status = msg;
                }
                BgResult::PackDone(Ok(path)) => {
                    self.busy = false;
                    self.bg_rx = None;
                    self.status = format!("パック完了: {}", path);
                    return;
                }
                BgResult::PackDone(Err(e)) => {
                    self.busy = false;
                    self.bg_rx = None;
                    self.status = format!("パックエラー: {}", e);
                    return;
                }
                BgResult::UnpackDone(Ok(path)) => {
                    self.busy = false;
                    self.bg_rx = None;
                    self.screen = AppScreen::Main;
                    self.preview_data = None;
                    self.preview_detail_idx = None;
                    self.status = format!("アンパック完了: {}", path);
                    if let Some(ref db_path) = self.db_path {
                        let p = db_path.clone();
                        self.load_db(p);
                    }
                    return;
                }
                BgResult::UnpackDone(Err(e)) => {
                    self.busy = false;
                    self.bg_rx = None;
                    self.status = format!("アンパックエラー: {}", e);
                    return;
                }
                BgResult::PreviewLoaded(Ok(data)) => {
                    self.busy = false;
                    self.bg_rx = None;
                    self.prev_content_id_inputs = data.tracks.iter().map(|t| t.content_id_input.clone()).collect();
                    self.status = format!(
                        "プレビュー: {} ({} トラック)",
                        data.playlist_name,
                        data.tracks.len()
                    );
                    self.preview_data = Some(data);
                    self.screen = AppScreen::UnpackPreview;
                    return;
                }
                BgResult::PreviewLoaded(Err(e)) => {
                    self.busy = false;
                    self.bg_rx = None;
                    self.status = format!("プレビュー読み込みエラー: {}", e);
                    return;
                }
            }
        }
    }

    fn draw_main(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("DB:");
                if let Some(ref path) = self.db_path {
                    ui.monospace(path.display().to_string());
                } else {
                    ui.label("(未接続)");
                }
                if ui.button("Select DB...").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .set_dialog_id("rkpack-select-db")
                        .set_title("master.db を選択")
                        .add_filter("SQLite DB", &["db"])
                        .pick_file()
                    {
                        self.load_db(path);
                    }
            });
            if let Some(ref err) = self.db_error {
                ui.colored_label(egui::Color32::RED, err);
            }
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let pack_enabled = !self.busy && self.selected_playlist_idx.is_some();
                if ui.add_enabled(pack_enabled, egui::Button::new("Pack Selected Playlist")).clicked() {
                    self.start_pack(ctx);
                }
                let unpack_enabled = !self.busy && self.db_path.is_some();
                if ui.add_enabled(unpack_enabled, egui::Button::new("Unpack .rkp File")).clicked() {
                    self.start_unpack_preview(ctx);
                }
                ui.checkbox(&mut self.keep_structure, "Keep structure");
            });
            ui.label(&self.status);
        });

        let mut newly_selected_playlist_id: Option<String> = None;

        egui::SidePanel::left("playlist_panel")
            .default_width(200.0)
            .frame(egui::Frame::central_panel(ctx.style().as_ref()))
            .show(ctx, |ui| {
                ui.heading("Playlists");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (i, pl) in self.playlists.iter().enumerate() {
                        if pl.attribute == -128 {
                            continue;
                        }
                        let label = format!("{} ({})", pl.name, pl.track_count);
                        let selected = self.selected_playlist_idx == Some(i);
                        if ui.selectable_label(selected, &label).clicked() && !selected {
                            self.selected_playlist_idx = Some(i);
                            newly_selected_playlist_id = Some(pl.id.clone());
                        }
                    }
                });
            });

        if let Some(playlist_id) = newly_selected_playlist_id {
            self.load_tracks(&playlist_id);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Tracks");
            ui.separator();

            if self.tracks.is_empty() {
                if self.selected_playlist_idx.is_some() {
                    ui.label("トラックがありません");
                } else {
                    ui.label("プレイリストを選択してください");
                }
            } else {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("tracks_grid")
                        .num_columns(7)
                        .striped(true)
                        .min_col_width(40.0)
                        .show(ui, |ui| {
                            ui.strong("#");
                            ui.strong("ContentID");
                            ui.strong("Title");
                            ui.strong("Artist");
                            ui.strong("Album");
                            ui.strong("Mem Cue");
                            ui.strong("Hot Cue");
                            ui.end_row();

                            for (i, track) in self.tracks.iter().enumerate() {
                                ui.label((i + 1).to_string());
                                ui.label(&track.id);
                                ui.label(&track.title);
                                ui.label(&track.artist);
                                ui.label(&track.album);
                                ui.label(track.memory_cue_count.to_string());
                                ui.label(track.hot_cue_count.to_string());
                                ui.end_row();
                            }
                        });
                });
            }
        });
    }

    fn draw_unpack_preview(&mut self, ctx: &egui::Context) {
        // Collect content_id changes to trigger duplicate checks after mutable borrow ends
        let mut content_id_checks: Vec<(usize, String)> = Vec::new();

        let preview_name = self
            .preview_data
            .as_ref()
            .map(|d| d.playlist_name.clone())
            .unwrap_or_default();

        egui::TopBottomPanel::top("preview_top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("アンパックプレビュー:");
                if let Some(ref mut preview) = self.preview_data {
                    ui.add(
                        egui::TextEdit::singleline(&mut preview.playlist_name)
                            .desired_width(200.0),
                    );
                } else {
                    ui.label(&preview_name);
                }
            });
        });

        let mut do_back = false;
        let mut do_execute = false;

        egui::TopBottomPanel::bottom("preview_bottom").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("← 戻る").clicked() {
                    do_back = true;
                }
                let can_execute = !self.busy && self.preview_data.is_some();
                if ui
                    .add_enabled(can_execute, egui::Button::new("実行"))
                    .clicked()
                {
                    do_execute = true;
                }
            });
            ui.label(&self.status);
        });

        // Detail modal
        let mut close_detail = false;
        let mut detail_decision: Option<(usize, core::DuplicateDecision)> = None;
        if let Some(detail_idx) = self.preview_detail_idx {
            if let Some(ref preview) = self.preview_data {
                if let Some(track) = preview.tracks.get(detail_idx) {
                    let mut open = true;
                    egui::Window::new("重複トラック詳細")
                        .collapsible(false)
                        .resizable(false)
                        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                        .open(&mut open)
                        .show(ctx, |ui| {
                            if let Some(ref dup) = track.duplicate {
                                ui.group(|ui| {
                                    ui.label("既存データ:");
                                    ui.label(format!(
                                        "  トラック名: {}",
                                        dup.info.existing_title
                                    ));
                                    ui.label(format!(
                                        "  メモリーキュー: {}個  ホットキュー: {}個",
                                        dup.info.existing_memory_cue_count,
                                        dup.info.existing_hot_cue_count
                                    ));
                                });
                                ui.add_space(4.0);
                                ui.group(|ui| {
                                    ui.label("新しいデータ:");
                                    ui.label(format!("  トラック名: {}", track.title));
                                    ui.label(format!(
                                        "  メモリーキュー: {}個  ホットキュー: {}個",
                                        track.memory_cue_count, track.hot_cue_count
                                    ));
                                });
                                ui.add_space(8.0);
                                ui.horizontal(|ui| {
                                    if ui.button("更新する").clicked() {
                                        detail_decision = Some((
                                            detail_idx,
                                            core::DuplicateDecision::Update,
                                        ));
                                        close_detail = true;
                                    }
                                    if ui.button("新規として追加").clicked() {
                                        detail_decision =
                                            Some((detail_idx, core::DuplicateDecision::New));
                                        close_detail = true;
                                    }
                                    if ui.button("スキップ").clicked() {
                                        detail_decision =
                                            Some((detail_idx, core::DuplicateDecision::Skip));
                                        close_detail = true;
                                    }
                                });
                            } else {
                                ui.label("重複情報がありません");
                            }
                        });
                    if !open {
                        close_detail = true;
                    }
                }
            }
        }

        if close_detail {
            self.preview_detail_idx = None;
        }

        if let Some((idx, decision)) = detail_decision {
            if let Some(ref mut preview) = self.preview_data {
                if let Some(track) = preview.tracks.get_mut(idx) {
                    if decision == core::DuplicateDecision::New {
                        // Auto-assign new ContentID
                        if let Some(ref db_path) = self.db_path {
                            if let Ok(conn) =
                                core::open_rekordbox_db(db_path, core::DEFAULT_KEY, true)
                            {
                                if let Ok(max_id) = conn.query_row(
                                    "SELECT MAX(CAST(ID AS INTEGER)) FROM djmdContent",
                                    [],
                                    |row| row.get::<_, Option<i64>>(0),
                                ) {
                                    let new_id = max_id.unwrap_or(0) + 1;
                                    track.content_id_input = new_id.to_string();
                                }
                            }
                        }
                        track.duplicate = None;
                    }
                    track.decision = decision;
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(ref mut preview) = self.preview_data {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    egui::Grid::new("preview_grid")
                        .num_columns(8)
                        .striped(true)
                        .min_col_width(40.0)
                        .show(ui, |ui| {
                            ui.strong("#");
                            ui.strong("状態");
                            ui.strong("ContentID");
                            ui.strong("Title");
                            ui.strong("Artist");
                            ui.strong("Album");
                            ui.strong("Mem Cue");
                            ui.strong("Hot Cue");
                            ui.end_row();

                            for (i, track) in preview.tracks.iter_mut().enumerate() {
                                ui.label((i + 1).to_string());

                                // Status column
                                if track.duplicate.is_some() {
                                    let status_label = match track.decision {
                                        core::DuplicateDecision::Skip => "⏭ Skip",
                                        core::DuplicateDecision::Update => "🔄 Update",
                                        core::DuplicateDecision::New => "➕ New",
                                    };
                                    if ui.button(format!("! {}", status_label)).clicked() {
                                        // Store idx for detail modal outside this borrow
                                        content_id_checks.push((usize::MAX, i.to_string()));
                                    }
                                } else {
                                    match track.decision {
                                        core::DuplicateDecision::New => {
                                            ui.label("✓ New");
                                        }
                                        _ => {
                                            ui.label("-");
                                        }
                                    }
                                }

                                // ContentID - editable
                                let response = ui.add(
                                    egui::TextEdit::singleline(&mut track.content_id_input)
                                        .desired_width(80.0),
                                );
                                if response.lost_focus() || response.changed() {
                                    // Will check for changes after the loop
                                    content_id_checks
                                        .push((i, track.content_id_input.clone()));
                                }

                                ui.label(&track.title);
                                ui.label(&track.artist);
                                ui.label(&track.album);
                                ui.label(track.memory_cue_count.to_string());
                                ui.label(track.hot_cue_count.to_string());
                                ui.end_row();
                            }
                        });
                });
            } else {
                ui.label("プレビューデータがありません");
            }
        });

        // Handle content_id checks and detail modal triggers
        for (idx, value) in content_id_checks {
            if idx == usize::MAX {
                // This is a detail modal trigger - value is the track index
                if let Ok(track_idx) = value.parse::<usize>() {
                    self.preview_detail_idx = Some(track_idx);
                }
            } else {
                // Check if content_id actually changed
                let prev = self.prev_content_id_inputs.get(idx).cloned().unwrap_or_default();
                if value != prev {
                    // Update prev
                    if idx < self.prev_content_id_inputs.len() {
                        self.prev_content_id_inputs[idx] = value.clone();
                    }
                    // Do synchronous duplicate check (fast DB query)
                    if let Some(ref db_path) = self.db_path {
                        if let Ok(conn) =
                            core::open_rekordbox_db(db_path, core::DEFAULT_KEY, true)
                        {
                            if let Ok(result) =
                                core::check_content_id_duplicate(&conn, &value)
                            {
                                if let Some(ref mut preview) = self.preview_data {
                                    if let Some(track) = preview.tracks.get_mut(idx) {
                                        if let Some(info) = result {
                                            track.duplicate = Some(core::DuplicateMatch {
                                                existing_content_id: value,
                                                info,
                                            });
                                            if track.decision == core::DuplicateDecision::New {
                                                track.decision = core::DuplicateDecision::Skip;
                                            }
                                        } else {
                                            track.duplicate = None;
                                            track.decision = core::DuplicateDecision::New;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if do_back {
            self.screen = AppScreen::Main;
            self.preview_data = None;
            self.preview_detail_idx = None;
        }

        if do_execute {
            self.start_unpack_execute(ctx);
        }
    }
}

impl eframe::App for RkpackApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_bg();

        match self.screen {
            AppScreen::Main => self.draw_main(ctx),
            AppScreen::UnpackPreview => self.draw_unpack_preview(ctx),
        }
    }
}
