//! Lights (`AmbientLight`, `HemisphereLight`, `DirectionalLight`, `PointLight` and
//! `SpotLight`), and their shadows ([`LightShadow`]).
//! Intensities are in physical units: a directional light of
//! intensity 3 is a bright sun, a point light is in candela.

use super::math::{Color, Vector3};

/// A light.
#[derive(Clone, Debug, PartialEq)]
pub enum Light {
    /// Light from everywhere, without shading.
    Ambient {
        /// Its color.
        color: Color,
        /// Its strength.
        intensity: f32,
    },
    /// The sky above, the ground below: surfaces facing up take the first, down the second.
    Hemisphere {
        /// The color of the sky.
        sky: Color,
        /// The color of the ground.
        ground: Color,
        /// Its strength.
        intensity: f32,
    },
    /// Parallel rays, like the sun's, from the light's position towards `target`.
    Directional {
        /// Its color.
        color: Color,
        /// Its strength.
        intensity: f32,
        /// Where the rays go, in world space (the origin by default).
        target: Vector3,
    },
    /// Light from a point in every direction, weaker with distance.
    Point {
        /// Its color.
        color: Color,
        /// Its strength (candela).
        intensity: f32,
        /// Beyond this distance, no light (0: no limit).
        distance: f32,
        /// How fast it weakens (2 is physical).
        decay: f32,
    },
    /// A cone of light from a point towards `target`: a stage light, a museum spot.
    Spot {
        /// Its color.
        color: Color,
        /// Its strength (candela).
        intensity: f32,
        /// Beyond this distance, no light (0: no limit).
        distance: f32,
        /// Half the opening of the cone, in radians (at most π/2).
        angle: f32,
        /// From 0 (a sharp edge) to 1 (the light fades from the axis to the edge).
        penumbra: f32,
        /// How fast it weakens (2 is physical).
        decay: f32,
        /// Where it shines, in world space (the origin by default).
        target: Vector3,
    },
}

/// The shadow a light casts (`light.shadow` of three.js): set
/// [`cast_shadow`](super::Object3D::cast_shadow) on the light, and on the objects that
/// cast and [`receive`](super::Object3D::receive_shadow) shadows. Directional lights and
/// spots cast shadows (4 lights at most); point lights do not.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightShadow {
    /// The size of the shadow map, in pixels (1024): larger is sharper and slower.
    pub map_size: u32,
    /// Pushes the shadow back by this depth, against "shadow acne" (stripes on lit faces).
    pub bias: f32,
    /// Pushes the surface towards its normal by this distance (world units), against
    /// acne on surfaces lit at a grazing angle.
    pub normal_bias: f32,
    /// How soft the edge is, in pixels of the shadow map (1: a little).
    pub radius: f32,
    /// Directional lights: the half size of the square that has shadows, around what
    /// the camera looks at (`None`: the whole scene, coarser in a large one).
    pub extent: Option<f32>,
}

impl Default for LightShadow {
    fn default() -> LightShadow {
        LightShadow {
            map_size: 1024,
            bias: 0.0005,
            normal_bias: 0.02,
            radius: 1.0,
            extent: None,
        }
    }
}

/// Light from everywhere.
pub struct AmbientLight;

// `new` gives the light to add to a scene, like `new THREE.AmbientLight(...)`.
#[allow(clippy::new_ret_no_self)]
impl AmbientLight {
    /// An ambient light.
    pub fn new(color: impl Into<Color>, intensity: f32) -> Light {
        Light::Ambient {
            color: color.into(),
            intensity,
        }
    }
}

/// The sky and the ground.
pub struct HemisphereLight;

// `new` gives the light to add to a scene, like `new THREE.HemisphereLight(...)`.
#[allow(clippy::new_ret_no_self)]
impl HemisphereLight {
    /// A hemisphere light.
    pub fn new(sky: impl Into<Color>, ground: impl Into<Color>, intensity: f32) -> Light {
        Light::Hemisphere {
            sky: sky.into(),
            ground: ground.into(),
            intensity,
        }
    }
}

/// Parallel rays.
pub struct DirectionalLight;

// `new` gives the light to add to a scene, like `new THREE.DirectionalLight(...)`.
#[allow(clippy::new_ret_no_self)]
impl DirectionalLight {
    /// A directional light aimed at the origin (move its object to aim it).
    pub fn new(color: impl Into<Color>, intensity: f32) -> Light {
        Light::Directional {
            color: color.into(),
            intensity,
            target: Vector3::ZERO,
        }
    }
}

/// Light from a point.
pub struct PointLight;

// `new` gives the light to add to a scene, like `new THREE.PointLight(...)`.
#[allow(clippy::new_ret_no_self)]
impl PointLight {
    /// A point light without distance limit and with physical decay.
    pub fn new(color: impl Into<Color>, intensity: f32) -> Light {
        Light::Point {
            color: color.into(),
            intensity,
            distance: 0.0,
            decay: 2.0,
        }
    }
}

/// A cone of light.
pub struct SpotLight;

// `new` gives the light to add to a scene, like `new THREE.SpotLight(...)`.
#[allow(clippy::new_ret_no_self)]
impl SpotLight {
    /// A spot aimed at the origin (`target` aims it), opening π/3 each side, with a sharp
    /// edge, no distance limit and physical decay.
    pub fn new(color: impl Into<Color>, intensity: f32) -> Light {
        Light::Spot {
            color: color.into(),
            intensity,
            distance: 0.0,
            angle: std::f32::consts::FRAC_PI_3,
            penumbra: 0.0,
            decay: 2.0,
            target: Vector3::ZERO,
        }
    }

    /// A spot with every setting: half its opening (radians), its penumbra (0 to 1) and
    /// where it shines.
    pub fn aimed(
        color: impl Into<Color>,
        intensity: f32,
        angle: f32,
        penumbra: f32,
        target: Vector3,
    ) -> Light {
        Light::Spot {
            color: color.into(),
            intensity,
            distance: 0.0,
            angle: angle.clamp(0.0, std::f32::consts::FRAC_PI_2),
            penumbra: penumbra.clamp(0.0, 1.0),
            decay: 2.0,
            target,
        }
    }
}
