use handler::APPLET;
use home::AppletView;
use leptos::prelude::*;
use leptos::tachys::html::attribute::Attribute;
use std::sync::OnceLock;
use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;

/// The current version of the app, a `usize` in the form of an `&str`.
const VERSION: &str = env!("BUILD_VERSION");

/// A `OnceLock` that stores whether the current device is touch-screen.
static IS_TOUCHSCREEN: OnceLock<bool> = OnceLock::new();

#[wasm_bindgen(start)]
fn main() {
    APPLET.set_applet(AppletView::Home);
    console_error_panic_hook::set_once();
    IS_TOUCHSCREEN.set(is_touchscreen()).unwrap();
    leptos::mount::mount_to_body(move || {
        view! {
            <style>
                "
                *, *::before, *::after {
                    font-family: monospace;
                    transition: all 0.3s ease;
                }
                "
            </style>
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
        return true;
    }
    if let Ok(Some(media_match)) = window.match_media("(any-pointer: coarse)")
        && media_match.matches()
    {
        return true;
    }
    false
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

/// Gets the position of a `&HtmlElement` without considering `style:transform` attributes.
fn get_position_untransformed(element: &HtmlElement) -> (f64, f64) {
    let mut x = 0_f64;
    let mut y = 0_f64;
    let mut current = Some(element.clone());
    while let Some(other_element) = current {
        x += f64::from(other_element.offset_left());
        y += f64::from(other_element.offset_top());
        if let Some(parent) = other_element.parent_element() {
            x -= f64::from(parent.scroll_left());
            x -= f64::from(parent.scroll_top());
        }
        current = other_element.offset_parent().map(JsCast::unchecked_into);
    }
    (x, y)
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

    /// This function creates and returns an `impl Attribute` and an `Interaction` object. It provides `ReadSignal`s for when a HTML element is hovered over, active, or disabled. It also changes the cursor accordingly:
    ///
    /// | interaction | cursor      |
    /// | :---------: | :---------: |
    /// | hover       | pointer     |
    /// | disabled    | not-allowed |
    pub fn interaction(
        handler: &'static impl Handler,
        mut on_click: impl FnMut() + 'static,
    ) -> (impl Attribute, Interaction) {
        let (is_hovered, set_is_hovered) = handler.add_signal(false);
        let (is_active, set_is_active) = handler.add_signal(false);
        let (is_disabled, set_is_disabled) = handler.add_signal(false);
        let mut new_on_click = if IS_TOUCHSCREEN.get() == Some(&false) {
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
        };
        let clicked = move || {
            set_is_active.set(true);
            handler.add_listener_once(ev::mouseup, move |_| set_is_active.set(false));
            handler.add_listener_once(ev::pointerup, move |_| set_is_active.set(false));
            handler.add_listener_once(ev::pointercancel, move |_| {
                set_is_active.set(false);
                set_is_hovered.set(false);
            });
            handler.add_listener_once(ev::touchend, move |_| {
                set_is_active.set(false);
                set_is_hovered.set(false);
            });
            handler.add_listener_once(ev::touchcancel, move |_| {
                set_is_active.set(false);
                set_is_hovered.set(false);
            });
        };
        (
            view! {
                <{..}
                    on:mouseup=move |_| set_is_active.set(false)
                    on:mousedown=move |_| {
                        if IS_TOUCHSCREEN.get() == Some(&false) {
                            clicked();
                        }
                    }
                    on:mouseenter=move |_| set_is_hovered.set(true)
                    on:mouseleave=move |_| set_is_hovered.set(false)
                    on:pointerup=move |_| set_is_active.set(false)
                    on:pointerdown=move |_| clicked()
                    on:pointerenter=move |_| set_is_hovered.set(true)
                    on:pointerleave=move |_| set_is_hovered.set(false)
                    on:pointercancel=move |_| {
                        set_is_active.set(false);
                        set_is_hovered.set(false);
                    }
                    on:touchend=move |_| {
                        set_is_active.set(false);
                        set_is_hovered.set(false);
                    }
                    on:touchstart=move |_| {
                        set_is_hovered.set(true);
                        clicked();
                    }
                    on:touchcancel=move |_| {
                        set_is_active.set(false);
                        set_is_hovered.set(false);
                    }
                    on:click=move |_| {
                        if IS_TOUCHSCREEN.get() == Some(&true) {
                            set_is_active.set(false);
                            set_is_hovered.set(false);
                            if let Some(ref mut on_click) = new_on_click {
                                on_click();
                            }
                        }
                    }
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
        fn add_new_signal<T>(&self, signal: (ReadSignal<T>, WriteSignal<T>))
        where
            T: 'static;

        /// Creates a `(ReadSignal<T>, WriteSignal<T>)` and adds both of them to the handle list.
        fn add_signal<T>(&self, value: T) -> (ReadSignal<T>, WriteSignal<T>)
        where
            T: Send + Sync + 'static,
        {
            let value = signal(value);
            self.add_new_signal(value);
            value
        }

        /// Creates a `window_event_listener`, then disposes of the event listener once it is called.
        fn add_listener_once<T>(&self, event: T, cb: impl Fn(T::EventType) + 'static)
        where
            T: EventDescriptor + 'static,
            T::EventType: JsCast,
        {
            let (handle, set_handle) = self.add_signal(None::<ListenerHandle>);
            set_handle.set(Some(self.add_listener(event, move |event_object| {
                cb(event_object);
                set_handle.update(|handle| {
                    if let Some(handle) = handle.take() {
                        AppletHandle::Window(handle).rid();
                    }
                });
                handle.dispose();
                set_handle.dispose();
            })));
        }

        /// Creates a `window_event_listener()` call and returns a `ListenerHandle`.
        fn add_listener<T>(&self, event: T, cb: impl Fn(T::EventType) + 'static) -> ListenerHandle
        where
            T: EventDescriptor + 'static,
            T::EventType: JsCast,
        {
            let handle = ListenerHandle::new(window_event_listener::<T>(event, cb));
            self.add_handle(AppletHandle::Window(handle.clone()));
            handle
        }

        /// Creates an `Effect::watch` effect handled by the handler.
        fn add_effect<T>(
            &self,
            dependency_fn: impl FnMut() -> T + 'static,
            mut handler: impl FnMut(&T, Option<&T>) + 'static,
        ) where
            T: 'static,
        {
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
            self.0.dispose();
            self.1.dispose();
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

        /// If the `StaticAppletHandler` is uninitializes, it is initialized starting from `applet_view`. Otherwise, the current applet is set to `applet_view`, and all handles are cleared.
        pub fn set_applet(&self, applet_view: AppletView) {
            *self.each_frame.lock().unwrap() = None;
            let mut handler = self.inner.lock().unwrap();
            if let Some(handler) = handler.as_mut() {
                handler.set_current_applet.set(applet_view);
                for applet_handle in mem::take(&mut handler.handles) {
                    applet_handle.rid();
                }
                for signal_handle in mem::replace(
                    &mut handler.old_signals,
                    mem::take(&mut handler.new_signals),
                ) {
                    signal_handle.rid();
                }
            } else {
                let (current_applet, set_current_applet) = signal(applet_view);
                *handler = Some(AppletHandler {
                    handles: Vec::new(),
                    new_signals: Vec::new(),
                    old_signals: Vec::new(),
                    current_applet,
                    set_current_applet,
                });
            }
        }

        /// Starts the animation loop.
        pub fn animate(&'static self, each_frame: impl FnMut(f64) + Send + 'static) {
            *self.each_frame.lock().unwrap() = Some(Arc::new(Mutex::new(each_frame)));
            let closure = Rc::new(RefCell::new(None::<Closure<dyn FnMut(f64)>>));
            let clone = Rc::clone(&closure);
            *clone.borrow_mut() = Some(Closure::new(move |time| {
                if let Some(ref arc) = *self.each_frame.lock().unwrap() {
                    Arc::clone(arc).lock().unwrap()(time);
                    web_sys::window()
                        .unwrap()
                        .request_animation_frame(
                            closure.borrow().as_ref().unwrap().as_ref().unchecked_ref(),
                        )
                        .unwrap();
                }
            }));
            web_sys::window()
                .unwrap()
                .request_animation_frame(clone.borrow().as_ref().unwrap().as_ref().unchecked_ref())
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

        fn add_new_signal<T>(&self, signal: (ReadSignal<T>, WriteSignal<T>))
        where
            T: 'static,
        {
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

    use super::APPLET;
    use super::handler::Handler as _;
    use super::interact;
    use leptos::either::Either;
    use leptos::ev;
    use leptos::html::Button;
    use leptos::prelude::*;
    use leptos::tachys::html::attribute::Attribute;
    use slab::Slab;
    use std::sync::Mutex;

    #[component]
    pub fn App() -> impl IntoView {
        let (clone_list, set_clone_list) = APPLET.add_signal(Slab::<NoteButtonClone>::new());
        APPLET.animate(move |time| {
            let vec = Mutex::new(Vec::new());
            clone_list.with_untracked(|clone_list| {
                *vec.lock().unwrap() = clone_list
                    .iter()
                    .filter(move |(_, note_button_clone)| {
                        let start_time = note_button_clone
                            .start_time
                            .get_untracked()
                            .unwrap_or_else(|| {
                                note_button_clone.set_start_time.set(Some(time));
                                time
                            });
                        let elapsed_time = time - start_time;
                        let window_size = leptos_use_window_size::use_window_size();
                        let window_size = (
                            window_size.width.get_untracked(),
                            window_size.height.get_untracked(),
                        );
                        let new_coordinates = (
                            note_button_clone.start_coordinates.0,
                            elapsed_time.mul_add(1_f64, note_button_clone.start_coordinates.1),
                        );
                        note_button_clone.set_coordinates.set(new_coordinates);
                        !((-window_size.0..window_size.0 * 2_f64).contains(&new_coordinates.0)
                            && (-window_size.1..window_size.1 * 2_f64).contains(&new_coordinates.1))
                    })
                    .map(|(key, _)| key)
                    .collect::<Vec<_>>();
            });
            set_clone_list.update(|clone_list| {
                for key in vec.lock().unwrap().iter() {
                    clone_list.try_remove(*key);
                }
            });
        });
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
                            clone_list.with(|clone_list| clone_list
                                .iter()
                                .map(|(_, note_button_clone)| note_button_clone.view())
                                .collect_view())
                        }}
                    </super::Center>
                </div>
            </div>
        }
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
        start_coordinates: (f64, f64),
        /// A `ReadSignal` to the coordinates of the note button. The first item in the tuple is the x-coordinate, and the second item in the tuple is the y-coordinate.
        coordinates: ReadSignal<(f64, f64)>,
        /// A `WriteSignal` to the coordinates of the note button.
        set_coordinates: WriteSignal<(f64, f64)>,
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
            let (coordinates, set_coordinates) = APPLET.add_signal(start_coordinates);
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
    }

    /// This function converts the input, a `u8` from 0 to 6, into a clickable note button using a `WriteSignal` to a list of `NoteButtonClone`s.
    fn into_note_button(
        note: u8,
        set_clone_list: WriteSignal<Slab<NoteButtonClone>>,
    ) -> impl IntoView {
        const TRIGGERS: [[&str; 7]; 3] = [
            ["KeyA", "KeyB", "KeyC", "KeyD", "KeyE", "KeyF", "KeyG"],
            [
                "Digit1", "Digit2", "Digit3", "Digit4", "Digit5", "Digit6", "Digit7",
            ],
            [
                "Numpad1", "Numpad2", "Numpad3", "Numpad4", "Numpad5", "Numpad6", "Numpad7",
            ],
        ];

        let target_ref = NodeRef::new();
        let (attribute, interaction) = interact::interactive(move || {
            add_clone(note, set_clone_list, target_ref);
        });
        let (triggered, set_triggered) = APPLET.add_signal([false; TRIGGERS.len()]);
        APPLET.add_listener(ev::keypress, move |keyboard_event| {
            for (i, trigger) in TRIGGERS.into_iter().enumerate() {
                if keyboard_event.code() == trigger[note as usize] {
                    set_triggered.update(|triggers| triggers[i] = true);
                    break;
                }
            }
            if triggered.get().into_iter().any(|boolean| boolean) {
                interaction.set_is_active.set(true);
            }
        });
        APPLET.add_listener(ev::keyup, move |keyboard_event| {
            for (i, trigger) in TRIGGERS.into_iter().enumerate() {
                if keyboard_event.code() == trigger[note as usize] {
                    set_triggered.update(|triggers| triggers[i] = false);
                    break;
                }
            }
            if triggered.get().into_iter().all(|boolean| !boolean) {
                interaction.set_is_active.set(false);
            }
        });
        view! { <NoteButton note attribute target_ref /> }
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
        /// The coordinates of the note button. The first item in the tuple is the x-coordinate, and the second item in the tuple is the y-coordinate.
        #[prop(optional)]
        coordinates: Option<ReadSignal<(f64, f64)>>,
    ) -> impl IntoView {
        const LETTERS: [char; 7] = ['A', 'B', 'C', 'D', 'E', 'F', 'G'];
        const NUMBERS: [char; 7] = [
            '\u{2081}', '\u{2082}', '\u{2083}', '\u{2084}', '\u{2085}', '\u{2086}', '\u{2087}',
        ];

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
