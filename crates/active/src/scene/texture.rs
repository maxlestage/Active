//! Textures: images laid on a surface (`THREE.Texture`, `THREE.DataTexture`,
//! `TextureLoader`), from pixels made in Rust or from an image the browser downloads.
//!
//! ```
//! use active::scene::*;
//!
//! // A 2 × 2 checkerboard, top row first, RGBA.
//! let checker = Texture::from_rgba8(2, 2, vec![
//!     255, 255, 255, 255,   0, 0, 0, 255,
//!     0, 0, 0, 255,   255, 255, 255, 255,
//! ]).nearest().repeat(8.0, 8.0);
//! let floor = Mesh::new(PlaneGeometry::new(10.0, 10.0), MeshStandardMaterial::new().map(checker));
//! // An image of the site (or of any server that allows it):
//! let painting = Texture::from_url("/images/monet.jpg");
//! assert_eq!(painting.size(), None); // known once the browser has it
//! ```

use std::cell::Cell;
use std::fmt;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};

use super::math::Vector2;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Where a texture's pixels come from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextureSource {
    /// Pixels: 4 bytes (red, green, blue, alpha) per pixel, the top row first.
    Rgba8 {
        /// Its width, in pixels.
        width: u32,
        /// Its height, in pixels.
        height: u32,
        /// `width × height × 4` bytes.
        data: Rc<[u8]>,
    },
    /// An image (PNG, JPEG, WebP, SVG…) downloaded by the browser: a path of the site, or
    /// the address of a server that allows it (CORS).
    Url(String),
}

/// How the texture repeats beyond its edges.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Wrapping {
    /// The edge pixels stretch (the default).
    #[default]
    ClampToEdge,
    /// The image repeats.
    Repeat,
    /// The image repeats, mirrored every other time.
    MirroredRepeat,
}

/// How pixels are blended when the texture is magnified or shrunk.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Filter {
    /// Smooth, with mipmaps when shrunk (the default).
    #[default]
    Linear,
    /// Sharp pixels (pixel art).
    Nearest,
}

/// What the pixels hold.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ColorSpace {
    /// Colors as images store them (photos, drawings): the default.
    #[default]
    Srgb,
    /// Raw numbers (normal, roughness or data maps).
    Linear,
}

struct Inner {
    id: u64,
    source: TextureSource,
    /// The image's size, once known (at once for pixels, when loaded for an image).
    size: Cell<Option<(u32, u32)>>,
    /// Bumped by [`Texture::needs_update`]: the renderer uploads the pixels again.
    version: Cell<u32>,
}

/// A texture: cheap to clone (the clones share the image, uploaded once), with its
/// settings (`wrap`, `repeat`, `offset`, filters…) per clone.
#[derive(Clone)]
pub struct Texture {
    inner: Rc<Inner>,
    /// Repeating beyond the horizontal edges.
    pub wrap_s: Wrapping,
    /// Repeating beyond the vertical edges.
    pub wrap_t: Wrapping,
    /// How many times the image fits across the surface (`(2, 2)`: four times).
    pub repeat: Vector2,
    /// Where the image starts, in fractions of it.
    pub offset: Vector2,
    /// A rotation of the image around `center`, in radians.
    pub rotation: f32,
    /// The point the image turns around (`(0.5, 0.5)`: its middle).
    pub center: Vector2,
    /// When magnified.
    pub mag_filter: Filter,
    /// When shrunk.
    pub min_filter: Filter,
    /// What the pixels hold.
    pub color_space: ColorSpace,
    /// The first row at the top, as in image files (true by default).
    pub flip_y: bool,
}

impl Texture {
    fn with(source: TextureSource, size: Option<(u32, u32)>) -> Texture {
        Texture {
            inner: Rc::new(Inner {
                id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
                source,
                size: Cell::new(size),
                version: Cell::new(0),
            }),
            wrap_s: Wrapping::ClampToEdge,
            wrap_t: Wrapping::ClampToEdge,
            repeat: Vector2::new(1.0, 1.0),
            offset: Vector2::new(0.0, 0.0),
            rotation: 0.0,
            center: Vector2::new(0.0, 0.0),
            mag_filter: Filter::Linear,
            min_filter: Filter::Linear,
            color_space: ColorSpace::Srgb,
            flip_y: true,
        }
    }

    /// A texture from pixels: 4 bytes per pixel (red, green, blue, alpha), the top row first
    /// (`new THREE.DataTexture(data, width, height)`). Missing bytes are transparent black.
    pub fn from_rgba8(width: u32, height: u32, data: impl Into<Vec<u8>>) -> Texture {
        let (width, height) = (width.max(1), height.max(1));
        let mut data = data.into();
        data.resize(width as usize * height as usize * 4, 0);
        Texture::with(
            TextureSource::Rgba8 {
                width,
                height,
                data: data.into(),
            },
            Some((width, height)),
        )
    }

    /// A texture made pixel by pixel: `f(x, y)` gives the color of each, as RGBA bytes,
    /// `(0, 0)` being the top left.
    pub fn from_fn(width: u32, height: u32, f: impl Fn(u32, u32) -> [u8; 4]) -> Texture {
        let mut data = Vec::with_capacity(width as usize * height as usize * 4);
        for y in 0..height.max(1) {
            for x in 0..width.max(1) {
                data.extend_from_slice(&f(x, y));
            }
        }
        Texture::from_rgba8(width, height, data)
    }

    /// One pixel of one color: RGBA bytes.
    pub fn solid(rgba: [u8; 4]) -> Texture {
        Texture::from_rgba8(1, 1, rgba.to_vec())
    }

    /// An image the browser downloads (`new THREE.TextureLoader().load(url)`): until it
    /// arrives, the surface shows without it.
    pub fn from_url(url: impl Into<String>) -> Texture {
        Texture::with(TextureSource::Url(url.into()), None)
    }

    /// Where its pixels come from.
    pub fn source(&self) -> &TextureSource {
        &self.inner.source
    }

    /// Its identity: clones share it.
    pub fn id(&self) -> u64 {
        self.inner.id
    }

    /// The image's size in pixels, once known: at once for pixels, once loaded for an
    /// image from an address.
    pub fn size(&self) -> Option<(u32, u32)> {
        self.inner.size.get()
    }

    /// Sets the size (the renderer does, once an image has arrived).
    pub fn set_size(&self, width: u32, height: u32) {
        self.inner.size.set(Some((width, height)));
    }

    /// How many times the pixels were changed ([`needs_update`](Self::needs_update)).
    pub fn version(&self) -> u32 {
        self.inner.version.get()
    }

    /// Asks the renderer to upload the image again (`texture.needsUpdate = true`), after
    /// the browser's image changed (an address to reload).
    pub fn needs_update(&self) {
        self.inner
            .version
            .set(self.inner.version.get().wrapping_add(1));
    }

    /// Repeats in both directions, `x` times across and `y` times up.
    pub fn repeat(mut self, x: f32, y: f32) -> Texture {
        self.wrap_s = Wrapping::Repeat;
        self.wrap_t = Wrapping::Repeat;
        self.repeat = Vector2::new(x, y);
        self
    }

    /// How it repeats beyond its edges, across and up.
    pub fn wrap(mut self, s: Wrapping, t: Wrapping) -> Texture {
        self.wrap_s = s;
        self.wrap_t = t;
        self
    }

    /// Starts the image this far, in fractions of it.
    pub fn offset(mut self, x: f32, y: f32) -> Texture {
        self.offset = Vector2::new(x, y);
        self
    }

    /// Sharp pixels, for pixel art.
    pub fn nearest(mut self) -> Texture {
        self.mag_filter = Filter::Nearest;
        self.min_filter = Filter::Nearest;
        self
    }

    /// Raw numbers rather than colors (data maps).
    pub fn linear(mut self) -> Texture {
        self.color_space = ColorSpace::Linear;
        self
    }

    pub(crate) fn linear_if_data(mut self, data: bool) -> Texture {
        if data {
            self.color_space = ColorSpace::Linear;
        }
        self
    }

    /// Keeps the bottom row first (the pixels are given bottom row first).
    pub fn flip_y(mut self, flip: bool) -> Texture {
        self.flip_y = flip;
        self
    }

    /// The 3 × 3 matrix the shaders apply to the coordinates (column-major): repeat,
    /// rotation around `center`, then offset (`texture.matrix` of three.js).
    pub fn uv_matrix(&self) -> [f32; 9] {
        let (sx, sy) = (self.repeat.x, self.repeat.y);
        let (c, s) = (self.rotation.cos(), self.rotation.sin());
        let (cx, cy) = (self.center.x, self.center.y);
        let (tx, ty) = (self.offset.x, self.offset.y);
        [
            sx * c,
            -sy * s,
            0.0,
            sx * s,
            sy * c,
            0.0,
            -sx * (c * cx + s * cy) + cx + tx,
            -sy * (-s * cx + c * cy) + cy + ty,
            1.0,
        ]
    }
}

impl PartialEq for Texture {
    fn eq(&self, other: &Texture) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
            && self.wrap_s == other.wrap_s
            && self.wrap_t == other.wrap_t
            && self.repeat == other.repeat
            && self.offset == other.offset
            && self.rotation == other.rotation
            && self.center == other.center
            && self.mag_filter == other.mag_filter
            && self.min_filter == other.min_filter
            && self.color_space == other.color_space
            && self.flip_y == other.flip_y
    }
}

impl fmt::Debug for Texture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_struct("Texture");
        d.field("id", &self.inner.id);
        match &self.inner.source {
            TextureSource::Rgba8 { width, height, .. } => d.field("pixels", &(width, height)),
            TextureSource::Url(url) => d.field("url", url),
        };
        d.field("repeat", &self.repeat).finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pixels_know_their_size_and_clones_share_them() {
        let t = Texture::from_fn(3, 2, |x, y| [x as u8, y as u8, 0, 255]);
        assert_eq!(t.size(), Some((3, 2)));
        let TextureSource::Rgba8 { data, .. } = t.source() else {
            panic!("pixels expected");
        };
        assert_eq!(data.len(), 24);
        assert_eq!(&data[4 * 4..4 * 4 + 4], &[1, 1, 0, 255]);
        let copy = t.clone().repeat(2.0, 2.0);
        assert_eq!(copy.id(), t.id());
        assert_ne!(copy, t);
        copy.needs_update();
        assert_eq!(t.version(), 1);
        // Too few bytes: the rest is transparent.
        assert_eq!(Texture::from_rgba8(2, 2, vec![255; 4]).size(), Some((2, 2)));
        let url = Texture::from_url("/a.png");
        assert_eq!(url.size(), None);
        url.set_size(640, 480);
        assert_eq!(url.size(), Some((640, 480)));
    }

    #[test]
    fn the_uv_matrix_repeats_then_offsets() {
        let t = Texture::solid([255; 4]).repeat(2.0, 3.0).offset(0.5, 0.25);
        let m = t.uv_matrix();
        let at = |u: f32, v: f32| (m[0] * u + m[3] * v + m[6], m[1] * u + m[4] * v + m[7]);
        assert_eq!(at(0.0, 0.0), (0.5, 0.25));
        assert_eq!(at(1.0, 1.0), (2.5, 3.25));
        let mut turned = Texture::solid([255; 4]);
        turned.rotation = std::f32::consts::FRAC_PI_2;
        turned.center = Vector2::new(0.5, 0.5);
        let m = turned.uv_matrix();
        // (1, 0) turns a quarter around the middle: to (0, 0).
        let (u, v) = (m[0] + m[6], m[1] + m[7]);
        assert!(u.abs() < 1e-5 && v.abs() < 1e-5, "{u} {v}");
    }
}
