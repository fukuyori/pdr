# PDR — Portable Document Reader

[English](README.md) | [日本語](README.ja.md)

A Rust desktop PDF viewer built with egui/eframe and PDFium. It supports single-page and two-page spread views, left-to-right and right-to-left page binding, table of contents, and adaptive-resolution rendering on a background thread.

## Features

- **PDF viewing**: single-page or two-page spread, left-to-right or right-to-left binding, and a standalone cover page
- **Table of contents (bookmarks)**: shown automatically for supported PDFs; click an entry to navigate, and resize the panel
- **Navigation**: previous/next buttons, arrow keys, Page Up/Down, Space, and the bottom slider
- **Text selection**: drag with the primary mouse button to highlight text on a page. Works in single-page and spread views, including zoomed and rotated pages. Image-only PDFs without text data are not supported.
- **Copy**: right-click highlighted text to copy the selected text.
- **Translation**: translate highlighted text with a selected local Ollama model. Choose Japanese, English, Chinese, German, Spanish, or French as the target language.
- **Panning**: drag with the secondary mouse button, use the wheel, or use the scrollbars. A secondary click opens the page rotation menu.
- **Zoom**: use the mouse wheel. The view snaps to fit-width or fit-height and follows window resizing.
- **Image enhancement**: per-page auto-level correction, adjustable contrast and unsharp mask, and Otsu binarization for scanned PDFs
- **Fast rendering**: render pages on a background thread without blocking the UI, with resolution adapted to the display size and adjacent-page prefetching
- **Settings restore**: display mode, page binding, standalone cover, image enhancement, table-of-contents width, translation settings, and the list of recently opened PDFs are restored at the next launch. PDR starts with a blank page; it does not automatically reopen a PDF or restore page position, temporary rotation, text selection, fit mode, or zoom level. Select a file from **History** to open it.

## Ollama translation

### Install Ollama and download the tested model

Install Ollama using the [official download page](https://ollama.com/download). The official command-line installation options are:

```sh
# macOS and Linux
curl -fsSL https://ollama.com/install.sh | sh
```

```powershell
# Windows PowerShell
irm https://ollama.com/install.ps1 | iex
```

PDR has been tested with **`translategemma:4b`**. Download it from a terminal after installing Ollama:

```sh
ollama pull translategemma:4b
```

See the [TranslateGemma model page](https://ollama.com/library/translategemma) for model details.

### Configure and translate

In the translation window, expand **「翻訳設定」** (Translation Settings), enter an Ollama endpoint (for example, `http://localhost:11434`), and click **「モデル一覧を更新」** (Refresh model list). Select **`translategemma:4b`** from the model dropdown and choose a target language. The endpoint, model, target language, and settings-panel collapsed state are saved.

The translation window can be resized. The source and translation panes have equal widths and separate scrollbars. Hold **Ctrl** while using the mouse wheel in the window to enlarge or reduce its font.

Select text in the PDF and right-click the highlighted text to open the translation window. If the selected model is already available, translation starts automatically; otherwise, refresh the model list and click **「翻訳する」** (Translate). The result appears in the same window; click **「コピー」** (Copy) to copy it. Translation runs in the background, and errors are shown in the window. Start Ollama and make sure the model you want to use is available before translating.

Translation settings are saved in:

- Windows: `%APPDATA%\pdr\translation.json`
- macOS: `~/Library/Application Support/pdr/translation.json`
- Linux: `$XDG_CONFIG_HOME/pdr/translation.json` (or `~/.config/pdr/translation.json` when unset)

Display settings are saved in:

- Windows: `%APPDATA%\pdr\settings.json`
- macOS: `~/Library/Application Support/pdr/settings.json`
- Linux: `$XDG_CONFIG_HOME/pdr/settings.json` (or `~/.config/pdr/settings.json` when unset)

The list of up to 12 recently opened PDFs is saved in `recent_files.json` in the same configuration folder. PDR loads the list at startup but does not open any of those files automatically.

## Requirements

- Rust (stable, MSVC on Windows)
- `pdfium.dll` (Windows x64) is included at `third_party/pdfium/pdfium.dll`; you can clone, build, and run the project as-is on Windows.

PDR searches for `pdfium.dll` in this order: next to the executable, the current directory, `./third_party/pdfium`, `./lib/bin`, and then system locations. When running `cargo run` from the repository root, the bundled file is found automatically. For distribution, place `pdfium.dll` next to the executable.

The bundled PDFium build is the Windows x64 build from [bblanchon/pdfium-binaries](https://github.com/bblanchon/pdfium-binaries) (see `third_party/pdfium/VERSION`). To update it:

```sh
curl -sL -o pdfium.tgz https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-win-x64.tgz
tar -xzf pdfium.tgz bin/pdfium.dll LICENSE VERSION licenses
mv bin/pdfium.dll third_party/pdfium/pdfium.dll
mv LICENSE VERSION licenses third_party/pdfium/
```

## Build and run

```sh
cargo run                 # Launch PDR
cargo run -- path/to.pdf  # Open a PDF on launch
cargo test                # Run tests
```

For distribution, place `pdfium.dll` next to the executable.

## Build a Linux deb package

Create a Linux deb package with:

```sh
scripts/package-deb.sh --download-pdfium
```

The output is `dist/pdr_<version>_<arch>.deb`. If `third_party/pdfium/libpdfium.so` is already present, omit `--download-pdfium`. To use an existing build, pass `--no-build` to `scripts/package-deb.sh`.

## Install on Windows and add PDR to PATH

The Inno Setup installer offers an unchecked **Add PDR to PATH** task. Select it to run `pdr` from a terminal; the installer removes the entry it added when you uninstall PDR. Open a new terminal after installation for the updated PATH to take effect.

Keep `pdr.exe` and `pdfium.dll` in the **same folder**. Add that folder to PATH to run `pdr` from a terminal or file association.

1. Extract the distribution archive (`pdr-x.y.z-win-x64.zip`), or use `target\release\pdr.exe` and `third_party\pdfium\pdfium.dll` after running `cargo build --release`.
2. Create a destination folder (for example, `%USERPROFILE%\bin`) and copy **both `pdr.exe` and `pdfium.dll`** there.
3. Add the folder to your user PATH. This PowerShell example takes effect in newly opened terminals:

   ```powershell
   setx PATH "$env:USERPROFILE\bin;$env:PATH"
   ```

4. Run PDR from any directory:

   ```powershell
   pdr "C:\path\to\file.pdf"
   ```

> **Important:** Keep `pdfium.dll` in the same folder as `pdr.exe`; placing it in another folder on PATH is not sufficient. PDR searches next to its executable first, so it can find the DLL regardless of the working directory.

## Utility binary

- `cargo run --bin smoke -- <pdf> [page] [none|contrast|auto|sharpen|binarize] [strength]` — render one page to PNG without the GUI
- `cargo run --bin smoke -- <pdf> toc` — dump the table of contents (bookmarks)
- `cargo run --bin smoke -- <pdf> bench [width] [count]` — measure the time to render one page

## License

The application code is licensed under **Apache License 2.0** ([LICENSE](LICENSE)).

PDR uses [PDFium](https://pdfium.googlesource.com/pdfium/) (**BSD-3-Clause** and other licenses) for rendering and includes a prebuilt `pdfium.dll` in `third_party/pdfium/`. Include the applicable licenses in distributions; see [NOTICE](NOTICE).

- `third_party/pdfium/LICENSE` — pdfium-binaries packaging (MIT, Benoit Blanchon)
- `third_party/pdfium/licenses/pdfium.txt` — PDFium (BSD-3-Clause, Google)
- `third_party/pdfium/licenses/` — licenses for FreeType, libpng, zlib, ICU, and other components
