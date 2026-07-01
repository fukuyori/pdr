//! macOS で Finder から「PDR で開く」/ ダブルクリックされた PDF を受け取る。
//!
//! winit/eframe は macOS のファイルオープンをアプリへ渡さない。winit は自前の
//! `NSApplicationDelegate` を設定するため、デリゲートを差し替えると競合する。
//! そこで `NSAppleEventManager` に open-documents Apple Event のハンドラを直接
//! 登録して受け取る（NSApplication 起動後に登録することで既定ハンドラを上書き）。
//! 受け取ったパスはグローバルに貯め、UI 側が毎フレーム取り出して開く。

use std::cell::RefCell;
use std::ffi::c_char;
use std::path::PathBuf;
use std::sync::Mutex;

use objc2::ffi::{class_addMethod, class_getInstanceMethod};
use objc2::rc::{Retained, autoreleasepool};
use objc2::runtime::{AnyClass, AnyObject, Imp, NSObject, NSObjectProtocol, ProtocolObject, Sel};
use objc2::{ClassType, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSApplication, NSApplicationDelegate, NSApplicationDelegateReply, NSDocument};
use objc2_foundation::{
    NSArray, NSAppleEventDescriptor, NSAppleEventManager, NSError, NSString, NSURL,
};

// FourCharCode 定数
const K_CORE_EVENT_CLASS: u32 = 0x6165_7674; // 'aevt'
const K_AE_OPEN_DOCUMENTS: u32 = 0x6f64_6f63; // 'odoc'
const KEY_DIRECT_OBJECT: u32 = 0x2d2d_2d2d; // '----'
const TYPE_FILE_URL: u32 = 0x6675_726c; // 'furl'
const OBJC_VOID_OPEN_URLS: *const c_char = b"v@:@@\0".as_ptr().cast();
const OBJC_BOOL_OPEN_FILE: *const c_char = b"B@:@@\0".as_ptr().cast();
const OBJC_VOID_OPEN_FILES: *const c_char = b"v@:@@\0".as_ptr().cast();

static PENDING: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

thread_local! {
    static HANDLER: RefCell<Option<Retained<OpenFilesHandler>>> = const { RefCell::new(None) };
}

/// 溜まっている「開くべきファイル」を取り出す（UI が毎フレーム呼ぶ）。
pub fn take_pending() -> Vec<PathBuf> {
    match PENDING.lock() {
        Ok(mut v) => std::mem::take(&mut *v),
        Err(_) => Vec::new(),
    }
}

fn push_path(p: PathBuf) {
    if let Ok(mut v) = PENDING.lock() {
        v.push(p);
    }
}

fn push_url(url: &NSURL) -> bool {
    let Some(path) = url.to_file_path() else {
        return false;
    };
    push_path(path);
    true
}

define_class!(
    #[unsafe(super(NSDocument))]
    #[thread_kind = MainThreadOnly]
    #[name = "PDRDocument"]
    struct PdrDocument;

    unsafe impl NSObjectProtocol for PdrDocument {}

    impl PdrDocument {
        #[unsafe(method(readFromURL:ofType:error:))]
        fn read_from_url(
            &self,
            url: &NSURL,
            _type_name: &NSString,
            _error: *mut *mut NSError,
        ) -> bool {
            push_url(url)
        }

        #[unsafe(method(readFromURL:ofType:))]
        fn read_from_url_deprecated(&self, url: &NSURL, _type_name: &NSString) -> bool {
            push_url(url)
        }

        #[unsafe(method(makeWindowControllers))]
        fn make_window_controllers(&self) {}
    }
);

/// Info.plist の NSDocumentClass から見えるようにクラス登録を確定する。
pub fn register_document_class() {
    let _ = PdrDocument::class();
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "PDROpenFilesHandler"]
    struct OpenFilesHandler;

    unsafe impl NSObjectProtocol for OpenFilesHandler {}

    impl OpenFilesHandler {
        #[unsafe(method(handleAppleEvent:withReplyEvent:))]
        fn handle_apple_event(
            &self,
            event: &NSAppleEventDescriptor,
            _reply: &NSAppleEventDescriptor,
        ) {
            for path in paths_from_event(event) {
                push_path(path);
            }
        }
    }
);

impl OpenFilesHandler {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        unsafe { msg_send![Self::alloc(mtm), init] }
    }
}

fn paths_from_event(event: &NSAppleEventDescriptor) -> Vec<PathBuf> {
    let Some(list) = event.paramDescriptorForKeyword(KEY_DIRECT_OBJECT) else {
        return Vec::new();
    };

    let mut paths = Vec::new();
    let n = list.numberOfItems();
    for i in 1..=n {
        let Some(item) = list.descriptorAtIndex(i) else {
            continue;
        };
        if let Some(path) = path_from_descriptor(&item) {
            paths.push(path);
        }
    }
    paths
}

fn path_from_descriptor(item: &NSAppleEventDescriptor) -> Option<PathBuf> {
    if let Some(url) = item.fileURLValue() {
        if let Some(path) = url.to_file_path() {
            return Some(path);
        }
    }

    if let Some(furl) = item.coerceToDescriptorType(TYPE_FILE_URL) {
        if let Some(url) = furl.fileURLValue() {
            if let Some(path) = url.to_file_path() {
                return Some(path);
            }
        }
        if let Some(s) = furl.stringValue() {
            if let Some(path) = file_url_to_path(&s.to_string()) {
                return Some(path);
            }
        }
        let bytes = furl.data().to_vec();
        if let Ok(s) = std::str::from_utf8(&bytes) {
            if let Some(path) = file_url_to_path(s) {
                return Some(path);
            }
        }
    }

    if let Some(s) = item.stringValue() {
        if let Some(path) = file_url_to_path(&s.to_string()) {
            return Some(path);
        }
    }

    None
}

/// open-documents Apple Event のハンドラを登録する。
///
/// コールドローンチでは Finder からの open-documents イベントが早い段階で届くため、
/// `run_native` の前に呼んでおく。eframe/winit の初期化後にも再度呼ぶと、もし
/// 既定ハンドラで上書きされていても同じ受信ハンドラへ戻せる。
pub fn install() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    HANDLER.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            *slot = Some(OpenFilesHandler::new(mtm));
        }
        let handler = slot.as_ref().expect("handler was just initialized");
        let mgr = NSAppleEventManager::sharedAppleEventManager();
        let obj: &AnyObject = handler;
        unsafe {
            mgr.setEventHandler_andSelector_forEventClass_andEventID(
                obj,
                sel!(handleAppleEvent:withReplyEvent:),
                K_CORE_EVENT_CLASS,
                K_AE_OPEN_DOCUMENTS,
            );
        }
    });
}

/// winit が作成した NSApplicationDelegate に open-document メソッドを追加する。
///
/// AppKit は cold launch の文書オープン時に delegate を先に見るため、ここで
/// `application:openURLs:` を実装済みにして既定の「開けない」アラートを抑止する。
pub fn patch_application_delegate() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    let Some(delegate) = app.delegate() else {
        return;
    };
    let delegate_ref: &ProtocolObject<dyn NSApplicationDelegate> = &delegate;
    let delegate_obj: &AnyObject = delegate_ref.as_ref();
    let cls = delegate_obj.class();

    unsafe {
        add_method_if_missing(
            cls,
            sel!(application:openURLs:),
            app_open_urls as unsafe extern "C-unwind" fn(_, _, _, _),
            OBJC_VOID_OPEN_URLS,
        );
        add_method_if_missing(
            cls,
            sel!(application:openFile:),
            app_open_file as unsafe extern "C-unwind" fn(_, _, _, _) -> bool,
            OBJC_BOOL_OPEN_FILE,
        );
        add_method_if_missing(
            cls,
            sel!(application:openFiles:),
            app_open_files as unsafe extern "C-unwind" fn(_, _, _, _),
            OBJC_VOID_OPEN_FILES,
        );
    }
}

unsafe fn add_method_if_missing<F>(cls: &AnyClass, selector: Sel, imp: F, types: *const c_char)
where
    F: Copy,
{
    if unsafe { class_getInstanceMethod(cls, selector) }.is_null() {
        let imp: Imp = unsafe { std::mem::transmute_copy(&imp) };
        let _ = unsafe { class_addMethod(cls as *const AnyClass as *mut AnyClass, selector, imp, types) };
    }
}

unsafe extern "C-unwind" fn app_open_urls(
    _this: &ProtocolObject<dyn NSApplicationDelegate>,
    _cmd: Sel,
    _app: &NSApplication,
    urls: &NSArray<NSURL>,
) {
    for url in urls {
        if let Some(path) = url.to_file_path() {
            push_path(path);
        }
    }
}

unsafe extern "C-unwind" fn app_open_file(
    _this: &ProtocolObject<dyn NSApplicationDelegate>,
    _cmd: Sel,
    _app: &NSApplication,
    filename: &NSString,
) -> bool {
    push_path(PathBuf::from(filename.to_string()));
    true
}

unsafe extern "C-unwind" fn app_open_files(
    _this: &ProtocolObject<dyn NSApplicationDelegate>,
    _cmd: Sel,
    app: &NSApplication,
    filenames: &NSArray<NSString>,
) {
    for filename in filenames {
        push_path(PathBuf::from(filename.to_string()));
    }
    autoreleasepool(|_| {
        app.replyToOpenOrPrint(NSApplicationDelegateReply::Success);
    });
}

/// 起動直後に既に処理中の open-documents Apple Event があれば取り込む。
pub fn capture_current_event() {
    let mgr = NSAppleEventManager::sharedAppleEventManager();
    if let Some(event) = mgr.currentAppleEvent() {
        for path in paths_from_event(&event) {
            push_path(path);
        }
    }
}

/// "file:///path%20x.pdf" のようなファイル URL 文字列をパスへ変換する。
fn file_url_to_path(s: &str) -> Option<PathBuf> {
    let rest = s.trim().strip_prefix("file://")?;
    // 先頭にホスト名(localhost 等)が付くことがあるので、最初の '/' 以降をパスとみなす
    let path_part = match rest.find('/') {
        Some(idx) => &rest[idx..],
        None => rest,
    };
    Some(PathBuf::from(percent_decode(path_part)))
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2])) {
                out.push(h * 16 + l);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}
