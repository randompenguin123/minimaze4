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
                .map(|parent| Self::of_scroll(&parent))
                .unwrap_or_default()
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
    #[derive(Clone)]
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
                    style:transform=move || {
                        format!(
                            "scale({})",
                            if let PointerState::Enabled { is_hovered: true, .. } = interaction
                                .pointer_state
                                .get()
                            {
                                1.05
                            } else {
                                1_f32
                            },
                        )
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
            let [closure, clone] = TimeClosure::new_two();
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
        fn new_two() -> [Self; 2] {
            let closure = Self(Rc::new(RefCell::new(None::<Closure<dyn FnMut(f64)>>)));
            let clone = closure.clone();
            [closure, clone]
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
    use super::handler::{APPLET, DefaultHandler};
    use super::interact;
    use super::interact::PointerState;
    use super::note_recognition;
    use base64::Engine as _;
    use leptos::prelude::*;
    use strum::IntoEnumIterator as _;

    /// This enum represents an applet that can be viewed.
    #[derive(Clone)]
    pub enum AppletView {
        /// This variant represents the Home or applet selection view.
        Home,
        /// This variant represents an `AppletView` that has a corresponding `AppletWithIconButton`.
        Specified(AppletWithIconButton),
    }

    /// This enum represents an applet that has an icon button.
    #[derive(Clone, strum_macros::EnumIter)]
    pub enum AppletWithIconButton {
        /// This variant is the Note Recognition game.
        NoteRecognition,
    }

    impl AppletView {
        /// This function returns the `Applet View` as an `impl IntoView`.
        fn view(&self) -> impl IntoView + use<> {
            match *self {
                Self::Home => Box::new(
                    AppletWithIconButton::iter()
                        .map(|specification| specification.icon_button())
                        .collect_view(),
                )
                .into_any(),
                Self::Specified(ref specification) => specification.view().into_any(),
            }
        }
    }

    impl AppletWithIconButton {
        /// This function returns the `AppletWithIconButton`'s icon embedded inside the `background-image` of a `<button>`.
        fn icon_button(&self) -> impl IntoView + use<> {
            let clone = AppletView::Specified(self.clone());
            view! {
                <button
                    style:margin="30px"
                    style:width="300px"
                    style:height="300px"
                    style:border="none"
                    style:border-radius="16px"
                    style:background-size="cover"
                    style:background-image=format!("url('{}')", self.icon_as_uri())
                    {..interact::interactive(move || APPLET.set_applet(clone.clone())).0}
                />
            }
        }

        /// This function returns the `AppletWithIconButton` as an `impl IntoView`.
        fn view(&self) -> impl IntoView + use<> {
            match *self {
                Self::NoteRecognition => note_recognition::App,
            }
        }

        /// This function returns the URI for the icon image asset of the `AppletWithIconButton`.
        fn icon_as_uri(&self) -> String {
            let bytes = match *self {
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
                <svg
                    xmlns="http://www.w3.org/2000/svg"
                    viewBox="0 0 24 24"
                    style:width="20px"
                    style:height="20px"
                >
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
    use super::{APPLET, Coordinates};
    use leptos::either::Either;
    use leptos::ev;
    use leptos::html::Button;
    use leptos::prelude::*;
    use leptos::tachys::html::attribute::Attribute;
    use leptos::tachys::html::event::KeyboardEvent;
    use slab::Slab;
    use std::sync::Mutex;

    #[component]
    pub fn App() -> impl IntoView {
        let (clone_list, set_clone_list) = APPLET.add_signal(Slab::<NoteButtonClone>::new());
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
                                <NoteButtons set_clone_list />
                            </super::Center>
                        </div>
                        <NoteFlashcard />
                        {move || {
                            clone_list
                                .with(|clone_list| {
                                    clone_list
                                        .iter()
                                        .map(|(_, note_button_clone)| note_button_clone.view())
                                        .collect_view()
                                })
                        }}
                    </super::Center>
                </div>
            </div>
        }
    }

    /// Animates all of the clones using the `time`.
    fn animate_clones(
        clone_list: ReadSignal<Slab<NoteButtonClone>>,
        set_clone_list: WriteSignal<Slab<NoteButtonClone>>,
        time: f64,
    ) {
        let vec = Mutex::new(Vec::new());
        clone_list.with_untracked(|clone_list| {
            *vec.lock().unwrap() = clone_list
                .iter()
                .filter(move |(_, note_button_clone)| note_button_clone.animate(time))
                .map(|(key, _)| key)
                .collect();
        });
        remove_selected_clones(&vec.lock().unwrap(), set_clone_list);
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
                    <TimingButton />
                    <TimingButton />
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
    fn TimingButton() -> impl IntoView {
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
                {..interact::interactive(|| ()).0}
            />
        }
    }

    #[component]
    fn NoteFlashcard() -> impl IntoView {
        view! {
            <div
                {..flashcard()}
                style:width="300px"
                style:height="402px"
                style:margin-left="25px"
                style:margin-right="25px"
                style:padding="0px 23px"
            ></div>
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

    /// This struct describes a moving clone of a note button.
    struct NoteButtonClone {
        /// The `u8` from 0 to 6 that describes the note.
        note: u8,
        /// The time when the clone was created. The value may be `None` if the clone has not run an update loop yet.
        start_time: ReadSignal<Option<f64>>,
        /// A `WriteSignal` to the time when the clone was created.
        set_start_time: WriteSignal<Option<f64>>,
        /// The coordinates at which the clone started to exist.
        start_coordinates: Coordinates,
        /// A `ReadSignal` to the coordinates of the note button.
        coordinates: ReadSignal<Coordinates>,
        /// A `WriteSignal` to the coordinates of the note button.
        set_coordinates: WriteSignal<Coordinates>,
    }

    impl NoteButtonClone {
        /// Creates an `impl IntoView` containing the HTML rendition of the current note button clone.
        fn view(&self) -> impl IntoView + use<> {
            view! {
                <NoteButton
                    note=self.note
                    attribute=()
                    target_ref=NodeRef::new()
                    coordinates=self.coordinates
                />
            }
        }

        /// Creates a new `NoteButtonCloneArc` from a note number and an HTML `Button` target.
        fn new(note: u8, target: NodeRef<Button>) -> Self {
            let start_coordinates =
                super::get_position_untransformed(target.get_untracked().unwrap().as_ref());
            let (coordinates, set_coordinates) = APPLET.add_signal(start_coordinates.clone());
            let (start_time, set_start_time) = APPLET.add_signal(None);
            Self {
                note,
                start_time,
                set_start_time,
                start_coordinates,
                coordinates,
                set_coordinates,
            }
        }

        /// Animates the `NoteButtonClone` using the current `time` as an `f64`.
        fn animate(&self, time: f64) -> bool {
            let window_size = leptos_use_window_size::use_window_size();
            let window_size = (
                window_size.width.get_untracked(),
                window_size.height.get_untracked(),
            );
            let new_coordinates = Coordinates(
                self.start_coordinates.0,
                self.get_elapsed_time(time)
                    .powi(2)
                    .mul_add(0.0001, self.start_coordinates.1),
            );
            self.set_coordinates.set(new_coordinates.clone());
            !((-window_size.0..window_size.0 * 2_f64).contains(&new_coordinates.0)
                && (-window_size.1..window_size.1 * 2_f64).contains(&new_coordinates.1))
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

    /// The trigger keys that cause `NoteButtons` to be pressed.
    const TRIGGERS: [[&str; 7]; 3] = [
        ["KeyA", "KeyB", "KeyC", "KeyD", "KeyE", "KeyF", "KeyG"],
        [
            "Digit1", "Digit2", "Digit3", "Digit4", "Digit5", "Digit6", "Digit7",
        ],
        [
            "Numpad1", "Numpad2", "Numpad3", "Numpad4", "Numpad5", "Numpad6", "Numpad7",
        ],
    ];
    /// The letters shown on the note buttons.
    const LETTERS: [char; 7] = ['A', 'B', 'C', 'D', 'E', 'F', 'G'];
    /// The subscript numbers shown on the note buttons.
    const NUMBERS: [char; 7] = [
        '\u{2081}', '\u{2082}', '\u{2083}', '\u{2084}', '\u{2085}', '\u{2086}', '\u{2087}',
    ];

    /// This function converts the input, a `u8` from 0 to 6, into a clickable note button using a `WriteSignal` to a list of `NoteButtonClone`s.
    fn into_note_button(
        note: u8,
        set_clone_list: WriteSignal<Slab<NoteButtonClone>>,
    ) -> impl IntoView {
        let target_ref = NodeRef::new();
        let (attribute, interaction) = interact::interactive(move || {
            add_clone(note, set_clone_list, target_ref);
        });
        add_triggers_to_interaction(note, interaction.set_is_active);
        view! { <NoteButton note attribute target_ref /> }
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

    /// Triggers `set_is_active` when the `TRIGGERS` are triggered.
    fn add_triggers_to_interaction(note: u8, set_is_active: WriteSignal<bool>) {
        let (triggered, set_triggered) = APPLET.add_signal([false; TRIGGERS.len()]);
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
        note: u8,
        keyboard_event: &KeyboardEvent,
        triggered: ReadSignal<[bool; TRIGGERS.len()]>,
        set_triggered: WriteSignal<[bool; TRIGGERS.len()]>,
        set_is_active: WriteSignal<bool>,
    ) {
        for (i, trigger) in TRIGGERS.into_iter().enumerate() {
            if keyboard_event.code() == trigger[note as usize] {
                set_triggered.update(|triggers| triggers[i] = IS_KEYPRESS);
                break;
            }
        }
        if triggered
            .get()
            .into_iter()
            .any(|boolean| boolean == IS_KEYPRESS)
        {
            set_is_active.set(IS_KEYPRESS);
        }
    }

    /// This function adds a `NoteButtonClone` for `note` to a list of `NoteButtonClone`s.
    fn add_clone(
        note: u8,
        set_clone_list: WriteSignal<Slab<NoteButtonClone>>,
        target: NodeRef<Button>,
    ) {
        set_clone_list.update(|slab| {
            slab.insert(NoteButtonClone::new(note, target));
        });
    }

    #[component]
    fn NoteButton(
        /// The `u8` from 0 to 6 that describes the note.
        note: u8,
        /// The attribute that describes how the note button is interactive.
        attribute: impl Attribute,
        /// The `NodeRef` that should represent the note button.
        target_ref: NodeRef<Button>,
        /// The coordinates of the note button.
        #[prop(optional)]
        coordinates: Option<ReadSignal<Coordinates>>,
    ) -> impl IntoView {
        let letter = *LETTERS.get(note as usize).unwrap_or(&'0');
        let number = *NUMBERS.get(note as usize).unwrap_or(&'\u{2080}');
        let letter_number = [letter, number].iter().collect::<String>();
        let coordinate_attribute = coordinates.map_or(Either::Left(()), |coordinates| {
            Either::Right(view! {
                <{..}
                    style:margin="0"
                    style:position="fixed"
                    style:left=move || format!("{}px", coordinates.get().0)
                    style:top=move || format!("{}px", coordinates.get().1)
                    style:transition="none"
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
                {..coordinate_attribute}
            >
                {letter_number}
            </button>
        }
    }

    #[component]
    fn NoteButtons(
        /// A `WriteSignal` to a list of `NoteButtonClone`s.
        set_clone_list: WriteSignal<Slab<NoteButtonClone>>,
    ) -> impl IntoView {
        view! { <div>{(0..7).map(|number| into_note_button(number, set_clone_list)).collect_view()}</div> }
    }
}
