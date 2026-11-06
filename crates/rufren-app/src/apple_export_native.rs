//! Main-thread platform adapter. Files first land in our own unique temporary
//! directory; only verified bytes reach a chooser-authorized destination/share.
use super::{ExportRequest, ExportResult, verify_file};
#[cfg(target_os = "macos")]
use super::{commit_file, commit_file_in};
use block2::RcBlock;
#[cfg(feature = "desktop")]
use dioxus::desktop as native_runtime;
#[cfg(all(not(feature = "desktop"), feature = "mobile"))]
use dioxus::mobile as native_runtime;
use native_runtime::DesktopContext;
#[cfg(target_os = "ios")]
use native_runtime::wry::WebViewExtIOS;
#[cfg(target_os = "macos")]
use native_runtime::wry::WebViewExtMacOS;
use objc2::{
    DeclaredClass, MainThreadOnly, define_class, msg_send,
    rc::Retained,
    runtime::{NSObject, ProtocolObject},
};
use objc2_foundation::{
    MainThreadMarker, NSData, NSError, NSHTTPURLResponse, NSObjectProtocol, NSOperationQueue,
    NSString, NSURL, NSURLRequest, NSURLResponse,
};
use objc2_web_kit::{WKDownload, WKDownloadDelegate, WKDownloadRedirectPolicy};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    path::PathBuf,
    ptr::NonNull,
    rc::Rc,
};
use uuid::Uuid;

struct State {
    window: DesktopContext,
    request: ExportRequest,
    terminal: Cell<bool>,
    checking: Cell<bool>,
    stage: RefCell<Option<PathBuf>>,
    directory: RefCell<Option<PathBuf>>,
    destination: RefCell<Option<PathBuf>>,
    delegate: RefCell<Option<Retained<ExportDelegate>>>,
    download: RefCell<Option<Retained<WKDownload>>>,
    #[cfg(target_os = "macos")]
    security_url: RefCell<Option<Retained<NSURL>>>,
    #[cfg(target_os = "ios")]
    activity: RefCell<Option<Retained<objc2_ui_kit::UIActivityViewController>>>,
}
thread_local! {static ACTIVE:RefCell<BTreeMap<Uuid,Rc<State>>>=const{RefCell::new(BTreeMap::new())};}
fn state(id: Uuid) -> Option<Rc<State>> {
    ACTIVE.with(|active| active.borrow().get(&id).cloned())
}

fn later(task: impl Fn() + 'static) {
    let block = RcBlock::new(task);
    // Always schedule on Apple's main queue; no retained UI object crosses a
    // worker thread, and delegate removal occurs after its callback returns.
    unsafe {
        NSOperationQueue::mainQueue().addOperationWithBlock(&block);
    }
}
fn send(window: &DesktopContext, result: &ExportResult) {
    if let Ok(value) = serde_json::to_string(result) {
        let _ = window
            .webview
            .evaluate_script(&format!("window.rufrenFileExports?.complete({value});"));
    }
}
fn finish(value: &Rc<State>, status: &'static str, message: String, path: Option<String>) {
    if value.terminal.replace(true) {
        return;
    }
    send(
        &value.window,
        &ExportResult {
            request_id: value.request.request_id,
            status,
            message,
            path,
        },
    );
    if let Some(path) = value.stage.borrow_mut().take() {
        let _ = std::fs::remove_file(path);
    }
    if let Some(path) = value.directory.borrow_mut().take() {
        let _ = std::fs::remove_dir(path);
    }
    #[cfg(target_os = "macos")]
    if let Some(url) = value.security_url.borrow_mut().take() {
        unsafe {
            url.stopAccessingSecurityScopedResource();
        }
    }
    let id = value.request.request_id;
    later(move || {
        ACTIVE.with(|active| {
            active.borrow_mut().remove(&id);
        });
    });
}

pub(crate) fn start_export(request: ExportRequest) {
    let window = native_runtime::window();
    if let Err(message) = request.validate() {
        send(
            &window,
            &ExportResult {
                request_id: request.request_id,
                status: "failed",
                message,
                path: None,
            },
        );
        return;
    }
    if MainThreadMarker::new().is_none() {
        send(
            &window,
            &ExportResult {
                request_id: request.request_id,
                status: "failed",
                message: "Open export from the app window.".into(),
                path: None,
            },
        );
        return;
    }
    if ACTIVE.with(|active| !active.borrow().is_empty()) {
        send(
            &window,
            &ExportResult {
                request_id: request.request_id,
                status: "failed",
                message: "Finish or cancel the current export first.".into(),
                path: None,
            },
        );
        return;
    }
    let id = request.request_id;
    let value = Rc::new(State {
        window: window.clone(),
        request,
        terminal: Cell::new(false),
        checking: Cell::new(false),
        stage: RefCell::new(None),
        directory: RefCell::new(None),
        destination: RefCell::new(None),
        delegate: RefCell::new(None),
        download: RefCell::new(None),
        #[cfg(target_os = "macos")]
        security_url: RefCell::new(None),
        #[cfg(target_os = "ios")]
        activity: RefCell::new(None),
    });
    let packet = serde_json::to_string(&value.request).expect("serializable export request");
    ACTIVE.with(|active| {
        active.borrow_mut().insert(id, value.clone());
    });
    // Scheme validation is insufficient for opaque-origin Blob URLs. Require
    // the exact live renderer registration, including name/length/hash/token.
    let script = format!("window.rufrenFileExports?.owns({packet}) === true");
    if let Err(error)=window.webview.evaluate_script_with_callback(&script,move|reply|{
        let owned=reply.trim()=="true";
        later(move||{if let Some(value)=state(id){if owned{begin(&value);}else{finish(&value,"failed","This export no longer belongs to the current app window. Your original is kept.".into(),None);}}});
    }){finish(&value,"failed",format!("The app could not verify this export: {error}"),None);}
}

fn begin(value: &Rc<State>) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    #[cfg(target_os = "macos")]
    {
        use objc2_app_kit::{NSModalResponseOK, NSSavePanel};
        let panel = NSSavePanel::savePanel(mtm);
        panel.setNameFieldStringValue(&NSString::from_str(&value.request.name));
        panel.setCanCreateDirectories(true);
        if panel.runModal() != NSModalResponseOK {
            finish(
                value,
                "cancelled",
                "Export cancelled. Your original is kept.".into(),
                None,
            );
            return;
        }
        let Some(url) = panel.URL().filter(|url| url.isFileURL()) else {
            finish(
                value,
                "failed",
                "The save dialog did not choose a file.".into(),
                None,
            );
            return;
        };
        let Some(path) = url
            .path()
            .map(|p| PathBuf::from(p.to_string()))
            .filter(|p| p.is_absolute())
        else {
            finish(
                value,
                "failed",
                "Choose a local export destination.".into(),
                None,
            );
            return;
        };
        // Save-panel access is an explicit user grant. Some returned URLs have
        // an additional security scope; balance only a scope we actually start.
        if unsafe { url.startAccessingSecurityScopedResource() } {
            value.security_url.replace(Some(url));
        }
        value.destination.replace(Some(path));
    }
    let directory =
        std::env::temp_dir().join(format!("rufren-export-{}", value.request.request_id));
    if let Err(error) = std::fs::create_dir(&directory) {
        finish(
            value,
            "failed",
            format!("Could not prepare the export’s temporary folder: {error}"),
            None,
        );
        return;
    }
    let stage = directory.join(&value.request.name);
    value.directory.replace(Some(directory));
    value.stage.replace(Some(stage));
    let Some(url) = NSURL::URLWithString(&NSString::from_str(&value.request.blob_url)) else {
        finish(
            value,
            "failed",
            "The registered export URL is invalid.".into(),
            None,
        );
        return;
    };
    let request = NSURLRequest::requestWithURL(&url);
    let delegate = ExportDelegate::new(value.request.request_id, mtm);
    value.delegate.replace(Some(delegate));
    let id = value.request.request_id;
    let block = RcBlock::new(move |download: NonNull<WKDownload>| {
        if let Some(value) = state(id) {
            if let Some(delegate) = value.delegate.borrow().as_ref() {
                unsafe {
                    download
                        .as_ref()
                        .setDelegate(Some(ProtocolObject::from_ref(&**delegate)));
                    value.download.replace(Retained::retain(download.as_ptr()));
                }
            }
        }
    });
    unsafe {
        value
            .window
            .webview
            .webview()
            .startDownloadUsingRequest_completionHandler(&request, &block);
    }
}

struct DelegateIvars {
    id: Uuid,
}
define_class!(
    #[unsafe(super(NSObject))]
    #[name = "RufrenExportDownloadDelegate"]
    #[thread_kind=MainThreadOnly]
    #[ivars=DelegateIvars]
    struct ExportDelegate;
    unsafe impl NSObjectProtocol for ExportDelegate {}
    unsafe impl WKDownloadDelegate for ExportDelegate {
        #[unsafe(method(download:decideDestinationUsingResponse:suggestedFilename:completionHandler:))]
        fn destination(
            &self,
            download: &WKDownload,
            _response: &NSURLResponse,
            _name: &NSString,
            handler: &block2::Block<dyn Fn(*const NSURL)>,
        ) {
            let Some(value) = state(self.ivars().id) else {
                unsafe {
                    handler.call((std::ptr::null(),));
                }
                return;
            };
            let original = unsafe { download.originalRequest() }
                .and_then(|request| request.URL())
                .and_then(|url| url.absoluteString())
                .map(|s| s.to_string());
            if original.as_deref() != Some(value.request.blob_url.as_str()) {
                unsafe {
                    handler.call((std::ptr::null(),));
                }
                finish(
                    &value,
                    "failed",
                    "The native download did not match the registered original.".into(),
                    None,
                );
                return;
            }
            let path = value.stage.borrow().clone();
            if let Some(path) = path {
                let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
                unsafe {
                    handler.call((Retained::as_ptr(&url),));
                }
            } else {
                unsafe {
                    handler.call((std::ptr::null(),));
                }
                finish(
                    &value,
                    "failed",
                    "The export lost its temporary destination.".into(),
                    None,
                );
            }
        }
        #[unsafe(method(download:willPerformHTTPRedirection:newRequest:decisionHandler:))]
        fn redirect(
            &self,
            _download: &WKDownload,
            _response: &NSHTTPURLResponse,
            _request: &NSURLRequest,
            handler: &block2::Block<dyn Fn(WKDownloadRedirectPolicy)>,
        ) {
            unsafe {
                handler.call((WKDownloadRedirectPolicy::Cancel,));
            }
        }
        #[unsafe(method(downloadDidFinish:))]
        fn completed(&self, _download: &WKDownload) {
            if let Some(value) = state(self.ivars().id) {
                check_completed(&value);
            }
        }
        #[unsafe(method(download:didFailWithError:resumeData:))]
        fn failed(&self, _download: &WKDownload, error: &NSError, _resume: Option<&NSData>) {
            if let Some(value) = state(self.ivars().id) {
                finish(
                    &value,
                    "failed",
                    format!(
                        "The export could not be saved: {}. Your original is kept.",
                        error.localizedDescription()
                    ),
                    None,
                );
            }
        }
    }
);
impl ExportDelegate {
    fn new(id: Uuid, mtm: MainThreadMarker) -> Retained<Self> {
        let value = mtm.alloc::<Self>().set_ivars(DelegateIvars { id });
        unsafe { msg_send![super(value), init] }
    }
}

fn check_completed(value: &Rc<State>) {
    if value.terminal.get() || value.checking.replace(true) {
        return;
    }
    let id = value.request.request_id;
    let stage = value.stage.borrow().clone();
    let Some(stage) = stage else {
        finish(
            value,
            "failed",
            "The completed export is missing.".into(),
            None,
        );
        return;
    };
    let request = value.request.clone();
    #[cfg(target_os = "macos")]
    let destination = value.destination.borrow().clone();
    if let Err(error) = std::thread::Builder::new()
        .name("rufren-export-check".into())
        .spawn(move || {
            #[cfg(target_os = "macos")]
            let result = destination
                .ok_or("The save destination is missing.".into())
                .and_then(|path| commit_chosen_file(&stage, &path, &request).map(|_| path));
            #[cfg(target_os = "ios")]
            let result = verify_file(&stage, request.byte_length, &request.sha256).map(|_| stage);
            later(move || {
                if let Some(value) = state(id) {
                    match &result {
                        Err(error) => finish(
                            &value,
                            "failed",
                            format!("Could not finish this export: {error}"),
                            None,
                        ),
                        #[cfg(target_os = "macos")]
                        Ok(path) => finish(
                            &value,
                            "saved",
                            format!("Saved to {}", path.display()),
                            Some(path.display().to_string()),
                        ),
                        #[cfg(target_os = "ios")]
                        Ok(path) => share(&value, path),
                    }
                }
            });
        })
    {
        finish(
            value,
            "failed",
            format!("Could not check the completed export: {error}"),
            None,
        );
    }
}

#[cfg(target_os = "macos")]
fn commit_chosen_file(
    source: &std::path::Path,
    destination: &std::path::Path,
    request: &ExportRequest,
) -> Result<(), String> {
    use objc2_foundation::{NSFileManager, NSSearchPathDirectory, NSSearchPathDomainMask};
    verify_file(source, request.byte_length, &request.sha256)?;
    if commit_file(source, destination, request).is_ok() {
        return Ok(());
    }
    // A removable/iCloud destination may be on a different volume. Foundation
    // provides an authorized replacement directory appropriate to the chosen
    // URL, instead of guessing that the app can write arbitrary sibling paths.
    let url = NSURL::fileURLWithPath(&NSString::from_str(&destination.to_string_lossy()));
    let manager = NSFileManager::defaultManager();
    let directory = manager
        .URLForDirectory_inDomain_appropriateForURL_create_error(
            NSSearchPathDirectory::ItemReplacementDirectory,
            NSSearchPathDomainMask::UserDomainMask,
            Some(&url),
            true,
        )
        .map_err(|error| {
            format!(
                "Could not prepare the chosen destination: {}",
                error.localizedDescription()
            )
        })?;
    let path = directory
        .path()
        .map(|value| PathBuf::from(value.to_string()))
        .filter(|path| path.is_absolute())
        .ok_or("The chosen volume has no safe replacement folder.")?;
    let result = commit_file_in(source, destination, request, &path);
    // This is the unique replacement directory created above, never an existing
    // user directory. remove_dir preserves anything unexpected left inside it.
    let _ = std::fs::remove_dir(&path);
    result
}

#[cfg(target_os = "ios")]
fn share(value: &Rc<State>, path: &std::path::Path) {
    use objc2::runtime::{AnyObject, Bool};
    use objc2_foundation::NSArray;
    use objc2_ui_kit::{UIActivityType, UIActivityViewController};
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let webview = value.window.webview.webview();
    let Some(controller) = webview
        .window()
        .and_then(|window| window.rootViewController())
    else {
        finish(
            value,
            "cancelled",
            "The export window closed. Your original is kept.".into(),
            None,
        );
        return;
    };
    if controller.presentedViewController().is_some() {
        finish(
            value,
            "failed",
            "Close the current dialog, then export again. Your original is kept.".into(),
            None,
        );
        return;
    }
    let url = NSURL::fileURLWithPath(&NSString::from_str(&path.to_string_lossy()));
    let items = NSArray::<AnyObject>::from_slice(&[&url]);
    let activity = unsafe {
        UIActivityViewController::initWithActivityItems_applicationActivities(
            mtm.alloc(),
            &items,
            None,
        )
    };
    let id = value.request.request_id;
    let completion = RcBlock::new(
        move |_kind: *mut UIActivityType,
              completed: Bool,
              _items: *mut NSArray,
              error: *mut NSError| {
            if let Some(value) = state(id) {
                if let Some(error) = unsafe { error.as_ref() } {
                    finish(
                        &value,
                        "failed",
                        format!(
                            "The export was not completed: {}",
                            error.localizedDescription()
                        ),
                        None,
                    );
                } else if completed.as_bool() {
                    finish(
                        &value,
                        "shared",
                        "Export completed in the app you chose.".into(),
                        None,
                    );
                } else {
                    finish(
                        &value,
                        "cancelled",
                        "Export cancelled. Your original is kept.".into(),
                        None,
                    );
                }
            }
        },
    );
    unsafe {
        activity.setCompletionWithItemsHandler(RcBlock::as_ptr(&completion));
    }
    if let Some(popover) = activity.popoverPresentationController() {
        popover.setSourceView(Some(&webview));
        let mut rect = webview.bounds();
        rect.origin.x += rect.size.width * 0.5;
        rect.origin.y += rect.size.height * 0.5;
        rect.size.width = 1.;
        rect.size.height = 1.;
        popover.setSourceRect(rect);
    }
    controller.presentViewController_animated_completion(&activity, true, None);
    value.activity.replace(Some(activity));
}
