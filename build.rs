// 同梱の pdfium.dll をビルド出力ディレクトリ（exe と同じ場所）へコピーする。
// これにより、起動時のカレントディレクトリに関係なく pdfium.dll が見つかる。
use std::path::Path;

fn main() {
    let src = Path::new("third_party/pdfium/pdfium.dll");
    println!("cargo:rerun-if-changed=third_party/pdfium/pdfium.dll");

    if !src.exists() {
        return;
    }
    // OUT_DIR = target/<profile>/build/<pkg>-<hash>/out
    // 3 つ上が target/<profile>（exe の置き場）。
    if let Ok(out_dir) = std::env::var("OUT_DIR") {
        if let Some(profile_dir) = Path::new(&out_dir).ancestors().nth(3) {
            let dst = profile_dir.join("pdfium.dll");
            let _ = std::fs::copy(src, dst);
        }
    }
}
