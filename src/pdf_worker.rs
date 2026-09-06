use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

use eframe::egui;
use pdfium_render::prelude::*;

use pdr::enhance::{Enhance, apply_enhance};

use crate::log_line;

/// 描画キャッシュのキー: (ページ, 描画幅px, 補正)
pub(crate) type RenderKey = (usize, i32, Enhance);

/// UI→描画スレッドへの指示
pub(crate) enum RenderCmd {
    Open {
        path: PathBuf,
        doc_gen: u64,
    },
    Render {
        page: usize,
        width: i32,
        enhance: Enhance,
        doc_gen: u64,
    },
}

/// 描画スレッド→UIへの結果
pub(crate) enum RenderEvt {
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

/// 目次(しおり)の 1 項目。PdfBookmark の借用を持たず、所有データだけ保持する。
pub(crate) struct TocEntry {
    pub(crate) depth: usize,
    pub(crate) title: String,
    pub(crate) page: Option<usize>,
}

/// 描画スレッド本体。pdfium はここだけが触る。UI スレッドは一切ブロックしない。
pub(crate) fn render_worker(rx: Receiver<RenderCmd>, tx: Sender<RenderEvt>, ctx: egui::Context) {
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
