//! 2D: a camera that shows one unit per pixel ([`Camera2d`]), images ([`Sprite`]) and flat
//! shapes ([`Mesh2d`] with a [`ColorMaterial`]), drawn by the same renderer as the 3D.
//!
//! ```
//! use active::ecs::prelude::*;
//!
//! fn setup(
//!     mut commands: Commands,
//!     mut meshes: ResMut<Assets<Mesh>>,
//!     mut materials: ResMut<Assets<ColorMaterial>>,
//!     mut images: ResMut<Assets<Image>>,
//! ) {
//!     commands.spawn(Camera2d);
//!     let ball = images.add(Image::new(1, 1, vec![255, 200, 0, 255]));
//!     commands.spawn((
//!         Sprite { custom_size: Some(Vec2::new(32.0, 32.0)), ..Sprite::from_image(ball) },
//!         Transform::from_xyz(-100.0, 0.0, 0.0),
//!     ));
//!     commands.spawn((Sprite::from_color(Color::srgb(0.2, 0.6, 1.0), Vec2::new(50.0, 20.0)),));
//!     commands.spawn((
//!         Mesh2d(meshes.add(Circle::new(40.0))),
//!         MeshMaterial2d(materials.add(Color::srgb(0.9, 0.3, 0.4))),
//!         Transform::from_xyz(100.0, 0.0, 0.0),
//!     ));
//! }
//!
//! let mut app = App::new();
//! app.add_plugins(DefaultPlugins).add_systems(Startup, setup);
//! app.update();
//! let frame = RenderFrame::extract(app.world(), Vec2::new(800.0, 600.0)).unwrap();
//! assert_eq!(frame.mesh_count(), 3);
//! ```
//!
//! The camera looks along `-z`: a larger `z` is drawn in front. Sprites are blended with
//! what is behind them, farthest first.

use std::fmt;

use super::pbr::{
    Assets, Camera, Color, DrawItem, Exposure, Handle, Image, Mesh, OrthographicProjection,
    Projection, ScalingMode, Tonemapping,
};
use super::transform::{GlobalTransform, Transform, Visibility};
use super::world::{Component, Entity, World, require};
use crate::scene::math::{Mat4, Quat, Vec2, Vec3};
use crate::scene::{self, Geometry, PlaneGeometry};

thread_local! {
    static QUAD: Geometry = PlaneGeometry::new(1.0, 1.0);
}

impl OrthographicProjection {
    /// The default for 2D: one unit per pixel, from `z = -1000` to `1000`.
    pub fn default_2d() -> OrthographicProjection {
        OrthographicProjection {
            near: -1000.0,
            far: 1000.0,
            scaling_mode: ScalingMode::WindowSize,
            scale: 1.0,
        }
    }
}

/// A 2D camera: `commands.spawn(Camera2d)`. One unit is one pixel, `(0, 0)` the middle
/// of the window, `y` up; colors are shown as they are (no tone mapping).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Camera2d;

impl Component for Camera2d {
    fn on_insert(world: &mut World, entity: Entity) {
        if world.get::<Projection>(entity).is_none() {
            world.insert(
                entity,
                Projection::Orthographic(OrthographicProjection::default_2d()),
            );
        }
        if world.get::<Tonemapping>(entity).is_none() {
            world.insert(entity, Tonemapping::None);
        }
        require::<Camera>(world, entity);
        require::<Exposure>(world, entity);
        require::<Transform>(world, entity);
        require::<Visibility>(world, entity);
    }
}

/// A rectangle, in pixels of an image: `Rect::new(0.0, 0.0, 16.0, 16.0)`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// Its top left corner (the top of an image is `y = 0`).
    pub min: Vec2,
    /// Its bottom right corner.
    pub max: Vec2,
}

impl Rect {
    /// The rectangle between two corners.
    pub fn new(x0: f32, y0: f32, x1: f32, y1: f32) -> Rect {
        Rect {
            min: Vec2::new(x0.min(x1), y0.min(y1)),
            max: Vec2::new(x0.max(x1), y0.max(y1)),
        }
    }

    /// Its size.
    pub fn size(&self) -> Vec2 {
        Vec2::new(self.max.x - self.min.x, self.max.y - self.min.y)
    }
}

/// An image in 2D (or in 3D, facing `+z`), sized in pixels: `Sprite::from_image(handle)`.
#[derive(Clone, PartialEq)]
pub struct Sprite {
    /// The image (`None`: a rectangle of `color`, sized by `custom_size`).
    pub image: Option<Handle<Image>>,
    /// Multiplied by the image.
    pub color: Color,
    /// Mirrored left to right.
    pub flip_x: bool,
    /// Mirrored top to bottom.
    pub flip_y: bool,
    /// Its size; `None`: the image's (or of `rect`).
    pub custom_size: Option<Vec2>,
    /// Only this part of the image (a sprite sheet), in pixels.
    pub rect: Option<Rect>,
}

impl Default for Sprite {
    fn default() -> Sprite {
        Sprite {
            image: None,
            color: Color::WHITE,
            flip_x: false,
            flip_y: false,
            custom_size: None,
            rect: None,
        }
    }
}

impl fmt::Debug for Sprite {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sprite")
            .field("image", &self.image)
            .field("color", &self.color)
            .field("custom_size", &self.custom_size)
            .finish_non_exhaustive()
    }
}

impl Sprite {
    /// An image at its size.
    pub fn from_image(image: Handle<Image>) -> Sprite {
        Sprite {
            image: Some(image),
            ..Sprite::default()
        }
    }

    /// A rectangle of one color.
    pub fn from_color(color: Color, size: Vec2) -> Sprite {
        Sprite {
            color,
            custom_size: Some(size),
            ..Sprite::default()
        }
    }

    /// Its size in pixels, when known (an image still downloading has none).
    pub fn size(&self, images: &Assets<Image>) -> Option<Vec2> {
        self.custom_size
            .or(self.rect.map(|r| r.size()))
            .or_else(|| images.get(self.image?)?.size())
    }
}

impl Component for Sprite {
    fn on_insert(world: &mut World, entity: Entity) {
        require::<Anchor>(world, entity);
        require::<Transform>(world, entity);
        require::<Visibility>(world, entity);
    }
}

/// Which point of a sprite is on its position: `Anchor::CENTER` (the default),
/// `Anchor::BOTTOM_LEFT`, or any `Anchor(Vec2::new(x, y))` from -0.5 to 0.5.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Anchor(pub Vec2);

impl Anchor {
    /// The middle.
    pub const CENTER: Anchor = Anchor(Vec2::new(0.0, 0.0));
    /// The bottom left corner.
    pub const BOTTOM_LEFT: Anchor = Anchor(Vec2::new(-0.5, -0.5));
    /// The middle of the bottom edge.
    pub const BOTTOM_CENTER: Anchor = Anchor(Vec2::new(0.0, -0.5));
    /// The bottom right corner.
    pub const BOTTOM_RIGHT: Anchor = Anchor(Vec2::new(0.5, -0.5));
    /// The middle of the left edge.
    pub const CENTER_LEFT: Anchor = Anchor(Vec2::new(-0.5, 0.0));
    /// The middle of the right edge.
    pub const CENTER_RIGHT: Anchor = Anchor(Vec2::new(0.5, 0.0));
    /// The top left corner.
    pub const TOP_LEFT: Anchor = Anchor(Vec2::new(-0.5, 0.5));
    /// The middle of the top edge.
    pub const TOP_CENTER: Anchor = Anchor(Vec2::new(0.0, 0.5));
    /// The top right corner.
    pub const TOP_RIGHT: Anchor = Anchor(Vec2::new(0.5, 0.5));
}

impl Component for Anchor {}

/// How a [`ColorMaterial`]'s alpha is used.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum AlphaMode2d {
    /// Opaque, whatever the alpha.
    Opaque,
    /// Invisible where the alpha is below the threshold, opaque elsewhere.
    Mask(f32),
    /// Blended with what is behind (the default).
    #[default]
    Blend,
}

/// The material of flat shapes: a color, and maybe an image, lights ignored.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorMaterial {
    /// The color (multiplied by the image).
    pub color: Color,
    /// How its alpha is used.
    pub alpha_mode: AlphaMode2d,
    /// An image.
    pub texture: Option<Handle<Image>>,
}

impl Default for ColorMaterial {
    fn default() -> ColorMaterial {
        ColorMaterial {
            color: Color::WHITE,
            alpha_mode: AlphaMode2d::Blend,
            texture: None,
        }
    }
}

impl From<Color> for ColorMaterial {
    fn from(color: Color) -> ColorMaterial {
        ColorMaterial {
            color,
            ..ColorMaterial::default()
        }
    }
}

impl From<Handle<Image>> for ColorMaterial {
    fn from(texture: Handle<Image>) -> ColorMaterial {
        ColorMaterial {
            texture: Some(texture),
            ..ColorMaterial::default()
        }
    }
}

impl ColorMaterial {
    fn to_scene_material(&self, images: &Assets<Image>) -> scene::Material {
        let (transparent, alpha_test) = match self.alpha_mode {
            AlphaMode2d::Opaque => (false, 0.0),
            AlphaMode2d::Mask(t) => (false, t.clamp(0.0, 1.0)),
            AlphaMode2d::Blend => (true, 0.0),
        };
        unlit(
            self.color,
            transparent,
            alpha_test,
            self.texture
                .and_then(|h| images.get(h))
                .map(|i| i.texture().clone()),
        )
    }
}

fn unlit(
    color: Color,
    transparent: bool,
    alpha_test: f32,
    map: Option<scene::Texture>,
) -> scene::Material {
    scene::Material::Basic(scene::MeshBasicMaterial {
        surface: scene::Surface {
            color: color.into(),
            opacity: color.alpha.clamp(0.0, 1.0),
            transparent,
            side: scene::Side::Double,
            wireframe: false,
        },
        map,
        alpha_test,
    })
}

/// A flat shape in 2D: `Mesh2d(meshes.add(Circle::new(50.0)))`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mesh2d(pub Handle<Mesh>);

impl Component for Mesh2d {
    fn on_insert(world: &mut World, entity: Entity) {
        require::<Transform>(world, entity);
        require::<Visibility>(world, entity);
    }
}

/// The material of a [`Mesh2d`]: `MeshMaterial2d(materials.add(Color::WHITE))`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshMaterial2d<M: 'static = ColorMaterial>(pub Handle<M>);

impl<M: 'static> Component for MeshMaterial2d<M> {}

/// The sprites and 2D meshes, for the renderer.
pub(crate) fn extract_2d(
    world: &World,
    meshes: &Assets<Mesh>,
    images: &Assets<Image>,
    visible: &dyn Fn(Entity) -> bool,
    out: &mut Vec<DrawItem>,
) {
    let quad = QUAD.with(Geometry::clone);
    for (e, sprite, at) in world
        .query::<(Entity, &Sprite, &GlobalTransform)>()
        .iter(world)
    {
        if !visible(e) {
            continue;
        }
        let image = sprite.image.and_then(|h| images.get(h));
        // An image not there yet is not drawn (a color without an image is).
        if sprite.image.is_some() && image.is_none() {
            continue;
        }
        let Some(size) = sprite.size(images) else {
            continue;
        };
        let anchor = world.get::<Anchor>(e).map_or(Vec2::ZERO, |a| a.0);
        let mut map = image.map(|i| i.texture().clone());
        if let (Some(texture), Some(rect), Some(full)) =
            (map.as_mut(), sprite.rect, image.and_then(Image::size))
        {
            // A part of the image: the coordinates are scaled and moved to it.
            let (w, h) = (full.x.max(1.0), full.y.max(1.0));
            let part = rect.size();
            texture.repeat = Vec2::new(part.x / w, part.y / h);
            texture.offset = Vec2::new(rect.min.x / w, 1.0 - rect.max.y / h);
        }
        let flip = Vec3::new(
            if sprite.flip_x { -1.0 } else { 1.0 },
            if sprite.flip_y { -1.0 } else { 1.0 },
            1.0,
        );
        let local = Mat4::compose(
            Vec3::new(-anchor.x * size.x, -anchor.y * size.y, 0.0),
            Quat::IDENTITY,
            Vec3::new(size.x * flip.x, size.y * flip.y, 1.0),
        );
        out.push(DrawItem {
            geometry: quad.clone(),
            material: unlit(sprite.color, true, 0.0, map),
            world: at.to_matrix() * local,
            cast_shadow: false,
            receive_shadow: false,
        });
    }
    let materials = world.get_resource::<Assets<ColorMaterial>>();
    let Some(materials) = materials.as_deref() else {
        return;
    };
    for (e, mesh, material, at) in world
        .query::<(
            Entity,
            &Mesh2d,
            &MeshMaterial2d<ColorMaterial>,
            &GlobalTransform,
        )>()
        .iter(world)
    {
        if !visible(e) {
            continue;
        }
        if let (Some(mesh), Some(material)) = (meshes.get(mesh.0), materials.get(material.0)) {
            out.push(DrawItem {
                geometry: mesh.geometry().clone(),
                material: material.to_scene_material(images),
                world: at.to_matrix(),
                cast_shadow: false,
                receive_shadow: false,
            });
        }
    }
}
