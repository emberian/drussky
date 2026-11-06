//! iOS-only public selectors omitted by objc2-pencil-kit0.3.2's generated bindings.
//! Match PencilKit/UIKit/AVFAudio SDK headers; no private classes or selectors.
use objc2::{
    extern_class, extern_methods, extern_protocol,
    rc::{Allocated, Retained},
    runtime::ProtocolObject,
    MainThreadOnly,
};
use objc2_foundation::{NSData, NSError, NSObject, NSObjectProtocol, NSString};
use objc2_pencil_kit::{PKDrawing, PKInk, PKTool};
use objc2_ui_kit::{UIColor, UIResponder, UIScrollView, UIView};

#[link(name = "PencilKit", kind = "framework")]
unsafe extern "C" {}
#[link(name = "AVFAudio", kind = "framework")]
unsafe extern "C" {}
extern_protocol!(
    pub unsafe trait PKCanvasViewDelegate: NSObjectProtocol + MainThreadOnly {
        #[optional]
        #[unsafe(method(canvasViewDrawingDidChange:))]
        #[unsafe(method_family=none)]
        fn canvasViewDrawingDidChange(&self, canvas: &PKCanvasView);
        #[optional]
        #[unsafe(method(canvasViewDidBeginUsingTool:))]
        #[unsafe(method_family=none)]
        fn canvasViewDidBeginUsingTool(&self, canvas: &PKCanvasView);
        #[optional]
        #[unsafe(method(canvasViewDidEndUsingTool:))]
        #[unsafe(method_family=none)]
        fn canvasViewDidEndUsingTool(&self, canvas: &PKCanvasView);
    }
);
extern_class!(
    #[unsafe(super(UIScrollView, UIView, UIResponder, NSObject))]
    #[thread_kind=MainThreadOnly]
    #[name = "PKCanvasView"]
    pub struct PKCanvasView;
);
impl PKCanvasView {
    extern_methods!(
        #[unsafe(method(init))]
        #[unsafe(method_family=init)]
        pub fn init(this: Allocated<Self>) -> Retained<Self>;
        #[unsafe(method(drawing))]
        #[unsafe(method_family=none)]
        pub fn drawing(&self) -> Retained<PKDrawing>;
        #[unsafe(method(setDrawing:))]
        #[unsafe(method_family=none)]
        pub fn setDrawing(&self, drawing: &PKDrawing);
        #[unsafe(method(setDelegate:))]
        #[unsafe(method_family=none)]
        pub fn setDrawingDelegate(
            &self,
            delegate: Option<&ProtocolObject<dyn PKCanvasViewDelegate>>,
        );
        #[unsafe(method(setDrawingPolicy:))]
        #[unsafe(method_family=none)]
        pub fn setDrawingPolicy(&self, policy: usize);
        #[unsafe(method(setTool:))]
        #[unsafe(method_family=none)]
        pub fn setTool(&self, tool: &PKTool);
        #[unsafe(method(drawingGestureRecognizer))]
        #[unsafe(method_family=none)]
        pub fn drawingGestureRecognizer(&self) -> Retained<objc2_ui_kit::UIGestureRecognizer>;
    );
}
extern_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind=MainThreadOnly]
    #[name = "PKToolPicker"]
    pub struct PKToolPicker;
);
impl PKToolPicker {
    extern_methods!(
        #[unsafe(method(init))]
        #[unsafe(method_family=init)]
        pub fn init(this: Allocated<Self>) -> Retained<Self>;
        #[unsafe(method(addObserver:))]
        #[unsafe(method_family=none)]
        pub fn addCanvasObserver(&self, canvas: &PKCanvasView);
        #[unsafe(method(removeObserver:))]
        #[unsafe(method_family=none)]
        pub fn removeCanvasObserver(&self, canvas: &PKCanvasView);
        #[unsafe(method(setVisible:forFirstResponder:))]
        #[unsafe(method_family=none)]
        pub fn setVisible_forFirstResponder(&self, visible: bool, responder: &UIResponder);
    );
}
extern_protocol!(
    pub unsafe trait AVAudioPlayerDelegate: NSObjectProtocol + MainThreadOnly {
        #[optional]
        #[unsafe(method(audioPlayerDidFinishPlaying:successfully:))]
        #[unsafe(method_family=none)]
        fn audioPlayerDidFinishPlaying_successfully(&self, player: &AVAudioPlayer, success: bool);
        #[optional]
        #[unsafe(method(audioPlayerDecodeErrorDidOccur:error:))]
        #[unsafe(method_family=none)]
        fn audioPlayerDecodeErrorDidOccur_error(
            &self,
            player: &AVAudioPlayer,
            error: Option<&NSError>,
        );
    }
);
extern_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind=MainThreadOnly]
    #[name = "AVAudioPlayer"]
    pub struct AVAudioPlayer;
);
impl AVAudioPlayer {
    extern_methods!(
        #[unsafe(method(initWithData:error:_))]
        #[unsafe(method_family=init)]
        pub fn initWithData_error(
            this: Allocated<Self>,
            data: &NSData,
        ) -> Result<Retained<Self>, Retained<NSError>>;
        #[unsafe(method(setDelegate:))]
        #[unsafe(method_family=none)]
        pub fn setDelegate(&self, delegate: Option<&ProtocolObject<dyn AVAudioPlayerDelegate>>);
        #[unsafe(method(play))]
        #[unsafe(method_family=none)]
        pub fn play(&self) -> bool;
        #[unsafe(method(stop))]
        #[unsafe(method_family=none)]
        pub fn stop(&self);
        #[unsafe(method(isPlaying))]
        #[unsafe(method_family=none)]
        pub fn isPlaying(&self) -> bool;
    );
}
/// UIKit variants are missing from the published data-model crate.
pub unsafe fn ink_color(ink: &PKInk) -> Retained<UIColor> {
    unsafe { objc2::msg_send![ink, color] }
}
pub unsafe fn ink_type(ink: &PKInk) -> Retained<NSString> {
    unsafe { objc2::msg_send![ink, inkType] }
}
