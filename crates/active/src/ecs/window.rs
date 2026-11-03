//! The window: in a page, the canvas the app draws in.

use super::app::{App, Plugin};
use super::world::Component;
use crate::three::math::Vec2;

/// The canvas the app draws in, on an entity with [`PrimaryWindow`]:
/// `window: Single<&Window>`, then `window.width()`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Window {
    /// The title of the page (`document.title`), if set.
    pub title: Option<String>,
    /// The canvas to draw in, as a CSS selector (`"#game"`), for `App::run` in a page;
    /// without it, a canvas is added to the page.
    pub canvas: Option<String>,
    /// The canvas fills its parent element.
    pub fit_canvas_to_parent: bool,
    /// The size, in CSS pixels, and the device pixel ratio.
    pub resolution: WindowResolution,
    /// The pointer, in CSS pixels from the top left corner, while it is over the canvas.
    pub cursor_position: Option<Vec2>,
    /// Whether the canvas has the keyboard.
    pub focused: bool,
}

impl Component for Window {}

impl Window {
    /// The width, in CSS pixels.
    pub fn width(&self) -> f32 {
        self.resolution.width
    }

    /// The height, in CSS pixels.
    pub fn height(&self) -> f32 {
        self.resolution.height
    }

    /// The size, in CSS pixels.
    pub fn size(&self) -> Vec2 {
        Vec2::new(self.resolution.width, self.resolution.height)
    }

    /// The width, in device pixels.
    pub fn physical_width(&self) -> u32 {
        (self.resolution.width * self.resolution.scale_factor).round() as u32
    }

    /// The height, in device pixels.
    pub fn physical_height(&self) -> u32 {
        (self.resolution.height * self.resolution.scale_factor).round() as u32
    }

    /// Device pixels per CSS pixel.
    pub fn scale_factor(&self) -> f32 {
        self.resolution.scale_factor
    }

    /// The pointer, while it is over the canvas.
    pub fn cursor_position(&self) -> Option<Vec2> {
        self.cursor_position
    }
}

/// The size of a [`Window`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowResolution {
    /// The width, in CSS pixels.
    pub width: f32,
    /// The height, in CSS pixels.
    pub height: f32,
    /// Device pixels per CSS pixel.
    pub scale_factor: f32,
}

impl Default for WindowResolution {
    fn default() -> WindowResolution {
        WindowResolution {
            width: 1280.0,
            height: 720.0,
            scale_factor: 1.0,
        }
    }
}

impl WindowResolution {
    /// A size in CSS pixels.
    pub fn new(width: f32, height: f32) -> WindowResolution {
        WindowResolution {
            width,
            height,
            scale_factor: 1.0,
        }
    }
}

/// Marks the entity of the app's window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PrimaryWindow;

impl Component for PrimaryWindow {}

/// Spawns the window entity.
#[derive(Clone, Debug)]
pub struct WindowPlugin {
    /// The window (`None`: no window).
    pub primary_window: Option<Window>,
}

impl Default for WindowPlugin {
    fn default() -> WindowPlugin {
        WindowPlugin {
            primary_window: Some(Window::default()),
        }
    }
}

impl Plugin for WindowPlugin {
    fn build(&self, app: &mut App) {
        if let Some(window) = &self.primary_window {
            app.world_mut().spawn((window.clone(), PrimaryWindow));
        }
    }
}
