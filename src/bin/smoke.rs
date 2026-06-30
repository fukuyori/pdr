// ヘッドレスでの描画検証用:
//   smoke <pdf path> [page] [none|contrast|binarize]
// 指定ページを補正適用して PNG 出力する。
use pdfium_render::prelude::*;
use pdr::enhance::{Enhance, apply_enhance};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let pdf = args
        .get(1)
        .expect("usage: smoke <pdf path> [page] [none|contrast|binarize]");
    let page_no: i32 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let mode = match args.get(3).map(|s| s.as_str()) {
        Some("contrast") => Enhance::Contrast,
        Some("binarize") => Enhance::Binarize,
        _ => Enhance::None,
    };

    let path = Pdfium::pdfium_platform_library_name_at_path(".");
    let bindings = Pdfium::bind_to_library(&path)
        .or_else(|_| Pdfium::bind_to_system_library())
        .expect("pdfium bind failed");
    let pdfium = Pdfium::new(bindings);

    let doc = pdfium.load_pdf_from_file(pdf, None).expect("load failed");
    println!("pages = {}", doc.pages().len());

    // 目次(しおり)確認モード: smoke <pdf> toc
    if args.get(2).map(|s| s.as_str()) == Some("toc") {
        fn walk(node: PdfBookmark<'_>, depth: usize, n: &mut usize) {
            let title = node.title().unwrap_or_default();
            let page = node.destination().and_then(|d| d.page_index().ok());
            println!("{}{} -> {:?}", "  ".repeat(depth), title, page);
            *n += 1;
            let mut child = node.first_child();
            while let Some(c) = child {
                let next = c.next_sibling();
                walk(c, depth + 1, n);
                child = next;
            }
        }
        let mut n = 0;
        let mut node = doc.bookmarks().root();
        while let Some(b) = node {
            let next = b.next_sibling();
            walk(b, 0, &mut n);
            node = next;
        }
        println!("toc entries = {n}");
        return;
    }

    // 計時モード: smoke <pdf> bench [width] [count]
    //   render + as_image + to_rgba8（アプリの1ページ描画ホットパス）の平均msを出す
    if args.get(2).map(|s| s.as_str()) == Some("bench") {
        let width: i32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1600);
        let count: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(10);
        let cfg = PdfRenderConfig::new()
            .set_target_width(width)
            .set_maximum_height(width * 2);
        let mut total = std::time::Duration::ZERO;
        for i in 0..count {
            let pg = doc.pages().get((i as i32) % doc.pages().len()).expect("page");
            let t = std::time::Instant::now();
            let bmp = pg.render_with_config(&cfg).expect("render");
            let img = bmp.as_image().expect("image");
            let _rgba = img.to_rgba8();
            total += t.elapsed();
        }
        println!(
            "width={width} count={count} avg={:.1}ms/page",
            total.as_secs_f64() * 1000.0 / count as f64
        );
        return;
    }

    let config = PdfRenderConfig::new().set_target_width(800);
    let page = doc.pages().get(page_no).expect("page");
    let bitmap = page.render_with_config(&config).expect("render");
    let image = apply_enhance(bitmap.as_image().expect("image"), mode);

    let out = format!("smoke_p{page_no}_{mode:?}.png");
    image.into_luma8().save(&out).expect("save");
    println!("wrote {out}");
    println!("OK");
}
