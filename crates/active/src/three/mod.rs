//! 3D in the browser, the way Three.js does it (its API as of r187), written in Rust for
//! Active: a scene of meshes, lights and a camera, drawn with WebGL 2.
//!
//! ```
//! use active::three::*;
//!
//! let mut scene = Scene::new();
//! let cube = scene.add(Mesh::new(
//!     BoxGeometry::new(1.0, 1.0, 1.0),
//!     MeshStandardMaterial::new().color(0x44aa88),
//! ));
//! let light = scene.add(DirectionalLight::new(0xffffff, 3.0));
//! scene[light].position.set(-1.0, 2.0, 4.0);
//! let mut camera = PerspectiveCamera::new(75.0, 0.0, 0.1, 100.0);
//! camera.position.z = 2.0;
//!
//! // Each frame: move things, then draw (in the browser, with the `client` feature):
//! scene[cube].rotation.y += 0.01;
//! // renderer.render(&scene, &camera);
//! ```
//!
//! Everything but the renderer works on the server too (and in tests): the scene, the
//! geometries and [`Scene::frame`], which lists what to draw.

mod camera;
mod controls;
mod geometry;
mod light;
mod material;
pub mod math;
mod scene;

#[cfg(feature = "client")]
pub(crate) mod renderer;

pub use camera::{Camera, OrthographicCamera, PerspectiveCamera};
pub use controls::{OrbitCamera, OrbitControls};
pub use geometry::{
    BoxGeometry, BufferGeometry, ConeGeometry, CylinderGeometry, Geometry, PlaneGeometry,
    SphereGeometry, TorusGeometry,
};
pub use light::{AmbientLight, DirectionalLight, HemisphereLight, Light, PointLight};
pub use material::{
    Material, MeshBasicMaterial, MeshLambertMaterial, MeshNormalMaterial, MeshStandardMaterial,
    Side, Surface,
};
pub use math::{Color, Euler, EulerOrder, Matrix4, Quaternion, Vector2, Vector3};
pub use scene::{
    Draw, Frame, Group, Lights, Mesh, Object3D, ObjectId, ObjectKind, PointLightData, Scene,
    ToneMapping,
};

#[cfg(feature = "client")]
pub use renderer::{AnimationLoop, WebGLRenderer, animation_loop};
