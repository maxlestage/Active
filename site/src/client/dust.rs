//! Hero animation: particles that gather into the word "Active", flee the pointer and
//! scatter while the page scrolls. Simulation and drawing are Rust.

use std::cell::RefCell;
use std::rc::Rc;

use active::State;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, Element, HtmlCanvasElement};

use super::{on, reduced_motion, request_frame, window};

struct Dust {
    pos: Vec<f32>,
    vel: Vec<f32>,
    home: Vec<f32>,
    drift: Vec<f32>,
    stiffness: Vec<f32>,
    width: f32,
    height: f32,
    time: f32,
}

impl Dust {
    fn new(count: usize, width: f32, height: f32, seed: u32) -> Dust {
        let mut rng = Rng(seed | 1);
        let mut pos = Vec::with_capacity(count * 2);
        let mut drift = Vec::with_capacity(count * 2);
        let mut stiffness = Vec::with_capacity(count);
        for _ in 0..count {
            pos.push(rng.next() * width);
            pos.push(rng.next() * height);
            let angle = rng.next() * std::f32::consts::TAU;
            let reach = 0.25 + rng.next() * 0.75;
            drift.push(angle.cos() * reach);
            drift.push(angle.sin() * reach);
            stiffness.push(0.55 + rng.next() * 0.9);
        }
        Dust {
            home: pos.clone(),
            vel: vec![0.0; count * 2],
            pos,
            drift,
            stiffness,
            width,
            height,
            time: 0.0,
        }
    }

    fn count(&self) -> usize {
        self.stiffness.len()
    }

    /// Target points (x, y pairs), reused cyclically when there are fewer than particles.
    fn set_home(&mut self, points: &[f32]) {
        let n = points.len() / 2;
        if n == 0 {
            return;
        }
        for i in 0..self.count() {
            let j = i % n;
            self.home[i * 2] = points[j * 2];
            self.home[i * 2 + 1] = points[j * 2 + 1];
        }
    }

    /// Spring towards home, soft noise, pointer repulsion; `scatter` (0..1) blows them away.
    fn step(&mut self, dt: f32, pointer: Option<(f32, f32)>, scatter: f32) {
        let dt = dt.clamp(0.0, 1.0 / 30.0);
        self.time += dt;
        let t = self.time;
        let damping = (-3.6 * dt).exp();
        let radius = 110.0_f32;
        let spread = self.width.max(self.height) * 0.9 * scatter * scatter;
        for i in 0..self.count() {
            let (ix, iy) = (i * 2, i * 2 + 1);
            let (x, y) = (self.pos[ix], self.pos[iy]);
            let tx = self.home[ix] + self.drift[ix] * spread;
            let ty = self.home[iy] + self.drift[iy] * spread;
            let k = 7.0 * self.stiffness[i];
            let phase = i as f32 * 0.37;
            let mut ax = (tx - x) * k + (y * 0.011 + t * 0.9 + phase).sin() * 26.0;
            let mut ay = (ty - y) * k + (x * 0.013 - t * 0.8 + phase).cos() * 26.0;
            if let Some((px, py)) = pointer {
                let (dx, dy) = (x - px, y - py);
                let d2 = dx * dx + dy * dy;
                if d2 < radius * radius && d2 > 0.01 {
                    let d = d2.sqrt();
                    let f = (1.0 - d / radius) * 5200.0;
                    ax += dx / d * f;
                    ay += dy / d * f;
                }
            }
            self.vel[ix] = (self.vel[ix] + ax * dt) * damping;
            self.vel[iy] = (self.vel[iy] + ay * dt) * damping;
            self.pos[ix] = x + self.vel[ix] * dt;
            self.pos[iy] = y + self.vel[iy] * dt;
        }
    }
}

/// xorshift32, values in [0, 1).
struct Rng(u32);
impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// Points of the word "Active" drawn in the display font, in canvas pixels.
fn word_points(width: u32, height: u32, mobile: bool) -> Vec<f32> {
    let doc = window().document().unwrap();
    let canvas: HtmlCanvasElement = doc.create_element("canvas").unwrap().unchecked_into();
    canvas.set_width(width);
    canvas.set_height(height);
    let ctx: CanvasRenderingContext2d = canvas.get_context("2d").unwrap().unwrap().unchecked_into();
    let (w, h) = (width as f64, height as f64);
    let size = (w / if mobile { 3.5 } else { 3.6 }).min(h * if mobile { 0.2 } else { 0.3 });
    ctx.set_font(&format!(
        "600 {}px \"Geist\", system-ui, sans-serif",
        size as u32
    ));
    ctx.set_text_align("center");
    ctx.set_text_baseline("middle");
    let _ = ctx.fill_text("Active", w / 2.0, h * if mobile { 0.25 } else { 0.28 });
    let Ok(image) = ctx.get_image_data(0.0, 0.0, w, h) else {
        return Vec::new();
    };
    let data = image.data();
    let step = ((size / 55.0).round() as u32).max(2);
    let mut points = Vec::new();
    for y in (0..height).step_by(step as usize) {
        for x in (0..width).step_by(step as usize) {
            if data[((y * width + x) * 4 + 3) as usize] > 140 {
                points.push(x as f32);
                points.push(y as f32);
            }
        }
    }
    // Shuffle, so particles pick targets all over the word.
    let mut rng = Rng(0x9e37_79b9);
    let n = points.len() / 2;
    for i in (1..n).rev() {
        let j = (rng.next() * (i + 1) as f32) as usize;
        points.swap(i * 2, j * 2);
        points.swap(i * 2 + 1, j * 2 + 1);
    }
    points
}

struct Scene {
    canvas: HtmlCanvasElement,
    ctx: CanvasRenderingContext2d,
    dust: Option<Dust>,
    dpr: f64,
    pointer: Option<(f32, f32)>,
    visible: bool,
    last: f64,
    fps: f64,
    last_hud: f64,
}

impl Scene {
    fn setup(&mut self, particles: State<usize>) {
        let rect = self.canvas.get_bounding_client_rect();
        let (w, h) = (
            ((rect.width() * self.dpr).round() as u32).max(1),
            ((rect.height() * self.dpr).round() as u32).max(1),
        );
        self.canvas.set_width(w);
        self.canvas.set_height(h);
        let mobile = rect.width() < 720.0;
        let density = if mobile { 140.0 } else { 210.0 };
        let count = ((rect.width() * rect.height()) / density).clamp(1400.0, 5200.0) as usize;
        let mut dust = match self.dust.take() {
            Some(d) if d.count() == count => d,
            _ => Dust::new(
                count,
                w as f32,
                h as f32,
                (js_sys::Math::random() * 2f64.powi(31)) as u32,
            ),
        };
        dust.width = w as f32;
        dust.height = h as f32;
        dust.set_home(&word_points(w, h, mobile));
        self.dust = Some(dust);
        particles.set(count);
    }

    fn draw(&self) {
        let Some(dust) = &self.dust else { return };
        let (w, h) = (self.canvas.width() as f64, self.canvas.height() as f64);
        let s = 1.35 * self.dpr;
        self.ctx.clear_rect(0.0, 0.0, w, h);
        self.ctx.set_fill_style_str("rgba(237, 233, 227, 0.82)");
        let (points, _) = dust.pos.as_chunks::<2>();
        for (i, [x, y]) in points.iter().enumerate() {
            if i % 11 != 0 {
                self.ctx.fill_rect(*x as f64, *y as f64, s, s);
            }
        }
        self.ctx.set_fill_style_str("#ff5a1f");
        for [x, y] in points.iter().step_by(11) {
            self.ctx.fill_rect(*x as f64, *y as f64, s * 1.3, s * 1.3);
        }
    }
}

pub fn start(canvas: &Element, particles: State<usize>, fps: State<u32>, live: State<bool>) {
    let canvas: HtmlCanvasElement = canvas.clone().unchecked_into();
    let ctx: CanvasRenderingContext2d = match canvas.get_context("2d") {
        Ok(Some(ctx)) => ctx.unchecked_into(),
        _ => return,
    };
    let scene = Rc::new(RefCell::new(Scene {
        canvas: canvas.clone(),
        ctx,
        dust: None,
        dpr: window().device_pixel_ratio().min(2.0),
        pointer: None,
        visible: true,
        last: 0.0,
        fps: 60.0,
        last_hud: 0.0,
    }));

    // The word is sampled in the display font: wait until fonts are loaded.
    let ready = window().document().unwrap().fonts().ready();
    let scene_ready = scene.clone();
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(promise) = ready {
            let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
        }
        scene_ready.borrow_mut().setup(particles);
        live.set(true);
        if reduced_motion() {
            // No motion: let the particles settle, then draw one still frame.
            let mut s = scene_ready.borrow_mut();
            if let Some(d) = s.dust.as_mut() {
                for _ in 0..240 {
                    d.step(1.0 / 30.0, None, 0.0);
                }
            }
            s.draw();
            return;
        }
        animate(scene_ready, fps);
    });

    // Pointer, visibility and resize.
    let hero = canvas
        .closest(".hero")
        .ok()
        .flatten()
        .unwrap_or_else(|| canvas.clone().into());
    let s = scene.clone();
    on(&hero, "pointermove", move |e: web_sys::PointerEvent| {
        let mut s = s.borrow_mut();
        let rect = s.canvas.get_bounding_client_rect();
        s.pointer = Some((
            ((e.client_x() as f64 - rect.left()) * s.dpr) as f32,
            ((e.client_y() as f64 - rect.top()) * s.dpr) as f32,
        ));
    });
    let s = scene.clone();
    on(&hero, "pointerleave", move |_: web_sys::Event| {
        s.borrow_mut().pointer = None
    });
    let s = scene.clone();
    let observer = Closure::<dyn Fn(js_sys::Array)>::new(move |entries: js_sys::Array| {
        if let Some(entry) = entries
            .get(0)
            .dyn_ref::<web_sys::IntersectionObserverEntry>()
        {
            s.borrow_mut().visible = entry.is_intersecting();
        }
    });
    if let Ok(io) = web_sys::IntersectionObserver::new(observer.as_ref().unchecked_ref()) {
        io.observe(&canvas);
    }
    observer.forget();
    let s = scene;
    let pending = Rc::new(RefCell::new(None::<i32>));
    on(&window(), "resize", move |_: web_sys::Event| {
        let s = s.clone();
        if let Some(id) = pending.borrow_mut().take() {
            window().clear_timeout_with_handle(id);
        }
        let done = pending.clone();
        let cb = Closure::once_into_js(move || {
            done.borrow_mut().take();
            let mut scene = s.borrow_mut();
            scene.setup(particles);
            if reduced_motion() {
                scene.draw();
            }
        });
        *pending.borrow_mut() = window()
            .set_timeout_with_callback_and_timeout_and_arguments_0(cb.unchecked_ref(), 150)
            .ok();
    });
}

fn animate(scene: Rc<RefCell<Scene>>, fps: State<u32>) {
    request_frame(move |now| {
        let mut s = scene.borrow_mut();
        let dt = if s.last == 0.0 {
            1.0 / 60.0
        } else {
            ((now - s.last) / 1000.0).min(0.05)
        };
        s.last = now;
        let hidden = window().document().is_some_and(|d| d.hidden());
        if s.visible && !hidden {
            let scatter = (window().scroll_y().unwrap_or(0.0)
                / (window()
                    .inner_height()
                    .ok()
                    .and_then(|h| h.as_f64())
                    .unwrap_or(800.0)
                    * 0.9))
                .clamp(0.0, 1.0) as f32;
            let pointer = s.pointer;
            if let Some(d) = s.dust.as_mut() {
                d.step(dt as f32, pointer, scatter);
            }
            s.draw();
            s.fps += (1.0 / dt.max(0.001) - s.fps) * 0.05;
            if now - s.last_hud > 500.0 {
                s.last_hud = now;
                let rounded = s.fps.round() as u32;
                drop(s);
                fps.set(rounded);
            }
        }
        true
    });
}
