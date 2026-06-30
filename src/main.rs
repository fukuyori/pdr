// Portable Document Reader (PDR)
// 第一段階: PDF の見開き表示
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::mpsc::{Receiver, Sender};

use eframe::egui;
use pdfium_render::prelude::*;

use pdr::enhance::{Enhance, apply_enhance};

/// テクスチャキャッシュの保持枚数（(ページ,解像度,補正)単位）。
const CACHE_CAP: usize = 24;

/// マウスホイール 1 目盛り相当の拡大係数の指数係数。
const ZOOM_WHEEL_K: f32 = 0.0015;
/// フィット倍率にスナップ(吸着)する相対しきい値（±2%）。
const FIT_SNAP: f32 = 0.02;

/// 描画解像度(px幅)の刻み・下限・上限。表示サイズに合わせ、この刻みに丸めて描画。
const BUCKET_STEP: i32 = 256;
const BUCKET_MIN: i32 = 512;
const BUCKET_MAX: i32 = 3000;

/// 目次パネルの標準幅・可変範囲・変更ステップ(px)。
const TOC_WIDTH_DEFAULT: f32 = 280.0;
const TOC_WIDTH_MIN: f32 = 160.0;
const TOC_WIDTH_MAX: f32 = 700.0;
const TOC_WIDTH_STEP: f32 = 40.0;

/// pdfium バインディングを生成する（所有権付き）。描画スレッドで 1 度だけ呼ぶ。
fn make_pdfium() -> Result<Pdfium, PdfiumError> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.to_path_buf());
        }
    }
    dirs.push(PathBuf::from("."));
    dirs.push(PathBuf::from("./third_party/pdfium"));
    dirs.push(PathBuf::from("./lib/bin"));

    for dir in &dirs {
        let path = Pdfium::pdfium_platform_library_name_at_path(dir);
        if let Ok(b) = Pdfium::bind_to_library(&path) {
            return Ok(Pdfium::new(b));
        }
    }
    Pdfium::bind_to_system_library().map(Pdfium::new)
}

/// 描画キャッシュのキー: (ページ, 描画幅px, 補正)
type RenderKey = (usize, i32, Enhance);

/// UI→描画スレッドへの指示
enum RenderCmd {
    Open { path: PathBuf, doc_gen: u64 },
    Render {
        page: usize,
        width: i32,
        enhance: Enhance,
        doc_gen: u64,
    },
}

/// 描画スレッド→UIへの結果
enum RenderEvt {
    Opened {
        doc_gen: u64,
        page_count: usize,
        toc: Vec<TocEntry>,
        page_w: f32,
        page_h: f32,
    },
    OpenFailed {
        doc_gen: u64,
        msg: String,
    },
    Rendered {
        doc_gen: u64,
        key: RenderKey,
        w: usize,
        h: usize,
        pixels: Vec<u8>,
    },
    /// 致命的エラー（pdfium 初期化失敗など）。世代に関係なく表示する。
    Fatal(String),
}

/// 描画スレッド本体。pdfium はここだけが触る。UI スレッドは一切ブロックしない。
fn render_worker(rx: Receiver<RenderCmd>, tx: Sender<RenderEvt>, ctx: egui::Context) {
    let pdfium = match make_pdfium() {
        Ok(p) => p,
        Err(e) => {
            log_line(&format!("描画スレッド: pdfium 初期化失敗: {e}"));
            let _ = tx.send(RenderEvt::Fatal(
                "pdfium.dll を読み込めませんでした。実行ファイルと同じフォルダに pdfium.dll を置いてください。".to_owned(),
            ));
            ctx.request_repaint();
            return;
        }
    };
    let mut doc: Option<PdfDocument> = None;
    let mut cur_doc_gen: u64 = 0;

    while let Ok(cmd) = rx.recv() {
        match cmd {
            RenderCmd::Open { path, doc_gen } => {
                cur_doc_gen = doc_gen;
                doc = None;
                match pdfium.load_pdf_from_file(&path, None) {
                    Ok(d) => {
                        let page_count = d.pages().len() as usize;
                        let toc = extract_toc(&d);
                        let (page_w, page_h) = d
                            .pages()
                            .get(0)
                            .map(|p| (p.width().value, p.height().value))
                            .unwrap_or((595.0, 842.0));
                        doc = Some(d);
                        let _ = tx.send(RenderEvt::Opened {
                            doc_gen,
                            page_count,
                            toc,
                            page_w,
                            page_h,
                        });
                    }
                    Err(e) => {
                        let _ = tx.send(RenderEvt::OpenFailed {
                            doc_gen,
                            msg: e.to_string(),
                        });
                    }
                }
                ctx.request_repaint();
            }
            RenderCmd::Render {
                page,
                width,
                enhance,
                doc_gen,
            } => {
                if doc_gen != cur_doc_gen {
                    continue; // 別ドキュメント宛ての古い要求は破棄
                }
                let Some(d) = doc.as_ref() else { continue };
                let Ok(pg) = d.pages().get(page as i32) else {
                    continue;
                };
                let cfg = PdfRenderConfig::new()
                    .set_target_width(width)
                    .set_maximum_height(width * 2);
                let Ok(bmp) = pg.render_with_config(&cfg) else {
                    continue;
                };
                let Ok(img) = bmp.as_image() else { continue };
                let rgba = apply_enhance(img, enhance).to_rgba8();
                let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                let _ = tx.send(RenderEvt::Rendered {
                    doc_gen,
                    key: (page, width, enhance),
                    w,
                    h,
                    pixels: rgba.into_raw(),
                });
                ctx.request_repaint();
            }
        }
    }
}

/// 表示幅(px)を刻みに丸めて描画解像度を決める。
fn bucketize(width_px: f32) -> i32 {
    let w = width_px.ceil().max(0.0) as i32;
    let rounded = ((w + BUCKET_STEP - 1) / BUCKET_STEP) * BUCKET_STEP;
    rounded.clamp(BUCKET_MIN, BUCKET_MAX)
}

/// 綴じ方向（見開き時の左右配置）
#[derive(Clone, Copy, PartialEq)]
enum Binding {
    /// 左綴じ（横書き・洋書）: 小さいページ番号が左
    LeftToRight,
    /// 右綴じ（縦書き・和書）: 小さいページ番号が右
    RightToLeft,
}

#[derive(Clone, Copy, PartialEq)]
enum ViewMode {
    Single,
    Spread,
}

/// ウィンドウへの合わせ方
#[derive(Clone, Copy, PartialEq)]
enum FitKind {
    /// ページ幅をウィンドウ幅に合わせる
    Width,
    /// ページ高さをウィンドウ高さに合わせる
    Height,
    /// ページ全体が収まるように合わせる（幅・高さの小さい方。初期表示用）
    Window,
}

/// 目次(しおり)の 1 項目。PdfBookmark の借用を持たず、所有データだけ保持する。
struct TocEntry {
    depth: usize,
    title: String,
    page: Option<usize>,
}

struct PdrApp {
    /// 読み込み済みなら総ページ数 > 0。
    page_count: usize,
    /// ページの寸法（ポイント、先頭ページ基準）。レイアウト・フィット計算に使う。
    page_size: (f32, f32),
    /// 現在の見開きの先頭(最小)ページインデックス。
    current: usize,
    view_mode: ViewMode,
    binding: Binding,
    /// 表紙(先頭1ページ)を単独表示するか
    cover_alone: bool,
    /// フィット基準（横/縦/全体）。表示倍率はこの基準に対する相対値で持つので、
    /// ウィンドウをリサイズすると毎フレーム再計算され、表示が追従する。
    fit_ref: FitKind,
    /// フィット基準に対する倍率（1.0 = ちょうどフィット）。マウスホイールで増減。
    zoom: f32,
    /// 画像補正モード
    enhance: Enhance,
    /// 目次（しおり）。空でなければ自動的に左パネルを表示する。
    toc: Vec<TocEntry>,
    /// 目次パネルの幅(px)。ドラッグ／ボタンで変更する。
    toc_width: f32,
    /// 最近開いたファイル（先頭が最新）
    recent: Vec<PathBuf>,
    /// 描画済みテクスチャのキャッシュ（描画スレッドの結果で埋まる）
    cache: HashMap<RenderKey, egui::TextureHandle>,
    /// 描画スレッドに依頼済みで未着のキー（重複依頼を防ぐ）
    requested: HashSet<RenderKey>,
    /// 直近フレームの描画解像度（先読み依頼で使う）
    cur_bucket: i32,
    /// ドキュメント世代。開くたびに +1 し、古い結果を捨てる。
    doc_gen: u64,
    /// 描画スレッドへの送信／受信
    to_worker: Sender<RenderCmd>,
    from_worker: Receiver<RenderEvt>,
    status: String,
}

impl PdrApp {
    fn new(to_worker: Sender<RenderCmd>, from_worker: Receiver<RenderEvt>) -> Self {
        Self {
            page_count: 0,
            page_size: (595.0, 842.0),
            current: 0,
            view_mode: ViewMode::Spread,
            binding: Binding::LeftToRight,
            cover_alone: true,
            fit_ref: FitKind::Window,
            zoom: 1.0,
            enhance: Enhance::None,
            toc: Vec::new(),
            toc_width: TOC_WIDTH_DEFAULT,
            recent: load_recent(),
            cache: HashMap::new(),
            requested: HashSet::new(),
            cur_bucket: BUCKET_MIN,
            doc_gen: 0,
            to_worker,
            from_worker,
            status: "ファイルを開いてください".to_owned(),
        }
    }

    fn open_path(&mut self, path: &Path) {
        self.doc_gen += 1;
        self.page_count = 0;
        self.current = 0;
        self.toc.clear();
        self.cache.clear();
        self.requested.clear();
        self.fit_ref = FitKind::Window; // 開いたら全体表示
        self.zoom = 1.0;
        self.status = format!(
            "{} を読み込み中…",
            path.file_name().and_then(|s| s.to_str()).unwrap_or("?")
        );
        let _ = self.to_worker.send(RenderCmd::Open {
            path: path.to_path_buf(),
            doc_gen: self.doc_gen,
        });
        self.push_recent(path);
    }

    /// 最近開いたファイル一覧の先頭に登録し、永続化する。
    fn push_recent(&mut self, path: &Path) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_path_buf());
        self.recent.truncate(12);
        save_recent(&self.recent);
    }

    /// 描画スレッドからの結果を取り込む（毎フレーム冒頭で呼ぶ）。
    fn drain_events(&mut self, ctx: &egui::Context) {
        while let Ok(evt) = self.from_worker.try_recv() {
            match evt {
                RenderEvt::Opened {
                    doc_gen,
                    page_count,
                    toc,
                    page_w,
                    page_h,
                } if doc_gen == self.doc_gen => {
                    self.page_count = page_count;
                    self.toc = toc;
                    self.page_size = (page_w.max(1.0), page_h.max(1.0));
                    self.status = format!("{page_count} ページ / 目次 {} 項目", self.toc.len());
                }
                RenderEvt::OpenFailed { doc_gen, msg } if doc_gen == self.doc_gen => {
                    self.status = format!("読み込み失敗: {msg}");
                }
                RenderEvt::Rendered {
                    doc_gen,
                    key,
                    w,
                    h,
                    pixels,
                } if doc_gen == self.doc_gen => {
                    self.requested.remove(&key);
                    let color = egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels);
                    let tex = ctx.load_texture(
                        format!("p{}_{}", key.0, key.1),
                        color,
                        egui::TextureOptions::LINEAR,
                    );
                    self.cache.insert(key, tex);
                }
                RenderEvt::Fatal(msg) => {
                    self.status = msg;
                }
                _ => {} // 古い世代の結果は無視
            }
        }
    }

    /// 指定キーがキャッシュにも依頼中にも無ければ、描画スレッドに依頼する。
    fn request_render(&mut self, key: RenderKey) {
        if self.cache.contains_key(&key) || self.requested.contains(&key) {
            return;
        }
        self.requested.insert(key);
        let _ = self.to_worker.send(RenderCmd::Render {
            page: key.0,
            width: key.1,
            enhance: key.2,
            doc_gen: self.doc_gen,
        });
    }

    /// 表示に使えるテクスチャを返す。完全一致が無ければ、同じページの別解像度を
    /// 暫定表示として返す（描き上がるまでのつなぎ。多少ぼやける）。
    fn display_texture(&self, page: usize) -> Option<egui::TextureHandle> {
        if let Some(t) = self.cache.get(&(page, self.cur_bucket, self.enhance)) {
            return Some(t.clone());
        }
        self.cache
            .iter()
            .filter(|((p, _, e), _)| *p == page && *e == self.enhance)
            .map(|(_, t)| t.clone())
            .next()
    }

    /// 指定ページを含む見開きの先頭(最小)ページ番号を返す。
    /// 表紙単独時は [0] [1,2] [3,4]… 、それ以外は [0,1] [2,3]… で組む。
    fn spread_start(&self, page: usize) -> usize {
        match self.view_mode {
            ViewMode::Single => page,
            ViewMode::Spread => {
                if self.cover_alone {
                    if page == 0 { 0 } else { ((page - 1) & !1) + 1 }
                } else {
                    page & !1
                }
            }
        }
    }

    /// 見開き先頭 `start` の見開きに含まれるページ番号を昇順で返す。
    fn pages_of_spread(&self, start: usize) -> Vec<usize> {
        if self.page_count == 0 {
            return vec![];
        }
        let start = start.min(self.page_count - 1);
        match self.view_mode {
            ViewMode::Single => vec![start],
            ViewMode::Spread => {
                if self.cover_alone && start == 0 {
                    return vec![0];
                }
                let mut pages = vec![start];
                if start + 1 < self.page_count {
                    pages.push(start + 1);
                }
                pages
            }
        }
    }

    /// 昇順のページ番号で現在の見開き内容を返す（表示順の反転前）。
    fn current_pages_sorted(&self) -> Vec<usize> {
        self.pages_of_spread(self.spread_start(self.current))
    }

    /// 先読み対象（前後の見開き）のページ番号を返す。
    fn prefetch_targets(&self) -> Vec<usize> {
        let cur = self.current_pages_sorted();
        let mut t = Vec::new();
        if let Some(&last) = cur.last() {
            if last + 1 < self.page_count {
                t.extend(self.pages_of_spread(self.spread_start(last + 1)));
            }
        }
        if let Some(&first) = cur.first() {
            if first > 0 {
                t.extend(self.pages_of_spread(self.spread_start(first - 1)));
            }
        }
        t
    }

    /// キャッシュが上限を超えたら、現在ページから遠いものから捨てる。
    fn evict_cache(&mut self) {
        if self.cache.len() <= CACHE_CAP {
            return;
        }
        let cur = self.current as isize;
        let mut keys: Vec<RenderKey> = self.cache.keys().copied().collect();
        keys.sort_by_key(|(p, _, _)| (*p as isize - cur).abs());
        for k in keys.into_iter().skip(CACHE_CAP) {
            self.cache.remove(&k);
        }
    }

    fn next(&mut self) {
        if let Some(&last) = self.current_pages_sorted().last() {
            let target = last + 1;
            if target < self.page_count {
                self.current = self.spread_start(target);
            }
        }
    }

    fn prev(&mut self) {
        let start = self.spread_start(self.current);
        if start > 0 {
            self.current = self.spread_start(start - 1);
        }
    }

    /// 目次などから任意ページへジャンプする。
    fn goto(&mut self, page: usize) {
        if self.page_count == 0 {
            return;
        }
        let p = page.min(self.page_count - 1);
        self.current = self.spread_start(p);
    }

    /// 現在表示すべきページ番号を左→右の表示順で返す。
    fn visible_pages(&self) -> Vec<usize> {
        let mut pages = self.current_pages_sorted();
        // 右綴じ: ページ順を反転して右に若いページを置く
        if self.binding == Binding::RightToLeft && pages.len() == 2 {
            pages.reverse();
        }
        pages
    }
}

impl eframe::App for PdrApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // 描画スレッドからの結果を取り込む
        self.drain_events(&ctx);

        let mut go_next = false;
        let mut go_prev = false;
        ctx.input(|i| {
            if i.key_pressed(egui::Key::PageDown)
                || i.key_pressed(egui::Key::ArrowDown)
                || i.key_pressed(egui::Key::Space)
            {
                go_next = true;
            }
            if i.key_pressed(egui::Key::PageUp) || i.key_pressed(egui::Key::ArrowUp) {
                go_prev = true;
            }

            // 左右矢印は綴じ方向に応じて「めくる向き」を割り当てる
            let left = i.key_pressed(egui::Key::ArrowLeft);
            let right = i.key_pressed(egui::Key::ArrowRight);
            match self.binding {
                // 左綴じ(洋): → が次、← が前
                Binding::LeftToRight => {
                    go_next |= right;
                    go_prev |= left;
                }
                // 右綴じ(和): ← が次、→ が前
                Binding::RightToLeft => {
                    go_next |= left;
                    go_prev |= right;
                }
            }
        });

        // クロージャ内では self を借用するため、操作は一旦ためてから後で適用する
        let mut open_path: Option<PathBuf> = None;
        let mut goto_page: Option<usize> = None;

        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui.button("📂 開く").clicked() {
                    if let Some(p) = rfd::FileDialog::new()
                        .add_filter("PDF", &["pdf"])
                        .pick_file()
                    {
                        open_path = Some(p);
                    }
                }

                // 履歴メニュー
                ui.add_enabled_ui(!self.recent.is_empty(), |ui| {
                    ui.menu_button("履歴 ▾", |ui| {
                        if self.recent.is_empty() {
                            ui.label("(履歴なし)");
                        }
                        for p in &self.recent {
                            let label = p
                                .file_name()
                                .and_then(|s| s.to_str())
                                .unwrap_or_else(|| p.to_str().unwrap_or("?"));
                            if ui
                                .button(label)
                                .on_hover_text(p.to_string_lossy())
                                .clicked()
                            {
                                open_path = Some(p.clone());
                                ui.close();
                            }
                        }
                    });
                });

                ui.separator();

                let has_doc = self.page_count > 0;
                ui.add_enabled_ui(has_doc, |ui| {
                    if ui.button("◀ 前").clicked() {
                        go_prev = true;
                    }
                    if ui.button("次 ▶").clicked() {
                        go_next = true;
                    }
                });

                ui.separator();
                egui::ComboBox::from_label("表示")
                    .selected_text(match self.view_mode {
                        ViewMode::Single => "単ページ",
                        ViewMode::Spread => "見開き",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.view_mode, ViewMode::Spread, "見開き");
                        ui.selectable_value(&mut self.view_mode, ViewMode::Single, "単ページ");
                    });

                ui.separator();
                egui::ComboBox::from_label("綴じ")
                    .selected_text(match self.binding {
                        Binding::RightToLeft => "右綴じ(和)",
                        Binding::LeftToRight => "左綴じ(洋)",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.binding, Binding::RightToLeft, "右綴じ(和)");
                        ui.selectable_value(&mut self.binding, Binding::LeftToRight, "左綴じ(洋)");
                    });

                ui.checkbox(&mut self.cover_alone, "表紙単独");

                ui.separator();
                ui.label("合わせる:");
                if ui
                    .selectable_label(self.fit_ref == FitKind::Width && self.zoom == 1.0, "⟷ 横")
                    .on_hover_text("ページ幅をウィンドウ幅に合わせる")
                    .clicked()
                {
                    self.fit_ref = FitKind::Width;
                    self.zoom = 1.0;
                }
                if ui
                    .selectable_label(self.fit_ref == FitKind::Height && self.zoom == 1.0, "↕ 縦")
                    .on_hover_text("ページの高さをウィンドウ高さに合わせる")
                    .clicked()
                {
                    self.fit_ref = FitKind::Height;
                    self.zoom = 1.0;
                }

                ui.separator();
                egui::ComboBox::from_label("補正")
                    .selected_text(match self.enhance {
                        Enhance::None => "なし",
                        Enhance::Contrast => "コントラスト",
                        Enhance::Binarize => "二値化",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.enhance, Enhance::None, "なし");
                        ui.selectable_value(&mut self.enhance, Enhance::Contrast, "コントラスト");
                        ui.selectable_value(&mut self.enhance, Enhance::Binarize, "二値化");
                    });

                ui.separator();
                if self.page_count > 0 {
                    ui.label(format!("{} / {}", self.current + 1, self.page_count));
                }
            });
        });

        egui::Panel::bottom("status").show(ui, |ui| {
            // ページ移動スライダー（複数ページのときだけ）
            if self.page_count > 1 {
                ui.horizontal(|ui| {
                    let total = self.page_count;
                    let mut sel = (self.current + 1).min(total);
                    let label = format!("{sel} / {total}");
                    // スライダーを残り幅いっぱいに広げ、右にページ番号を置く
                    let label_w = 90.0;
                    ui.spacing_mut().slider_width = (ui.available_width() - label_w).max(80.0);
                    let resp = ui.add(egui::Slider::new(&mut sel, 1..=total).show_value(false));
                    if resp.changed() {
                        goto_page = Some(sel - 1);
                    }
                    ui.label(label);
                });
            }
            ui.label(&self.status);
        });

        // 左の目次パネル（しおりのある PDF でのみ自動表示）。
        // 幅は固定 (exact_size)。変更はヘッダのボタン、または自前のドラッグハンドル
        // (この後 CentralPanel の上に重ねて描画) で行う。egui 組み込みのリサイズは
        // 本文スクロール領域との境界で安定して掴めなかったため使わない。
        let mut toc_edge: Option<egui::Rect> = None;
        if self.page_count > 0 && !self.toc.is_empty() {
            let inner = egui::Panel::left("toc")
                .resizable(false)
                .exact_size(self.toc_width)
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.strong("目次");
                        if ui.small_button("－").on_hover_text("幅を狭く").clicked() {
                            self.toc_width = (self.toc_width - TOC_WIDTH_STEP).max(TOC_WIDTH_MIN);
                        }
                        if ui.small_button("＋").on_hover_text("幅を広く").clicked() {
                            self.toc_width = (self.toc_width + TOC_WIDTH_STEP).min(TOC_WIDTH_MAX);
                        }
                        if ui.small_button("標準").on_hover_text("標準幅に戻す").clicked() {
                            self.toc_width = TOC_WIDTH_DEFAULT;
                        }
                    });
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for entry in &self.toc {
                                ui.horizontal(|ui| {
                                    ui.add_space(entry.depth as f32 * 16.0);
                                    let title = if entry.title.is_empty() {
                                        "(無題)"
                                    } else {
                                        entry.title.as_str()
                                    };
                                    let resp = match entry.page {
                                        Some(p) => ui
                                            .link(title)
                                            .on_hover_text(format!("{} ページ", p + 1)),
                                        None => ui.add(egui::Label::new(
                                            egui::RichText::new(title).weak(),
                                        )),
                                    };
                                    if resp.clicked() {
                                        if let Some(p) = entry.page {
                                            goto_page = Some(p);
                                        }
                                    }
                                });
                            }
                        });
                });
            toc_edge = Some(inner.response.rect);
        }

        // ナビゲーション適用（描画前に確定）
        if go_next {
            self.next();
        }
        if go_prev {
            self.prev();
        }
        if let Some(p) = goto_page {
            self.goto(p);
        }
        if let Some(p) = open_path {
            self.open_path(&p);
        }

        // 戻り値: 現在ページの (横フィット表示幅pt, 縦フィット表示幅pt)。ホイール処理で使う。
        let central = egui::CentralPanel::default_margins().show(ui, |ui| -> Option<(f32, f32)> {
            if self.page_count == 0 {
                ui.centered_and_justified(|ui| {
                    ui.label("PDF ファイルを開いてください（📂 開く）");
                });
                return None;
            }

            let pages = self.visible_pages();
            let avail = ui.available_size();
            let gap = 8.0;
            let per_w = if pages.len() == 2 {
                (avail.x - gap) / 2.0
            } else {
                avail.x
            };

            // ページのアスペクト比（高さ/幅）から、表示サイズ(pt)を直接求める。
            let aspect = (self.page_size.1 / self.page_size.0).max(0.01);
            let fit_w = per_w.max(1.0); // 横に合わせたときの表示幅(pt)
            let fit_h = (avail.y / aspect).max(1.0); // 縦に合わせたときの表示幅(pt)
            let ref_w = match self.fit_ref {
                FitKind::Width => fit_w,
                FitKind::Height => fit_h,
                FitKind::Window => fit_w.min(fit_h),
            };
            let disp_w = (ref_w * self.zoom).max(1.0);
            let disp = egui::vec2(disp_w, disp_w * aspect);

            // 表示幅から必要な描画解像度(px)を決め、刻みに丸める（=適応解像度）。
            let bucket = bucketize(disp_w * ctx.pixels_per_point());
            self.cur_bucket = bucket;

            // ホイールは拡大縮小に使うので、スクロール領域のホイール操作は無効化し、
            // パン(移動)はスクロールバーとドラッグで行う。
            egui::ScrollArea::both()
                .scroll_source(
                    egui::scroll_area::ScrollSource::SCROLL_BAR
                        | egui::scroll_area::ScrollSource::DRAG,
                )
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        for &idx in &pages {
                            let enhance = self.enhance;
                            self.request_render((idx, bucket, enhance));
                            match self.display_texture(idx) {
                                Some(tex) => {
                                    ui.add(egui::Image::new(&tex).fit_to_exact_size(disp));
                                }
                                None => {
                                    // 描き上がるまでのプレースホルダ
                                    let (rect, _) =
                                        ui.allocate_exact_size(disp, egui::Sense::hover());
                                    ui.painter().rect_filled(
                                        rect,
                                        2.0,
                                        egui::Color32::from_gray(238),
                                    );
                                    ui.painter().text(
                                        rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        "…",
                                        egui::FontId::proportional(28.0),
                                        egui::Color32::from_gray(130),
                                    );
                                }
                            }
                        }
                    });
                });

            Some((fit_w, fit_h))
        });

        // マウスホイールで表示倍率を増減（ポインタが本文領域にあるときだけ）。
        // 横/縦フィット倍率の近くでは、その基準に乗り換えて zoom=1.0 に吸着させる。
        if let Some((fw, fh)) = central.inner {
            let scroll_y = ctx.input(|i| i.smooth_scroll_delta.y);
            let over_central = ctx
                .input(|i| i.pointer.hover_pos())
                .is_some_and(|p| central.response.rect.contains(p));
            if scroll_y.abs() > 0.0 && over_central {
                let ref_fit = match self.fit_ref {
                    FitKind::Width => fw,
                    FitKind::Height => fh,
                    FitKind::Window => fw.min(fh),
                };
                let factor = (scroll_y * ZOOM_WHEEL_K).exp();
                // いったん絶対倍率に直して増減・クランプ
                let s = (ref_fit * self.zoom * factor).clamp(0.1 * fw.min(fh), 10.0 * fw.max(fh));

                // 横/縦フィットの近くなら、その基準に乗り換えて zoom=1.0（=リサイズ追従）
                if (s - fw).abs() <= fw * FIT_SNAP && (s - fw).abs() <= (s - fh).abs() {
                    self.fit_ref = FitKind::Width;
                    self.zoom = 1.0;
                } else if (s - fh).abs() <= fh * FIT_SNAP {
                    self.fit_ref = FitKind::Height;
                    self.zoom = 1.0;
                } else {
                    // 基準は維持し、その基準に対する相対倍率として保持
                    self.zoom = s / ref_fit;
                }
                ctx.request_repaint();
            }
        }

        // 目次パネルの右端に、自前のドラッグハンドルを CentralPanel の上に重ねる。
        // 最後に登録するので最前面で当たり、確実にドラッグを拾える。
        if let Some(rect) = toc_edge {
            let edge_x = rect.right();
            let handle = egui::Rect::from_x_y_ranges(
                egui::Rangef::new(edge_x - 4.0, edge_x + 4.0),
                rect.y_range(),
            );
            let resp = ui.interact(
                handle,
                egui::Id::new("toc_resizer"),
                egui::Sense::click_and_drag(),
            );
            if resp.hovered() || resp.dragged() {
                ctx.set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }
            if resp.dragged() {
                let dx = resp.drag_delta().x;
                let nw = (self.toc_width + dx).clamp(TOC_WIDTH_MIN, TOC_WIDTH_MAX);
                if (nw - self.toc_width).abs() > f32::EPSILON {
                    log_line(&format!(
                        "drag resize: dx={dx:.1} width {:.1} -> {nw:.1}",
                        self.toc_width
                    ));
                    self.toc_width = nw;
                }
                ctx.request_repaint();
            }
        }

        // 先読み: 前後の見開きを現在の解像度で描画スレッドに依頼しておく。
        // 描画は別スレッドなので UI はブロックされず、めくった瞬間はキャッシュ表示になる。
        if self.page_count > 0 {
            let enhance = self.enhance;
            let bucket = self.cur_bucket;
            for idx in self.prefetch_targets() {
                self.request_render((idx, bucket, enhance));
            }
            self.evict_cache();
        }
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 850.0])
            .with_title("PDR - ポータブル・ドキュメント・リーダー"),
        ..Default::default()
    };
    eframe::run_native(
        "PDR",
        options,
        Box::new(|cc| {
            log_line("=== PDR 起動 ===");
            if let Some(p) = log_path() {
                log_line(&format!("log file: {}", p.display()));
            }
            install_japanese_font(&cc.egui_ctx);

            // 描画スレッドを起動（pdfium はこのスレッドだけが触る）
            let (cmd_tx, cmd_rx) = std::sync::mpsc::channel();
            let (evt_tx, evt_rx) = std::sync::mpsc::channel();
            let worker_ctx = cc.egui_ctx.clone();
            std::thread::Builder::new()
                .name("pdf-render".into())
                .spawn(move || render_worker(cmd_rx, evt_tx, worker_ctx))
                .expect("描画スレッド起動失敗");

            let mut app = PdrApp::new(cmd_tx, evt_rx);
            // 引数で PDF パスが渡されたら起動時に開く（pdr.exe <path.pdf>）
            let argv: Vec<String> = std::env::args().collect();
            log_line(&format!("argv = {argv:?}"));
            if let Some(arg) = argv.get(1) {
                let path = PathBuf::from(arg);
                log_line(&format!("arg path={} is_file={}", path.display(), path.is_file()));
                if path.is_file() {
                    app.open_path(&path);
                } else {
                    app.status = format!("ファイルが見つかりません: {arg}");
                }
            }
            Ok(Box::new(app))
        }),
    )
}

/// 日本語が豆腐(□)にならないよう、Windows 同梱フォントを読み込む
fn install_japanese_font(ctx: &egui::Context) {
    let candidates = [
        r"C:\Windows\Fonts\meiryo.ttc",
        r"C:\Windows\Fonts\YuGothM.ttc",
        r"C:\Windows\Fonts\msgothic.ttc",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("jp".to_owned(), egui::FontData::from_owned(bytes).into());
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "jp".to_owned());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .push("jp".to_owned());
            ctx.set_fonts(fonts);
            return;
        }
    }
}

// ---- 目次(しおり)抽出 -------------------------------------------------------

const TOC_MAX_DEPTH: usize = 32;
const TOC_MAX_ENTRIES: usize = 10000;

/// PDF のしおり(outline)を、深さ付きの平坦なリストに変換する。
fn extract_toc(doc: &PdfDocument<'_>) -> Vec<TocEntry> {
    let mut out = Vec::new();
    let mut node = doc.bookmarks().root();
    while let Some(n) = node {
        let next = n.next_sibling();
        walk_bookmark(n, 0, &mut out);
        node = next;
    }
    out
}

fn walk_bookmark(node: PdfBookmark<'_>, depth: usize, out: &mut Vec<TocEntry>) {
    if depth > TOC_MAX_DEPTH || out.len() >= TOC_MAX_ENTRIES {
        return;
    }
    let title = node.title().unwrap_or_default();
    let page = node
        .destination()
        .and_then(|d| d.page_index().ok())
        .map(|i| i as usize);
    out.push(TocEntry { depth, title, page });

    let mut child = node.first_child();
    while let Some(c) = child {
        let next = c.next_sibling();
        walk_bookmark(c, depth + 1, out);
        child = next;
    }
}

// ---- 最近開いたファイル -----------------------------------------------------

/// 履歴ファイルのパス（%APPDATA%\pdr\recent.txt）。
fn config_file() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("pdr").join("recent.txt"))
}

/// ログファイルのパス（%APPDATA%\pdr\pdr.log）。
fn log_path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|a| PathBuf::from(a).join("pdr").join("pdr.log"))
}

/// 1 行ログを追記する。プロセス起動からの経過ミリ秒を先頭に付ける。
fn log_line(msg: &str) {
    use std::io::Write;
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    let elapsed = START
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_millis();
    let Some(path) = log_path() else {
        return;
    };
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(f, "[{elapsed:>8}ms] {msg}");
    }
}

fn load_recent() -> Vec<PathBuf> {
    let Some(f) = config_file() else {
        return Vec::new();
    };
    match std::fs::read_to_string(&f) {
        Ok(s) => s
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(PathBuf::from)
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn save_recent(recent: &[PathBuf]) {
    let Some(f) = config_file() else {
        return;
    };
    if let Some(dir) = f.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let body = recent
        .iter()
        .filter_map(|p| p.to_str())
        .collect::<Vec<_>>()
        .join("\n");
    let _ = std::fs::write(&f, body);
}
