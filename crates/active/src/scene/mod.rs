//! Active's 3D: a scene of meshes, lights and a camera, drawn in the browser with
//! WebGL 2, from the same Rust code as the pages around it.
//!
//! ```
//! use active::scene::*;
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
pub(crate) mod geometry;
mod graph;
mod light;
mod material;
pub mod math;
pub(crate) mod physics;

#[cfg(feature = "client")]
pub(crate) mod renderer;

pub use camera::{Camera, OrthographicCamera, PerspectiveCamera};
pub use controls::{OrbitCamera, OrbitControls};
pub use geometry::{
    BoxGeometry, BufferGeometry, ConeGeometry, CylinderGeometry, Geometry, PlaneGeometry,
    SphereGeometry, TorusGeometry,
};
pub use graph::{
    Draw, Frame, Group, Lights, Mesh, Object3D, ObjectId, ObjectKind, PointLightData, Scene,
    ToneMapping,
};
pub use light::{AmbientLight, DirectionalLight, HemisphereLight, Light, PointLight};
pub use material::{
    Material, MeshBasicMaterial, MeshLambertMaterial, MeshNormalMaterial, MeshStandardMaterial,
    Side, Surface,
};
pub use math::{Color, Euler, EulerOrder, Matrix4, Quaternion, Vector2, Vector3};
pub use physics::Physics;

#[cfg(feature = "client")]
pub use renderer::{AnimationLoop, Renderer, animation_loop, device_pixel_ratio};
