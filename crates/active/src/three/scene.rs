//! The scene graph: objects placed relative to their parent (Three.js's `Object3D`,
//! `Scene`, `Mesh` and `Group`), and the frame that a renderer draws from it.

use std::ops::{Index, IndexMut};

use super::camera::{Camera, euler_looking};
use super::geometry::Geometry;
use super::light::Light;
use super::material::Material;
use super::math::{Color, Euler, Matrix4, Quaternion, Vector3};

/// A geometry drawn with a material (Three.js's `Mesh`).
#[derive(Clone, Debug)]
pub struct Mesh {
    /// Its vertices and triangles.
    pub geometry: Geometry,
    /// Its look.
    pub material: Material,
}

impl Mesh {
    /// A mesh (`new THREE.Mesh(geometry, material)`).
    pub fn new(geometry: Geometry, material: impl Into<Material>) -> Mesh {
        Mesh {
            geometry,
            material: material.into(),
        }
    }
}

/// An empty object that only groups others (Three.js's `Group`).
#[derive(Clone, Copy, Debug, Default)]
pub struct Group;

impl Group {
    /// A group.
    pub fn new() -> Group {
        Group
    }
}

/// What an object is.
#[derive(Clone, Debug)]
pub enum ObjectKind {
    /// A group.
    Group,
    /// A mesh.
    Mesh(Mesh),
    /// A light.
    Light(Light),
}

impl From<Group> for ObjectKind {
    fn from(_: Group) -> ObjectKind {
        ObjectKind::Group
    }
}

impl From<Mesh> for ObjectKind {
    fn from(m: Mesh) -> ObjectKind {
        ObjectKind::Mesh(m)
    }
}

impl From<Light> for ObjectKind {
    fn from(l: Light) -> ObjectKind {
        ObjectKind::Light(l)
    }
}

/// The address of an object in its [`Scene`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ObjectId(usize);

/// An object of the scene, placed relative to its parent (Three.js's `Object3D`).
#[derive(Clone, Debug)]
pub struct Object3D {
    /// A name, for [`Scene::get_object_by_name`].
    pub name: String,
    /// Its position.
    pub position: Vector3,
    /// Its rotation.
    pub rotation: Euler,
    /// Its scale.
    pub scale: Vector3,
    /// Hidden objects, and their children, are not drawn.
    pub visible: bool,
    /// What it is.
    pub kind: ObjectKind,
    /// Up, for [`look_at`](Self::look_at).
    pub up: Vector3,
    parent: Option<ObjectId>,
    children: Vec<ObjectId>,
}

impl Object3D {
    fn new(kind: ObjectKind) -> Object3D {
        Object3D {
            name: String::new(),
            position: Vector3::ZERO,
            rotation: Euler::default(),
            scale: Vector3::ONE,
            visible: true,
            kind,
            up: Vector3::Y,
            parent: None,
            children: Vec::new(),
        }
    }

    /// Its matrix relative to its parent.
    pub fn local_matrix(&self) -> Matrix4 {
        Matrix4::compose(
            self.position,
            Quaternion::from_euler(self.rotation),
            self.scale,
        )
    }

    /// Turns its `-z` axis (for a light or a camera; `+z` for Three.js meshes, which face
    /// their target with their front) towards a point of its parent's space.
    pub fn look_at(&mut self, target: impl Into<Vector3>) {
        let target = target.into();
        self.rotation = match self.kind {
            // Comme Three.js : les objets autres que caméras et lumières regardent avec +z.
            ObjectKind::Mesh(_) | ObjectKind::Group => {
                euler_looking(target, self.position, self.up)
            }
            ObjectKind::Light(_) => euler_looking(self.position, target, self.up),
        };
    }

    /// The parent, if any (the scene itself is not an object).
    pub fn parent(&self) -> Option<ObjectId> {
        self.parent
    }

    /// The children.
    pub fn children(&self) -> &[ObjectId] {
        &self.children
    }

    /// The mesh, if it is one.
    pub fn mesh(&self) -> Option<&Mesh> {
        match &self.kind {
            ObjectKind::Mesh(m) => Some(m),
            _ => None,
        }
    }

    /// The mesh, to change its material or geometry.
    pub fn mesh_mut(&mut self) -> Option<&mut Mesh> {
        match &mut self.kind {
            ObjectKind::Mesh(m) => Some(m),
            _ => None,
        }
    }
}

/// A world of objects (Three.js's `Scene`): `let cube = scene.add(mesh);` then
/// `scene[cube].rotation.y += 0.01`.
#[derive(Clone, Debug, Default)]
pub struct Scene {
    /// The color behind everything (`None`: transparent, the page shows through).
    pub background: Option<Color>,
    objects: Vec<Option<Object3D>>,
    free: Vec<usize>,
    roots: Vec<ObjectId>,
}

impl Scene {
    /// An empty scene.
    pub fn new() -> Scene {
        Scene::default()
    }

    /// Adds an object at the root of the scene (a [`Mesh`], a [`Group`] or a light).
    pub fn add(&mut self, object: impl Into<ObjectKind>) -> ObjectId {
        let id = self.insert(Object3D::new(object.into()));
        self.roots.push(id);
        id
    }

    /// Adds an object as a child of another: it follows its parent.
    pub fn add_to(&mut self, parent: ObjectId, object: impl Into<ObjectKind>) -> ObjectId {
        let mut child = Object3D::new(object.into());
        child.parent = Some(parent);
        let id = self.insert(child);
        self[parent].children.push(id);
        id
    }

    fn insert(&mut self, object: Object3D) -> ObjectId {
        match self.free.pop() {
            Some(i) => {
                self.objects[i] = Some(object);
                ObjectId(i)
            }
            None => {
                self.objects.push(Some(object));
                ObjectId(self.objects.len() - 1)
            }
        }
    }

    /// Removes an object and its children.
    pub fn remove(&mut self, id: ObjectId) {
        let Some(object) = self.objects.get_mut(id.0).and_then(Option::take) else {
            return;
        };
        match object.parent {
            Some(p) => {
                if let Some(parent) = self.objects[p.0].as_mut() {
                    parent.children.retain(|c| *c != id);
                }
            }
            None => self.roots.retain(|r| *r != id),
        }
        self.free.push(id.0);
        for child in object.children {
            // Les enfants partent avec leur parent ; ils ne sont plus rattachés.
            if let Some(c) = self.objects[child.0].as_mut() {
                c.parent = None;
            }
            self.roots.push(child);
            self.remove(child);
        }
    }

    /// The object, if it is still in the scene.
    pub fn get(&self, id: ObjectId) -> Option<&Object3D> {
        self.objects.get(id.0).and_then(Option::as_ref)
    }

    /// The object, to change it.
    pub fn get_mut(&mut self, id: ObjectId) -> Option<&mut Object3D> {
        self.objects.get_mut(id.0).and_then(Option::as_mut)
    }

    /// The first object with this name.
    pub fn get_object_by_name(&self, name: &str) -> Option<ObjectId> {
        self.objects
            .iter()
            .position(|o| o.as_ref().is_some_and(|o| o.name == name))
            .map(ObjectId)
    }

    /// The objects at the root.
    pub fn children(&self) -> &[ObjectId] {
        &self.roots
    }

    /// The number of objects.
    pub fn len(&self) -> usize {
        self.objects.iter().filter(|o| o.is_some()).count()
    }

    /// The ids of every object, in the order they were added.
    pub fn object_ids(&self) -> impl Iterator<Item = ObjectId> + '_ {
        self.objects
            .iter()
            .enumerate()
            .filter(|(_, o)| o.is_some())
            .map(|(i, _)| ObjectId(i))
    }

    /// No object.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Its matrix in the world: its parents' matrices, then its own.
    pub fn world_matrix(&self, id: ObjectId) -> Matrix4 {
        let object = &self[id];
        let local = object.local_matrix();
        match object.parent {
            Some(p) => self.world_matrix(p) * local,
            None => local,
        }
    }

    /// Calls `f` with every visible object and its world matrix, parents first.
    pub fn traverse_visible<'a>(&'a self, mut f: impl FnMut(ObjectId, &'a Object3D, &Matrix4)) {
        fn walk<'a>(
            scene: &'a Scene,
            id: ObjectId,
            parent: &Matrix4,
            f: &mut dyn FnMut(ObjectId, &'a Object3D, &Matrix4),
        ) {
            let object = &scene[id];
            if !object.visible {
                return;
            }
            let world = *parent * object.local_matrix();
            f(id, object, &world);
            for child in &object.children {
                walk(scene, *child, &world, f);
            }
        }
        for root in &self.roots {
            walk(self, *root, &Matrix4::IDENTITY, &mut f);
        }
    }

    /// What a renderer draws: the visible meshes with their world matrices, the lights,
    /// and the camera's matrices for a viewport of this aspect ratio.
    pub fn frame<'a>(&'a self, camera: &dyn Camera, aspect: f32) -> Frame<'a> {
        let mut frame = Frame::new(camera.world_matrix(), camera.projection(aspect));
        frame.background = self.background;
        self.traverse_visible(|_, object, world| match &object.kind {
            ObjectKind::Mesh(mesh) => frame.meshes.push(Draw {
                geometry: &mesh.geometry,
                material: &mesh.material,
                world: *world,
            }),
            ObjectKind::Light(light) => frame.lights.add(light, world.translation()),
            ObjectKind::Group => {}
        });
        frame
    }
}

impl Index<ObjectId> for Scene {
    type Output = Object3D;
    fn index(&self, id: ObjectId) -> &Object3D {
        self.get(id)
            .expect("this object was removed from the scene")
    }
}

impl IndexMut<ObjectId> for Scene {
    fn index_mut(&mut self, id: ObjectId) -> &mut Object3D {
        self.get_mut(id)
            .expect("this object was removed from the scene")
    }
}

/// A mesh to draw.
#[derive(Clone, Debug)]
pub struct Draw<'a> {
    /// Its geometry.
    pub geometry: &'a Geometry,
    /// Its material.
    pub material: &'a Material,
    /// Its world matrix.
    pub world: Matrix4,
}

/// A point light, ready for the shaders.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointLightData {
    /// Its position in the world.
    pub position: Vector3,
    /// Color × intensity.
    pub color: Color,
    /// Beyond this distance, no light (0: no limit).
    pub distance: f32,
    /// How fast it weakens.
    pub decay: f32,
}

/// The lights of a frame, added up by kind.
#[derive(Clone, Debug, PartialEq)]
pub struct Lights {
    /// The sum of the ambient lights.
    pub ambient: Color,
    /// The sky and ground colors of the hemisphere lights.
    pub hemisphere: Option<(Color, Color)>,
    /// Directional lights: the direction towards the light, and color × intensity.
    pub directional: Vec<(Vector3, Color)>,
    /// Point lights.
    pub point: Vec<PointLightData>,
}

impl Default for Lights {
    fn default() -> Lights {
        Lights {
            ambient: Color::BLACK,
            hemisphere: None,
            directional: Vec::new(),
            point: Vec::new(),
        }
    }
}

impl Lights {
    /// Adds a light placed at `position`.
    pub fn add(&mut self, light: &Light, position: Vector3) {
        match *light {
            Light::Ambient { color, intensity } => {
                let c = color.scale(intensity);
                self.ambient = Color::linear_rgb(
                    self.ambient.r + c.r,
                    self.ambient.g + c.g,
                    self.ambient.b + c.b,
                );
            }
            Light::Hemisphere {
                sky,
                ground,
                intensity,
            } => {
                self.hemisphere = Some((sky.scale(intensity), ground.scale(intensity)));
            }
            Light::Directional {
                color,
                intensity,
                target,
            } => {
                let direction = (position - target).normalize();
                let direction = if direction == Vector3::ZERO {
                    Vector3::Y
                } else {
                    direction
                };
                self.directional.push((direction, color.scale(intensity)));
            }
            Light::Point {
                color,
                intensity,
                distance,
                decay,
            } => {
                self.point.push(PointLightData {
                    position,
                    color: color.scale(intensity),
                    distance,
                    decay,
                });
            }
        }
    }
}

/// Everything a renderer needs to draw one image.
#[derive(Clone, Debug)]
pub struct Frame<'a> {
    /// The camera's world matrix.
    pub camera: Matrix4,
    /// The inverse of the camera's world matrix.
    pub view: Matrix4,
    /// The projection.
    pub projection: Matrix4,
    /// The background color.
    pub background: Option<Color>,
    /// The lights.
    pub lights: Lights,
    /// The meshes.
    pub meshes: Vec<Draw<'a>>,
    /// A factor applied to the light before tone mapping (1 for Three.js scenes; Bevy's
    /// physical units need a camera exposure).
    pub exposure: f32,
    /// How bright colors are brought into the screen's range.
    pub tone_mapping: ToneMapping,
}

impl Frame<'_> {
    /// An empty frame seen from a camera.
    pub fn new(camera: Matrix4, projection: Matrix4) -> Self {
        Frame {
            camera,
            view: camera.inverse(),
            projection,
            background: None,
            lights: Lights::default(),
            meshes: Vec::new(),
            exposure: 1.0,
            tone_mapping: ToneMapping::None,
        }
    }
}

/// How colors brighter than white are brought into the screen's range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToneMapping {
    /// Clamped (Three.js's default, `NoToneMapping`).
    #[default]
    None,
    /// The filmic curve of ACES (`ACESFilmicToneMapping`, close to Bevy's default look).
    AcesFilmic,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::three::{
        BoxGeometry, DirectionalLight, MeshStandardMaterial, PerspectiveCamera, PointLight,
    };

    #[test]
    fn children_follow_their_parents() {
        let mut scene = Scene::new();
        let group = scene.add(Group::new());
        scene[group].position.set(1.0, 0.0, 0.0);
        scene[group].rotation.y = std::f32::consts::FRAC_PI_2;
        let cube = scene.add_to(
            group,
            Mesh::new(BoxGeometry::new(1.0, 1.0, 1.0), MeshStandardMaterial::new()),
        );
        scene[cube].position.set(0.0, 0.0, 2.0);
        scene[cube].name = "cube".into();
        let world = scene.world_matrix(cube).translation();
        assert!(
            (world - Vector3::new(3.0, 0.0, 0.0)).length() < 1e-4,
            "{world:?}"
        );
        assert_eq!(scene.get_object_by_name("cube"), Some(cube));
        assert_eq!(scene[group].children(), &[cube]);
        scene[group].visible = false;
        let camera = PerspectiveCamera::new(75.0, 1.0, 0.1, 100.0);
        assert!(scene.frame(&camera, 1.0).meshes.is_empty());
        scene.remove(group);
        assert!(scene.is_empty() && scene.get(cube).is_none());
        // Une place libérée est reprise.
        let again = scene.add(Group::new());
        assert_eq!(scene.len(), 1);
        assert!(again == group || again == cube);
    }

    #[test]
    fn a_frame_has_meshes_lights_and_camera() {
        let mut scene = Scene::new();
        scene.background = Some(Color::hex(0x202020));
        scene.add(Mesh::new(
            BoxGeometry::new(1.0, 1.0, 1.0),
            MeshStandardMaterial::new().color(0x44aa88),
        ));
        let sun = scene.add(DirectionalLight::new(0xffffff, 3.0));
        scene[sun].position.set(0.0, 10.0, 0.0);
        let bulb = scene.add(PointLight::new(0xff0000, 50.0));
        scene[bulb].position.set(1.0, 2.0, 3.0);
        let mut camera = PerspectiveCamera::new(75.0, 0.0, 0.1, 100.0);
        camera.position.z = 5.0;
        let frame = scene.frame(&camera, 2.0);
        assert_eq!(frame.meshes.len(), 1);
        assert_eq!(
            frame.lights.directional,
            vec![(Vector3::Y, Color::WHITE.scale(3.0))]
        );
        assert_eq!(frame.lights.point[0].position, Vector3::new(1.0, 2.0, 3.0));
        assert_eq!(frame.view.translation(), Vector3::new(0.0, 0.0, -5.0));
        // Le rapport largeur/hauteur du canevas sert quand la caméra n'en a pas.
        assert!((frame.projection.elements[5] / frame.projection.elements[0] - 2.0).abs() < 1e-4);
    }
}
