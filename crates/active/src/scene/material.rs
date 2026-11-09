//! Materials: how a surface answers the light (`MeshBasicMaterial`,
//! `MeshLambertMaterial`, `MeshStandardMaterial` and `MeshNormalMaterial`), with their
//! textures.

use super::math::{Color, Vector2};
use super::texture::Texture;

/// Which faces are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Side {
    /// The faces seen from the front (counter-clockwise), the default.
    #[default]
    Front,
    /// The faces seen from behind.
    Back,
    /// Both.
    Double,
}

/// What every material has.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    /// The base color.
    pub color: Color,
    /// From 0 (invisible) to 1; below 1, set `transparent`.
    pub opacity: f32,
    /// Drawn after the opaque meshes, blended with what is behind.
    pub transparent: bool,
    /// Which faces are drawn.
    pub side: Side,
    /// The edges of the triangles only.
    pub wireframe: bool,
}

impl Default for Surface {
    fn default() -> Surface {
        Surface {
            color: Color::WHITE,
            opacity: 1.0,
            transparent: false,
            side: Side::Front,
            wireframe: false,
        }
    }
}

/// A material: one of the kinds below, made by their builders.
#[derive(Clone, Debug, PartialEq)]
pub enum Material {
    /// [`MeshBasicMaterial`].
    Basic(MeshBasicMaterial),
    /// [`MeshLambertMaterial`].
    Lambert(MeshLambertMaterial),
    /// [`MeshStandardMaterial`].
    Standard(MeshStandardMaterial),
    /// [`MeshNormalMaterial`].
    Normal(MeshNormalMaterial),
}

impl Material {
    /// What every material has.
    pub fn surface(&self) -> &Surface {
        match self {
            Material::Basic(m) => &m.surface,
            Material::Lambert(m) => &m.surface,
            Material::Standard(m) => &m.surface,
            Material::Normal(m) => &m.surface,
        }
    }

    /// Its color texture, if any.
    pub fn map(&self) -> Option<&Texture> {
        match self {
            Material::Basic(m) => m.map.as_ref(),
            Material::Lambert(m) => m.map.as_ref(),
            Material::Standard(m) => m.map.as_ref(),
            Material::Normal(_) => None,
        }
    }

    /// Below this alpha (texture × opacity), a pixel is not drawn at all (0: never).
    pub fn alpha_test(&self) -> f32 {
        match self {
            Material::Basic(m) => m.alpha_test,
            Material::Lambert(m) => m.alpha_test,
            Material::Standard(m) => m.alpha_test,
            Material::Normal(_) => 0.0,
        }
    }

    /// The same, to change.
    pub fn surface_mut(&mut self) -> &mut Surface {
        match self {
            Material::Basic(m) => &mut m.surface,
            Material::Lambert(m) => &mut m.surface,
            Material::Standard(m) => &mut m.surface,
            Material::Normal(m) => &mut m.surface,
        }
    }
}

macro_rules! surface_builders {
    ($ty:ident, $variant:ident) => {
        impl $ty {
            /// The base color: an sRGB hexadecimal like `0x44aa88`, or a [`Color`].
            pub fn color(mut self, color: impl Into<Color>) -> Self {
                self.surface.color = color.into();
                self
            }

            /// The opacity, from 0 to 1 (it also makes the material transparent below 1).
            pub fn opacity(mut self, opacity: f32) -> Self {
                self.surface.opacity = opacity;
                self.surface.transparent |= opacity < 1.0;
                self
            }

            /// Blended with what is behind it.
            pub fn transparent(mut self, transparent: bool) -> Self {
                self.surface.transparent = transparent;
                self
            }

            /// Which faces are drawn.
            pub fn side(mut self, side: Side) -> Self {
                self.surface.side = side;
                self
            }

            /// The edges of the triangles only.
            pub fn wireframe(mut self, wireframe: bool) -> Self {
                self.surface.wireframe = wireframe;
                self
            }
        }

        impl From<$ty> for Material {
            fn from(m: $ty) -> Material {
                Material::$variant(m)
            }
        }
    };
}

macro_rules! map_builders {
    ($ty:ident) => {
        impl $ty {
            /// The color texture (`material.map`): multiplied by the color, its alpha by
            /// the opacity.
            pub fn map(mut self, texture: Texture) -> Self {
                self.map = Some(texture);
                self
            }

            /// Below this alpha, a pixel is not drawn at all (cut-outs: leaves, fences,
            /// sprites); 0, the default, never.
            pub fn alpha_test(mut self, alpha: f32) -> Self {
                self.alpha_test = alpha.clamp(0.0, 1.0);
                self
            }
        }
    };
}

/// A flat color, lights ignored.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshBasicMaterial {
    /// Color, opacity, sides.
    pub surface: Surface,
    /// The color texture.
    pub map: Option<Texture>,
    /// Below this alpha, a pixel is not drawn (0: never).
    pub alpha_test: f32,
}

impl MeshBasicMaterial {
    /// A white material.
    pub fn new() -> Self {
        Self::default()
    }
}

surface_builders!(MeshBasicMaterial, Basic);
map_builders!(MeshBasicMaterial);

/// A matte surface: light spread evenly in every direction.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshLambertMaterial {
    /// Color, opacity, sides.
    pub surface: Surface,
    /// Light the surface gives out itself.
    pub emissive: Color,
    /// The color texture.
    pub map: Option<Texture>,
    /// Below this alpha, a pixel is not drawn (0: never).
    pub alpha_test: f32,
}

impl Default for MeshLambertMaterial {
    fn default() -> Self {
        MeshLambertMaterial {
            surface: Surface::default(),
            emissive: Color::BLACK,
            map: None,
            alpha_test: 0.0,
        }
    }
}

impl MeshLambertMaterial {
    /// A white material.
    pub fn new() -> Self {
        Self::default()
    }

    /// Light the surface gives out itself.
    pub fn emissive(mut self, color: impl Into<Color>) -> Self {
        self.emissive = color.into();
        self
    }
}

surface_builders!(MeshLambertMaterial, Lambert);
map_builders!(MeshLambertMaterial);

/// Physically based: a surface between plastic and metal, between mirror and rough
/// (the same model as the game engine's `StandardMaterial`).
#[derive(Clone, Debug, PartialEq)]
pub struct MeshStandardMaterial {
    /// Color, opacity, sides.
    pub surface: Surface,
    /// From 0 (smooth, sharp reflections) to 1 (rough).
    pub roughness: f32,
    /// From 0 (dielectric: plastic, wood…) to 1 (metal).
    pub metalness: f32,
    /// Light the surface gives out itself.
    pub emissive: Color,
    /// Normals of the faces instead of the smoothed ones: facets show.
    pub flat_shading: bool,
    /// The color texture.
    pub map: Option<Texture>,
    /// Below this alpha, a pixel is not drawn (0: never).
    pub alpha_test: f32,
    /// The light given out, as a texture (multiplied by `emissive`).
    pub emissive_map: Option<Texture>,
    /// Bumps and grooves: a tangent-space normal map (blue-violet images).
    pub normal_map: Option<Texture>,
    /// How strongly the normal map bends the light, across and up (1, 1).
    pub normal_scale: Vector2,
    /// Roughness in the green channel and metalness in the blue one (glTF's
    /// `metallicRoughnessTexture`), multiplied by `roughness` and `metalness`.
    pub roughness_metalness_map: Option<Texture>,
}

impl Default for MeshStandardMaterial {
    fn default() -> Self {
        MeshStandardMaterial {
            surface: Surface::default(),
            roughness: 1.0,
            metalness: 0.0,
            emissive: Color::BLACK,
            flat_shading: false,
            map: None,
            alpha_test: 0.0,
            emissive_map: None,
            normal_map: None,
            normal_scale: Vector2::new(1.0, 1.0),
            roughness_metalness_map: None,
        }
    }
}

impl MeshStandardMaterial {
    /// A white, rough, dielectric material (the usual defaults).
    pub fn new() -> Self {
        Self::default()
    }

    /// From 0 (smooth) to 1 (rough).
    pub fn roughness(mut self, roughness: f32) -> Self {
        self.roughness = roughness.clamp(0.0, 1.0);
        self
    }

    /// From 0 (dielectric) to 1 (metal).
    pub fn metalness(mut self, metalness: f32) -> Self {
        self.metalness = metalness.clamp(0.0, 1.0);
        self
    }

    /// Light the surface gives out itself.
    pub fn emissive(mut self, color: impl Into<Color>) -> Self {
        self.emissive = color.into();
        self
    }

    /// Facets show.
    pub fn flat_shading(mut self, flat: bool) -> Self {
        self.flat_shading = flat;
        self
    }

    /// The light given out, as a texture (multiplied by `emissive`, white if it is black).
    pub fn emissive_map(mut self, texture: Texture) -> Self {
        if self.emissive == Color::BLACK {
            self.emissive = Color::WHITE;
        }
        self.emissive_map = Some(texture);
        self
    }

    /// Bumps and grooves: a tangent-space normal map, read as raw numbers.
    pub fn normal_map(mut self, texture: Texture) -> Self {
        self.normal_map = Some(texture.linear_if_data(true));
        self
    }

    /// How strongly the normal map bends the light.
    pub fn normal_scale(mut self, x: f32, y: f32) -> Self {
        self.normal_scale = Vector2::new(x, y);
        self
    }

    /// Roughness (green) and metalness (blue) from a texture, read as raw numbers.
    pub fn roughness_metalness_map(mut self, texture: Texture) -> Self {
        self.roughness_metalness_map = Some(texture.linear_if_data(true));
        self
    }
}

surface_builders!(MeshStandardMaterial, Standard);
map_builders!(MeshStandardMaterial);

/// The direction of each point's normal as a color: handy to check a geometry.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshNormalMaterial {
    /// Opacity and sides (its color is not used).
    pub surface: Surface,
}

impl MeshNormalMaterial {
    /// A normal material.
    pub fn new() -> Self {
        Self::default()
    }
}

surface_builders!(MeshNormalMaterial, Normal);
