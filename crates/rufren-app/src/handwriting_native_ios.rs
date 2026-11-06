//! Main-thread PencilKit renderer. Rust owns activities, cue policy and persistence.
use super::*;
#[path = "pencilkit_bindings.rs"]
mod pk;
use block2::RcBlock;
use dioxus::mobile::{self as native_runtime, wry::WebViewExtIOS};
use objc2::{
    define_class, msg_send, rc::Retained, runtime::ProtocolObject, sel, AnyThread, DeclaredClass,
    MainThreadOnly,
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{
    MainThreadMarker, NSData, NSNotification, NSNotificationCenter, NSObjectProtocol,
    NSOperationQueue, NSRunLoop, NSRunLoopCommonModes, NSString, NSTimer,
};
use objc2_pencil_kit::PKDrawing;
use objc2_ui_kit::{
    UIApplicationWillResignActiveNotification, UIButton, UIButtonType, UIColor, UIControlEvents,
    UIControlState, UIFont, UILabel, UIModalPresentationStyle, UIResponder, UIScrollViewDelegate,
    UIView, UIViewController,
};
use pk::*;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, VecDeque},
    time::{SystemTime, UNIX_EPOCH},
};

struct Pending {
    reason: NativeCheckpointReason,
    checkpoint_id: Uuid,
    archive_sha256: String,
}
struct State {
    request: NativeWritingRequest,
    outbox: Rc<EventQueue>,
    controller: Retained<WritingController>,
    canvas: Retained<PKCanvasView>,
    picker: Retained<PKToolPicker>,
    title: Retained<UILabel>,
    prompt: Retained<UILabel>,
    model: Retained<UILabel>,
    status: Retained<UILabel>,
    buttons: Vec<Retained<UIButton>>,
    player: RefCell<Option<Retained<AVAudioPlayer>>>,
    sequence: Cell<u64>,
    latest_ack: Cell<u64>,
    pending: RefCell<BTreeMap<u64, Pending>>,
    latest: RefCell<Option<NativeWritingCheckpoint>>,
    tool_active: Cell<bool>,
    requested: Cell<Option<NativeCheckpointReason>>,
    scheduled: Cell<bool>,
    timer: RefCell<Option<Retained<NSTimer>>>,
    timer_active_mode: Cell<bool>,
    comparison: Cell<bool>,
    compared: RefCell<Option<(Uuid, u64)>>,
    finger: Cell<bool>,
    model_visible: Cell<bool>,
    closing: Cell<bool>,
}
thread_local! {static ACTIVE:RefCell<Option<Rc<State>>>=const{RefCell::new(None)};}
fn state(id: Uuid) -> Option<Rc<State>> {
    ACTIVE.with(|a| {
        a.borrow()
            .as_ref()
            .filter(|s| s.request.instance_id == id)
            .cloned()
    })
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}
fn later(f: impl Fn() + 'static) {
    let b = RcBlock::new(f);
    unsafe {
        NSOperationQueue::mainQueue().addOperationWithBlock(&b);
    }
}

/// Read-only rendering for a selected history original. Never attach to a
/// canvas, reproject control points, archive, journal, or emit writing events.
pub(crate) fn native_drawing_preview(
    checkpoint: &NativeWritingCheckpoint,
) -> NativeWritingResult<Option<String>> {
    let mtm = MainThreadMarker::new()
        .ok_or("View the saved native drawing from the app's main window.")?;
    checkpoint.validate()?;
    objc2::rc::autoreleasepool(|_| {
        let data = NSData::with_bytes(&checkpoint.archive()?);
        let drawing = unsafe { PKDrawing::initWithData_error(PKDrawing::alloc(), &data) }
            .map_err(|_| "The saved PencilKit archive could not be rendered.")?;
        // Empty drawings may have CGRectNull bounds. The recorded paper is
        // still a meaningful blank preview; nonempty drawings must have valid
        // finite rendered bounds before any bitmap allocation.
        let bounds = if unsafe { drawing.strokes().is_empty() } {
            None
        } else {
            let bounds = unsafe { drawing.bounds() };
            if bounds.size.width == 0.0 || bounds.size.height == 0.0 {
                // A fully masked drawing also has no rendered area.
                None
            } else {
                Some([
                    bounds.origin.x,
                    bounds.origin.y,
                    bounds.size.width,
                    bounds.size.height,
                ])
            }
        };
        let ([x, y, width, height], scale) =
            native_preview_geometry(&checkpoint.canonical.paper, bounds)?;
        let image = unsafe {
            drawing_image(
                &drawing,
                CGRect::new(CGPoint::new(x, y), CGSize::new(width, height)),
                scale,
                mtm,
            )
        };
        let png = image
            .png_representation()
            .ok_or("The native drawing preview could not be encoded as PNG.")?;
        // The raster is bounded above; separately bound the encoded transport
        // before copying it into the transient WebView image source.
        if png.is_empty() || png.len() > 12 * 1024 * 1024 {
            return Err("The native drawing preview exceeds its image byte bound.".into());
        }
        Ok(Some(format!(
            "data:image/png;base64,{}",
            STANDARD.encode(png.to_vec())
        )))
    })
}

struct EventQueue {
    callback: NativeWritingCallback,
    events: RefCell<VecDeque<NativeWritingEvent>>,
    scheduled: Cell<bool>,
}
fn emit(s: &State, event: NativeWritingEvent) {
    // FIFO and nonrecursive: persistence may acknowledge a checkpoint from inside
    // its Dioxus callback. ACK-generated events run only after that callback returns.
    let queue = s.outbox.clone();
    queue.events.borrow_mut().push_back(event);
    if queue.scheduled.replace(true) {
        return;
    }
    later(move || loop {
        let event = queue.events.borrow_mut().pop_front();
        match event {
            Some(event) => (queue.callback)(event),
            None => {
                queue.scheduled.set(false);
                break;
            }
        }
    });
}

fn status(s: &State, text: &str) {
    s.status.setText(Some(&NSString::from_str(text)));
}
fn fail(s: &State, message: String) {
    status(s, &message);
    emit(
        s,
        NativeWritingEvent::Failure {
            instance_id: s.request.instance_id,
            message,
        },
    );
}
struct Ivars {
    id: Uuid,
}
define_class!(
    #[unsafe(super(UIViewController,UIResponder,objc2_foundation::NSObject))]
    #[name="RufrenPencilWritingController"]
    #[thread_kind=MainThreadOnly]
    #[ivars=Ivars]
    struct WritingController;
    unsafe impl NSObjectProtocol for WritingController {}
    unsafe impl UIScrollViewDelegate for WritingController {}
    unsafe impl PKCanvasViewDelegate for WritingController {
        #[unsafe(method(canvasViewDrawingDidChange:))]
        fn drawing_changed(&self,_canvas:&PKCanvasView) {
            if let Some(s)=state(self.ivars().id) {if s.closing.get(){checkpoint(&s,NativeCheckpointReason::Autosave);}else{schedule(&s);}}
        }
        #[unsafe(method(canvasViewDidBeginUsingTool:))]
        fn began(&self,_canvas:&PKCanvasView) {if let Some(s)=state(self.ivars().id){s.tool_active.set(true);}}
        #[unsafe(method(canvasViewDidEndUsingTool:))]
        fn ended(&self,_canvas:&PKCanvasView) {if let Some(s)=state(self.ivars().id){s.tool_active.set(false);if s.requested.get().is_some(){s.canvas.drawingGestureRecognizer().setEnabled(false);}schedule(&s);}}
    }
    unsafe impl AVAudioPlayerDelegate for WritingController {
        #[unsafe(method(audioPlayerDidFinishPlaying:successfully:))]
        fn audio_ended(&self,_player:&AVAudioPlayer,success:bool) {
            if let Some(s)=state(self.ivars().id) {
                if let Some(a)=&s.request.audio {
                    if success {emit(&s,NativeWritingEvent::AudioCompleted{instance_id:s.request.instance_id,audio_id:a.id.clone(),sha256:a.sha256.clone(),at_ms:now()});}
                    else {emit(&s,NativeWritingEvent::AudioStopped{instance_id:s.request.instance_id,audio_id:a.id.clone(),at_ms:now()});}
                }
            }
        }
        #[unsafe(method(audioPlayerDecodeErrorDidOccur:error:))]
        fn audio_error(&self,_player:&AVAudioPlayer,_error:Option<&objc2_foundation::NSError>) {
            if let Some(s)=state(self.ivars().id) {fail(&s,"The spoken prompt could not finish playing. Your writing is kept.".into());}
        }
    }
    impl WritingController {
        #[unsafe(method(writingWillResignActive:))]
        fn will_resign(&self,_note:&NSNotification) {
            if let Some(s)=state(self.ivars().id) {if !s.closing.get(){cancel_timer(&s);checkpoint(&s,NativeCheckpointReason::Autosave);}}
        }
        #[unsafe(method(viewDidLayoutSubviews))]
        fn laid_out(&self) {
            unsafe {let _:()=msg_send![super(self),viewDidLayoutSubviews];}
            if let Some(s)=state(self.ivars().id) {layout(&s);}
        }
        #[unsafe(method(back:))]
        fn back(&self,_sender:&UIButton) {if let Some(s)=state(self.ivars().id){request_checkpoint(&s,NativeCheckpointReason::Back);}}
        #[unsafe(method(listen:))]
        fn listen(&self,_sender:&UIButton) {if let Some(s)=state(self.ivars().id){listen(&s);}}
        #[unsafe(method(reveal:))]
        fn reveal(&self,_sender:&UIButton) {if let Some(s)=state(self.ivars().id){if s.request.allow_model_reveal&&!s.model_visible.get(){reveal(&s);}}}
        #[unsafe(method(undoInk:))]
        fn undo_ink(&self,_sender:&UIButton) {if let Some(s)=state(self.ivars().id){if !s.comparison.get()&&!s.tool_active.get(){if let Some(u)=s.canvas.undoManager(){unsafe{u.undo();}}}}}
        #[unsafe(method(redoInk:))]
        fn redo_ink(&self,_sender:&UIButton) {if let Some(s)=state(self.ivars().id){if !s.comparison.get()&&!s.tool_active.get(){if let Some(u)=s.canvas.undoManager(){unsafe{u.redo();}}}}}
        #[unsafe(method(fingerInk:))]
        fn finger_ink(&self,_sender:&UIButton) {if let Some(s)=state(self.ivars().id){if !s.comparison.get(){let finger=!s.finger.get();s.finger.set(finger);s.canvas.setDrawingPolicy(if finger{1}else{2});s.buttons[5].setTitle_forState(Some(&NSString::from_str(if finger{"Finger: on"}else{"Pencil only"})),UIControlState::Normal);}}}
        #[unsafe(method(compareInk:))]
        fn compare_ink(&self,_sender:&UIButton) {if let Some(s)=state(self.ivars().id){if s.comparison.get(){continue_speaking(&s);}else{request_checkpoint(&s,NativeCheckpointReason::Compare);}}}
    }
);
impl WritingController {
    fn new(id: Uuid, mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(Ivars { id });
        unsafe { msg_send![super(this), init] }
    }
}
fn label(mtm: MainThreadMarker, size: f64) -> Retained<UILabel> {
    let l = UILabel::init(mtm.alloc());
    l.setNumberOfLines(0);
    unsafe {
        l.setFont(Some(&UIFont::systemFontOfSize(size)));
    }
    unsafe {
        l.setTextColor(Some(&UIColor::labelColor()));
    }
    l
}
fn button(
    controller: &WritingController,
    title: &str,
    selector: objc2::runtime::Sel,
    mtm: MainThreadMarker,
) -> Retained<UIButton> {
    let b = UIButton::buttonWithType(UIButtonType::System, mtm);
    b.setTitle_forState(Some(&NSString::from_str(title)), UIControlState::Normal);
    unsafe {
        b.addTarget_action_forControlEvents(
            Some(controller),
            selector,
            UIControlEvents::TouchUpInside,
        );
    }
    b
}
fn rect(x: f64, y: f64, w: f64, h: f64) -> CGRect {
    CGRect::new(CGPoint::new(x, y), CGSize::new(w, h))
}
fn layout(s: &State) {
    let Some(view) = s.controller.view() else {
        return;
    };
    let bounds = view.bounds();
    let safe = view.safeAreaInsets();
    let x = safe.left + 16.0;
    let w = (bounds.size.width - safe.left - safe.right - 32.0).max(240.0);
    let top = safe.top + 4.0;
    s.buttons[0].setFrame(rect(x, top, 76.0, 44.0));
    s.title
        .setFrame(rect(x + 88.0, top, (w - 184.0).max(80.0), 44.0));
    s.buttons[1].setFrame(rect(x + w - 92.0, top, 92.0, 44.0));
    s.prompt.setFrame(rect(x, top + 48.0, w, 58.0));
    s.model.setFrame(rect(x, top + 110.0, w - 108.0, 88.0));
    s.buttons[2].setFrame(rect(x + w - 104.0, top + 114.0, 104.0, 44.0));
    let bottom = bounds.size.height - safe.bottom - 12.0;
    s.buttons[6].setFrame(rect(x, bottom - 52.0, w, 52.0));
    s.status.setFrame(rect(x, bottom - 98.0, w, 42.0));
    s.buttons[3].setFrame(rect(x, bottom - 142.0, 76.0, 44.0));
    s.buttons[4].setFrame(rect(x + 80.0, bottom - 142.0, 76.0, 44.0));
    s.buttons[5].setFrame(rect(x + w - 136.0, bottom - 142.0, 136.0, 44.0));
    let canvas_y = top + 206.0;
    s.canvas
        .setFrame(rect(x, canvas_y, w, (bottom - 148.0 - canvas_y).max(100.0)));
}

fn reveal(s: &State) {
    s.model_visible.set(true);
    s.model.setText(Some(&NSString::from_str(&model_text(s))));
    emit(
        s,
        NativeWritingEvent::ModelRevealed {
            instance_id: s.request.instance_id,
            at_ms: now(),
        },
    );
}
fn model_text(s: &State) -> String {
    if s.model_visible.get() {
        match &s.request.meaning {
            Some(m) => format!("Printed example: {}\n{}", s.request.model_text, m),
            None => format!("Printed example: {}", s.request.model_text),
        }
    } else {
        "Write what you hear. You can show the model if you need it.".into()
    }
}
fn stop_audio(s: &State) {
    if let Some(p) = s.player.borrow().as_ref() {
        if p.isPlaying() {
            p.stop();
            if let Some(a) = &s.request.audio {
                emit(
                    s,
                    NativeWritingEvent::AudioStopped {
                        instance_id: s.request.instance_id,
                        audio_id: a.id.clone(),
                        at_ms: now(),
                    },
                );
            }
        }
    }
}
fn listen(s: &Rc<State>) {
    if !s.request.allow_listen {
        return;
    }
    let Some(a) = &s.request.audio else {
        return;
    };
    stop_audio(s);
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let data = NSData::with_bytes(&a.wav_bytes);
    match AVAudioPlayer::initWithData_error(mtm.alloc(), &data) {
        Ok(player) => {
            player.setDelegate(Some(ProtocolObject::from_ref(&*s.controller)));
            if player.play() {
                s.player.replace(Some(player));
                emit(
                    s,
                    NativeWritingEvent::AudioStarted {
                        instance_id: s.request.instance_id,
                        audio_id: a.id.clone(),
                        sha256: a.sha256.clone(),
                        at_ms: now(),
                    },
                );
            } else {
                fail(
                    s,
                    "The spoken prompt did not start. Your writing is kept.".into(),
                );
            }
        }
        Err(_) => fail(
            s,
            "The exact spoken-prompt WAV could not be opened. Your writing is kept.".into(),
        ),
    }
}
fn request_checkpoint(s: &Rc<State>, reason: NativeCheckpointReason) {
    if s.closing.get() {
        return;
    }
    s.requested.set(Some(reason));
    if s.tool_active.get() {
        status(s, "Finish this stroke; your writing will then be saved.");
        return;
    }
    s.canvas.drawingGestureRecognizer().setEnabled(false);
    status(s, "Saving your writing…");
    schedule(s);
}
fn cancel_timer(s: &State) {
    if let Some(timer) = s.timer.borrow_mut().take() {
        timer.invalidate();
    }
}
fn schedule(s: &Rc<State>) {
    if s.closing.get() {
        return;
    }
    let interval = native_checkpoint_interval(s.tool_active.get(), s.requested.get().is_some());
    let id = s.request.instance_id;
    if interval == 0.0 {
        cancel_timer(s);
        if s.scheduled.replace(true) {
            return;
        }
        later(move || {
            if let Some(s) = state(id) {
                s.scheduled.set(false);
                if s.closing.get() || s.tool_active.get() {
                    return;
                }
                let reason = s
                    .requested
                    .get()
                    .unwrap_or(NativeCheckpointReason::Autosave);
                checkpoint(&s, reason);
            }
        });
        return;
    }
    // An end-of-stroke batch supersedes a slower long-stroke checkpoint. Further
    // changes join that batch; they cannot keep pushing a trailing timer away.
    if s.timer_active_mode.get() && !s.tool_active.get() {
        cancel_timer(s);
    }
    if s.timer.borrow().is_some() {
        return;
    }
    s.timer_active_mode.set(s.tool_active.get());
    let block = RcBlock::new(move |_timer: std::ptr::NonNull<NSTimer>| {
        if let Some(s) = state(id) {
            s.timer.borrow_mut().take();
            if s.closing.get() {
                return;
            }
            let reason = if s.tool_active.get() {
                NativeCheckpointReason::Autosave
            } else {
                s.requested
                    .get()
                    .unwrap_or(NativeCheckpointReason::Autosave)
            };
            checkpoint(&s, reason);
        }
    });
    let timer = unsafe { NSTimer::timerWithTimeInterval_repeats_block(interval, false, &block) };
    unsafe {
        NSRunLoop::mainRunLoop().addTimer_forMode(&timer, NSRunLoopCommonModes);
    }
    s.timer.replace(Some(timer));
}

fn projection(s: &State, d: &PKDrawing) -> NativeWritingResult<NativeInkProjection> {
    let native = unsafe { d.strokes() };
    if native.len() > 1000 {
        return Err(
            "This drawing has too many strokes to save safely. Keep the paper open.".into(),
        );
    }
    let mut strokes = Vec::with_capacity(native.len());
    let mut total = 0usize;
    for stroke in native.iter() {
        let path = unsafe { stroke.path() };
        let n = unsafe { path.count() };
        total = total.checked_add(n).ok_or("Too many native ink points.")?;
        if n == 0 || total > 200_000 {
            return Err(
                "This drawing has too many points to save safely. Keep the paper open.".into(),
            );
        }
        let transform = unsafe { stroke.transform() };
        let ink = unsafe { stroke.ink() };
        let color = unsafe { ink_color(&ink) };
        let (mut r, mut g, mut b, mut a) = (0.0, 0.0, 0.0, 1.0);
        let got: bool =
            unsafe { msg_send![&*color,getRed:&mut r,green:&mut g,blue:&mut b,alpha:&mut a] };
        if !got {
            return Err("This native ink color could not be preserved safely.".into());
        }
        let ranges = unsafe { stroke.maskedPathRanges() };
        let mut visible_ranges = Vec::with_capacity(ranges.len());
        for range in ranges.iter() {
            visible_ranges.push([unsafe { range.lowerBound() }, unsafe { range.upperBound() }]);
        }
        let mut points = Vec::with_capacity(n);
        for i in 0..n {
            let p = unsafe { path.pointAtIndex(i) };
            let loc = unsafe { p.location() };
            let size = unsafe { p.size() };
            points.push(NativeInkPoint {
                x: loc.x,
                y: loc.y,
                time_offset_seconds: unsafe { p.timeOffset() },
                width: size.width,
                height: size.height,
                force: unsafe { p.force() },
                opacity: unsafe { p.opacity() },
                azimuth_radians: unsafe { p.azimuth() },
                altitude_radians: unsafe { p.altitude() },
            });
        }
        strokes.push(NativeInkStroke {
            creation_unix_ms: unsafe { path.creationDate().timeIntervalSince1970() } * 1000.0,
            transform: [
                transform.a,
                transform.b,
                transform.c,
                transform.d,
                transform.tx,
                transform.ty,
            ],
            ink_type: unsafe { ink_type(&ink) }.to_string(),
            color_rgba: [r, g, b, a],
            visible_ranges,
            points,
        });
    }
    let result = NativeInkProjection {
        format: "pencilkit-control-points-v1".into(),
        paper: s.request.paper.clone(),
        strokes,
    };
    result.validate()?;
    Ok(result)
}
fn checkpoint(s: &Rc<State>, reason: NativeCheckpointReason) {
    let drawing = s.canvas.drawing();
    if reason == NativeCheckpointReason::Compare && unsafe { drawing.strokes().is_empty() } {
        s.requested.set(None);
        s.canvas.drawingGestureRecognizer().setEnabled(true);
        status(s, "Write a word or phrase before comparing.");
        return;
    }
    let archive = unsafe { drawing.dataRepresentation() };
    if archive.len() > MAX_NATIVE_ARCHIVE_BYTES {
        fail(
            s,
            "This native drawing is too large to save safely. Keep the paper open.".into(),
        );
        return;
    }
    let bytes = archive.to_vec();
    let archive_sha256 = hash(&bytes);
    if reason == NativeCheckpointReason::Autosave
        && s.latest
            .borrow()
            .as_ref()
            .is_some_and(|old| old.archive_sha256 == archive_sha256)
    {
        return;
    }
    let canonical = match projection(s, &drawing) {
        Ok(v) => v,
        Err(e) => {
            fail(s, e);
            return;
        }
    };
    let Some(seq) = s.sequence.get().checked_add(1) else {
        fail(
            s,
            "This writing session has too many revisions. Keep the paper open.".into(),
        );
        return;
    };
    s.sequence.set(seq);
    let canonical_bytes = match serde_json::to_vec(&canonical) {
        Ok(b) => b,
        Err(_) => {
            fail(s, "The native ink could not be encoded.".into());
            return;
        }
    };
    let offset = s.canvas.contentOffset();
    let c = NativeWritingCheckpoint {
        version: 1,
        checkpoint_id: Uuid::new_v4(),
        instance_id: s.request.instance_id,
        scope_id: s.request.scope_id.clone(),
        attempt_id: s.request.attempt_id,
        task_id: s.request.task_id.clone(),
        task_revision: s.request.task_revision.clone(),
        sequence: seq,
        captured_at_ms: now(),
        archive_base64: STANDARD.encode(&bytes),
        archive_sha256,
        canonical,
        canonical_sha256: hash(&canonical_bytes),
        zoom_scale: s.canvas.zoomScale(),
        content_offset: [offset.x, offset.y],
    };
    if let Err(e) = c.validate() {
        fail(s, e);
        return;
    }
    if let Err(e) = journal_native(&s.request, &c) {
        fail(
            s,
            format!("The native recovery could not be saved: {e}. Keep the paper open."),
        );
        return;
    }
    s.latest.replace(Some(c.clone()));
    s.pending.borrow_mut().insert(
        seq,
        Pending {
            reason,
            checkpoint_id: c.checkpoint_id,
            archive_sha256: c.archive_sha256.clone(),
        },
    );
    // Coalesce obsolete autosave receipts; a sequence can never acknowledge newer ink.
    s.pending
        .borrow_mut()
        .retain(|k, _| *k >= seq.saturating_sub(32));
    emit(
        s,
        NativeWritingEvent::Checkpoint {
            reason,
            checkpoint: c,
        },
    );
}
pub(crate) fn ack(
    instance: Uuid,
    sequence: u64,
    ok: bool,
    error: Option<String>,
) -> NativeWritingResult<()> {
    let s = state(instance).ok_or("The native writing controller is no longer active.")?;
    if MainThreadMarker::new().is_none() {
        return Err("A native ink acknowledgment must run on the main thread.".into());
    }
    let Some(pending) = s.pending.borrow_mut().remove(&sequence) else {
        return Err("This native ink acknowledgment is stale or duplicated.".into());
    };
    if !ok {
        if sequence != s.sequence.get() {
            return Ok(());
        }
        status(
            &s,
            error
                .as_deref()
                .unwrap_or("Your writing still needs saving. Keep the paper open and retry."),
        );
        s.requested.set(None);
        if !s.comparison.get() {
            s.canvas.drawingGestureRecognizer().setEnabled(true);
        }
        return Ok(());
    }
    s.latest_ack.set(s.latest_ack.get().max(sequence));
    if sequence != s.sequence.get() {
        return Ok(());
    }
    match pending.reason {
        NativeCheckpointReason::Autosave => status(
            &s,
            if s.comparison.get() {
                &s.request.comparison_hint
            } else {
                "Writing saved. Pinch to zoom; use two fingers to move the paper."
            },
        ),
        NativeCheckpointReason::Compare => {
            s.requested.set(None);
            s.comparison.set(true);
            s.picker.setVisible_forFirstResponder(false, &s.canvas);
            for i in [2, 3, 4, 5] {
                s.buttons[i].setEnabled(false);
            }
            s.compared.replace(Some((pending.checkpoint_id, sequence)));
            s.canvas.drawingGestureRecognizer().setEnabled(false);
            if !s.model_visible.get() {
                reveal(&s);
            }
            s.buttons[6].setTitle_forState(
                Some(&NSString::from_str("Continue to speaking")),
                UIControlState::Normal,
            );
            status(&s, &s.request.comparison_hint);
            emit(
                &s,
                NativeWritingEvent::CompareReady {
                    instance_id: instance,
                    checkpoint_id: pending.checkpoint_id,
                    sequence,
                    archive_sha256: pending.archive_sha256,
                },
            );
        }
        NativeCheckpointReason::Back => {
            s.requested.set(None);
            emit(
                &s,
                NativeWritingEvent::Paused {
                    instance_id: instance,
                    checkpoint_id: pending.checkpoint_id,
                    sequence,
                },
            );
            dismiss(&s);
        }
    }
    Ok(())
}
fn continue_speaking(s: &Rc<State>) {
    if s.latest_ack.get() != s.sequence.get() {
        status(
            s,
            "Your latest ink still needs saving. Keep the paper open.",
        );
        return;
    }
    let latest = *s.compared.borrow();
    if let Some((checkpoint_id, sequence)) = latest {
        emit(
            s,
            NativeWritingEvent::ContinueToSpeaking {
                instance_id: s.request.instance_id,
                checkpoint_id,
                sequence,
            },
        );
        dismiss(s);
    }
}
fn dismiss(s: &Rc<State>) {
    if s.closing.replace(true) {
        return;
    }
    stop_audio(s);
    cancel_timer(s);
    unsafe {
        NSNotificationCenter::defaultCenter().removeObserver(&s.controller);
    }
    s.picker.setVisible_forFirstResponder(false, &s.canvas);
    s.picker.removeCanvasObserver(&s.canvas);
    let id = s.request.instance_id;
    // Retain the state until UIKit's actual dismissal completion, never tear down
    // a delegate from inside its own callback.
    let completion = RcBlock::new(move || {
        if let Some(s) = state(id) {
            emit(&s, NativeWritingEvent::Closed { instance_id: id });
        }
        later(move || {
            ACTIVE.with(|a| {
                if a.borrow()
                    .as_ref()
                    .is_some_and(|s| s.request.instance_id == id)
                {
                    a.borrow_mut().take();
                }
            });
        });
    });
    s.controller
        .dismissViewControllerAnimated_completion(true, Some(&completion));
}
pub(crate) fn open_native(
    mut request: NativeWritingRequest,
    callback: NativeWritingCallback,
) -> NativeWritingResult<()> {
    request.validate()?;
    if !request.initial_compared {
        if let Some(c) = native_recovery(&request)? {
            if request
                .initial
                .as_ref()
                .is_none_or(|old| c.sequence > old.sequence)
            {
                request.initial = Some(c);
            }
        }
    }
    let mtm = MainThreadMarker::new().ok_or("Open native writing from the app's main window.")?;
    if ACTIVE.with(|a| a.borrow().is_some()) {
        return Err("Finish or pause the current native writing activity first.".into());
    }
    let window = native_runtime::window();
    let webview = window.webview.webview();
    let presenter = webview
        .window()
        .and_then(|w| w.rootViewController())
        .ok_or("The app window is not ready for native paper.")?;
    if presenter.presentedViewController().is_some() {
        return Err("Close the current dialog before opening native paper.".into());
    }
    let controller = WritingController::new(request.instance_id, mtm);
    controller.setModalPresentationStyle(UIModalPresentationStyle::FullScreen);
    controller.setModalInPresentation(true);
    let view = controller
        .view()
        .ok_or("The native writing view did not load.")?;
    view.setBackgroundColor(Some(&UIColor::systemBackgroundColor()));
    let canvas = PKCanvasView::init(mtm.alloc());
    canvas.setBackgroundColor(Some(&UIColor::secondarySystemBackgroundColor()));
    canvas.setDrawingPolicy(2);
    canvas.setContentSize(CGSize::new(
        request.paper.width_points,
        request.paper.height_points,
    ));
    canvas.setMinimumZoomScale(0.25);
    canvas.setMaximumZoomScale(4.0);
    canvas.setBouncesZoom(true);
    if let Some(c) = &request.initial {
        let data = NSData::with_bytes(&c.archive()?);
        let drawing = unsafe { PKDrawing::initWithData_error(PKDrawing::alloc(), &data) }
            .map_err(|_| "The saved PencilKit archive could not be reopened.")?;
        canvas.setDrawing(&drawing);
        canvas.setZoomScale(c.zoom_scale);
        canvas.setContentOffset(CGPoint::new(c.content_offset[0], c.content_offset[1]));
    }
    let title = label(mtm, 22.0);
    title.setText(Some(&NSString::from_str("Write")));
    let prompt = label(mtm, 19.0);
    prompt.setText(Some(&NSString::from_str(&request.prompt)));
    let model = label(mtm, 26.0);
    let status_label = label(mtm, 15.0);
    let buttons = vec![
        button(&controller, "Back", sel!(back:), mtm),
        button(&controller, "Listen", sel!(listen:), mtm),
        button(&controller, "Show model", sel!(reveal:), mtm),
        button(&controller, "Undo", sel!(undoInk:), mtm),
        button(&controller, "Redo", sel!(redoInk:), mtm),
        button(&controller, "Pencil only", sel!(fingerInk:), mtm),
        button(&controller, "Save and compare", sel!(compareInk:), mtm),
    ];
    buttons[6].setBackgroundColor(Some(&UIColor::systemBlueColor()));
    buttons[6].setTitleColor_forState(Some(&UIColor::whiteColor()), UIControlState::Normal);
    buttons[1].setEnabled(request.allow_listen && request.audio.is_some());
    buttons[2].setEnabled(request.allow_model_reveal && !request.model_visible);
    for v in [
        &*title as &UIView,
        &*prompt as &UIView,
        &*model as &UIView,
        &*status_label as &UIView,
        &*canvas as &UIView,
    ] {
        view.addSubview(v);
    }
    for b in &buttons {
        view.addSubview(b);
    }
    let picker = PKToolPicker::init(mtm.alloc());
    picker.addCanvasObserver(&canvas);
    let initial_sequence = request.initial.as_ref().map_or(0, |c| c.sequence);
    let visible = request.model_visible || request.initial_compared;
    let compared = request.initial_compared;
    let original = if compared {
        request
            .initial
            .as_ref()
            .map(|c| (c.checkpoint_id, c.sequence))
    } else {
        None
    };
    let latest_initial = request.initial.clone();
    let s = Rc::new(State {
        request,
        outbox: Rc::new(EventQueue {
            callback,
            events: RefCell::new(VecDeque::new()),
            scheduled: Cell::new(false),
        }),
        controller,
        canvas,
        picker,
        title,
        prompt,
        model,
        status: status_label,
        buttons,
        player: RefCell::new(None),
        sequence: Cell::new(initial_sequence),
        latest_ack: Cell::new(initial_sequence),
        pending: RefCell::new(BTreeMap::new()),
        latest: RefCell::new(latest_initial),
        tool_active: Cell::new(false),
        requested: Cell::new(None),
        scheduled: Cell::new(false),
        timer: RefCell::new(None),
        timer_active_mode: Cell::new(false),
        comparison: Cell::new(compared),
        compared: RefCell::new(original),
        finger: Cell::new(false),
        model_visible: Cell::new(visible),
        closing: Cell::new(false),
    });
    if compared {
        s.canvas.drawingGestureRecognizer().setEnabled(false);
        s.buttons[6].setTitle_forState(
            Some(&NSString::from_str("Continue to speaking")),
            UIControlState::Normal,
        );
        for i in [2, 3, 4, 5] {
            s.buttons[i].setEnabled(false);
        }
    }
    s.model.setText(Some(&NSString::from_str(&model_text(&s))));
    status(
        &s,
        if compared {
            &s.request.comparison_hint
        } else {
            "Write a whole word or phrase. Pinch to zoom; use two fingers to move."
        },
    );
    s.canvas
        .setDrawingDelegate(Some(ProtocolObject::from_ref(&*s.controller)));
    ACTIVE.with(|a| a.replace(Some(s.clone())));
    unsafe {
        NSNotificationCenter::defaultCenter().addObserver_selector_name_object(
            &s.controller,
            sel!(writingWillResignActive:),
            Some(UIApplicationWillResignActiveNotification),
            None,
        );
    }
    layout(&s);
    let id = s.request.instance_id;
    let completion = RcBlock::new(move || {
        if let Some(s) = state(id) {
            s.canvas.becomeFirstResponder();
            if !s.comparison.get() {
                s.picker.setVisible_forFirstResponder(true, &s.canvas);
            }
            emit(&s, NativeWritingEvent::Ready { instance_id: id });
        }
    });
    presenter.presentViewController_animated_completion(&s.controller, true, Some(&completion));
    Ok(())
}

/// Navigation must wait for Paused, not treat an absent browser pad as saved.
pub(crate) fn is_open() -> bool {
    ACTIVE.with(|a| a.borrow().is_some())
}
pub(crate) fn request_pause(instance: Uuid) -> NativeWritingResult<()> {
    if MainThreadMarker::new().is_none() {
        return Err("Pause native writing on the main thread.".into());
    }
    let s = state(instance).ok_or("The native writing controller is no longer active.")?;
    request_checkpoint(&s, NativeCheckpointReason::Back);
    Ok(())
}
