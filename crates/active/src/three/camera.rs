//! Cameras (Three.js's `PerspectiveCamera` and `OrthographicCamera`): where the scene is
//! seen from, and how it is projected on the screen.

use super::math::{Euler, Matrix4, Quaternion, Vector3, deg_to_rad};

/// What every camera does.
pub trait Camera {
    /// Where it is and where it looks, as a world matrix.
    fn world_matrix(&self) -> Matrix4;
    /// The projection, for a viewport of this aspect ratio (width / height).
    fn projection(&self, aspect: f32) -> Matrix4;
}

/// A camera that sees like an eye: far things look smaller.
#[derive(Clone, Debug, PartialEq)]
pub struct PerspectiveCamera {
    /// The vertical field of view, in degrees.
    pub fov: f32,
    /// Width / height; 0 takes the canvas's.
    pub aspect: f32,
    /// Nothing nearer is drawn.
    pub near: f32,
    /// Nothing farther is drawn.
    pub far: f32,
    /// Its position.
    pub position: Vector3,
    /// Its rotation (it looks along its `-z`).
    pub rotation: Euler,
    /// The direction of up, for [`look_at`](Self::look_at).
    pub up: Vector3,
}

impl PerspectiveCamera {
    /// A camera at the origin looking along `-z` (`new THREE.PerspectiveCamera(75, aspect,
    /// 0.1, 1000)`).
    pub fn new(fov: f32, aspect: f32, near: f32, far: f32) -> PerspectiveCamera {
        PerspectiveCamera {
            fov,
            aspect,
            near,
            far,
            position: Vector3::ZERO,
            rotation: Euler::default(),
            up: Vector3::Y,
        }
    }

    /// Turns to look at a point.
    pub fn look_at(&mut self, target: impl Into<Vector3>) {
        self.rotation = euler_looking(self.position, target.into(), self.up);
    }
}

impl Camera for PerspectiveCamera {
    fn world_matrix(&self) -> Matrix4 {
        Matrix4::compose(
            self.position,
            Quaternion::from_euler(self.rotation),
            Vector3::ONE,
        )
    }

    fn projection(&self, aspect: f32) -> Matrix4 {
        let aspect = if self.aspect > 0.0 {
            self.aspect
        } else {
            aspect
        };
        Matrix4::perspective(deg_to_rad(self.fov), aspect, self.near, self.far)
    }
}

/// A camera without perspective: sizes do not change with distance (plans, 2D, CAD).
#[derive(Clone, Debug, PartialEq)]
pub struct OrthographicCamera {
    /// The left edge of the view, in world units.
    pub left: f32,
    /// The right edge.
    pub right: f32,
    /// The top edge.
    pub top: f32,
    /// The bottom edge.
    pub bottom: f32,
    /// Nothing nearer is drawn.
    pub near: f32,
    /// Nothing farther is drawn.
    pub far: f32,
    /// Its position.
    pub position: Vector3,
    /// Its rotation.
    pub rotation: Euler,
    /// Up, for [`look_at`](Self::look_at).
    pub up: Vector3,
}

impl OrthographicCamera {
    /// A camera seeing the box from `left` to `right`, `bottom` to `top`.
    pub fn new(
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
        near: f32,
        far: f32,
    ) -> OrthographicCamera {
        OrthographicCamera {
            left,
            right,
            top,
            bottom,
            near,
            far,
            position: Vector3::ZERO,
            rotation: Euler::default(),
            up: Vector3::Y,
        }
    }

    /// Turns to look at a point.
    pub fn look_at(&mut self, target: impl Into<Vector3>) {
        self.rotation = euler_looking(self.position, target.into(), self.up);
    }
}

impl Camera for OrthographicCamera {
    fn world_matrix(&self) -> Matrix4 {
        Matrix4::compose(
            self.position,
            Quaternion::from_euler(self.rotation),
            Vector3::ONE,
        )
    }

    fn projection(&self, _aspect: f32) -> Matrix4 {
        Matrix4::orthographic(
            self.left,
            self.right,
            self.top,
            self.bottom,
            self.near,
            self.far,
        )
    }
}

/// The Euler angles (order `XYZ`) of a rotation looking from `eye` to `target`.
pub(crate) fn euler_looking(eye: Vector3, target: Vector3, up: Vector3) -> Euler {
    euler_from_matrix(&Matrix4::look_rotation(target - eye, up))
}

/// The `XYZ` Euler angles of a rotation matrix (Three.js's `setFromRotationMatrix`).
pub(crate) fn euler_from_matrix(m: &Matrix4) -> Euler {
    let e = &m.elements;
    let (m11, m12, m13) = (e[0], e[4], e[8]);
    let (m22, m23) = (e[5], e[9]);
    let (m32, m33) = (e[6], e[10]);
    let y = m13.clamp(-1.0, 1.0).asin();
    let (x, z) = if m13.abs() < 0.999_999_9 {
        ((-m23).atan2(m33), (-m12).atan2(m11))
    } else {
        (m32.atan2(m22), 0.0)
    };
    Euler::new(x, y, z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn look_at_points_minus_z_at_the_target() {
        let mut camera = PerspectiveCamera::new(60.0, 1.0, 0.1, 100.0);
        camera.position = Vector3::new(3.0, 4.0, 5.0);
        camera.look_at(Vector3::ZERO);
        let forward = Quaternion::from_euler(camera.rotation) * Vector3::NEG_Z;
        assert!(
            (forward - (-camera.position).normalize()).length() < 1e-4,
            "{forward:?}"
        );
        // La cible se projette au centre de l'écran.
        let view = camera.world_matrix().inverse();
        let clip = camera.projection(1.0) * (view * Vector3::ZERO);
        assert!(clip.x.abs() < 1e-4 && clip.y.abs() < 1e-4 && clip.z > -1.0 && clip.z < 1.0);
    }
}
