// 同梱の pdfium ライブラリをビルド出力ディレクトリ（exe と同じ場所）へコピーする。
// これにより、起動時のカレントディレクトリに関係なく pdfium が見つかる。
//   - Windows: pdfium.dll
//   - macOS:   libpdfium.dylib
use std::path::Path;

fn main() {
    // プラットフォームごとの同梱ライブラリ名。存在するものだけコピーする。
    let lib_names = ["pdfium.dll", "libpdfium.dylib"];

    let out_dir = match std::env::var("OUT_DIR") {
        Ok(d) => d,
        Err(_) => return,
    };
    // OUT_DIR = target/<profile>/build/<pkg>-<hash>/out
    // 3 つ上が target/<profile>（exe の置き場）。
    let profile_dir = match Path::new(&out_dir).ancestors().nth(3) {
        Some(p) => p,
        None => return,
    };

    for name in lib_names {
        println!("cargo:rerun-if-changed=third_party/pdfium/{name}");
        let src = Path::new("third_party/pdfium").join(name);
        if src.exists() {
            let _ = std::fs::copy(&src, profile_dir.join(name));
        }
    }
}
