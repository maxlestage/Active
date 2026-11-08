//! Runs an [`App`] in a canvas: the keyboard and the pointer go to its input, each frame
//! runs its schedules and draws what its camera sees.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wasm_bindgen::JsCast;

use super::With;
use super::app::App;
use super::input::{KeyCode, MouseButton, PendingInput};
use super::pbr::RenderFrame;
use super::window::{PrimaryWindow, Window};
use crate::scene::math::Vec2;
use crate::scene::renderer::Listeners;
use crate::scene::{AnimationLoop, Renderer, animation_loop};

/// An app running in a canvas: it stops, and lets the canvas go, when dropped.
#[must_use = "the app stops when this handle is dropped"]
pub struct Running {
    app: Rc<RefCell<App>>,
    _frames: AnimationLoop,
    _listeners: Listeners,
}

impl Running {
    /// The app, to look at or change between frames.
    pub fn app(&self) -> std::cell::RefMut<'_, App> {
        self.app.borrow_mut()
    }
}

impl std::fmt::Debug for Running {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Running")
    }
}

impl App {
    /// Runs the app in `canvas` (the element given to `on_mount` or `use_action`), until the
    /// returned handle is dropped or a system writes `AppExit`.
    ///
    /// ```ignore
    /// canvas().class("game").use_action(|canvas| {
    ///     let mut app = App::new();
    ///     app.add_plugins(DefaultPlugins).add_systems(Startup, setup).add_systems(Update, spin);
    ///     let running = app.run_in(canvas).ok();
    ///     move || drop(running)
    /// })
    /// ```
    pub fn run_in(mut self, canvas: &crate::event::Dom) -> Result<Running, String> {
        let mut renderer = Renderer::new(canvas)?;
        let window = web_sys::window().ok_or("no window")?;
        renderer.set_pixel_ratio(window.device_pixel_ratio().min(2.0));
        let canvas: web_sys::HtmlElement = canvas
            .clone()
            .dyn_into()
            .map_err(|_| "not an HTML element".to_owned())?;
        // The canvas takes the keyboard once clicked (or reached with Tab).
        if canvas.get_attribute("tabindex").is_none() {
            canvas.set_tab_index(0);
        }
        let _ = canvas.style().set_property("touch-action", "none");
        let fit = self
            .world()
            .query_filtered::<&Window, With<PrimaryWindow>>()
            .single(self.world())
            .is_ok_and(|w| w.fit_canvas_to_parent);
        if fit {
            let style = canvas.style();
            let _ = style.set_property("width", "100%");
            let _ = style.set_property("height", "100%");
            let _ = style.set_property("display", "block");
        }
        self.world_mut().init_resource::<PendingInput>();
        let app = Rc::new(RefCell::new(self));
        let listeners = input_listeners(&canvas, app.clone());

        let exited = Rc::new(Cell::new(false));
        let (frame_app, frame_canvas) = (app.clone(), canvas.clone());
        let mut title = None::<String>;
        let frames = animation_loop(move |_time| {
            if exited.get() {
                return;
            }
            let Ok(mut app) = frame_app.try_borrow_mut() else {
                return;
            };
            let ratio = renderer.pixel_ratio() as f32;
            let size = Vec2::new(
                frame_canvas.client_width() as f32,
                frame_canvas.client_height() as f32,
            );
            let focused = frame_canvas
                .owner_document()
                .and_then(|d| d.active_element())
                .is_some_and(|e| {
                    let element: &web_sys::Element = &frame_canvas;
                    &e == element
                });
            for mut window in app
                .world()
                .query_filtered::<&mut Window, With<PrimaryWindow>>()
                .iter(app.world())
            {
                let resolution = &mut window.resolution;
                if (resolution.width, resolution.height, resolution.scale_factor)
                    != (size.x, size.y, ratio)
                {
                    resolution.width = size.x.max(1.0);
                    resolution.height = size.y.max(1.0);
                    resolution.scale_factor = ratio;
                }
                if window.focused != focused {
                    window.focused = focused;
                }
                if window.title != title {
                    title.clone_from(&window.title);
                    if let (Some(t), Some(document)) = (&title, frame_canvas.owner_document()) {
                        document.set_title(t);
                    }
                }
            }
            app.update();
            if app.should_exit().is_some() {
                exited.set(true);
                return;
            }
            renderer.aspect(); // follows the canvas's size
            if let Some(frame) = RenderFrame::extract(app.world(), size) {
                renderer.draw_frame(&frame.frame());
            }
        });
        Ok(Running {
            app,
            _frames: frames,
            _listeners: listeners,
        })
    }
}

/// `App::run` in a page: the canvas of `Window::canvas`, or a new one filling the page.
#[cfg(target_arch = "wasm32")]
pub(crate) fn run_in_page(app: App) -> Result<(), String> {
    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or("no document")?;
    let selector = app
        .world()
        .query_filtered::<&Window, With<PrimaryWindow>>()
        .single(app.world())
        .ok()
        .and_then(|w| w.canvas.clone());
    let canvas = match selector {
        Some(selector) => document
            .query_selector(&selector)
            .ok()
            .flatten()
            .ok_or_else(|| format!("no element matches `{selector}` (Window::canvas)"))?,
        None => {
            let canvas = document
                .create_element("canvas")
                .map_err(|_| "cannot create a canvas")?;
            let _ = canvas.set_attribute("style", "display:block;width:100vw;height:100vh");
            let body = document.body().ok_or("no <body>")?;
            body.append_child(&canvas)
                .map_err(|_| "cannot add the canvas")?;
            canvas
        }
    };
    // The app runs as long as the page.
    std::mem::forget(app.run_in(&canvas)?);
    Ok(())
}

/// Sends the canvas's keyboard and pointer events to the app's input.
fn input_listeners(canvas: &web_sys::HtmlElement, app: Rc<RefCell<App>>) -> Listeners {
    let mut listeners = Listeners::new(canvas);
    let send = move |f: &dyn Fn(&mut PendingInput)| {
        if let Ok(app) = app.try_borrow()
            && let Some(mut pending) = app.world().get_resource_mut::<PendingInput>()
        {
            f(&mut pending);
        }
    };
    let send = Rc::new(send);

    for (name, down) in [("keydown", true), ("keyup", false)] {
        let send = send.clone();
        listeners.on_not_passive(name, move |e| {
            let Some(e) = e.dyn_ref::<web_sys::KeyboardEvent>() else {
                return;
            };
            let Some(key) = KeyCode::from_code(&e.code()) else {
                return;
            };
            // The keys a game uses must not scroll the page while it has them.
            if matches!(
                key,
                KeyCode::ArrowUp
                    | KeyCode::ArrowDown
                    | KeyCode::ArrowLeft
                    | KeyCode::ArrowRight
                    | KeyCode::Space
            ) {
                e.prevent_default();
            }
            if !e.repeat() || !down {
                send(&|p| p.keys.push((key, down)));
            }
        });
    }
    let blur = send.clone();
    listeners.on("blur", move |_| blur(&|p| p.blurred = true));

    let last = Rc::new(Cell::new(None::<Vec2>));
    let (down, el, pressed_at) = (send.clone(), canvas.clone(), last.clone());
    listeners.on("pointerdown", move |e| {
        let Some(e) = e.dyn_ref::<web_sys::PointerEvent>() else {
            return;
        };
        // The keyboard comes with the click, without scrolling the page to the canvas.
        let options = web_sys::FocusOptions::new();
        options.set_prevent_scroll(true);
        let _ = el.focus_with_options(&options);
        let _ = el.set_pointer_capture(e.pointer_id());
        let (button, at) = (MouseButton::from_dom(e.button()), position(&el, e));
        pressed_at.set(Some(at));
        // A finger has no position before it touches: give it with the press.
        down(&|p| {
            p.cursor = Some(Some(at));
            p.buttons.push((button, true));
        });
    });
    for name in ["pointerup", "pointercancel"] {
        let (up, el) = (send.clone(), canvas.clone());
        listeners.on(name, move |e| {
            let Some(e) = e.dyn_ref::<web_sys::PointerEvent>() else {
                return;
            };
            let (button, at) = (MouseButton::from_dom(e.button()), position(&el, e));
            up(&|p| {
                p.cursor = Some(Some(at));
                p.buttons.push((button, false));
            });
        });
    }
    // The motion from one position to the next: `movementX` is 0 for fingers in some
    // browsers.
    let (moved, el, previous) = (send.clone(), canvas.clone(), last.clone());
    listeners.on("pointermove", move |e| {
        let Some(e) = e.dyn_ref::<web_sys::PointerEvent>() else {
            return;
        };
        let at = position(&el, e);
        let delta = match previous.replace(Some(at)) {
            Some(before) => at - before,
            None => Vec2::new(e.movement_x() as f32, e.movement_y() as f32),
        };
        moved(&|p| {
            p.cursor = Some(Some(at));
            p.motion.x += delta.x;
            p.motion.y += delta.y;
        });
    });
    let (left, forget) = (send.clone(), last.clone());
    listeners.on("pointerleave", move |e| {
        // The next motion starts where the pointer comes back.
        forget.set(None);
        // A finger leaves when it lifts, in the frame of its tap: keep where it was.
        if e.dyn_ref::<web_sys::PointerEvent>()
            .is_some_and(|e| e.pointer_type() == "mouse")
        {
            left(&|p| p.cursor = Some(None));
        }
    });
    let (wheel, el) = (send, canvas.clone());
    listeners.on_not_passive("wheel", move |e| {
        let Some(e) = e.dyn_ref::<web_sys::WheelEvent>() else {
            return;
        };
        // Only while the canvas has the keyboard: otherwise the page scrolls.
        let focused = el
            .owner_document()
            .and_then(|d| d.active_element())
            .is_some_and(|a| {
                let element: &web_sys::Element = &el;
                &a == element
            });
        if !focused {
            return;
        }
        e.prevent_default();
        let pixels = match e.delta_mode() {
            1 => 16.0,
            2 => 100.0,
            _ => 1.0,
        };
        let delta = Vec2::new(
            (-e.delta_x() * pixels) as f32,
            (-e.delta_y() * pixels) as f32,
        );
        wheel(&|p| {
            p.scroll.x += delta.x;
            p.scroll.y += delta.y;
        });
    });
    listeners.on("contextmenu", |e| e.prevent_default());
    listeners
}

/// Where the pointer is on the element, in CSS pixels from its top left corner.
fn position(element: &web_sys::HtmlElement, e: &web_sys::PointerEvent) -> Vec2 {
    let rect = element.get_bounding_client_rect();
    Vec2::new(
        (f64::from(e.client_x()) - rect.left()) as f32,
        (f64::from(e.client_y()) - rect.top()) as f32,
    )
}
