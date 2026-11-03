//! Lights (Three.js's `AmbientLight`, `HemisphereLight`, `DirectionalLight` and
//! `PointLight`). Intensities follow Three.js's physical units: a directional light of
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
