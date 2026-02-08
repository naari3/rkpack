use std::path::PathBuf;
use std::sync::mpsc;

use eframe::egui;

use crate::core::{self, PlaylistInfo, TrackInfo};

enum BgResult {
    Progress(String),
    PackDone(Result<String, String>),
    UnpackDone(Result<String, String>),
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

    // フォントメトリクスから y_offset_factor を算出
    // ascender + descender が 0 でないと上下にずれるので、その分を補正
    if let Ok(face) = ttf_parser::Face::parse(&data, 0) {
        let ascender = face.ascender() as f32;
        let descender = face.descender() as f32; // 負の値
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
        let Some(ref db_path) = self.db_path else {
            return;
        };

        let save_path = rfd::FileDialog::new()
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
                let _ = tx_progress.send(BgResult::Progress(msg.to_string()));
                ctx_progress.request_repaint();
            };
            let result = (|| -> anyhow::Result<String> {
                let conn = core::open_rekordbox_db(&db_path, core::DEFAULT_KEY, true)?;
                let output = save_path.to_string_lossy().to_string();
                core::pack_playlist(&conn, &output, &playlist_name, keep_structure, &progress)?;
                Ok(output)
            })();
            let _ = tx.send(BgResult::PackDone(
                result.map_err(|e| e.to_string()),
            ));
            ctx.request_repaint();
        });
    }

    fn start_unpack(&mut self, ctx: &egui::Context) {
        let Some(ref db_path) = self.db_path else {
            self.status = "DBが接続されていません".to_string();
            return;
        };

        let rkp_path = rfd::FileDialog::new()
            .set_title("Unpack する .rkp ファイル")
            .add_filter("rkp", &["rkp"])
            .pick_file();

        let Some(rkp_path) = rkp_path else {
            return;
        };

        let dest_dir = rfd::FileDialog::new()
            .set_title("音声ファイルの配置先")
            .pick_folder();

        let Some(dest_dir) = dest_dir else {
            return;
        };

        let (tx, rx) = mpsc::channel();
        self.bg_rx = Some(rx);
        self.busy = true;
        self.status = format!("アンパック中: {}...", rkp_path.display());

        let db_path = db_path.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let tx_progress = tx.clone();
            let ctx_progress = ctx.clone();
            let progress = move |msg: &str| {
                let _ = tx_progress.send(BgResult::Progress(msg.to_string()));
                ctx_progress.request_repaint();
            };
            let result = (|| -> anyhow::Result<String> {
                let conn = core::open_rekordbox_db(&db_path, core::DEFAULT_KEY, false)?;
                let pack_str = rkp_path.to_string_lossy().to_string();
                let dest_str = dest_dir.to_string_lossy().to_string();
                core::unpack_playlist(&conn, &pack_str, &dest_str, &progress)?;
                Ok(pack_str)
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
            }
        }
    }
}

impl eframe::App for RkpackApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_bg();

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
                    self.start_unpack(ctx);
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
                        .num_columns(4)
                        .striped(true)
                        .min_col_width(40.0)
                        .show(ui, |ui| {
                            ui.strong("#");
                            ui.strong("Title");
                            ui.strong("Artist");
                            ui.strong("Album");
                            ui.end_row();

                            for (i, track) in self.tracks.iter().enumerate() {
                                ui.label((i + 1).to_string());
                                ui.label(&track.title);
                                ui.label(&track.artist);
                                ui.label(&track.album);
                                ui.end_row();
                            }
                        });
                });
            }
        });
    }
}
