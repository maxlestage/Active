//! Where entities are: `Transform` (relative to the parent), `GlobalTransform` (in the
//! world, computed in `PostUpdate`), the hierarchy (`ChildOf`, `Children`) and visibility.

use std::ops::{Deref, Mul};

use super::query::{Query, With, Without};
use super::world::{Component, Entity, World, require};
use crate::scene::math::{Mat4, Quat, Vec3};

/// Where an entity is, relative to its parent: a translation, a rotation and a scale.
///
/// ```
/// use active::ecs::prelude::*;
///
/// let camera = Transform::from_xyz(0.0, 2.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y);
/// let forward = camera.forward();
/// assert!((forward - (-camera.translation).normalize()).length() < 1e-5);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Transform {
    /// The position.
    pub translation: Vec3,
    /// The rotation.
    pub rotation: Quat,
    /// The scale.
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Transform {
        Transform::IDENTITY
    }
}

impl Component for Transform {
    fn on_insert(world: &mut World, entity: Entity) {
        require::<GlobalTransform>(world, entity);
        require::<Visibility>(world, entity);
    }
}

impl Transform {
    /// No translation, rotation nor scale.
    pub const IDENTITY: Transform = Transform {
        translation: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    /// At a position.
    pub const fn from_xyz(x: f32, y: f32, z: f32) -> Transform {
        Transform::from_translation(Vec3::new(x, y, z))
    }

    /// At a position.
    pub const fn from_translation(translation: Vec3) -> Transform {
        Transform {
            translation,
            ..Transform::IDENTITY
        }
    }

    /// With a rotation.
    pub const fn from_rotation(rotation: Quat) -> Transform {
        Transform {
            rotation,
            ..Transform::IDENTITY
        }
    }

    /// With a scale.
    pub const fn from_scale(scale: Vec3) -> Transform {
        Transform {
            scale,
            ..Transform::IDENTITY
        }
    }

    /// From a matrix.
    pub fn from_matrix(matrix: Mat4) -> Transform {
        let (translation, rotation, scale) = matrix.decompose();
        Transform {
            translation,
            rotation,
            scale,
        }
    }

    /// The same, at another position.
    pub const fn with_translation(mut self, translation: Vec3) -> Transform {
        self.translation = translation;
        self
    }

    /// The same, with another rotation.
    pub const fn with_rotation(mut self, rotation: Quat) -> Transform {
        self.rotation = rotation;
        self
    }

    /// The same, with another scale.
    pub const fn with_scale(mut self, scale: Vec3) -> Transform {
        self.scale = scale;
        self
    }

    /// The same, turned so that its forward (`-z`) points at `target`.
    pub fn looking_at(mut self, target: Vec3, up: Vec3) -> Transform {
        self.look_at(target, up);
        self
    }

    /// The same, turned so that its forward points along `direction`.
    pub fn looking_to(mut self, direction: Vec3, up: Vec3) -> Transform {
        self.look_to(direction, up);
        self
    }

    /// Turns so that its forward points at `target`.
    pub fn look_at(&mut self, target: Vec3, up: Vec3) {
        self.look_to(target - self.translation, up);
    }

    /// Turns so that its forward points along `direction`.
    pub fn look_to(&mut self, direction: Vec3, up: Vec3) {
        if direction.length_squared() > 0.0 {
            self.rotation = Quat::looking_to(direction, up);
        }
    }

    /// Turns by a rotation (around the parent's axes).
    pub fn rotate(&mut self, rotation: Quat) {
        self.rotation = (rotation * self.rotation).normalize();
    }

    /// Turns around the parent's x axis.
    pub fn rotate_x(&mut self, angle: f32) {
        self.rotate(Quat::from_rotation_x(angle));
    }

    /// Turns around the parent's y axis.
    pub fn rotate_y(&mut self, angle: f32) {
        self.rotate(Quat::from_rotation_y(angle));
    }

    /// Turns around the parent's z axis.
    pub fn rotate_z(&mut self, angle: f32) {
        self.rotate(Quat::from_rotation_z(angle));
    }

    /// Turns around an axis.
    pub fn rotate_axis(&mut self, axis: Vec3, angle: f32) {
        self.rotate(Quat::from_axis_angle(axis.normalize(), angle));
    }

    /// Turns by a rotation around its own axes.
    pub fn rotate_local(&mut self, rotation: Quat) {
        self.rotation = (self.rotation * rotation).normalize();
    }

    /// Turns around its own x axis.
    pub fn rotate_local_x(&mut self, angle: f32) {
        self.rotate_local(Quat::from_rotation_x(angle));
    }

    /// Turns around its own y axis.
    pub fn rotate_local_y(&mut self, angle: f32) {
        self.rotate_local(Quat::from_rotation_y(angle));
    }

    /// Turns around its own z axis.
    pub fn rotate_local_z(&mut self, angle: f32) {
        self.rotate_local(Quat::from_rotation_z(angle));
    }

    /// Turns around a point.
    pub fn rotate_around(&mut self, point: Vec3, rotation: Quat) {
        self.translation = point + rotation * (self.translation - point);
        self.rotate(rotation);
    }

    /// Where its `-z` points.
    pub fn forward(&self) -> Vec3 {
        self.rotation * Vec3::NEG_Z
    }

    /// Where its `+z` points.
    pub fn back(&self) -> Vec3 {
        self.rotation * Vec3::Z
    }

    /// Where its `+x` points.
    pub fn right(&self) -> Vec3 {
        self.rotation * Vec3::X
    }

    /// Where its `-x` points.
    pub fn left(&self) -> Vec3 {
        self.rotation * Vec3::NEG_X
    }

    /// Where its `+y` points.
    pub fn up(&self) -> Vec3 {
        self.rotation * Vec3::Y
    }

    /// Where its `-y` points.
    pub fn down(&self) -> Vec3 {
        self.rotation * Vec3::NEG_Y
    }

    /// The matrix (scale, then rotation, then translation).
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::compose(self.translation, self.rotation, self.scale)
    }

    /// A point of the entity's space, in the parent's space.
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        self.translation + self.rotation * point.mul_vec(self.scale)
    }

    /// This transform, then `other` inside it.
    pub fn mul_transform(&self, other: Transform) -> Transform {
        Transform {
            translation: self.transform_point(other.translation),
            rotation: (self.rotation * other.rotation).normalize(),
            scale: self.scale.mul_vec(other.scale),
        }
    }
}

impl Mul for Transform {
    type Output = Transform;
    fn mul(self, other: Transform) -> Transform {
        self.mul_transform(other)
    }
}

impl From<Transform> for Mat4 {
    fn from(t: Transform) -> Mat4 {
        t.to_matrix()
    }
}

/// Where an entity is in the world: its `Transform` times its parents'. Computed in
/// `PostUpdate`; read it, do not write it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobalTransform(Mat4);

impl Default for GlobalTransform {
    fn default() -> GlobalTransform {
        GlobalTransform(Mat4::IDENTITY)
    }
}

impl Component for GlobalTransform {}

impl GlobalTransform {
    /// No transform.
    pub const IDENTITY: GlobalTransform = GlobalTransform(Mat4::IDENTITY);

    /// The matrix.
    pub fn to_matrix(&self) -> Mat4 {
        self.0
    }

    /// The position.
    pub fn translation(&self) -> Vec3 {
        self.0.translation()
    }

    /// As a `Transform`.
    pub fn compute_transform(&self) -> Transform {
        Transform::from_matrix(self.0)
    }

    /// Where its `-z` points, in the world.
    pub fn forward(&self) -> Vec3 {
        let e = &self.0.elements;
        -Vec3::new(e[8], e[9], e[10]).normalize()
    }

    /// Where its `+z` points.
    pub fn back(&self) -> Vec3 {
        -self.forward()
    }

    /// A point of the entity's space, in the world.
    pub fn transform_point(&self, point: Vec3) -> Vec3 {
        self.0 * point
    }
}

impl From<Transform> for GlobalTransform {
    fn from(t: Transform) -> GlobalTransform {
        GlobalTransform(t.to_matrix())
    }
}

impl Mul<Transform> for GlobalTransform {
    type Output = GlobalTransform;
    fn mul(self, t: Transform) -> GlobalTransform {
        GlobalTransform(self.0 * t.to_matrix())
    }
}

// ----- The hierarchy --------------------------------------------------------------------

/// The parent of an entity: `commands.spawn((Name, ChildOf(parent)))`. Its `Transform` is
/// relative to the parent's, and despawning the parent despawns it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChildOf(pub Entity);

impl ChildOf {
    /// The parent.
    pub fn parent(&self) -> Entity {
        self.0
    }
}

impl Component for ChildOf {
    fn on_insert(world: &mut World, entity: Entity) {
        let Some(parent) = world.get::<ChildOf>(entity).map(|c| c.0) else {
            return;
        };
        if !world.contains(parent) {
            return;
        }
        if let Some(mut children) = world.get_mut::<Children>(parent) {
            if !children.0.contains(&entity) {
                children.0.push(entity);
            }
            return;
        }
        world.insert(parent, Children(vec![entity]));
    }

    fn on_remove(world: &mut World, entity: Entity) {
        let Some(parent) = world.get::<ChildOf>(entity).map(|c| c.0) else {
            return;
        };
        let empty = match world.get_mut::<Children>(parent) {
            Some(mut children) => {
                children.0.retain(|e| *e != entity);
                children.0.is_empty()
            }
            None => false,
        };
        if empty {
            world.remove::<Children>(parent);
        }
    }
}

/// The children of an entity, kept up to date from their `ChildOf`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Children(Vec<Entity>);

impl Component for Children {}

impl Deref for Children {
    type Target = [Entity];
    fn deref(&self) -> &[Entity] {
        &self.0
    }
}

impl<'a> IntoIterator for &'a Children {
    type Item = &'a Entity;
    type IntoIter = std::slice::Iter<'a, Entity>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// Whether an entity is drawn: `Inherited` follows the parent.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Visibility {
    /// Like its parent (visible without one).
    #[default]
    Inherited,
    /// Hidden, with its children.
    Hidden,
    /// Visible, even if the parent is hidden.
    Visible,
}

impl Component for Visibility {
    fn on_insert(world: &mut World, entity: Entity) {
        require::<InheritedVisibility>(world, entity);
    }
}

/// Whether the entity ends up visible (computed in `PostUpdate`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InheritedVisibility(bool);

impl Default for InheritedVisibility {
    fn default() -> InheritedVisibility {
        InheritedVisibility(true)
    }
}

impl Component for InheritedVisibility {}

impl InheritedVisibility {
    /// Whether it is visible.
    pub fn get(&self) -> bool {
        self.0
    }
}

/// Computes the `GlobalTransform` of every entity from the roots down.
pub fn propagate_transforms(
    roots: Query<'_, '_, (Entity, &Transform), Without<ChildOf>>,
    nodes: Query<'_, '_, (&Transform, Option<&Children>)>,
    root_children: Query<'_, '_, &Children, Without<ChildOf>>,
    mut globals: Query<'_, '_, &mut GlobalTransform>,
) {
    fn walk(
        entity: Entity,
        parent: Mat4,
        nodes: &Query<'_, '_, (&Transform, Option<&Children>)>,
        globals: &mut Query<'_, '_, &mut GlobalTransform>,
        depth: usize,
    ) {
        let Ok((transform, children)) = nodes.get(entity) else {
            return;
        };
        let world = parent * transform.to_matrix();
        if let Ok(mut global) = globals.get_mut(entity)
            && global.0 != world
        {
            global.0 = world;
        }
        let children: Vec<Entity> = children.map(|c| c.to_vec()).unwrap_or_default();
        drop(transform);
        // A cycle of parents would never end: stop deep down.
        if depth < 1024 {
            for child in children {
                walk(child, world, nodes, globals, depth + 1);
            }
        }
    }
    for (entity, transform) in &roots {
        let matrix = transform.to_matrix();
        if let Ok(mut global) = globals.get_mut(entity)
            && global.0 != matrix
        {
            global.0 = matrix;
        }
        drop(transform);
        if let Ok(children) = root_children.get(entity) {
            let children = children.to_vec();
            for child in children {
                walk(child, matrix, &nodes, &mut globals, 1);
            }
        }
    }
}

/// Computes the `InheritedVisibility` of every entity from the roots down.
pub fn propagate_visibility(
    roots: Query<'_, '_, (Entity, &Visibility, Option<&Children>), Without<ChildOf>>,
    nodes: Query<'_, '_, (&Visibility, Option<&Children>), With<ChildOf>>,
    mut inherited: Query<'_, '_, &mut InheritedVisibility>,
) {
    fn walk(
        entity: Entity,
        parent_visible: bool,
        nodes: &Query<'_, '_, (&Visibility, Option<&Children>), With<ChildOf>>,
        inherited: &mut Query<'_, '_, &mut InheritedVisibility>,
        depth: usize,
    ) {
        let Ok((visibility, children)) = nodes.get(entity) else {
            return;
        };
        let visible = match *visibility {
            Visibility::Inherited => parent_visible,
            Visibility::Hidden => false,
            Visibility::Visible => true,
        };
        if let Ok(mut v) = inherited.get_mut(entity)
            && v.0 != visible
        {
            v.0 = visible;
        }
        let children: Vec<Entity> = children.map(|c| c.to_vec()).unwrap_or_default();
        if depth < 1024 {
            for child in children {
                walk(child, visible, nodes, inherited, depth + 1);
            }
        }
    }
    for (entity, visibility, children) in &roots {
        let visible = *visibility != Visibility::Hidden;
        if let Ok(mut v) = inherited.get_mut(entity)
            && v.0 != visible
        {
            v.0 = visible;
        }
        for child in children.map(|c| c.to_vec()).unwrap_or_default() {
            walk(child, visible, &nodes, &mut inherited, 1);
        }
    }
}
