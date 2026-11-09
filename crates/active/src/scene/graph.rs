//! The scene graph: objects placed relative to their parent (`Object3D`, `Scene`,
//! `Mesh` and `Group`), and the frame that a renderer draws from it.

use std::ops::{Index, IndexMut};

use super::camera::{Camera, euler_looking};
use super::geometry::{Geometry, PlaneGeometry};
use super::light::{Light, LightShadow};
use super::material::{Material, MeshBasicMaterial};
use super::math::{Color, Euler, Matrix4, Quaternion, Vector2, Vector3};
use super::texture::Texture;

/// The most lights casting shadows in one frame (the others light without shadows).
pub const MAX_SHADOWS: usize = 4;

/// A geometry drawn with a material.
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

/// An image that always faces the camera (`THREE.Sprite`): a label, a particle, a tree
/// far away. One unit wide and high: scale its object to size it.
///
/// ```
/// use active::scene::*;
/// let mut scene = Scene::new();
/// let dot = Texture::from_fn(16, 16, |x, y| {
///     let d = ((x as f32 - 7.5).powi(2) + (y as f32 - 7.5).powi(2)).sqrt();
///     [255, 220, 120, if d < 8.0 { 255 } else { 0 }]
/// });
/// let sprite = scene.add(Sprite::from_texture(dot));
/// scene[sprite].scale.set(0.5, 0.5, 1.0);
/// ```
#[derive(Clone, Debug)]
pub struct Sprite {
    /// Its look: usually a [`MeshBasicMaterial`] with a texture (`map`), transparent.
    pub material: Material,
    /// The point of the image on the object's position: `(0.5, 0.5)` its middle,
    /// `(0.5, 0)` the middle of its bottom edge.
    pub center: Vector2,
    /// A rotation in the plane of the screen, in radians.
    pub rotation: f32,
    geometry: Geometry,
}

impl Sprite {
    /// A sprite (`new THREE.Sprite(material)`).
    pub fn new(material: impl Into<Material>) -> Sprite {
        Sprite {
            material: material.into(),
            center: Vector2::new(0.5, 0.5),
            rotation: 0.0,
            geometry: PlaneGeometry::new(1.0, 1.0),
        }
    }

    /// A sprite showing an image, blended with what is behind it, lights ignored.
    pub fn from_texture(texture: Texture) -> Sprite {
        Sprite::new(MeshBasicMaterial::new().map(texture).transparent(true))
    }

    /// Its world matrix seen by a camera whose rotation is `facing`.
    fn billboard(&self, world: &Matrix4, facing: Quaternion) -> Matrix4 {
        let (position, _, scale) = world.decompose();
        Matrix4::compose(
            position,
            facing * Quaternion::from_rotation_z(self.rotation),
            scale,
        ) * Matrix4::compose(
            Vector3::new(0.5 - self.center.x, 0.5 - self.center.y, 0.0),
            Quaternion::IDENTITY,
            Vector3::ONE,
        )
    }
}

/// An empty object that only groups others.
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
    /// An image facing the camera.
    Sprite(Sprite),
}

impl From<Sprite> for ObjectKind {
    fn from(s: Sprite) -> ObjectKind {
        ObjectKind::Sprite(s)
    }
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

/// An object of the scene, placed relative to its parent.
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
    /// A mesh: it darkens what is behind it from a light casting shadows. A light
    /// (directional or spot): it casts shadows.
    pub cast_shadow: bool,
    /// A mesh: the shadows of the others fall on it.
    pub receive_shadow: bool,
    /// A light casting shadows: their settings (`light.shadow`).
    pub shadow: LightShadow,
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
            cast_shadow: false,
            receive_shadow: false,
            shadow: LightShadow::default(),
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

    /// Turns its `-z` axis (for a light or a camera; `+z` for meshes, which face
    /// their target with their front) towards a point of its parent's space.
    pub fn look_at(&mut self, target: impl Into<Vector3>) {
        let target = target.into();
        self.rotation = match self.kind {
            // Les objets autres que caméras et lumières regardent avec +z.
            ObjectKind::Mesh(_) | ObjectKind::Group | ObjectKind::Sprite(_) => {
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

/// A world of objects: `let cube = scene.add(mesh);` then
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
        let facing = frame.camera.decompose().1;
        self.traverse_visible(|_, object, world| match &object.kind {
            ObjectKind::Mesh(mesh) => frame.meshes.push(Draw {
                geometry: &mesh.geometry,
                material: &mesh.material,
                world: *world,
                cast_shadow: object.cast_shadow,
                receive_shadow: object.receive_shadow,
            }),
            ObjectKind::Light(light) => {
                let shadow = object.cast_shadow.then_some(&object.shadow);
                frame
                    .lights
                    .add_with_shadow(light, world.translation(), shadow);
            }
            ObjectKind::Sprite(sprite) => frame.meshes.push(Draw {
                geometry: &sprite.geometry,
                material: &sprite.material,
                world: sprite.billboard(world, facing),
                cast_shadow: false,
                receive_shadow: object.receive_shadow,
            }),
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
    /// It casts shadows.
    pub cast_shadow: bool,
    /// Shadows fall on it.
    pub receive_shadow: bool,
}

impl<'a> Draw<'a> {
    /// A mesh to draw, without shadows.
    pub fn new(geometry: &'a Geometry, material: &'a Material, world: Matrix4) -> Draw<'a> {
        Draw {
            geometry,
            material,
            world,
            cast_shadow: false,
            receive_shadow: false,
        }
    }

    /// The box around it, in the world.
    pub fn world_bounds(&self) -> (Vector3, Vector3) {
        let (min, max) = self.geometry.cached_bounds();
        if min.x > max.x {
            let at = self.world.translation();
            return (at, at);
        }
        let mut lo = Vector3::splat(f32::INFINITY);
        let mut hi = Vector3::splat(f32::NEG_INFINITY);
        for i in 0..8 {
            let corner = Vector3::new(
                if i & 1 == 0 { min.x } else { max.x },
                if i & 2 == 0 { min.y } else { max.y },
                if i & 4 == 0 { min.z } else { max.z },
            )
            .apply_matrix4(&self.world);
            lo = lo.min(corner);
            hi = hi.max(corner);
        }
        (lo, hi)
    }
}

/// A spot light, ready for the shaders.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpotLightData {
    /// Its position in the world.
    pub position: Vector3,
    /// The axis of its cone, from the light towards where it shines.
    pub direction: Vector3,
    /// Color × intensity.
    pub color: Color,
    /// Beyond this distance, no light (0: no limit).
    pub distance: f32,
    /// How fast it weakens.
    pub decay: f32,
    /// Half its opening, in radians.
    pub angle: f32,
    /// From 0 (sharp edge) to 1.
    pub penumbra: f32,
}

/// Which light casts a shadow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowLight {
    /// The directional light at this index of [`Lights::directional`].
    Directional(usize),
    /// The spot at this index of [`Lights::spot`].
    Spot(usize),
}

/// A light that casts shadows, and how.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowRequest {
    /// The light.
    pub light: ShadowLight,
    /// Its shadow's settings.
    pub settings: LightShadow,
}

/// Where a shadow map is seen from, for one frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShadowCamera {
    /// The light.
    pub light: ShadowLight,
    /// From the world to the light's clip space.
    pub view_projection: Matrix4,
    /// The settings.
    pub settings: LightShadow,
    /// An orthographic camera (directional light), or a perspective one (spot).
    pub orthographic: bool,
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
    /// Spot lights.
    pub spot: Vec<SpotLightData>,
    /// The lights casting shadows ([`MAX_SHADOWS`] are drawn).
    pub shadows: Vec<ShadowRequest>,
}

impl Default for Lights {
    fn default() -> Lights {
        Lights {
            ambient: Color::BLACK,
            hemisphere: None,
            directional: Vec::new(),
            point: Vec::new(),
            spot: Vec::new(),
            shadows: Vec::new(),
        }
    }
}

impl Lights {
    /// Adds a light placed at `position`, casting shadows with these settings (only
    /// directional lights and spots do).
    pub fn add_with_shadow(
        &mut self,
        light: &Light,
        position: Vector3,
        shadow: Option<&LightShadow>,
    ) {
        self.add(light, position);
        let Some(settings) = shadow else {
            return;
        };
        let light = match light {
            Light::Directional { .. } => ShadowLight::Directional(self.directional.len() - 1),
            Light::Spot { .. } => ShadowLight::Spot(self.spot.len() - 1),
            _ => return,
        };
        self.shadows.push(ShadowRequest {
            light,
            settings: *settings,
        });
    }

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
            Light::Spot {
                color,
                intensity,
                distance,
                angle,
                penumbra,
                decay,
                target,
            } => {
                let direction = (target - position).normalize();
                self.spot.push(SpotLightData {
                    position,
                    direction: if direction == Vector3::ZERO {
                        -Vector3::Y
                    } else {
                        direction
                    },
                    color: color.scale(intensity),
                    distance,
                    decay,
                    angle: angle.clamp(0.0, std::f32::consts::FRAC_PI_2),
                    penumbra: penumbra.clamp(0.0, 1.0),
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
    /// A factor applied to the light before tone mapping (1 for a scene; the game
    /// engine's physical units need a camera exposure).
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

impl Frame<'_> {
    /// Where each shadow map of this frame is seen from ([`MAX_SHADOWS`] at most): a
    /// directional light covers the meshes that cast or receive shadows (or the square of
    /// its [`extent`](LightShadow::extent) around what the camera looks at), a spot its
    /// cone.
    pub fn shadow_cameras(&self) -> Vec<ShadowCamera> {
        if self.lights.shadows.is_empty() {
            return Vec::new();
        }
        // The box around everything that has to do with shadows.
        let mut lo = Vector3::splat(f32::INFINITY);
        let mut hi = Vector3::splat(f32::NEG_INFINITY);
        for draw in &self.meshes {
            if draw.cast_shadow || draw.receive_shadow {
                let (a, b) = draw.world_bounds();
                lo = lo.min(a);
                hi = hi.max(b);
            }
        }
        if lo.x > hi.x {
            return Vec::new();
        }
        let corners: Vec<Vector3> = (0..8)
            .map(|i| {
                Vector3::new(
                    if i & 1 == 0 { lo.x } else { hi.x },
                    if i & 2 == 0 { lo.y } else { hi.y },
                    if i & 4 == 0 { lo.z } else { hi.z },
                )
            })
            .collect();
        let center = (lo + hi) * 0.5;
        let radius = ((hi - lo).length() * 0.5).max(0.01);
        let mut cameras = Vec::new();
        for request in self.lights.shadows.iter().take(MAX_SHADOWS) {
            let settings = request.settings;
            let camera = match request.light {
                ShadowLight::Directional(i) => {
                    let Some((towards, _)) = self.lights.directional.get(i) else {
                        continue;
                    };
                    let rotation = Quaternion::looking_to(-*towards, up_for(*towards));
                    let (focus, half) = match settings.extent {
                        Some(extent) => {
                            let e = &self.camera.elements;
                            let forward = -Vector3::new(e[8], e[9], e[10]).normalize();
                            (
                                self.camera.translation() + forward * (extent * 0.5),
                                extent.max(0.01),
                            )
                        }
                        None => (center, radius),
                    };
                    // Moved by whole pixels of the map: the edges do not shimmer.
                    let texel = 2.0 * half / settings.map_size.max(1) as f32;
                    let inverse = rotation.inverse();
                    let mut local = focus.apply_quaternion(inverse);
                    local.x = (local.x / texel).round() * texel;
                    local.y = (local.y / texel).round() * texel;
                    let focus = local.apply_quaternion(rotation);
                    let view = Matrix4::compose(focus, rotation, Vector3::ONE).inverse();
                    // Deep enough for everything between the light and the meshes.
                    let (mut near, mut far) = (f32::INFINITY, f32::NEG_INFINITY);
                    for c in &corners {
                        let depth = -c.apply_matrix4(&view).z;
                        near = near.min(depth);
                        far = far.max(depth);
                    }
                    let margin = (far - near).max(0.1) * 0.02 + 0.05;
                    ShadowCamera {
                        light: request.light,
                        view_projection: Matrix4::orthographic(
                            -half,
                            half,
                            half,
                            -half,
                            near - margin,
                            far + margin,
                        ) * view,
                        settings,
                        orthographic: true,
                    }
                }
                ShadowLight::Spot(i) => {
                    let Some(spot) = self.lights.spot.get(i) else {
                        continue;
                    };
                    let rotation = Quaternion::looking_to(spot.direction, up_for(spot.direction));
                    let view = Matrix4::compose(spot.position, rotation, Vector3::ONE).inverse();
                    let farthest = corners
                        .iter()
                        .map(|c| (*c - spot.position).length())
                        .fold(0.0f32, f32::max);
                    let far = if spot.distance > 0.0 {
                        spot.distance.min(farthest.max(0.1))
                    } else {
                        farthest
                    }
                    .max(0.1);
                    let near = (far / 1000.0).max(0.01);
                    let fov = (spot.angle * 2.0 * 1.05).clamp(0.01, 3.0);
                    ShadowCamera {
                        light: request.light,
                        view_projection: Matrix4::perspective(fov, 1.0, near, far) * view,
                        settings,
                        orthographic: false,
                    }
                }
            };
            cameras.push(camera);
        }
        cameras
    }
}

/// An up direction not along `direction`.
fn up_for(direction: Vector3) -> Vector3 {
    if direction.normalize().y.abs() > 0.99 {
        Vector3::Z
    } else {
        Vector3::Y
    }
}

/// How colors brighter than white are brought into the screen's range.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToneMapping {
    /// Clamped (the default).
    #[default]
    None,
    /// The filmic curve of ACES (the filmic look of games).
    AcesFilmic,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{
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

    #[test]
    fn spots_sprites_and_shadow_cameras() {
        use crate::scene::{LightShadow, MeshBasicMaterial, PlaneGeometry, SpotLight, Texture};
        let mut scene = Scene::new();
        let floor = scene.add(Mesh::new(
            PlaneGeometry::new(10.0, 10.0),
            MeshStandardMaterial::new(),
        ));
        scene[floor].rotation.x = -std::f32::consts::FRAC_PI_2;
        scene[floor].receive_shadow = true;
        let cube = scene.add(Mesh::new(
            BoxGeometry::new(1.0, 1.0, 1.0),
            MeshStandardMaterial::new().map(Texture::solid([200, 100, 50, 255])),
        ));
        scene[cube].position.y = 0.5;
        scene[cube].cast_shadow = true;
        let spot = scene.add(SpotLight::aimed(
            0xffffff,
            100.0,
            0.5,
            0.2,
            Vector3::new(0.0, 0.0, 0.0),
        ));
        scene[spot].position.set(0.0, 6.0, 0.0);
        scene[spot].cast_shadow = true;
        let sun = scene.add(DirectionalLight::new(0xffffff, 2.0));
        scene[sun].position.set(3.0, 10.0, 2.0);
        scene[sun].cast_shadow = true;
        scene[sun].shadow = LightShadow {
            map_size: 2048,
            ..LightShadow::default()
        };
        let label = scene.add(Sprite::new(MeshBasicMaterial::new()));
        scene[label].position.set(0.0, 2.0, 0.0);
        scene[label].scale.set(2.0, 1.0, 1.0);

        let mut camera = PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0);
        camera.position.set(0.0, 2.0, 8.0);
        let frame = scene.frame(&camera, 1.0);
        assert_eq!(frame.meshes.len(), 3);
        assert_eq!(frame.lights.spot.len(), 1);
        assert!((frame.lights.spot[0].direction - -Vector3::Y).length() < 1e-5);
        assert_eq!(frame.lights.shadows.len(), 2);
        assert!(frame.meshes[1].cast_shadow && frame.meshes[0].receive_shadow);
        assert!(frame.meshes[0].material.map().is_none());
        assert!(frame.meshes[1].material.map().is_some());

        // The sprite faces the camera: its +z points to it, at its size.
        let sprite = &frame.meshes[2];
        let (at, _, scale) = sprite.world.decompose();
        assert!((at - Vector3::new(0.0, 2.0, 0.0)).length() < 1e-4, "{at:?}");
        assert!((scale - Vector3::new(2.0, 1.0, 1.0)).length() < 1e-4);
        let facing = Vector3::Z.apply_quaternion(sprite.world.decompose().1);
        assert!((facing - Vector3::Z).length() < 1e-4, "{facing:?}");

        // Every shadow camera sees the cube, in its clip space.
        let cameras = frame.shadow_cameras();
        assert_eq!(cameras.len(), 2);
        assert!(!cameras[0].orthographic && cameras[1].orthographic);
        for c in &cameras {
            for corner in [Vector3::new(0.5, 1.0, 0.5), Vector3::new(-0.5, 0.0, -0.5)] {
                let m = &c.view_projection.elements;
                let w = m[3] * corner.x + m[7] * corner.y + m[11] * corner.z + m[15];
                let p = corner.apply_matrix4(&c.view_projection);
                assert!(w > 0.0, "{c:?}");
                for v in [p.x, p.y, p.z] {
                    assert!((-1.0..=1.0).contains(&v), "{p:?} for {:?}", c.light);
                }
            }
        }
        // Without anything casting or receiving, no shadow map.
        scene[floor].receive_shadow = false;
        scene[cube].cast_shadow = false;
        assert!(scene.frame(&camera, 1.0).shadow_cameras().is_empty());
    }
}
