#![expect(clippy::empty_enums)]

use derive_more::{Add, AddAssign};
use handler::APPLET;
use home::AppletView;
use leptos::prelude::*;
use leptos::tachys::html::attribute::Attribute;
use leptos::tachys::renderer::dom::Element;
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;

/// The current version of the app, a `usize` in the form of an `&str`.
const VERSION: &str = env!("BUILD_VERSION");

/// A `OnceLock` that stores whether the current device is touch-screen.
static IS_TOUCHSCREEN: OnceLock<bool> = OnceLock::new();

/// The CSS style string.
const STYLE: &str = "
*, *::before, *::after {
    font-family: monospace;
    transition: all 0.3s ease;
}
";

/// The link to the SVG XML namespace
const SVG: &str = "http://www.w3.org/2000/svg";

#[wasm_bindgen(start)]
fn main() {
    APPLET.set_applet(AppletView::Home);
    console_error_panic_hook::set_once();
    IS_TOUCHSCREEN.set(is_touchscreen()).unwrap();
    leptos::mount::mount_to_body(move || {
        view! {
            <style>{STYLE}</style>
            <main>
                <home::App />
            </main>
        }
    });
}

/// A function that returns whether or not the current device is touch-screen. It is called once at
fn is_touchscreen() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    let navigator = window.navigator();
    if navigator.max_touch_points() > 0_i32 {
        true
    } else if let Ok(Some(media_match)) = window.match_media("(any-pointer: coarse)")
        && media_match.matches()
    {
        true
    } else {
        false
    }
}

/// The size of the window as a tuple of `f64`s as `(width, height)`.
fn window_size() -> (f64, f64) {
    let window_size = leptos_use_window_size::use_window_size();
    (
        window_size.width.get_untracked(),
        window_size.height.get_untracked(),
    )
}

#[component]
fn Center(
    /// This field contains all of the elements inside the `<Center>` block. It is optional.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! {
        <div
            style:display="flex"
            style:align-items="center"
            style:justify-content="center"
            style:flex-wrap="wrap"
            style:width="100%"
            style:min-height="100%"
        >
            {children.map(|children| children())}
        </div>
    }
}

/// This function creates an `impl Attribute` that places the element such that it fills up all of the space it is given.
fn absolute_position() -> impl Attribute {
    view! {
        <{..}
            style:position="absolute"
            style:display="block"
            style:top="0px"
            style:bottom="0px"
            style:left="0"
            style:right="0"
        />
    }
}

/// A pair of two-dimensional coordinates.
///
/// | coordinate | letter | increment effect | decrement effect |
/// | ---------- | :----: | ---------------- | ---------------- |
/// | first      | x      | move right       | move left        |
/// | second     | y      | move down        | move up          |
#[derive(Add, AddAssign, Clone, Default)]
struct Coordinates(f64, f64);

impl Coordinates {
    /// Calculates the offset vector of an `&HtmlElement`.
    fn of_offset(element: &HtmlElement) -> Self {
        Self(
            f64::from(element.offset_left()),
            f64::from(element.offset_top()),
        )
    }

    /// Calculates the scroll offset vector of an `&Element`.
    fn of_scroll(element: &Element) -> Self {
        Self(
            f64::from(element.scroll_left()),
            f64::from(element.scroll_top()),
        )
    }

    /// Calculates the total offset of an `&HtmlElement`, combining normal offset and its parent's scroll offset.
    fn element_offset(element: &HtmlElement) -> Self {
        Self::of_offset(element)
            + element
                .parent_element()
                .map_or_default(|parent| Self::of_scroll(&parent))
    }
}

/// Gets the position of a `&HtmlElement` without considering `style:transform` attributes.
fn get_position_untransformed(element: &HtmlElement) -> Coordinates {
    let mut coordinates = Coordinates::default();
    let mut current = Some(element.clone());
    while let Some(other_element) = current {
        coordinates += Coordinates::element_offset(&other_element);
        current = other_element.offset_parent().map(Element::unchecked_into);
    }
    coordinates
}

/// A module of utilities for interacting with elements through the pointer.
mod interact {
    extern crate alloc;

    use super::IS_TOUCHSCREEN;
    use super::handler::{APPLET, Handler};
    use leptos::attr::NextAttribute as _;
    use leptos::ev;
    use leptos::prelude::*;
    use leptos::tachys::html::attribute::Attribute;

    /// This struct represents an interaction, where the pointer state is given and there is the ability to disable the interaction.
    pub struct Interaction {
        /// This field represents the state of the pointer in regards to the HTML element the interaction is applied to.
        pub pointer_state: Signal<PointerState>,
        /// This field is a `WriteSignal` that toggles whether the HTML element is displayed as active.
        pub set_is_active: WriteSignal<bool>,
        /// This field is a `WriteSignal` that toggles whether the HTML element the interaction is applied to is disabled.
        pub set_is_disabled: WriteSignal<bool>,
    }

    /// This enum represents the location and status of a pointer in regards to an HTML element.
    #[derive(Clone, Copy)]
    pub enum PointerState {
        /// This enum variant represents the state when the HTML element is disabled.
        Disabled,
        /// When the HTML element is enabled, we track some facts about the pointer.
        Enabled {
            /// This field represents whether or not the pointer is on top of the HTML element.
            is_hovered: bool,
            /// This field represents whether or not the pointer is pressed down on the element. The pointer does not necessarily have to be on the element, the pointer just needs to be down, and the place where it was clicked down must be on the element.
            is_active: bool,
        },
    }

    /// Creates three `(ReadSignal<bool>, WriteSignal<bool>)`s initialized as `false`.
    fn get_three_signals<T: Handler>(handler: &T) -> [(ReadSignal<bool>, WriteSignal<bool>); 3] {
        [
            handler.add_signal(false),
            handler.add_signal(false),
            handler.add_signal(false),
        ]
    }

    /// This function creates and returns an `impl Attribute` and an `Interaction` object. It provides `ReadSignal`s for when a HTML element is hovered over, active, or disabled. It also changes the cursor accordingly:
    ///
    /// | interaction | cursor      |
    /// | :---------: | :---------: |
    /// | hover       | pointer     |
    /// | disabled    | not-allowed |
    pub fn interaction<T: Handler, U: FnMut() + 'static>(
        handler: &'static T,
        on_click: U,
    ) -> (impl Attribute, Interaction) {
        let [
            (is_hovered, set_is_hovered),
            (is_active, set_is_active),
            (is_disabled, set_is_disabled),
        ] = get_three_signals(handler);
        let on_click_event = get_on_click_event(handler, is_active, on_click);
        (
            view! {
                <{..}
                    {..bare_interaction(set_is_hovered, set_is_active, on_click_event)}
                    style:cursor=move || {
                        if !is_hovered.get() {
                            "auto"
                        } else if is_disabled.get() {
                            "not-allowed"
                        } else {
                            "pointer"
                        }
                    }
                />
            },
            Interaction {
                pointer_state: Signal::derive(move || {
                    if is_disabled.get() {
                        PointerState::Disabled
                    } else {
                        PointerState::Enabled {
                            is_hovered: is_hovered.get(),
                            is_active: is_active.get(),
                        }
                    }
                }),
                set_is_active,
                set_is_disabled,
            },
        )
    }

    /// If the device is touchscreen, this function returns `Some(on_click)`. Otherwise, it returns `None`, but it adds an effect to `handler` that calls `on_click()` whenever `is_active` changes from `true` to `false`.
    pub fn get_on_click_event<T: Handler, U: FnMut() + 'static>(
        handler: &T,
        is_active: ReadSignal<bool>,
        mut on_click: U,
    ) -> Option<U> {
        if IS_TOUCHSCREEN.get() == Some(&false) {
            handler.add_effect(
                move || is_active.get(),
                move |current, previous| {
                    if !current && previous != Some(&false) {
                        on_click();
                    }
                },
            );
            None
        } else {
            Some(on_click)
        }
    }

    /// This macro contains a combination of different utilities to make adding window event listeners easier.
    macro_rules! listen {
        ($set_is_hovered:ident, $set_is_active:ident, $clicked:ident: active) => {
            $set_is_active.set(true)
        };
        ($set_is_hovered:ident, $set_is_active:ident, $clicked:ident: not_active) => {
            $set_is_active.set(false)
        };
        ($set_is_hovered:ident, $set_is_active:ident, $clicked:ident: hover) => {
            $set_is_hovered.set(true)
        };
        ($set_is_hovered:ident, $set_is_active:ident, $clicked:ident: not_hovered) => {
            $set_is_hovered.set(false)
        };
        ($set_is_hovered:ident, $set_is_active:ident, $clicked:ident: deactivate) => {
            listen!($set_is_hovered, $set_is_active, $clicked: not_hovered);
            listen!($set_is_hovered, $set_is_active, $clicked: not_active)
        };
        ($set_is_hovered:ident, $set_is_active:ident, $clicked:ident: click) => {
            $clicked()
        };
        ($set_is_hovered:ident, $set_is_active:ident, $clicked:ident: hover_and_click) => {
            listen!($set_is_hovered, $set_is_active, $clicked: hover);
            listen!($set_is_hovered, $set_is_active, $clicked: click)
        };
        ($set_is_hovered:ident, $set_is_active:ident, $clicked:ident: click_if_not_touchscreen) => {
            if IS_TOUCHSCREEN.get() == Some(&false) {
                listen!($set_is_hovered, $set_is_active, $clicked: click);
            }
        };
        {
            $set_is_hovered:ident
            $set_is_active:ident
            $clicked:ident
            $($x:ident -> $flag:ident)*
        } => {
            $(APPLET.add_listener_once(ev::$x, move |_| {
                listen!($set_is_hovered, $set_is_active, $clicked: $flag);
            });)*
        };
        {
            @view
            $set_is_hovered:ident
            $set_is_active:ident
            $clicked:ident
            $($x:ident -> $flag:ident)*
        } => {
            view! {
                <{..}
                    $(on:$x=move |_| {
                        listen!($set_is_hovered, $set_is_active, $clicked: $flag);
                    })*
                />
            }
        };
    }

    /// Creates an `impl Attribute` that controls the input parameters `set_is_hovered` and `set_is_active` and can also call `on_click_event()`.
    fn bare_interaction<T: FnMut() + 'static>(
        set_is_hovered: WriteSignal<bool>,
        set_is_active: WriteSignal<bool>,
        mut on_click_event: Option<T>,
    ) -> impl Attribute {
        let clicked = clicked_closure(set_is_hovered, set_is_active);
        view! {
            <{..}
                {..event_attribute(set_is_hovered, set_is_active, clicked)}
                on:click=move |_| {
                    if IS_TOUCHSCREEN.get() == Some(&true) {
                        listen!(set_is_hovered, set_is_active, clicked: deactivate);
                        if let Some(ref mut on_click) = on_click_event {
                            on_click();
                        }
                    }
                }
            />
        }
    }

    /// Creates a closure that when called, creates window event listeners that control `set_is_hovered` and `set_is_active`.
    fn clicked_closure(
        set_is_hovered: WriteSignal<bool>,
        set_is_active: WriteSignal<bool>,
    ) -> impl Fn() + Copy {
        move || {
            listen!(set_is_hovered, set_is_active, clicked: active);
            listen! {
                set_is_hovered
                set_is_active
                clicked
                mouseup -> not_active
                touchend -> deactivate
                pointerup -> not_active
                touchcancel -> deactivate
                pointercancel -> deactivate
            }
        }
    }

    /// Creates an attribute that controls `set_is_hovered` and `set_is_active` `on:` certain HTML events.
    fn event_attribute<T: Fn() + Copy + 'static>(
        set_is_hovered: WriteSignal<bool>,
        set_is_active: WriteSignal<bool>,
        clicked: T,
    ) -> impl Attribute {
        listen! {
            @view
            set_is_hovered
            set_is_active
            clicked
            mouseup -> not_active
            touchend -> deactivate
            mousedown -> click_if_not_touchscreen
            pointerup -> not_active
            mouseenter -> hover
            mouseleave -> not_hovered
            touchstart -> hover_and_click
            pointerdown -> click
            touchcancel -> deactivate
            pointerenter -> hover
            pointerleave -> not_hovered
            pointercancel -> deactivate
        }
    }

    /// This creates an `interaction` framework as an attribute.
    ///
    /// | interaction | effect                                                   |
    /// | :---------: | -------------------------------------------------------- |
    /// | hover       | The element gets 1.05 times larger with a box shadow.    |
    /// | active      | The element becomes 0.9 times brighter (it gets darker). |
    pub fn interactive(on_click: impl FnMut() + 'static) -> (impl Attribute, Interaction) {
        let (attribute, interaction) = interaction(&APPLET, on_click);
        (
            view! {
                <{..}
                    {..attribute}
                    style:scale=move || {
                        if let PointerState::Enabled { is_hovered: true, .. } = interaction
                            .pointer_state
                            .get()
                        {
                            "1.05"
                        } else {
                            "1"
                        }
                    }
                    style:filter=move || {
                        format!(
                            "brightness({})",
                            if let PointerState::Enabled { is_active: true, .. } = interaction
                                .pointer_state
                                .get()
                            {
                                0.9
                            } else {
                                1_f32
                            },
                        )
                    }
                    style:box-shadow=move || {
                        if let PointerState::Enabled { is_hovered: true, .. } = interaction
                            .pointer_state
                            .get()
                        {
                            "0 5px 5px rgba(0, 0, 0, 0.15)"
                        } else {
                            "none"
                        }
                    }
                />
            },
            interaction,
        )
    }
}

/// This module contains all things related to handling the lifetimes of signals and window event listeners.
mod handler {
    extern crate alloc;

    use super::home::AppletView;
    use alloc::rc::Rc;
    use alloc::sync::Arc;
    use core::cell::RefCell;
    use core::mem;
    use leptos::ev::EventDescriptor;
    use leptos::prelude::*;
    use std::sync::Mutex;
    use wasm_bindgen::prelude::*;

    /// This trait describes a handler. It is required to implement the method `add_handle`, which asks the `Handler` to keep track of the lifetime of an `impl AppletHandle`.
    pub trait Handler {
        /// Adds a handle to the handler.
        fn add_handle(&self, handle: AppletHandle);

        /// Adds a new signal to be handled and dropped.
        fn add_new_signal<T: 'static>(&self, signal: (ReadSignal<T>, WriteSignal<T>));

        /// Creates a `(ReadSignal<T>, WriteSignal<T>)` and adds both of them to the handle list.
        fn add_signal<T: Send + Sync + 'static>(
            &self,
            value: T,
        ) -> (ReadSignal<T>, WriteSignal<T>) {
            let value = signal(value);
            self.add_new_signal(value);
            value
        }

        /// Creates a `window_event_listener`, then disposes of the event listener once it is called.
        fn add_listener_once<T: EventDescriptor + 'static, U: Fn(T::EventType) + 'static>(
            &self,
            event: T,
            cb: U,
        ) where
            T::EventType: JsCast,
        {
            let (handle, set_handle) = self.add_signal(None::<ListenerHandle>);
            set_handle.set(Some(self.add_listener(event, move |event_object| {
                Self::call_function_and_remove_handles(&cb, event_object, handle, set_handle);
            })));
        }

        /// Calls `cb` with the event object `event_object`, and removes `handle`, `set_handle`, and the `ListenerHandle` they point to after.
        fn call_function_and_remove_handles<T: Fn(U) + 'static, U: JsCast>(
            cb: &T,
            event_object: U,
            handle: ReadSignal<Option<ListenerHandle>>,
            set_handle: WriteSignal<Option<ListenerHandle>>,
        ) {
            cb(event_object);
            set_handle.update(|handle| {
                if let Some(handle) = handle.take() {
                    AppletHandle::Window(handle).rid();
                }
            });
            dispose_handles(handle, set_handle);
        }

        /// Creates a `window_event_listener()` call and returns a `ListenerHandle`.
        fn add_listener<T: EventDescriptor + 'static>(
            &self,
            event: T,
            cb: impl Fn(T::EventType) + 'static,
        ) -> ListenerHandle
        where
            T::EventType: JsCast,
        {
            let handle = ListenerHandle::new(window_event_listener::<T>(event, cb));
            self.add_handle(AppletHandle::Window(handle.clone()));
            handle
        }

        /// Creates an `Effect::watch` effect handled by the handler.
        fn add_effect<T: 'static>(
            &self,
            dependency_fn: impl FnMut() -> T + 'static,
            mut handler: impl FnMut(&T, Option<&T>) + 'static,
        ) {
            self.add_handle(AppletHandle::Effect(Effect::watch(
                dependency_fn,
                move |current, previous, _| handler(current, previous),
                false,
            )));
        }
    }

    /// This type denotes a `ListenerHandle` that can be cloned, so the handle can be called from multiple places.
    #[derive(Clone)]
    pub struct ListenerHandle(Arc<Mutex<Option<WindowListenerHandle>>>);

    impl ListenerHandle {
        /// Creates a new `ListenerHandle` from a `WindowListenerHandle`.
        fn new(handle: WindowListenerHandle) -> Self {
            Self(Arc::new(Mutex::new(Some(handle))))
        }
    }

    /// An event listener that must be disposed of.
    pub enum AppletHandle {
        /// An `Effect::watch` listener.
        Effect(Effect<LocalStorage>),
        /// A `window_event_listener`.
        Window(ListenerHandle),
    }

    impl AppletHandle {
        /// This method rids the current `AppletHandle` when given a `Box` to it.
        fn rid(self) {
            match self {
                Self::Effect(effect) => effect.stop(),
                Self::Window(window) => {
                    let handle = window.0.lock().unwrap().take();
                    if let Some(handle) = handle {
                        handle.remove();
                    }
                }
            }
        }
    }

    /// This trait describes all `(ReadSignal<T>, WriteSignal<T>)` tuples.
    pub trait SignalHandle: Send {
        /// This method rids the current `SignalHandle` when given a `Box` to it.
        fn rid(self: Box<Self>);
    }

    impl<T> SignalHandle for (ReadSignal<T>, WriteSignal<T>) {
        fn rid(self: Box<Self>) {
            dispose_handles(self.0, self.1);
        }
    }

    /// This is an applet handler. It keeps track of handles that need to be released when the current applet changes.
    struct AppletHandler {
        /// The handles to be released when the current applet changes.
        handles: Vec<AppletHandle>,
        /// The newly allocated signals for the current applet.
        new_signals: Vec<Box<dyn SignalHandle>>,
        /// The old signals from the previous applet.
        old_signals: Vec<Box<dyn SignalHandle>>,
        /// The `ReadSignal` for the current applet.
        current_applet: ReadSignal<AppletView>,
        /// The `WriteSignal` for the current applet.
        set_current_applet: WriteSignal<AppletView>,
    }

    impl AppletHandler {
        /// Creates a new `AppletHandler` with the given `applet_view`.
        fn new(applet_view: AppletView) -> Self {
            let (current_applet, set_current_applet) = signal(applet_view);
            Self {
                handles: Vec::new(),
                new_signals: Vec::new(),
                old_signals: Vec::new(),
                current_applet,
                set_current_applet,
            }
        }

        /// Rids all handles, rids all old signals, and moves all new signals to the `Vec` of old signals.
        fn rid_signals_and_handles(&mut self) {
            for applet_handle in mem::take(&mut self.handles) {
                applet_handle.rid();
            }
            for signal_handle in
                mem::replace(&mut self.old_signals, mem::take(&mut self.new_signals))
            {
                signal_handle.rid();
            }
        }
    }

    /// A `Mutex` to an `Option` of an `Arc` to a `Mutex` to an `FnMut` that takes an `f64`. This is how the animation function that runs every frame is stored.
    type F64FnMutMutexArcOptionMutex = Mutex<Option<Arc<Mutex<dyn FnMut(f64) + Send>>>>;

    /// This struct is the type for a global applet handler. It can be used merely with an immutable reference to it. It may also be uninitialized.
    pub struct StaticAppletHandler {
        /// The animation function that runs each frame.
        each_frame: F64FnMutMutexArcOptionMutex,
        /// The inner `AppletHandler`.
        inner: Mutex<Option<AppletHandler>>,
    }

    impl StaticAppletHandler {
        /// Returns the `ReadSignal` that reads from the current applet.
        ///
        /// # Panics
        ///
        /// Panics if the `StaticAppletHandler` is uninitialized.
        pub fn applet(&self) -> ReadSignal<AppletView> {
            self.inner.lock().unwrap().as_ref().unwrap().current_applet
        }

        /// If the `StaticAppletHandler` is uninitialized, it is initialized starting from `applet_view`. Otherwise, the current applet is set to `applet_view`, and all handles are cleared.
        pub fn set_applet(&self, applet_view: AppletView) {
            *self.each_frame.lock().unwrap() = None;
            self.reset_inner_handler(applet_view);
        }

        /// Resets the inner `AppletHandler` if it has been initialized, and constructs it otherwise using `applet_view`.
        fn reset_inner_handler(&self, applet_view: AppletView) {
            let mut handler = self.inner.lock().unwrap();
            if let Some(handler) = handler.as_mut() {
                handler.set_current_applet.set(applet_view);
                handler.rid_signals_and_handles();
            } else {
                *handler = Some(AppletHandler::new(applet_view));
            }
        }

        /// Starts the animation loop.
        pub fn animate<T: FnMut(f64) + Send + 'static>(&'static self, each_frame: T) {
            *self.each_frame.lock().unwrap() = Some(Arc::new(Mutex::new(each_frame)));
            self.start_animation();
        }

        /// Starts the animation loop, given that `self.each_frame` is already initialized.
        fn start_animation(&'static self) {
            let (closure, clone) = TimeClosure::new_two();
            *clone.0.borrow_mut() = Some(Closure::new(move |time| {
                self.animate_frame_and_request(&closure, time);
            }));
            clone.call();
        }

        /// Animates the frame using `time`, and then calles the `&TimeClosure` to animate the next frame.
        fn animate_frame_and_request(&self, closure: &TimeClosure, time: f64) {
            if let Some(ref arc) = *self.each_frame.lock().unwrap() {
                Arc::clone(arc).lock().unwrap()(time);
                closure.call();
            }
        }
    }

    /// An `Rc` to a `RefCell` to an `Option` to a `Closure` containing a `dyn FnMut(f64)`.
    type F64FnMutClosureOptionRefCellRc = Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>>;

    /// An `Rc` to a closure that calls a frame-by-frame animation function and requests another animation frame for itself.
    #[derive(Clone)]
    struct TimeClosure(F64FnMutClosureOptionRefCellRc);

    impl TimeClosure {
        /// Creates two `TimeClosure`s, one of which is the clone of the other.
        fn new_two() -> (Self, Self) {
            let closure = Self(Rc::new(RefCell::new(None::<Closure<dyn FnMut(f64)>>)));
            let clone = closure.clone();
            (closure, clone)
        }

        /// Calls the `TimeClosure`.
        fn call(&self) {
            web_sys::window()
                .unwrap()
                .request_animation_frame(self.0.borrow().as_ref().unwrap().as_ref().unchecked_ref())
                .unwrap();
        }
    }

    impl Handler for StaticAppletHandler {
        fn add_handle(&self, handle: AppletHandle) {
            self.inner
                .lock()
                .unwrap()
                .as_mut()
                .unwrap()
                .handles
                .push(handle);
        }

        fn add_new_signal<T: 'static>(&self, signal: (ReadSignal<T>, WriteSignal<T>)) {
            self.inner
                .lock()
                .unwrap()
                .as_mut()
                .unwrap()
                .new_signals
                .push(Box::new(signal));
        }
    }

    /// The default handler. It does nothing when asked to add a handle and doesn't keep track of `AppletHandle`s.
    pub struct DefaultHandler;

    impl Handler for DefaultHandler {
        fn add_handle(&self, _handle: AppletHandle) {}

        fn add_new_signal<T>(&self, _signal: (ReadSignal<T>, WriteSignal<T>)) {}
    }

    /// This is the global applet handler.
    pub static APPLET: StaticAppletHandler = StaticAppletHandler {
        each_frame: Mutex::new(None),
        inner: Mutex::new(None),
    };

    /// Disposes of a `ReadSignal` and a `WriteSignal` of the same type.
    fn dispose_handles<T>(handle: ReadSignal<T>, set_handle: WriteSignal<T>) {
        handle.dispose();
        set_handle.dispose();
    }
}

/// This module contains all things related to the "Home" page: the page that has to do with selecting an applet.
mod home {
    use super::SVG;
    use super::handler::{APPLET, DefaultHandler};
    use super::interact;
    use super::interact::PointerState;
    use super::note_recognition;
    use base64::Engine as _;
    use leptos::prelude::*;
    use strum::IntoEnumIterator as _;
    use strum_macros::EnumIter;

    /// This enum represents an applet that can be viewed.
    #[derive(Clone, Copy)]
    pub enum AppletView {
        /// This variant represents the Home or applet selection view.
        Home,
        /// This variant represents an `AppletView` that has a corresponding `AppletWithIconButton`.
        Specified(AppletWithIconButton),
    }

    /// This enum represents an applet that has an icon button.
    #[derive(Clone, Copy, EnumIter)]
    pub enum AppletWithIconButton {
        /// This variant is the Note Recognition game.
        NoteRecognition,
    }

    impl AppletView {
        /// This function returns the `Applet View` as an `impl IntoView`.
        fn view(self) -> impl IntoView + use<> {
            match self {
                Self::Home => Box::new(
                    AppletWithIconButton::iter()
                        .map(AppletWithIconButton::icon_button)
                        .collect_view(),
                )
                .into_any(),
                Self::Specified(ref specification) => specification.view().into_any(),
            }
        }
    }

    impl AppletWithIconButton {
        /// This function returns the `AppletWithIconButton`'s icon embedded inside the `background-image` of a `<button>`.
        fn icon_button(self) -> impl IntoView + use<> {
            let view = AppletView::Specified(self);
            view! {
                <button
                    style:margin="30px"
                    style:width="300px"
                    style:height="300px"
                    style:border="none"
                    style:border-radius="16px"
                    style:background-size="cover"
                    style:background-image=format!("url('{}')", self.icon_as_uri())
                    {..interact::interactive(move || APPLET.set_applet(view)).0}
                />
            }
        }

        /// This function returns the `AppletWithIconButton` as an `impl IntoView`.
        fn view(self) -> impl IntoView + use<> {
            match self {
                Self::NoteRecognition => note_recognition::App,
            }
        }

        /// This function returns the URI for the icon image asset of the `AppletWithIconButton`.
        fn icon_as_uri(self) -> String {
            let bytes = match self {
                Self::NoteRecognition => include_bytes!("note_recognition_icon.png"),
            };
            let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
            format!("data:image/png;base64,{encoded}")
        }
    }

    #[component]
    pub fn App() -> impl IntoView {
        view! {
            <BackButton />
            <AppletShell />
            <CornerFootnote is_on_right=false>
                {format!("Version: v{} ", super::VERSION)}
            </CornerFootnote>
            <CornerFootnote is_on_right=true>"This site is entirely human-made."</CornerFootnote>
        }
    }

    #[component]
    fn BackButton() -> impl IntoView {
        let (attribute, interaction) =
            interact::interaction(&DefaultHandler, move || APPLET.set_applet(AppletView::Home));
        Effect::new(move || {
            interaction
                .set_is_disabled
                .set(match APPLET.applet().get() {
                    AppletView::Home => true,
                    AppletView::Specified(_) => false,
                });
        });
        view! {
            <button
                {..attribute}
                style:display="inline-flex"
                style:padding="6px 6px"
                style:border="none"
                style:border-radius="6px"
                style:background-color=move || {
                    if let PointerState::Enabled { is_hovered, is_active } = interaction
                        .pointer_state
                        .get() && (is_hovered || is_active)
                    {
                        "#27ae60"
                    } else {
                        "#fff"
                    }
                }
                style:box-shadow=move || {
                    if let PointerState::Enabled { is_hovered: true, .. } = interaction
                        .pointer_state
                        .get()
                    {
                        "0 4px 12px rgba(0, 0, 0, 0.15)"
                    } else {
                        "none"
                    }
                }
            >
                <svg xmlns=SVG viewBox="0 0 24 24" style:width="20px" style:height="20px">
                    <path
                        d="M 6 12
                        H 18
                        M 6 12
                        L 11 7
                        M 6 12
                        L 11 17"
                        stroke=move || {
                            if matches!(interaction.pointer_state.get(), PointerState::Disabled) {
                                "#888"
                            } else {
                                "#000"
                            }
                        }
                        stroke-width="2"
                        stroke-linecap="round"
                        stroke-linejoin="round"
                        fill="none"
                    />
                </svg>
            </button>
        }
    }

    #[component]
    fn CornerFootnote(
        /// This boolean represents whether or not the `<CornerFootnote>` will be located in the bottom right.
        ///
        /// | value of `is_on_right` | outcome                                  |
        /// | :--------------------: | ---------------------------------------- |
        /// | `true`                 | The element appears on the bottom right. |
        /// | `false`                | The element appears on the bottom left.  |
        is_on_right: bool,
        /// This contains all child elements of the `<CornerFootnote>` block. There can be no children.
        #[prop(optional)]
        children: Option<Children>,
    ) -> impl IntoView {
        view! {
            <p
                style:position="fixed"
                style:bottom="0px"
                style:z-index="1000"
                style:color="#333"
                style:font-size="14px"
                style=(if is_on_right { "right" } else { "left" }, "15px")
            >
                {children.map(|children| children())}
            </p>
        }
    }

    #[component]
    fn AppletShell() -> impl IntoView {
        view! {
            <div style:position="absolute" style:display="block" style:inset="50px 0">
                {move || APPLET.applet().get().view()}
            </div>
        }
    }
}

/// This module contains all of the components and functions for the Note Recognition applet.
mod note_recognition {
    extern crate alloc;

    use super::handler::Handler as _;
    use super::interact;
    use super::{APPLET, Coordinates, SVG};
    use core::iter;
    use leptos::either::Either;
    use leptos::ev;
    use leptos::html::Button;
    use leptos::prelude::*;
    use leptos::tachys::html::attribute::Attribute;
    use leptos::tachys::html::event::KeyboardEvent;
    use num_enum::TryFromPrimitive;
    use slab::Slab;
    use std::sync::Mutex;
    use strum::IntoEnumIterator as _;
    use strum_macros::EnumIter;

    #[component]
    pub fn App() -> impl IntoView {
        let (clone_list, set_clone_list) = APPLET.add_signal(Slab::<NoteButtonClone>::new());
        let (note_index, set_note_index) = APPLET.add_signal(NoteIndex::random());
        APPLET.animate(move |time| animate_clones(clone_list, set_clone_list, time));
        view! {
            <div style:overflow="hidden" {..super::absolute_position()}>
                <div
                    style:overflow-x="hidden"
                    style:overflow-y="auto"
                    {..super::absolute_position()}
                >
                    <super::Center>
                        <div
                            style:position="relative"
                            style:width="600px"
                            style:justify-content="center"
                        >
                            <super::Center>
                                <TimeCard />
                                <NoteButtons set_clone_list note_index set_note_index />
                            </super::Center>
                        </div>
                        <NoteFlashcard note_index=note_index />
                    </super::Center>
                </div>
                {move || {
                    clone_list
                        .with(|clone_list| {
                            clone_list
                                .iter()
                                .map(|(_, note_button_clone)| note_button_clone.view())
                                .collect_view()
                        })
                }}
            </div>
        }
    }

    /// A note from Pedal B to Top D, represented as an `u8` in `0..=30`.
    #[repr(u8)]
    #[derive(Clone, Copy, TryFromPrimitive)]
    enum NoteIndex {
        PedalB = 0,
        LowC = 1,
        LowD = 2,
        LowE = 3,
        LowF = 4,
        LowG = 5,
        LowA = 6,
        LowB = 7,
        BassC = 8,
        BassD = 9,
        BassE = 10,
        BassF = 11,
        BassG = 12,
        BassA = 13,
        BassB = 14,
        MiddleC = 15,
        TrebleD = 16,
        TrebleE = 17,
        TrebleF = 18,
        TrebleG = 19,
        TrebleA = 20,
        TrebleB = 21,
        TrebleC = 22,
        HighD = 23,
        HighE = 24,
        HighF = 25,
        HighG = 26,
        HighA = 27,
        HighB = 28,
        HighC = 29,
        TopD = 30,
    }

    impl NoteIndex {
        /// Creates a random note index.
        fn random() -> Self {
            Self::try_from(rand::random_range(0..31)).unwrap()
        }

        /// Given a mutable reference to a `NoteIndex`, change the `NoteIndex` behind that reference to a random `NoteIndex` that is necessarily different from the current `NoteIndex` behind the reference.
        fn change(&mut self) {
            let random = rand::random_range(0..30);
            let random = random + u8::from(random >= *self as u8);
            *self = Self::try_from(random).unwrap();
        }

        /// Converts the `NoteIndex` into the `NoteLetter` found at the end of the enum variant's name. For example, `NoteIndex::TrebleA` will be converted into `NoteLetter::A`.
        const fn letter(self) -> NoteLetter {
            match self {
                Self::LowA | Self::BassA | Self::TrebleA | Self::HighA => NoteLetter::A,
                Self::PedalB | Self::LowB | Self::BassB | Self::TrebleB | Self::HighB => {
                    NoteLetter::B
                }
                Self::LowC | Self::BassC | Self::MiddleC | Self::TrebleC | Self::HighC => {
                    NoteLetter::C
                }
                Self::LowD | Self::BassD | Self::TrebleD | Self::HighD | Self::TopD => {
                    NoteLetter::D
                }
                Self::LowE | Self::BassE | Self::TrebleE | Self::HighE => NoteLetter::E,
                Self::LowF | Self::BassF | Self::TrebleF | Self::HighF => NoteLetter::F,
                Self::LowG | Self::BassG | Self::TrebleG | Self::HighG => NoteLetter::G,
            }
        }
    }

    /// Animates all of the clones using the `time`.
    fn animate_clones(
        clone_list: ReadSignal<Slab<NoteButtonClone>>,
        set_clone_list: WriteSignal<Slab<NoteButtonClone>>,
        time: f64,
    ) {
        let mutex: Mutex<Box<_>> = Mutex::new([][..].into());
        clone_list.with_untracked(|clone_list| {
            *mutex.lock().unwrap() = clone_list
                .iter()
                .filter(move |(_, note_button_clone)| note_button_clone.animate(time))
                .map(|(key, _)| key)
                .collect();
        });
        remove_selected_clones(&mutex.lock().unwrap(), set_clone_list);
    }

    /// Removes the selected clones, whose keys are stored in an `&[usize]`, from a `Slab<NoteButtonClone>`.
    fn remove_selected_clones(slice: &[usize], set_clone_list: WriteSignal<Slab<NoteButtonClone>>) {
        set_clone_list.update(|clone_list| {
            for key in slice {
                clone_list.try_remove(*key);
            }
        });
    }

    #[component]
    fn TimeCard() -> impl IntoView {
        let (start_stop_restart_button_text, _) = APPLET.add_signal("Start");
        let (pause_resume_reset_button_text, _) = APPLET.add_signal("Reset");
        view! {
            <div
                {..flashcard()}
                style:display="flex"
                style:flex-direction="row"
                style:overflow="hidden"
                style:width="400px"
                style:height="200px"
                style:margin-bottom="15px"
            >
                <StatisticsColumn>
                    <Statistic name="Time".to_owned() />
                    <Statistic name="Pauses".to_owned() />
                    <Statistic name="Correct".to_owned() />
                    <Statistic name="Incorrect".to_owned() />
                </StatisticsColumn>
                <StatisticsColumn>
                    <TimingButton text=start_stop_restart_button_text />
                    <TimingButton text=pause_resume_reset_button_text />
                </StatisticsColumn>
            </div>
        }
    }

    #[component]
    fn StatisticsColumn(
        /// This optional field contains the elements that go inside the statistics column element.
        #[prop(optional)]
        children: Option<Children>,
    ) -> impl IntoView {
        view! {
            <div
                style:display="flex"
                style:flex-direction="column"
                style:justify-content="center"
                style:width="180px"
                style:margin-left="20px"
                style:gap="0px"
            >
                {children.map(|children| children())}
            </div>
        }
    }

    #[component]
    fn Statistic(
        /// This argument takes a `String` less than or equal to 9 characters long.
        name: String,
    ) -> impl IntoView {
        view! { <p>{format!("{name:\u{a0}>9}:")}</p> }
    }

    #[component]
    fn TimingButton(
        /// The text shown on the timing button.
        text: ReadSignal<&'static str>,
    ) -> impl IntoView {
        view! {
            <div
                style:width="150px"
                style:height="50px"
                style:margin="10px"
                style:border="none"
                style:border-radius="15px"
                style:background-color="#fff"
                style:font-family="inherit"
                style:font-size="x-large"
                style:display="flex"
                style:align-items="center"
                style:justify-content="center"
                {..interact::interactive(|| ()).0}
            >
                {move || text.get()}
            </div>
        }
    }

    #[component]
    fn NoteFlashcard(
        /// A `ReadSignal` to a `NoteIndex` representing at which note the whole note SVG is located on the stave.
        note_index: ReadSignal<NoteIndex>,
    ) -> impl IntoView {
        view! {
            <div
                {..flashcard()}
                style:width="300px"
                style:height="402px"
                style:margin-left="25px"
                style:margin-right="25px"
                style:padding="0px 23px"
            >
                <Stave />
                <TrebleClef />
                <BassClef />
                <WholeNote note_index=note_index />
            </div>
        }
    }

    #[component]
    fn Stave() -> impl IntoView {
        view! {
            <div
                style:display="flex"
                style:flex-direction="column"
                style:align-items="center"
                style:width="100%"
                style:gap="23px"
            >
                <StaveLine length=StaveLength::Gone />
                {iter::repeat_with(|| view! { <StaveLine length=StaveLength::Short /> })
                    .take(2)
                    .collect_view()}
                {iter::repeat_with(|| view! { <StaveLine length=StaveLength::Long /> })
                    .take(5)
                    .collect_view()}
                <StaveLine length=StaveLength::Short />
                {iter::repeat_with(|| view! { <StaveLine length=StaveLength::Long /> })
                    .take(5)
                    .collect_view()}
                {iter::repeat_with(|| view! { <StaveLine length=StaveLength::Short /> })
                    .take(2)
                    .collect_view()}
                <StaveLine length=StaveLength::Gone />
            </div>
        }
    }

    #[component]
    fn StaveLine(
        /// The length of the stave.
        length: StaveLength,
    ) -> impl IntoView {
        view! { <div style:height="0" style:border-top="2px solid #000" style:width=length.css_width() /> }
    }

    #[component]
    fn Clef(
        /// The gap from the top of the stave to the clef in pixels.
        top: usize,
        /// The height of the clef in pixels.
        height: usize,
        /// The SVG's viewbox, as 4 f64s.
        viewbox: [f64; 4],
        /// The clef's SVG path.
        path: &'static str,
    ) -> impl IntoView {
        view! {
            <div
                style:position="absolute"
                style:left="35px"
                style:width="50px"
                style:top=format!("{top}px")
                style:height=format!("{height}px")
            >
                <svg
                    xmlns=SVG
                    viewBox=format!("{} {} {} {}", viewbox[0], viewbox[1], viewbox[2], viewbox[3])
                >
                    <path d=path stroke="#000" />
                </svg>
            </div>
        }
    }

    #[component]
    fn TrebleClef() -> impl IntoView {
        view! {
            <Clef
                top=62
                height=150
                viewbox=[20.0, 4.0, 52.0, 145.0]
                path="M 51.688 5.25
                c -5.427 -0.1409 -11.774 12.818 -11.563 24.375
                c 0.049 3.52 1.16 10.659 2.781 19.625
                c -10.223 10.581 -22.094 21.44 -22.094 35.688
                c -0.163 13.057 7.817 29.692 26.75 29.532
                c 2.906 -0.02 5.521 -0.38 7.844 -1
                c 1.731 9.49 2.882 16.98 2.875 20.44
                c 0.061 13.64 -17.86 14.99 -18.719 7.15
                c 3.777 -0.13 6.782 -3.13 6.782 -6.84
                c 0 -3.79 -3.138 -6.88 -7.032 -6.88
                c -2.141 0 -4.049 0.94 -5.343 2.41
                c -0.03 0.03 -0.065 0.06 -0.094 0.09
                c -0.292 0.31 -0.538 0.68 -0.781 1.1
                c -0.798 1.35 -1.316 3.29 -1.344 6.06
                c 0 11.42 28.875 18.77 28.875 -3.75
                c 0.045 -3.03 -1.258 -10.72 -3.156 -20.41
                c 20.603 -7.45 15.427 -38.04 -3.531 -38.184
                c -1.47 0.015 -2.887 0.186 -4.25 0.532
                c -1.08 -5.197 -2.122 -10.241 -3.032 -14.876
                c 7.199 -7.071 13.485 -16.224 13.344 -33.093
                c 0.022 -12.114 -4.014 -21.828 -8.312 -21.969
                z
                m 1.281 11.719
                c 2.456 -0.237 4.406 2.043 4.406 7.062
                c 0.199 8.62 -5.84 16.148 -13.031 23.719
                c -0.688 -4.147 -1.139 -7.507 -1.188 -9.5
                c 0.204 -13.466 5.719 -20.886 9.813 -21.281
                z
                m -7.719 44.687
                c 0.877 4.515 1.824 9.272 2.781 14.063
                c -12.548 4.464 -18.57 21.954 -0.781 29.781
                c -10.843 -9.231 -5.506 -20.158 2.312 -22.062
                c 1.966 9.816 3.886 19.502 5.438 27.872
                c -2.107 0.74 -4.566 1.17 -7.438 1.19
                c -7.181 0 -21.531 -4.57 -21.531 -21.875
                c 0 -14.494 10.047 -20.384 19.219 -28.969
                z
                m 6.094 21.469
                c 0.313 -0.019 0.652 -0.011 0.968 0
                c 13.063 0 17.99 20.745 4.688 27.375
                c -1.655 -8.32 -3.662 -17.86 -5.656 -27.375
                z"
            />
        }
    }

    #[component]
    fn BassClef() -> impl IntoView {
        view! {
            <Clef
                top=233
                height=75
                viewbox=[160.0, 278.0, 424.09, 496.4]
                path="m 190.85 451.25
                c 11.661 14.719 32.323 24.491 55.844 24.491
                c 36.401 0 65.889 -23.372 65.889 -52.214
                s -29.488 -52.214 -65.889 -52.214
                c -20.314 4.1522 -28.593 9.0007 -33.143 -2.9091
                c 17.976 -54.327 46.918 -66.709 96.546 -66.709
                c 65.914 0 96.969 59.897 96.969 142.97
                c -18.225 190.63 -205.95 286.75 -246.57 316.19
                c 5.6938 13.103 5.3954 12.631 5.3954 12.009
                c 189.78 -86.203 330.69 -204.43 330.69 -320.74
                c 0 -92.419 -58.579 -175.59 -187.72 -172.8
                c -77.575 0 -170.32 86.203 -118 171.93
                z
                m 328.1 -89.88
                c 0 17.852 14.471 32.323 32.323 32.323
                s 32.323 -14.471 32.323 -32.323
                s -14.471 -32.323 -32.323 -32.323
                s -32.323 14.471 -32.323 32.323
                z
                m 0 136.75
                c 0 17.852 14.471 32.323 32.323 32.323
                s 32.323 -14.471 32.323 -32.323
                s -14.471 -32.323 -32.323 -32.323
                s -32.323 14.471-32.323 32.323
                z"
            />
        }
    }

    #[component]
    fn WholeNote(
        /// A `ReadSignal` to a `NoteIndex` representing at which note the whole note SVG is located on the stave.
        note_index: ReadSignal<NoteIndex>,
    ) -> impl IntoView {
        view! {
            <div
                style:position="absolute"
                style:display="flex"
                style:justify-content="center"
                style:top=move || {
                    format!(
                        "{}px",
                        (30_f64 - f64::from(note_index.get() as u8)).mul_add(12.5_f64, 2_f64),
                    )
                }
                style:left="calc(50% - 25px)"
                style:width="50px"
                style:height="23px"
            >
                <svg xmlns=SVG viewBox="177 900 3482.9198 2187.222">
                    <path
                        d="m 2532 1826
                        c -43 -149 -114 -304 -189 -415
                        c -74 -111 -193 -206 -311 -259
                        c -117 -52 -269 -55 -380 -29
                        c -111 27 -235 98 -310 196
                        c -74 98 -127 203 -144 345
                        c -18 143 26 311 70 460
                        c 43 150 97 315 190 444
                        c 94 130 209 197 327 249
                        c 118 53 253 37 364 10
                        c 111 -27 218 -78 292 -175
                        c 75 -98 150 -217 169 -360
                        c 18 -143 -34 -316 -78 -466
                        z
                        m 901 693
                        c -141 164 -350 301 -607 395
                        c -256 95 -551 145 -854 145
                        c -303 0 -604 -50 -872 -145
                        C 832 2820 605 2683 443 2519
                        C 282 2355 190 2169 178 1980
                        c -12 -190 56 -376 198 -540
                        c 141 -164 350 -301 607 -395
                        c 256 -95 551 -145 854 -145
                        c 303 0 603 50 872 145
                        c 268 94 494 231 656 395
                        c 162 164 253 350 265 539
                        c 12 190 -56 376 -197 540
                        z"
                        fill="#000"
                        stroke="#fff"
                    />
                    <path
                        d="m 2532 1826
                        c -43 -149 -114 -304 -189 -415
                        c -74 -111 -193 -206 -311 -259
                        c -117 -52 -269 -55 -380 -29
                        c -111 27 -235 98 -310 196
                        c -74 98 -127 203 -144 345
                        c -18 143 26 311 70 460
                        c 43 150 97 315 190 444
                        c 94 130 209 197 327 249
                        c 118 53 253 37 364 10
                        c 111 -27 218 -78 292 -175
                        c 75 -98 150 -217 169 -360
                        c 18 -143 -34 -316 -78 -466
                        z"
                        fill="none"
                        stroke="#000"
                    />
                    <path
                        d="m 3433 2519
                        c -141 164 -350 301 -607 395
                        c -256 95 -551 145 -854 145
                        c -303 0 -604 -50 -872 -145
                        C 832 2820 605 2683 443 2519
                        C 282 2355 190 2169 178 1980
                        c -12 -190 56 -376 198 -540
                        c 141 -164 350 -301 607 -395
                        c 256 -95 551 -145 854 -145
                        c 303 0 603 50 872 145
                        c 268 94 494 231 656 395
                        c 162 164 253 350 265 539
                        c 12 190 -56 376 -197 540
                        z"
                        fill="none"
                        stroke="#000"
                    />
                </svg>
            </div>
        }
    }

    /// An enum describing the three possible lengths of a stave line.
    enum StaveLength {
        /// No stave line. This variant is used to create a gap.
        Gone,
        /// A short stave line for Low C, Low E, Middle C, High A, and High C.
        Short,
        /// A long stave line that fills the space given.
        Long,
    }

    impl StaveLength {
        /// Returns the corresponding width as a CSS `width` style.
        const fn css_width(&self) -> &'static str {
            match self {
                Self::Gone => "0",
                Self::Short => "80px",
                Self::Long => "100%",
            }
        }
    }

    /// This function creates an `impl Attribute` that creates rounded corners and a silver gradient background.
    fn flashcard() -> impl Attribute {
        view! {
            <{..}
                style:position="relative"
                style:border-radius="25px"
                style:background="linear-gradient(135deg, #f1f5f9 0%, #f8fafc 50%, #e2e8f0 100%)"
            />
        }
    }

    /// The letter representation of a note.
    #[derive(Clone, Copy, EnumIter, PartialEq, Eq)]
    enum NoteLetter {
        A = 0,
        B = 1,
        C = 2,
        D = 3,
        E = 4,
        F = 5,
        G = 6,
    }

    impl NoteLetter {
        /// This function creates a clickable note button using a `WriteSignal` to a list of `NoteButtonClone`s.
        fn into_note_button(
            self,
            set_clone_list: WriteSignal<Slab<NoteButtonClone>>,
            note_index: ReadSignal<NoteIndex>,
            set_note_index: WriteSignal<NoteIndex>,
        ) -> impl IntoView {
            let target_ref = NodeRef::new();
            let (attribute, interaction) = interact::interactive(move || {
                self.note_button_clicked(set_clone_list, target_ref, note_index, set_note_index);
            });
            add_triggers_to_interaction(self, interaction.set_is_active);
            view! { <NoteButton note=self attribute target_ref /> }
        }

        /// This function is called when a note button is clicked. It creates a clone of the note button, then changes the `note_index` to another random value if the note button that was clicked matches `note_index`.
        fn note_button_clicked(
            self,
            set_clone_list: WriteSignal<Slab<NoteButtonClone>>,
            target_ref: NodeRef<Button>,
            note_index: ReadSignal<NoteIndex>,
            set_note_index: WriteSignal<NoteIndex>,
        ) {
            if self == note_index.get().letter() {
                set_note_index.update(NoteIndex::change);
            }
            add_clone(self, set_clone_list, target_ref);
        }

        /// The text that a note button of this `NoteLetter` would display. It consists of an uppercase letter followed by a number in subscript.
        const fn display_text(self) -> &'static str {
            match self {
                Self::A => "A\u{2081}",
                Self::B => "B\u{2082}",
                Self::C => "C\u{2083}",
                Self::D => "D\u{2084}",
                Self::E => "E\u{2085}",
                Self::F => "F\u{2086}",
                Self::G => "G\u{2087}",
            }
        }

        /// Given a `NoteLetter`, this function returns the three trigger keycodes of a `KeyboardEvent` object that can trigger note buttons with that `NoteLetter`. The three triggers are a letter key, a digit key, and a numpad key.
        const fn triggers(self) -> [&'static str; 3] {
            match self {
                Self::A => ["KeyA", "Digit1", "Numpad1"],
                Self::B => ["KeyB", "Digit2", "Numpad2"],
                Self::C => ["KeyC", "Digit3", "Numpad3"],
                Self::D => ["KeyD", "Digit4", "Numpad4"],
                Self::E => ["KeyE", "Digit5", "Numpad5"],
                Self::F => ["KeyF", "Digit6", "Numpad6"],
                Self::G => ["KeyG", "Digit7", "Numpad7"],
            }
        }
    }

    /// This struct describes a moving clone of a note button.
    struct NoteButtonClone {
        /// The `NoteLetter` that describes the note.
        note: NoteLetter,
        /// The time when the clone was created. The value may be `None` if the clone has not run an update loop yet.
        start_time: ReadSignal<Option<f64>>,
        /// A `WriteSignal` to the time when the clone was created.
        set_start_time: WriteSignal<Option<f64>>,
        /// The coordinates at which the clone started to exist.
        start_coordinates: Coordinates,
        /// The `NoteButtonClone`'s angular momentum.
        angular_momentum: f64,
        /// A `ReadSignal` to the coordinates of the note button.
        position: ReadSignal<(Coordinates, f64)>,
        /// A `WriteSignal` to the coordinates of the note button.
        set_position: WriteSignal<(Coordinates, f64)>,
    }

    impl NoteButtonClone {
        /// Creates an `impl IntoView` containing the HTML rendition of the current note button clone.
        fn view(&self) -> impl IntoView + use<> {
            view! {
                <NoteButton
                    note=self.note
                    attribute=()
                    target_ref=NodeRef::new()
                    position=self.position
                />
            }
        }

        /// Creates a new `NoteButtonCloneArc` from a note number and an HTML `Button` target.
        fn new(note: NoteLetter, target: NodeRef<Button>) -> Self {
            let start_coordinates =
                super::get_position_untransformed(target.get_untracked().unwrap().as_ref());
            let (position, set_position) = APPLET.add_signal((start_coordinates.clone(), 0_f64));
            let (start_time, set_start_time) = APPLET.add_signal(None);
            Self {
                note,
                start_time,
                set_start_time,
                start_coordinates,
                angular_momentum: rand::random_range(-0.5..0.5),
                position,
                set_position,
            }
        }

        /// Animates the `NoteButtonClone` using the current `time` as an `f64`.
        fn animate(&self, time: f64) -> bool {
            let window_size = super::window_size();
            let elapsed_time = self.get_elapsed_time(time);
            let new_position = (
                Coordinates(
                    self.start_coordinates.0,
                    elapsed_time
                        .powi(2)
                        .mul_add(0.0006, self.start_coordinates.1),
                ),
                elapsed_time * self.angular_momentum,
            );
            self.set_position.set(new_position.clone());
            !((-window_size.0..window_size.0 * 2_f64).contains(&new_position.0.0)
                && (-window_size.1..window_size.1 * 2_f64).contains(&new_position.0.1))
        }

        /// Gets the current elapsed time using the `current_time` as an `f64`. This function takes into account `self`'s `set_start_time` field.
        fn get_elapsed_time(&self, current_time: f64) -> f64 {
            self.start_time.get_untracked().map_or_else(
                || {
                    self.set_start_time.set(Some(current_time));
                    0_f64
                },
                |start_time| current_time - start_time,
            )
        }
    }

    /// A macro for triggering actions based on keypresses.
    macro_rules! trigger {
        {
            $note:ident
            $triggered:ident
            $set_triggered:ident
            $set_is_active:ident
            $(
                $event:ident -> $is_keypress:expr
            )*
        } => {
            $(
                APPLET.add_listener(ev::$event, move |keyboard_event| {
                    key_triggered::<$is_keypress>(
                        $note,
                        &keyboard_event,
                        $triggered,
                        $set_triggered,
                        $set_is_active,
                    );
                });
            )*
        };
    }

    /// This function triggers `set_is_active` when the keycode triggers defined by `note.triggers()` are pressed.
    fn add_triggers_to_interaction(note: NoteLetter, set_is_active: WriteSignal<bool>) {
        let (triggered, set_triggered) = APPLET.add_signal([false; 3]);
        trigger! {
            note
            triggered
            set_triggered
            set_is_active
            keypress -> true
            keyup -> false
        }
    }

    /// This function is called when a key is pressed.
    fn key_triggered<const IS_KEYPRESS: bool>(
        note: NoteLetter,
        keyboard_event: &KeyboardEvent,
        triggered: ReadSignal<[bool; 3]>,
        set_triggered: WriteSignal<[bool; 3]>,
        set_is_active: WriteSignal<bool>,
    ) {
        for (i, trigger) in note.triggers().into_iter().enumerate() {
            if keyboard_event.code() == trigger {
                set_triggered.update(|triggers| triggers[i] = IS_KEYPRESS);
                break;
            }
        }
        if triggered.get().into_iter().any(|boolean| boolean) == IS_KEYPRESS {
            set_is_active.set(IS_KEYPRESS);
        }
    }

    /// This function adds a `NoteButtonClone` for `note` to a list of `NoteButtonClone`s.
    fn add_clone(
        note: NoteLetter,
        set_clone_list: WriteSignal<Slab<NoteButtonClone>>,
        target: NodeRef<Button>,
    ) {
        set_clone_list.update(|slab| {
            slab.insert(NoteButtonClone::new(note, target));
        });
    }

    #[component]
    fn NoteButton(
        /// The `NoteLetter` that describes the note.
        note: NoteLetter,
        /// The attribute that describes how the note button is interactive.
        attribute: impl Attribute,
        /// The `NodeRef` that should represent the note button.
        target_ref: NodeRef<Button>,
        /// The position and rotation of the note button.
        #[prop(optional)]
        position: Option<ReadSignal<(Coordinates, f64)>>,
    ) -> impl IntoView {
        let letter_number = note.display_text();
        let position_attribute = position.map_or(Either::Left(()), |position| {
            Either::Right(view! {
                <{..}
                    style:margin="0"
                    style:position="fixed"
                    style:left=move || format!("{}px", position.get().0.0)
                    style:top=move || format!("{}px", position.get().0.1)
                    style:transition="none"
                    style:rotate=move || format!("{}deg", position.get().1)
                    style:pointer-events="none"
                />
            })
        });
        view! {
            <button
                node_ref=target_ref
                style:width="50px"
                style:height="50px"
                style:margin="7px"
                style:border="2px solid #a0a0a0"
                style:border-radius="8px"
                style:background-color="#fafafa"
                style:font-size="20px"
                style:text-align="center"
                {..attribute}
                {..position_attribute}
            >
                {letter_number}
            </button>
        }
    }

    #[component]
    fn NoteButtons(
        /// A `WriteSignal` to a list of `NoteButtonClone`s.
        set_clone_list: WriteSignal<Slab<NoteButtonClone>>,
        /// A `ReadSignal` to the `NoteIndex` describing where the whole note SVG is on the stave. This is needed in order to verify if the user recognized the note correctly when clicking a note button.
        note_index: ReadSignal<NoteIndex>,
        /// A `WriteSignal` to the `NoteIndex` describing where the whole note SVG is on the stave. This is needed in order to change the location of the whole note SVG when the user recognizes the note correctly and clicks a note button.
        set_note_index: WriteSignal<NoteIndex>,
    ) -> impl IntoView {
        view! {
            <div>
                {NoteLetter::iter()
                    .map(|number| {
                        number.into_note_button(set_clone_list, note_index, set_note_index)
                    })
                    .collect_view()}
            </div>
        }
    }
}
