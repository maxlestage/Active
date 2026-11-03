//! 3D the way Bevy writes it: assets (`Assets<Mesh>`, `Assets<StandardMaterial>`), mesh
//! primitives, materials, lights in physical units, cameras with an exposure and a tone
//! mapping, drawn by [`active::three`](crate::three)'s renderer.

use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_4, PI, TAU};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

use super::app::{App, Plugin, PostUpdate};
use super::query::{Query, With};
use super::transform::{GlobalTransform, InheritedVisibility, Transform, Visibility};
use super::window::{PrimaryWindow, Window};
use super::world::{Component, Entity, Resource, World, require};
use crate::three::math::{Color as Linear, Mat4, Quat, Vec2, Vec3, linear_to_srgb, srgb_to_linear};
use crate::three::{self, BufferGeometry, Geometry, Lights, PointLightData, ToneMapping};

// ----- Colors ---------------------------------------------------------------------------

/// A color with its opacity, as Bevy writes it: `Color::srgb(0.8, 0.7, 0.6)`,
/// `Color::WHITE`, `Color::srgb_u8(124, 144, 255)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    /// Red, from 0 to 1 (sRGB, as color pickers show it).
    pub red: f32,
    /// Green.
    pub green: f32,
    /// Blue.
    pub blue: f32,
    /// The opacity, from 0 (invisible) to 1.
    pub alpha: f32,
}

impl Default for Color {
    fn default() -> Color {
        Color::WHITE
    }
}

impl Color {
    /// White.
    pub const WHITE: Color = Color::srgb(1.0, 1.0, 1.0);
    /// Black.
    pub const BLACK: Color = Color::srgb(0.0, 0.0, 0.0);
    /// Transparent.
    pub const NONE: Color = Color::srgba(0.0, 0.0, 0.0, 0.0);

    /// An sRGB color (components from 0 to 1).
    pub const fn srgb(red: f32, green: f32, blue: f32) -> Color {
        Color {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }

    /// An sRGB color with an opacity.
    pub const fn srgba(red: f32, green: f32, blue: f32, alpha: f32) -> Color {
        Color {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// An sRGB color from bytes (0 to 255).
    pub const fn srgb_u8(red: u8, green: u8, blue: u8) -> Color {
        Color::srgba_u8(red, green, blue, 255)
    }

    /// An sRGB color with an opacity, from bytes.
    pub const fn srgba_u8(red: u8, green: u8, blue: u8, alpha: u8) -> Color {
        Color::srgba(
            red as f32 / 255.0,
            green as f32 / 255.0,
            blue as f32 / 255.0,
            alpha as f32 / 255.0,
        )
    }

    /// A color in linear light (what lighting adds up).
    pub fn linear_rgb(red: f32, green: f32, blue: f32) -> Color {
        Color::linear_rgba(red, green, blue, 1.0)
    }

    /// A color in linear light, with an opacity.
    pub fn linear_rgba(red: f32, green: f32, blue: f32, alpha: f32) -> Color {
        Color::srgba(
            linear_to_srgb(red),
            linear_to_srgb(green),
            linear_to_srgb(blue),
            alpha,
        )
    }

    /// A color from its hue (degrees), saturation and lightness (0 to 1).
    pub fn hsl(hue: f32, saturation: f32, lightness: f32) -> Color {
        Color::hsla(hue, saturation, lightness, 1.0)
    }

    /// The same, with an opacity.
    pub fn hsla(hue: f32, saturation: f32, lightness: f32, alpha: f32) -> Color {
        let c = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
        let h = hue.rem_euclid(360.0) / 60.0;
        let x = c * (1.0 - (h % 2.0 - 1.0).abs());
        let (r, g, b) = match h as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = lightness - c / 2.0;
        Color::srgba(r + m, g + m, b + m, alpha)
    }

    /// The same color, with another opacity.
    pub const fn with_alpha(mut self, alpha: f32) -> Color {
        self.alpha = alpha;
        self
    }

    /// The opacity.
    pub const fn alpha(&self) -> f32 {
        self.alpha
    }

    /// Changes the opacity.
    pub fn set_alpha(&mut self, alpha: f32) {
        self.alpha = alpha;
    }

    /// In linear light.
    pub fn to_linear(&self) -> LinearRgba {
        LinearRgba {
            red: srgb_to_linear(self.red),
            green: srgb_to_linear(self.green),
            blue: srgb_to_linear(self.blue),
            alpha: self.alpha,
        }
    }

    /// The sRGB components, `[r, g, b, a]`.
    pub const fn to_srgba_array(&self) -> [f32; 4] {
        [self.red, self.green, self.blue, self.alpha]
    }
}

impl From<Color> for Linear {
    fn from(c: Color) -> Linear {
        let l = c.to_linear();
        Linear::linear_rgb(l.red, l.green, l.blue)
    }
}

impl From<Linear> for Color {
    fn from(c: Linear) -> Color {
        Color::linear_rgb(c.r, c.g, c.b)
    }
}

/// A color in linear light, for emissive surfaces: `LinearRgba::rgb(13.99, 5.32, 2.0)`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LinearRgba {
    /// Red (more than 1 for glowing things).
    pub red: f32,
    /// Green.
    pub green: f32,
    /// Blue.
    pub blue: f32,
    /// The opacity.
    pub alpha: f32,
}

impl LinearRgba {
    /// Black: no light.
    pub const BLACK: LinearRgba = LinearRgba::rgb(0.0, 0.0, 0.0);
    /// White.
    pub const WHITE: LinearRgba = LinearRgba::rgb(1.0, 1.0, 1.0);

    /// A color.
    pub const fn rgb(red: f32, green: f32, blue: f32) -> LinearRgba {
        LinearRgba {
            red,
            green,
            blue,
            alpha: 1.0,
        }
    }

    /// A color with an opacity.
    pub const fn new(red: f32, green: f32, blue: f32, alpha: f32) -> LinearRgba {
        LinearRgba {
            red,
            green,
            blue,
            alpha,
        }
    }

    fn linear(self) -> Linear {
        Linear::linear_rgb(self.red, self.green, self.blue)
    }
}

impl From<LinearRgba> for Color {
    fn from(c: LinearRgba) -> Color {
        Color::linear_rgba(c.red, c.green, c.blue, c.alpha)
    }
}

impl From<Color> for LinearRgba {
    fn from(c: Color) -> LinearRgba {
        c.to_linear()
    }
}

/// Named colors (`palettes::css::TOMATO`).
pub mod palettes {
    /// The colors of CSS.
    pub mod css {
        use super::super::Color;

        macro_rules! colors {
            ($($name:ident = $r:literal, $g:literal, $b:literal;)*) => {
                $(
                    #[doc = concat!("`", stringify!($name), "` of CSS.")]
                    pub const $name: Color = Color::srgb_u8($r, $g, $b);
                )*
            };
        }

        colors! {
            BLACK = 0, 0, 0;
            WHITE = 255, 255, 255;
            RED = 255, 0, 0;
            LIME = 0, 255, 0;
            GREEN = 0, 128, 0;
            BLUE = 0, 0, 255;
            YELLOW = 255, 255, 0;
            CYAN = 0, 255, 255;
            AQUA = 0, 255, 255;
            MAGENTA = 255, 0, 255;
            FUCHSIA = 255, 0, 255;
            SILVER = 192, 192, 192;
            GRAY = 128, 128, 128;
            MAROON = 128, 0, 0;
            OLIVE = 128, 128, 0;
            PURPLE = 128, 0, 128;
            TEAL = 0, 128, 128;
            NAVY = 0, 0, 128;
            ORANGE = 255, 165, 0;
            ORANGE_RED = 255, 69, 0;
            GOLD = 255, 215, 0;
            PINK = 255, 192, 203;
            HOT_PINK = 255, 105, 180;
            TOMATO = 255, 99, 71;
            CORAL = 255, 127, 80;
            CRIMSON = 220, 20, 60;
            INDIGO = 75, 0, 130;
            VIOLET = 238, 130, 238;
            BEIGE = 245, 245, 220;
            BROWN = 165, 42, 42;
            CHOCOLATE = 210, 105, 30;
            SKY_BLUE = 135, 206, 235;
            STEEL_BLUE = 70, 130, 180;
            ROYAL_BLUE = 65, 105, 225;
            DODGER_BLUE = 30, 144, 255;
            TURQUOISE = 64, 224, 208;
            SEA_GREEN = 46, 139, 87;
            FOREST_GREEN = 34, 139, 34;
            LIME_GREEN = 50, 205, 50;
            DARK_GREEN = 0, 100, 0;
            SALMON = 250, 128, 114;
            KHAKI = 240, 230, 140;
            LAVENDER = 230, 230, 250;
            SLATE_GRAY = 112, 128, 144;
            DARK_GRAY = 169, 169, 169;
            LIGHT_GRAY = 211, 211, 211;
            WHITE_SMOKE = 245, 245, 245;
        }
    }
}

// ----- Assets ---------------------------------------------------------------------------

/// A shared asset of an [`Assets`] collection: cheap to copy, put it in a component
/// (`Mesh3d(handle)`).
pub struct Handle<T> {
    id: u32,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    /// Its number in its collection.
    pub fn id(&self) -> u32 {
        self.id
    }
}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Handle<T> {}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T> Eq for Handle<T> {}

impl<T> Hash for Handle<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T> fmt::Debug for Handle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Handle<{}>({})",
            std::any::type_name::<T>()
                .rsplit("::")
                .next()
                .unwrap_or("?"),
            self.id
        )
    }
}

impl<T> From<&Handle<T>> for Handle<T> {
    fn from(h: &Handle<T>) -> Handle<T> {
        *h
    }
}

/// The assets of one type: `ResMut<Assets<Mesh>>`, then `meshes.add(Cuboid::new(1.0, 1.0,
/// 1.0))`.
pub struct Assets<T: 'static> {
    items: HashMap<u32, T>,
    next: u32,
}

impl<T: 'static> Default for Assets<T> {
    fn default() -> Self {
        Assets {
            items: HashMap::new(),
            next: 0,
        }
    }
}

impl<T: 'static> Resource for Assets<T> {}

impl<T: 'static> Assets<T> {
    /// Adds an asset (or what turns into one) and gives its handle.
    pub fn add(&mut self, asset: impl Into<T>) -> Handle<T> {
        let id = self.next;
        self.next += 1;
        self.items.insert(id, asset.into());
        Handle {
            id,
            _marker: PhantomData,
        }
    }

    /// The asset of a handle.
    pub fn get(&self, handle: impl Into<Handle<T>>) -> Option<&T> {
        self.items.get(&handle.into().id)
    }

    /// The same, to change it.
    pub fn get_mut(&mut self, handle: impl Into<Handle<T>>) -> Option<&mut T> {
        self.items.get_mut(&handle.into().id)
    }

    /// Replaces an asset.
    pub fn insert(&mut self, handle: impl Into<Handle<T>>, asset: impl Into<T>) {
        self.items.insert(handle.into().id, asset.into());
    }

    /// Removes an asset.
    pub fn remove(&mut self, handle: impl Into<Handle<T>>) -> Option<T> {
        self.items.remove(&handle.into().id)
    }

    /// Whether the handle has an asset.
    pub fn contains(&self, handle: impl Into<Handle<T>>) -> bool {
        self.items.contains_key(&handle.into().id)
    }

    /// How many there are.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The assets and their handles.
    pub fn iter(&self) -> impl Iterator<Item = (Handle<T>, &T)> {
        self.items.iter().map(|(id, a)| {
            (
                Handle {
                    id: *id,
                    _marker: PhantomData,
                },
                a,
            )
        })
    }
}

// ----- Meshes ---------------------------------------------------------------------------

/// The shape of something drawn: triangles with normals.
#[derive(Clone, Debug)]
pub struct Mesh {
    geometry: Geometry,
}

impl Mesh {
    /// A mesh from a Three.js geometry.
    pub fn from_geometry(geometry: Geometry) -> Mesh {
        Mesh { geometry }
    }

    /// Its geometry.
    pub fn geometry(&self) -> &Geometry {
        &self.geometry
    }

    /// How many vertices it has.
    pub fn count_vertices(&self) -> usize {
        self.geometry.positions.len() / 3
    }
}

impl From<Geometry> for Mesh {
    fn from(geometry: Geometry) -> Mesh {
        Mesh { geometry }
    }
}

/// The same geometry, turned by a rotation.
fn rotated(geometry: &BufferGeometry, rotation: Quat) -> Geometry {
    let turn = |v: &[f32]| -> Vec<f32> {
        v.chunks_exact(3)
            .flat_map(|p| (rotation * Vec3::new(p[0], p[1], p[2])).to_array())
            .collect()
    };
    BufferGeometry::new(
        turn(&geometry.positions),
        turn(&geometry.normals),
        geometry.uvs.clone(),
        geometry.indices.clone(),
    )
}

/// A box: `Cuboid::new(1.0, 1.0, 1.0)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cuboid {
    /// Half its size on each axis.
    pub half_size: Vec3,
}

impl Cuboid {
    /// A box of this width (x), height (y) and depth (z).
    pub fn new(x: f32, y: f32, z: f32) -> Cuboid {
        Cuboid::from_size(Vec3::new(x, y, z))
    }

    /// A box of this size.
    pub fn from_size(size: Vec3) -> Cuboid {
        Cuboid {
            half_size: size * 0.5,
        }
    }

    /// A cube.
    pub fn from_length(length: f32) -> Cuboid {
        Cuboid::new(length, length, length)
    }
}

impl Default for Cuboid {
    fn default() -> Cuboid {
        Cuboid::from_length(1.0)
    }
}

impl From<Cuboid> for Mesh {
    fn from(c: Cuboid) -> Mesh {
        let s = c.half_size * 2.0;
        Mesh::from(three::BoxGeometry::new(s.x, s.y, s.z))
    }
}

/// A ball: `Sphere::new(0.5)`, or `Sphere::new(0.5).mesh().uv(32, 18)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sphere {
    /// Its radius.
    pub radius: f32,
}

impl Sphere {
    /// A ball of this radius.
    pub fn new(radius: f32) -> Sphere {
        Sphere { radius }
    }

    /// Its mesh, to choose how fine.
    pub fn mesh(&self) -> SphereMeshBuilder {
        SphereMeshBuilder {
            radius: self.radius,
            sectors: 32,
            stacks: 18,
        }
    }
}

impl Default for Sphere {
    fn default() -> Sphere {
        Sphere::new(0.5)
    }
}

/// How fine a sphere's mesh is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SphereMeshBuilder {
    radius: f32,
    sectors: u32,
    stacks: u32,
}

impl SphereMeshBuilder {
    /// Meridians and parallels.
    pub fn uv(mut self, sectors: u32, stacks: u32) -> SphereMeshBuilder {
        self.sectors = sectors;
        self.stacks = stacks;
        self
    }

    /// As finely as an icosphere subdivided this many times (Bevy's default mesh is
    /// `ico(5)`; this one is made of meridians and parallels too).
    pub fn ico(self, subdivisions: u32) -> SphereMeshBuilder {
        let n = 5 * (subdivisions + 1).min(12);
        self.uv(n * 2, n)
    }

    /// The mesh.
    pub fn build(&self) -> Mesh {
        Mesh::from(three::SphereGeometry::new(
            self.radius,
            self.sectors,
            self.stacks,
        ))
    }
}

impl From<SphereMeshBuilder> for Mesh {
    fn from(b: SphereMeshBuilder) -> Mesh {
        b.build()
    }
}

impl From<Sphere> for Mesh {
    fn from(s: Sphere) -> Mesh {
        s.mesh().build()
    }
}

/// A flat square facing up: `Plane3d::default().mesh().size(5.0, 5.0)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane3d {
    /// The direction it faces.
    pub normal: Vec3,
    /// Half its size.
    pub half_size: Vec2,
}

impl Default for Plane3d {
    fn default() -> Plane3d {
        Plane3d {
            normal: Vec3::Y,
            half_size: Vec2::new(0.5, 0.5),
        }
    }
}

impl Plane3d {
    /// A plane facing `normal`.
    pub fn new(normal: Vec3, half_size: Vec2) -> Plane3d {
        Plane3d {
            normal: normal.normalize(),
            half_size,
        }
    }

    /// Its mesh, to choose its size.
    pub fn mesh(&self) -> PlaneMeshBuilder {
        PlaneMeshBuilder { plane: *self }
    }
}

/// The size of a plane's mesh.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaneMeshBuilder {
    plane: Plane3d,
}

impl PlaneMeshBuilder {
    /// Its width and depth.
    pub fn size(mut self, width: f32, depth: f32) -> PlaneMeshBuilder {
        self.plane.half_size = Vec2::new(width / 2.0, depth / 2.0);
        self
    }

    /// The mesh.
    pub fn build(&self) -> Mesh {
        let size = self.plane.half_size;
        // Three.js's plane faces +z, with its height along y: turned so that y becomes -z
        // and it faces the normal.
        let flat = three::PlaneGeometry::new(size.x * 2.0, size.y * 2.0);
        let up = Quat::from_rotation_x(-PI / 2.0);
        let turn = Quat::from_unit_vectors(Vec3::Y, self.plane.normal) * up;
        Mesh::from(rotated(&flat, turn))
    }
}

impl From<PlaneMeshBuilder> for Mesh {
    fn from(b: PlaneMeshBuilder) -> Mesh {
        b.build()
    }
}

impl From<Plane3d> for Mesh {
    fn from(p: Plane3d) -> Mesh {
        p.mesh().build()
    }
}

/// A flat rectangle facing `+z`: `Rectangle::new(2.0, 1.0)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rectangle {
    /// Half its size.
    pub half_size: Vec2,
}

impl Rectangle {
    /// A rectangle of this width and height.
    pub fn new(width: f32, height: f32) -> Rectangle {
        Rectangle {
            half_size: Vec2::new(width / 2.0, height / 2.0),
        }
    }
}

impl From<Rectangle> for Mesh {
    fn from(r: Rectangle) -> Mesh {
        Mesh::from(three::PlaneGeometry::new(
            r.half_size.x * 2.0,
            r.half_size.y * 2.0,
        ))
    }
}

/// A flat disc facing `+z` (turn it to make a floor): `Circle::new(4.0)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Circle {
    /// Its radius.
    pub radius: f32,
}

impl Circle {
    /// A disc of this radius.
    pub fn new(radius: f32) -> Circle {
        Circle { radius }
    }
}

impl From<Circle> for Mesh {
    fn from(c: Circle) -> Mesh {
        const SEGMENTS: u32 = 48;
        let mut positions = vec![0.0, 0.0, 0.0];
        let mut uvs = vec![0.5, 0.5];
        for i in 0..SEGMENTS {
            let a = i as f32 / SEGMENTS as f32 * TAU;
            positions.extend([c.radius * a.cos(), c.radius * a.sin(), 0.0]);
            uvs.extend([0.5 + a.cos() / 2.0, 0.5 + a.sin() / 2.0]);
        }
        let normals = [0.0, 0.0, 1.0].repeat(SEGMENTS as usize + 1);
        let indices = (0..SEGMENTS)
            .flat_map(|i| [0, 1 + i, 1 + (i + 1) % SEGMENTS])
            .collect();
        Mesh::from(BufferGeometry::new(positions, normals, uvs, indices))
    }
}

/// A cylinder standing along `y`: `Cylinder::new(radius, height)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cylinder {
    /// Its radius.
    pub radius: f32,
    /// Half its height.
    pub half_height: f32,
}

impl Cylinder {
    /// A cylinder of this radius and height.
    pub fn new(radius: f32, height: f32) -> Cylinder {
        Cylinder {
            radius,
            half_height: height / 2.0,
        }
    }
}

impl From<Cylinder> for Mesh {
    fn from(c: Cylinder) -> Mesh {
        Mesh::from(three::CylinderGeometry::new(
            c.radius,
            c.radius,
            c.half_height * 2.0,
            32,
        ))
    }
}

/// A cone standing along `y`, its tip up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cone {
    /// The radius of its base.
    pub radius: f32,
    /// Its height.
    pub height: f32,
}

impl Cone {
    /// A cone of this radius and height.
    pub fn new(radius: f32, height: f32) -> Cone {
        Cone { radius, height }
    }
}

impl From<Cone> for Mesh {
    fn from(c: Cone) -> Mesh {
        Mesh::from(three::ConeGeometry::new(c.radius, c.height, 32))
    }
}

/// A ring lying flat (around `y`): `Torus::new(inner_radius, outer_radius)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Torus {
    /// The radius of the tube.
    pub minor_radius: f32,
    /// From the center to the middle of the tube.
    pub major_radius: f32,
}

impl Torus {
    /// A ring from `inner_radius` to `outer_radius`.
    pub fn new(inner_radius: f32, outer_radius: f32) -> Torus {
        Torus {
            minor_radius: (outer_radius - inner_radius) / 2.0,
            major_radius: (outer_radius + inner_radius) / 2.0,
        }
    }
}

impl From<Torus> for Mesh {
    fn from(t: Torus) -> Mesh {
        // Three.js's torus lies around z: turned to lie around y, like Bevy's.
        let ring = three::TorusGeometry::new(t.major_radius, t.minor_radius, 24, 48);
        Mesh::from(rotated(&ring, Quat::from_rotation_x(PI / 2.0)))
    }
}

/// The mesh of an entity: `Mesh3d(meshes.add(Cuboid::default()))`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mesh3d(pub Handle<Mesh>);

impl Component for Mesh3d {
    fn on_insert(world: &mut World, entity: Entity) {
        require::<Transform>(world, entity);
        require::<Visibility>(world, entity);
    }
}

// ----- Materials ------------------------------------------------------------------------

/// How a material's opacity is used.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum AlphaMode {
    /// Opaque, whatever the alpha.
    #[default]
    Opaque,
    /// Opaque, but invisible where the alpha is below the threshold (drawn opaque here).
    Mask(f32),
    /// Blended with what is behind.
    Blend,
    /// Blended, the color already multiplied by the alpha (drawn as `Blend` here).
    Premultiplied,
    /// Added to what is behind (drawn as `Blend` here).
    Add,
    /// Multiplied with what is behind (drawn as `Blend` here).
    Multiply,
}

/// Which faces are not drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Face {
    /// The faces seen from the front.
    Front,
    /// The faces seen from behind.
    Back,
}

/// The physically based material of Bevy: `materials.add(Color::srgb(0.3, 0.5, 0.3))`, or
/// with its fields: `StandardMaterial { base_color: .., metallic: 1.0, ..default() }`.
#[derive(Clone, Debug, PartialEq)]
pub struct StandardMaterial {
    /// The color, and its opacity (used with `alpha_mode`).
    pub base_color: Color,
    /// The light it gives off, in linear light (more than 1 glows).
    pub emissive: LinearRgba,
    /// From 0 (mirror) to 1 (rough).
    pub perceptual_roughness: f32,
    /// From 0 (plastic, wood) to 1 (metal).
    pub metallic: f32,
    /// How much light a non-metal reflects (0.5 is 4 %, kept for compatibility).
    pub reflectance: f32,
    /// How the alpha of `base_color` is used.
    pub alpha_mode: AlphaMode,
    /// Not lit: the color as it is.
    pub unlit: bool,
    /// Both faces are drawn and lit.
    pub double_sided: bool,
    /// The faces not drawn (`None`: none, like `double_sided`).
    pub cull_mode: Option<Face>,
}

impl Default for StandardMaterial {
    fn default() -> StandardMaterial {
        StandardMaterial {
            base_color: Color::WHITE,
            emissive: LinearRgba::BLACK,
            perceptual_roughness: 0.5,
            metallic: 0.0,
            reflectance: 0.5,
            alpha_mode: AlphaMode::Opaque,
            unlit: false,
            double_sided: false,
            cull_mode: Some(Face::Back),
        }
    }
}

impl From<Color> for StandardMaterial {
    fn from(base_color: Color) -> StandardMaterial {
        let alpha_mode = if base_color.alpha < 1.0 {
            AlphaMode::Blend
        } else {
            AlphaMode::Opaque
        };
        StandardMaterial {
            base_color,
            alpha_mode,
            ..StandardMaterial::default()
        }
    }
}

impl StandardMaterial {
    /// The Three.js material that draws it.
    pub fn to_three(&self) -> three::Material {
        let transparent = !matches!(self.alpha_mode, AlphaMode::Opaque | AlphaMode::Mask(_));
        let side = match (self.double_sided, self.cull_mode) {
            (true, _) | (_, None) => three::Side::Double,
            (false, Some(Face::Front)) => three::Side::Back,
            (false, Some(Face::Back)) => three::Side::Front,
        };
        let surface = three::Surface {
            color: self.base_color.into(),
            opacity: self.base_color.alpha.clamp(0.0, 1.0),
            transparent,
            side,
            wireframe: false,
        };
        if self.unlit {
            three::Material::Basic(three::MeshBasicMaterial { surface })
        } else {
            let mut standard = three::MeshStandardMaterial::new()
                .roughness(self.perceptual_roughness)
                .metalness(self.metallic)
                .emissive(self.emissive.linear());
            standard.surface = surface;
            three::Material::Standard(standard)
        }
    }
}

/// The material of an entity: `MeshMaterial3d(materials.add(Color::WHITE))`.
pub struct MeshMaterial3d<M: 'static = StandardMaterial>(pub Handle<M>);

impl<M: 'static> Clone for MeshMaterial3d<M> {
    fn clone(&self) -> Self {
        MeshMaterial3d(self.0)
    }
}

impl<M: 'static> fmt::Debug for MeshMaterial3d<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("MeshMaterial3d").field(&self.0).finish()
    }
}

impl<M: 'static> Component for MeshMaterial3d<M> {}

// ----- Lights ---------------------------------------------------------------------------

/// The units of light, as Bevy names them.
pub mod light_consts {
    /// Illuminance (for `DirectionalLight`), in lux.
    pub mod lux {
        /// A night without the moon.
        pub const MOONLESS_NIGHT: f32 = 0.0001;
        /// A night of full moon.
        pub const FULL_MOON_NIGHT: f32 = 0.05;
        /// Civil twilight.
        pub const CIVIL_TWILIGHT: f32 = 3.4;
        /// A living room.
        pub const LIVING_ROOM: f32 = 50.;
        /// A hallway.
        pub const HALLWAY: f32 = 80.;
        /// A very dark overcast day.
        pub const DARK_OVERCAST_DAY: f32 = 100.;
        /// An office.
        pub const OFFICE: f32 = 320.;
        /// A clear sunrise or sunset.
        pub const CLEAR_SUNRISE: f32 = 400.;
        /// An overcast day.
        pub const OVERCAST_DAY: f32 = 1000.;
        /// Daylight, not in direct sunlight (the default).
        pub const AMBIENT_DAYLIGHT: f32 = 10_000.;
        /// Full daylight.
        pub const FULL_DAYLIGHT: f32 = 20_000.;
        /// Direct sunlight.
        pub const DIRECT_SUNLIGHT: f32 = 100_000.;
    }

    /// Luminous power (for `PointLight`), in lumens.
    pub mod lumens {
        /// Per watt of an LED bulb.
        pub const LUMENS_PER_LED_WATTS: f32 = 90.0;
        /// Per watt of an incandescent bulb.
        pub const LUMENS_PER_INCANDESCENT_WATTS: f32 = 13.8;
        /// Per watt of a halogen bulb.
        pub const LUMENS_PER_HALOGEN_WATTS: f32 = 19.8;
        /// A very large cinema light (the default).
        pub const VERY_LARGE_CINEMA_LIGHT: f32 = 1_000_000.0;
    }
}

/// A light shining all around from a point: a bulb.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointLight {
    /// Its color.
    pub color: Color,
    /// Its power, in lumens (a 60 W bulb: 800).
    pub intensity: f32,
    /// Beyond this distance, it lights nothing.
    pub range: f32,
    /// The size of the bulb (not used here).
    pub radius: f32,
    /// Whether it casts shadows (shadows are not drawn here).
    pub shadows_enabled: bool,
}

impl Default for PointLight {
    fn default() -> PointLight {
        PointLight {
            color: Color::WHITE,
            intensity: light_consts::lumens::VERY_LARGE_CINEMA_LIGHT,
            range: 20.0,
            radius: 0.0,
            shadows_enabled: false,
        }
    }
}

impl Component for PointLight {
    fn on_insert(world: &mut World, entity: Entity) {
        require::<Transform>(world, entity);
        require::<Visibility>(world, entity);
    }
}

/// A light from far away, all in one direction (its forward, `-z`): the sun.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectionalLight {
    /// Its color.
    pub color: Color,
    /// How much light it brings, in lux.
    pub illuminance: f32,
    /// Whether it casts shadows (shadows are not drawn here).
    pub shadows_enabled: bool,
}

impl Default for DirectionalLight {
    fn default() -> DirectionalLight {
        DirectionalLight {
            color: Color::WHITE,
            illuminance: light_consts::lux::AMBIENT_DAYLIGHT,
            shadows_enabled: false,
        }
    }
}

impl Component for DirectionalLight {
    fn on_insert(world: &mut World, entity: Entity) {
        require::<Transform>(world, entity);
        require::<Visibility>(world, entity);
    }
}

/// The light everywhere, of the whole world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobalAmbientLight {
    /// Its color.
    pub color: Color,
    /// Its brightness, in candelas per square meter.
    pub brightness: f32,
    /// Kept for compatibility.
    pub affects_lightmapped_meshes: bool,
}

impl Default for GlobalAmbientLight {
    fn default() -> GlobalAmbientLight {
        GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 80.0,
            affects_lightmapped_meshes: true,
        }
    }
}

impl Resource for GlobalAmbientLight {}

impl GlobalAmbientLight {
    /// No ambient light.
    pub const NONE: GlobalAmbientLight = GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 0.0,
        affects_lightmapped_meshes: true,
    };
}

/// The ambient light seen by one camera (on the camera's entity), instead of the global one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AmbientLight {
    /// Its color.
    pub color: Color,
    /// Its brightness, in candelas per square meter.
    pub brightness: f32,
    /// Kept for compatibility.
    pub affects_lightmapped_meshes: bool,
}

impl Default for AmbientLight {
    fn default() -> AmbientLight {
        let g = GlobalAmbientLight::default();
        AmbientLight {
            color: g.color,
            brightness: g.brightness,
            affects_lightmapped_meshes: true,
        }
    }
}

impl Component for AmbientLight {}

// ----- Cameras --------------------------------------------------------------------------

/// The color behind everything: a resource, `ClearColor(Color::BLACK)`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClearColor(pub Color);

impl Default for ClearColor {
    fn default() -> ClearColor {
        ClearColor(Color::srgb_u8(43, 44, 47))
    }
}

impl Resource for ClearColor {}

/// What a camera draws behind the scene.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ClearColorConfig {
    /// The [`ClearColor`] resource.
    #[default]
    Default,
    /// This color.
    Custom(Color),
    /// Nothing: the page shows through.
    None,
}

/// What every camera has.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// Whether it draws.
    pub is_active: bool,
    /// The camera with the highest order draws.
    pub order: isize,
    /// What it draws behind the scene.
    pub clear_color: ClearColorConfig,
    computed: ComputedCamera,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ComputedCamera {
    projection: Mat4,
    viewport: Vec2,
}

impl Default for Camera {
    fn default() -> Camera {
        Camera {
            is_active: true,
            order: 0,
            clear_color: ClearColorConfig::Default,
            computed: ComputedCamera {
                projection: Mat4::IDENTITY,
                viewport: Vec2::new(1.0, 1.0),
            },
        }
    }
}

impl Component for Camera {}

/// Why a point of the screen has no ray.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewportConversionError {
    /// The camera has not drawn yet, or its projection cannot be inverted.
    InvalidData,
}

impl fmt::Display for ViewportConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the camera has no valid projection yet")
    }
}

impl std::error::Error for ViewportConversionError {}

impl Camera {
    /// The ray from the camera through a point of the canvas (CSS pixels from the top left,
    /// like `Window::cursor_position`), to find what the pointer is over.
    pub fn viewport_to_world(
        &self,
        camera_transform: &GlobalTransform,
        viewport_position: Vec2,
    ) -> Result<Ray3d, ViewportConversionError> {
        let size = self.computed.viewport;
        if size.x <= 0.0 || size.y <= 0.0 {
            return Err(ViewportConversionError::InvalidData);
        }
        let ndc = Vec2::new(
            viewport_position.x / size.x * 2.0 - 1.0,
            1.0 - viewport_position.y / size.y * 2.0,
        );
        let to_world = camera_transform.to_matrix() * self.computed.projection.inverse();
        let near = Vec3::new(ndc.x, ndc.y, -1.0).apply_matrix4(&to_world);
        let far = Vec3::new(ndc.x, ndc.y, 1.0).apply_matrix4(&to_world);
        let direction = (far - near).normalize();
        if !direction.x.is_finite() || direction == Vec3::ZERO {
            return Err(ViewportConversionError::InvalidData);
        }
        Ok(Ray3d {
            origin: near,
            direction,
        })
    }

    /// Where a point of the world is on the canvas, in CSS pixels from the top left.
    pub fn world_to_viewport(
        &self,
        camera_transform: &GlobalTransform,
        world_position: Vec3,
    ) -> Result<Vec2, ViewportConversionError> {
        let view = camera_transform.to_matrix().inverse();
        let clip = world_position.apply_matrix4(&(self.computed.projection * view));
        if !clip.x.is_finite() {
            return Err(ViewportConversionError::InvalidData);
        }
        let size = self.computed.viewport;
        Ok(Vec2::new(
            (clip.x + 1.0) / 2.0 * size.x,
            (1.0 - clip.y) / 2.0 * size.y,
        ))
    }
}

/// A half-line: from `origin`, along `direction`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray3d {
    /// Where it starts.
    pub origin: Vec3,
    /// Where it goes (a unit vector).
    pub direction: Vec3,
}

impl Ray3d {
    /// The point at `distance` along it.
    pub fn get_point(&self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }

    /// The distance at which it crosses a plane, if it does in front of its origin.
    pub fn intersect_plane(&self, plane_origin: Vec3, plane: InfinitePlane3d) -> Option<f32> {
        let denominator = plane.normal.dot(self.direction);
        if denominator.abs() < 1e-6 {
            return None;
        }
        let distance = (plane_origin - self.origin).dot(plane.normal) / denominator;
        (distance >= 0.0).then_some(distance)
    }
}

/// An endless plane, by its normal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InfinitePlane3d {
    /// The direction it faces.
    pub normal: Vec3,
}

impl InfinitePlane3d {
    /// A plane facing `normal`.
    pub fn new(normal: Vec3) -> InfinitePlane3d {
        InfinitePlane3d {
            normal: normal.normalize(),
        }
    }
}

/// Marks a 3D camera: `commands.spawn((Camera3d::default(), Transform::from_xyz(..)
/// .looking_at(Vec3::ZERO, Vec3::Y)))`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Camera3d {
    // Like Bevy's, made with `Camera3d::default()`.
    _private: (),
}

impl Component for Camera3d {
    fn on_insert(world: &mut World, entity: Entity) {
        require::<Camera>(world, entity);
        require::<Projection>(world, entity);
        require::<Exposure>(world, entity);
        require::<Tonemapping>(world, entity);
        require::<Transform>(world, entity);
        require::<Visibility>(world, entity);
    }
}

/// How a camera projects the scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Projection {
    /// With perspective (the default).
    Perspective(PerspectiveProjection),
    /// Without perspective.
    Orthographic(OrthographicProjection),
}

impl Default for Projection {
    fn default() -> Projection {
        Projection::Perspective(PerspectiveProjection::default())
    }
}

impl From<PerspectiveProjection> for Projection {
    fn from(p: PerspectiveProjection) -> Projection {
        Projection::Perspective(p)
    }
}

impl From<OrthographicProjection> for Projection {
    fn from(p: OrthographicProjection) -> Projection {
        Projection::Orthographic(p)
    }
}

impl Component for Projection {}

impl Projection {
    /// The matrix, for a viewport of this size in pixels.
    pub fn matrix(&self, viewport: Vec2) -> Mat4 {
        let aspect = viewport.x.max(1.0) / viewport.y.max(1.0);
        match self {
            Projection::Perspective(p) => Mat4::perspective(p.fov, aspect, p.near, p.far),
            Projection::Orthographic(o) => {
                let (w, h) = match o.scaling_mode {
                    ScalingMode::WindowSize => (viewport.x, viewport.y),
                    ScalingMode::Fixed { width, height } => (width, height),
                    ScalingMode::FixedVertical { viewport_height } => {
                        (viewport_height * aspect, viewport_height)
                    }
                    ScalingMode::FixedHorizontal { viewport_width } => {
                        (viewport_width, viewport_width / aspect)
                    }
                };
                let (w, h) = (w * o.scale / 2.0, h * o.scale / 2.0);
                Mat4::orthographic(-w, w, h, -h, o.near, o.far)
            }
        }
    }
}

/// A projection with perspective.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PerspectiveProjection {
    /// The vertical field of view, in radians (π/4).
    pub fov: f32,
    /// Width / height (follows the window).
    pub aspect_ratio: f32,
    /// Nothing nearer is drawn.
    pub near: f32,
    /// Nothing farther is drawn.
    pub far: f32,
}

impl Default for PerspectiveProjection {
    fn default() -> PerspectiveProjection {
        PerspectiveProjection {
            fov: FRAC_PI_4,
            aspect_ratio: 1.0,
            near: 0.1,
            far: 1000.0,
        }
    }
}

/// A projection without perspective.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrthographicProjection {
    /// Nothing nearer is drawn.
    pub near: f32,
    /// Nothing farther is drawn.
    pub far: f32,
    /// How the view's size follows the window's.
    pub scaling_mode: ScalingMode,
    /// A zoom factor (2 shows twice as much).
    pub scale: f32,
}

impl OrthographicProjection {
    /// The default for 3D.
    pub fn default_3d() -> OrthographicProjection {
        OrthographicProjection {
            near: 0.0,
            far: 1000.0,
            scaling_mode: ScalingMode::WindowSize,
            scale: 1.0,
        }
    }
}

/// How an orthographic view's size follows the window's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScalingMode {
    /// One unit per pixel.
    WindowSize,
    /// This many units, whatever the window.
    Fixed {
        /// The width shown.
        width: f32,
        /// The height shown.
        height: f32,
    },
    /// This height, the width following the window's proportions.
    FixedVertical {
        /// The height shown.
        viewport_height: f32,
    },
    /// This width, the height following the window's proportions.
    FixedHorizontal {
        /// The width shown.
        viewport_width: f32,
    },
}

/// How much light makes white, as a camera's EV100: higher needs more light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exposure {
    /// The exposure value at ISO 100.
    pub ev100: f32,
}

impl Exposure {
    /// Outside in the sun.
    pub const SUNLIGHT: Exposure = Exposure { ev100: 15.0 };
    /// Outside under clouds.
    pub const OVERCAST: Exposure = Exposure { ev100: 12.0 };
    /// Inside.
    pub const INDOOR: Exposure = Exposure { ev100: 7.0 };
    /// Blender's default, and this one's.
    pub const BLENDER: Exposure = Exposure { ev100: 9.7 };

    /// The factor applied to the light.
    pub fn exposure(&self) -> f32 {
        1.0 / (2f32.powf(self.ev100) * 1.2)
    }
}

impl Default for Exposure {
    fn default() -> Exposure {
        Exposure::BLENDER
    }
}

impl Component for Exposure {}

/// How bright colors are brought into the screen's range. Every curve but `None` is drawn
/// with ACES here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(missing_docs)]
pub enum Tonemapping {
    None,
    Reinhard,
    ReinhardLuminance,
    AcesFitted,
    AgX,
    SomewhatBoringDisplayTransform,
    #[default]
    TonyMcMapface,
    BlenderFilmic,
}

impl Component for Tonemapping {}

/// Keeps each camera's projection matching the window.
fn update_cameras(
    windows: Query<'_, '_, &Window, With<PrimaryWindow>>,
    mut cameras: Query<'_, '_, (&mut Camera, &Projection)>,
) {
    let size = windows
        .single()
        .map(|w| w.size())
        .unwrap_or(Vec2::new(1280.0, 720.0));
    for (mut camera, projection) in &mut cameras {
        let computed = ComputedCamera {
            projection: projection.matrix(size),
            viewport: size,
        };
        if camera.computed != computed {
            camera.bypass_change_detection().computed = computed;
        }
    }
}

/// Meshes, materials, lights and cameras.
#[derive(Clone, Copy, Debug, Default)]
pub struct PbrPlugin;

impl Plugin for PbrPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<StandardMaterial>>()
            .init_resource::<ClearColor>()
            .init_resource::<GlobalAmbientLight>()
            .add_systems(PostUpdate, update_cameras)
            .add_systems(super::PostStartup, update_cameras);
    }
}

// ----- What the renderer draws ----------------------------------------------------------

/// What the camera sees, made from the world, for the renderer: `frame()` gives the
/// Three.js [`Frame`](three::Frame).
#[derive(Clone, Debug)]
pub struct RenderFrame {
    meshes: Vec<(Geometry, three::Material, Mat4)>,
    camera: Mat4,
    projection: Mat4,
    background: Option<Linear>,
    lights: Lights,
    exposure: f32,
    tone_mapping: ToneMapping,
}

impl RenderFrame {
    /// What the active camera with the highest order sees; `None` without a camera.
    pub fn extract(world: &World, viewport: Vec2) -> Option<RenderFrame> {
        let cameras = world.query::<(Entity, &Camera, &Projection, &GlobalTransform)>();
        let (entity, camera, projection, transform) = cameras
            .iter(world)
            .filter(|(_, c, _, _)| c.is_active)
            .max_by_key(|(_, c, _, _)| c.order)?;
        let exposure = world
            .get::<Exposure>(entity)
            .map_or(Exposure::default(), |e| *e)
            .exposure();
        let tone_mapping = match world
            .get::<Tonemapping>(entity)
            .map_or(Tonemapping::default(), |t| *t)
        {
            Tonemapping::None => ToneMapping::None,
            _ => ToneMapping::AcesFilmic,
        };
        let background = match camera.clear_color {
            ClearColorConfig::Default => world.get_resource::<ClearColor>().map(|c| c.0),
            ClearColorConfig::Custom(color) => Some(color),
            ClearColorConfig::None => None,
        };
        let background = background.filter(|c| c.alpha > 0.0).map(Linear::from);
        let ambient = match world.get::<AmbientLight>(entity) {
            Some(a) => (a.color, a.brightness),
            None => world
                .get_resource::<GlobalAmbientLight>()
                .map_or((Color::BLACK, 0.0), |a| (a.color, a.brightness)),
        };
        let (camera_matrix, projection) = (transform.to_matrix(), projection.matrix(viewport));
        drop((camera, transform));

        // Bevy's ambient light is not divided by π like the others: the shader does.
        let mut lights = Lights {
            ambient: Linear::from(ambient.0).scale(ambient.1 * PI),
            ..Lights::default()
        };
        let visible = |e: Entity| world.get::<InheritedVisibility>(e).is_none_or(|v| v.get());
        for (e, light, at) in world
            .query::<(Entity, &PointLight, &GlobalTransform)>()
            .iter(world)
        {
            if visible(e) {
                lights.point.push(PointLightData {
                    position: at.translation(),
                    // Lumens to candelas: spread over the whole sphere.
                    color: Linear::from(light.color).scale(light.intensity / (4.0 * PI)),
                    distance: light.range,
                    decay: 2.0,
                });
            }
        }
        for (e, light, at) in world
            .query::<(Entity, &DirectionalLight, &GlobalTransform)>()
            .iter(world)
        {
            if visible(e) {
                lights.directional.push((
                    at.back(),
                    Linear::from(light.color).scale(light.illuminance),
                ));
            }
        }

        let mut meshes = Vec::new();
        let assets = world.get_resource::<Assets<Mesh>>()?;
        let materials = world.get_resource::<Assets<StandardMaterial>>()?;
        let drawn = world.query::<(
            Entity,
            &Mesh3d,
            &MeshMaterial3d<StandardMaterial>,
            &GlobalTransform,
        )>();
        for (e, mesh, material, at) in drawn.iter(world) {
            if !visible(e) {
                continue;
            }
            if let (Some(mesh), Some(material)) = (assets.get(mesh.0), materials.get(material.0)) {
                meshes.push((mesh.geometry.clone(), material.to_three(), at.to_matrix()));
            }
        }
        Some(RenderFrame {
            meshes,
            camera: camera_matrix,
            projection,
            background,
            lights,
            exposure,
            tone_mapping,
        })
    }

    /// The frame to draw.
    pub fn frame(&self) -> three::Frame<'_> {
        let mut frame = three::Frame::new(self.camera, self.projection);
        frame.background = self.background;
        frame.lights = self.lights.clone();
        frame.exposure = self.exposure;
        frame.tone_mapping = self.tone_mapping;
        frame.meshes = self
            .meshes
            .iter()
            .map(|(geometry, material, world)| three::Draw {
                geometry,
                material,
                world: *world,
            })
            .collect();
        frame
    }

    /// How many meshes it draws.
    pub fn mesh_count(&self) -> usize {
        self.meshes.len()
    }
}
