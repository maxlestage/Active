//! Materials: how a surface answers the light (`MeshBasicMaterial`,
//! `MeshLambertMaterial`, `MeshStandardMaterial` and `MeshNormalMaterial`).

use super::math::Color;

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

/// A flat color, lights ignored.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshBasicMaterial {
    /// Color, opacity, sides.
    pub surface: Surface,
}

impl MeshBasicMaterial {
    /// A white material.
    pub fn new() -> Self {
        Self::default()
    }
}

surface_builders!(MeshBasicMaterial, Basic);

/// A matte surface: light spread evenly in every direction.
#[derive(Clone, Debug, PartialEq)]
pub struct MeshLambertMaterial {
    /// Color, opacity, sides.
    pub surface: Surface,
    /// Light the surface gives out itself.
    pub emissive: Color,
}

impl Default for MeshLambertMaterial {
    fn default() -> Self {
        MeshLambertMaterial {
            surface: Surface::default(),
            emissive: Color::BLACK,
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
}

impl Default for MeshStandardMaterial {
    fn default() -> Self {
        MeshStandardMaterial {
            surface: Surface::default(),
            roughness: 1.0,
            metalness: 0.0,
            emissive: Color::BLACK,
            flat_shading: false,
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
}

surface_builders!(MeshStandardMaterial, Standard);

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
