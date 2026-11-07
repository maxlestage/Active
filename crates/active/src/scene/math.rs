//! The maths of 3D: vectors, quaternions, Euler angles, 4×4 matrices and colors.
//!
//! `Vector3`, `Quaternion`, `Matrix4` and `Color`, with the short aliases `Vec3`, `Quat`
//! and `Mat4` used by the game engine. Matrices are stored in column-major order, as WebGL
//! wants them; angles are in radians.

use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

/// A 2D vector (texture coordinates, pointer positions).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector2 {
    /// The first component.
    pub x: f32,
    /// The second component.
    pub y: f32,
}

impl Vector2 {
    /// `(0, 0)`.
    pub const ZERO: Vector2 = Vector2 { x: 0.0, y: 0.0 };

    /// A vector from its components.
    pub const fn new(x: f32, y: f32) -> Vector2 {
        Vector2 { x, y }
    }

    /// Its length.
    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }
}

/// A 3D vector: a position, a direction or a scale.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector3 {
    /// The `x` component (right).
    pub x: f32,
    /// The `y` component (up).
    pub y: f32,
    /// The `z` component (towards the viewer).
    pub z: f32,
}

/// The short name of [`Vector3`].
pub type Vec3 = Vector3;

/// The short name of [`Vector2`].
pub type Vec2 = Vector2;

impl Vector3 {
    /// `(0, 0, 0)`.
    pub const ZERO: Vector3 = Vector3::new(0.0, 0.0, 0.0);
    /// `(1, 1, 1)`.
    pub const ONE: Vector3 = Vector3::new(1.0, 1.0, 1.0);
    /// The unit `x` axis.
    pub const X: Vector3 = Vector3::new(1.0, 0.0, 0.0);
    /// The unit `y` axis, up.
    pub const Y: Vector3 = Vector3::new(0.0, 1.0, 0.0);
    /// The unit `z` axis.
    pub const Z: Vector3 = Vector3::new(0.0, 0.0, 1.0);
    /// `-x`.
    pub const NEG_X: Vector3 = Vector3::new(-1.0, 0.0, 0.0);
    /// `-y`, down.
    pub const NEG_Y: Vector3 = Vector3::new(0.0, -1.0, 0.0);
    /// `-z`, the direction a camera looks at.
    pub const NEG_Z: Vector3 = Vector3::new(0.0, 0.0, -1.0);

    /// A vector from its components.
    pub const fn new(x: f32, y: f32, z: f32) -> Vector3 {
        Vector3 { x, y, z }
    }

    /// The same value in every component.
    pub const fn splat(v: f32) -> Vector3 {
        Vector3::new(v, v, v)
    }

    /// Sets the three components.
    pub fn set(&mut self, x: f32, y: f32, z: f32) -> &mut Self {
        *self = Vector3::new(x, y, z);
        self
    }

    /// The dot product.
    pub fn dot(self, other: Vector3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// The cross product.
    pub fn cross(self, other: Vector3) -> Vector3 {
        Vector3::new(
            self.y * other.z - self.z * other.y,
            self.z * other.x - self.x * other.z,
            self.x * other.y - self.y * other.x,
        )
    }

    /// The square of the length.
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    /// The length.
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// The distance to another point.
    pub fn distance(self, other: Vector3) -> f32 {
        (self - other).length()
    }

    /// The same direction with a length of 1 (zero stays zero).
    pub fn normalize(self) -> Vector3 {
        let len = self.length();
        if len > 0.0 { self / len } else { Vector3::ZERO }
    }

    /// Linear interpolation towards `other`: `t = 0` is `self`, `t = 1` is `other`.
    pub fn lerp(self, other: Vector3, t: f32) -> Vector3 {
        self + (other - self) * t
    }

    /// Multiplies component by component.
    pub fn mul_vec(self, other: Vector3) -> Vector3 {
        Vector3::new(self.x * other.x, self.y * other.y, self.z * other.z)
    }

    /// The smallest components of both.
    pub fn min(self, other: Vector3) -> Vector3 {
        Vector3::new(
            self.x.min(other.x),
            self.y.min(other.y),
            self.z.min(other.z),
        )
    }

    /// The largest components of both.
    pub fn max(self, other: Vector3) -> Vector3 {
        Vector3::new(
            self.x.max(other.x),
            self.y.max(other.y),
            self.z.max(other.z),
        )
    }

    /// The point transformed by a matrix (with the perspective division).
    pub fn apply_matrix4(self, m: &Matrix4) -> Vector3 {
        let e = &m.elements;
        let w = e[3] * self.x + e[7] * self.y + e[11] * self.z + e[15];
        let w = if w != 0.0 { 1.0 / w } else { 1.0 };
        Vector3::new(
            (e[0] * self.x + e[4] * self.y + e[8] * self.z + e[12]) * w,
            (e[1] * self.x + e[5] * self.y + e[9] * self.z + e[13]) * w,
            (e[2] * self.x + e[6] * self.y + e[10] * self.z + e[14]) * w,
        )
    }

    /// The vector rotated by a quaternion.
    pub fn apply_quaternion(self, q: Quaternion) -> Vector3 {
        q * self
    }

    /// The components as an array.
    pub fn to_array(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }
}

impl Add for Vector2 {
    type Output = Vector2;
    fn add(self, o: Vector2) -> Vector2 {
        Vector2::new(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Vector2 {
    type Output = Vector2;
    fn sub(self, o: Vector2) -> Vector2 {
        Vector2::new(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f32> for Vector2 {
    type Output = Vector2;
    fn mul(self, s: f32) -> Vector2 {
        Vector2::new(self.x * s, self.y * s)
    }
}

impl AddAssign for Vector2 {
    fn add_assign(&mut self, o: Vector2) {
        *self = *self + o;
    }
}

impl From<[f32; 3]> for Vector3 {
    fn from([x, y, z]: [f32; 3]) -> Vector3 {
        Vector3::new(x, y, z)
    }
}

impl Add for Vector3 {
    type Output = Vector3;
    fn add(self, o: Vector3) -> Vector3 {
        Vector3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl AddAssign for Vector3 {
    fn add_assign(&mut self, o: Vector3) {
        *self = *self + o;
    }
}

impl Sub for Vector3 {
    type Output = Vector3;
    fn sub(self, o: Vector3) -> Vector3 {
        Vector3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl SubAssign for Vector3 {
    fn sub_assign(&mut self, o: Vector3) {
        *self = *self - o;
    }
}

impl Mul<f32> for Vector3 {
    type Output = Vector3;
    fn mul(self, s: f32) -> Vector3 {
        Vector3::new(self.x * s, self.y * s, self.z * s)
    }
}

impl Mul<Vector3> for f32 {
    type Output = Vector3;
    fn mul(self, v: Vector3) -> Vector3 {
        v * self
    }
}

impl MulAssign<f32> for Vector3 {
    fn mul_assign(&mut self, s: f32) {
        *self = *self * s;
    }
}

impl Div<f32> for Vector3 {
    type Output = Vector3;
    fn div(self, s: f32) -> Vector3 {
        Vector3::new(self.x / s, self.y / s, self.z / s)
    }
}

impl Neg for Vector3 {
    type Output = Vector3;
    fn neg(self) -> Vector3 {
        Vector3::new(-self.x, -self.y, -self.z)
    }
}

/// The order in which the rotations of an [`Euler`] apply (`XYZ` by default).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EulerOrder {
    /// Around `x`, then `y`, then `z` (as the matrix product `X · Y · Z`).
    #[default]
    XYZ,
    /// `Y · X · Z`: yaw, pitch, roll, handy for cameras.
    YXZ,
    /// `Z · X · Y`.
    ZXY,
    /// `Z · Y · X`.
    ZYX,
    /// `Y · Z · X`.
    YZX,
    /// `X · Z · Y`.
    XZY,
}

/// A rotation as three angles around the axes, in radians.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Euler {
    /// The angle around `x`.
    pub x: f32,
    /// The angle around `y`.
    pub y: f32,
    /// The angle around `z`.
    pub z: f32,
    /// The order of the rotations.
    pub order: EulerOrder,
}

impl Euler {
    /// Angles in the default order (`XYZ`).
    pub const fn new(x: f32, y: f32, z: f32) -> Euler {
        Euler {
            x,
            y,
            z,
            order: EulerOrder::XYZ,
        }
    }

    /// Sets the three angles.
    pub fn set(&mut self, x: f32, y: f32, z: f32) -> &mut Self {
        self.x = x;
        self.y = y;
        self.z = z;
        self
    }
}

/// A rotation, as a unit quaternion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quaternion {
    /// The `x` part of the axis.
    pub x: f32,
    /// The `y` part of the axis.
    pub y: f32,
    /// The `z` part of the axis.
    pub z: f32,
    /// The scalar part.
    pub w: f32,
}

/// The short name of [`Quaternion`].
pub type Quat = Quaternion;

impl Default for Quaternion {
    fn default() -> Quaternion {
        Quaternion::IDENTITY
    }
}

impl Quaternion {
    /// No rotation.
    pub const IDENTITY: Quaternion = Quaternion {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        w: 1.0,
    };

    /// A quaternion from its components (normalize it if it is not a unit one).
    pub const fn from_xyzw(x: f32, y: f32, z: f32, w: f32) -> Quaternion {
        Quaternion { x, y, z, w }
    }

    /// A rotation of `angle` radians around `axis` (which must be a unit vector).
    pub fn from_axis_angle(axis: Vector3, angle: f32) -> Quaternion {
        let (s, c) = (angle * 0.5).sin_cos();
        Quaternion::from_xyzw(axis.x * s, axis.y * s, axis.z * s, c)
    }

    /// A rotation around `x`.
    pub fn from_rotation_x(angle: f32) -> Quaternion {
        Quaternion::from_axis_angle(Vector3::X, angle)
    }

    /// A rotation around `y`.
    pub fn from_rotation_y(angle: f32) -> Quaternion {
        Quaternion::from_axis_angle(Vector3::Y, angle)
    }

    /// A rotation around `z`.
    pub fn from_rotation_z(angle: f32) -> Quaternion {
        Quaternion::from_axis_angle(Vector3::Z, angle)
    }

    /// The rotation of Euler angles.
    pub fn from_euler(e: Euler) -> Quaternion {
        let (x, y, z) = (
            Quaternion::from_rotation_x(e.x),
            Quaternion::from_rotation_y(e.y),
            Quaternion::from_rotation_z(e.z),
        );
        match e.order {
            EulerOrder::XYZ => x * y * z,
            EulerOrder::YXZ => y * x * z,
            EulerOrder::ZXY => z * x * y,
            EulerOrder::ZYX => z * y * x,
            EulerOrder::YZX => y * z * x,
            EulerOrder::XZY => x * z * y,
        }
    }

    /// The rotation that turns `-z` towards `forward`, with `up` as near to `+y` as can be
    /// (what a camera does with `look_at`).
    pub fn looking_to(forward: Vector3, up: Vector3) -> Quaternion {
        Quaternion::from_rotation_matrix(&Matrix4::look_rotation(forward, up))
    }

    /// The rotation part of a matrix without scale.
    pub fn from_rotation_matrix(m: &Matrix4) -> Quaternion {
        let e = &m.elements;
        let (m11, m12, m13) = (e[0], e[4], e[8]);
        let (m21, m22, m23) = (e[1], e[5], e[9]);
        let (m31, m32, m33) = (e[2], e[6], e[10]);
        let trace = m11 + m22 + m33;
        let q = if trace > 0.0 {
            let s = 0.5 / (trace + 1.0).sqrt();
            Quaternion::from_xyzw((m32 - m23) * s, (m13 - m31) * s, (m21 - m12) * s, 0.25 / s)
        } else if m11 > m22 && m11 > m33 {
            let s = 2.0 * (1.0 + m11 - m22 - m33).sqrt();
            Quaternion::from_xyzw(0.25 * s, (m12 + m21) / s, (m13 + m31) / s, (m32 - m23) / s)
        } else if m22 > m33 {
            let s = 2.0 * (1.0 + m22 - m11 - m33).sqrt();
            Quaternion::from_xyzw((m12 + m21) / s, 0.25 * s, (m23 + m32) / s, (m13 - m31) / s)
        } else {
            let s = 2.0 * (1.0 + m33 - m11 - m22).sqrt();
            Quaternion::from_xyzw((m13 + m31) / s, (m23 + m32) / s, 0.25 * s, (m21 - m12) / s)
        };
        q.normalize()
    }

    /// The same rotation with a length of 1.
    pub fn normalize(self) -> Quaternion {
        let len = (self.x * self.x + self.y * self.y + self.z * self.z + self.w * self.w).sqrt();
        if len == 0.0 {
            Quaternion::IDENTITY
        } else {
            Quaternion::from_xyzw(self.x / len, self.y / len, self.z / len, self.w / len)
        }
    }

    /// The opposite rotation.
    pub fn inverse(self) -> Quaternion {
        Quaternion::from_xyzw(-self.x, -self.y, -self.z, self.w)
    }

    /// The shortest rotation turning the unit vector `from` into the unit vector `to`
    /// (`from_rotation_arc`).
    pub fn from_unit_vectors(from: Vector3, to: Vector3) -> Quaternion {
        let r = from.dot(to) + 1.0;
        if r < 1e-6 {
            // Opposite vectors: half a turn around any perpendicular axis.
            if from.x.abs() > from.z.abs() {
                Quaternion::from_xyzw(-from.y, from.x, 0.0, 0.0).normalize()
            } else {
                Quaternion::from_xyzw(0.0, -from.z, from.y, 0.0).normalize()
            }
        } else {
            let c = from.cross(to);
            Quaternion::from_xyzw(c.x, c.y, c.z, r).normalize()
        }
    }

    /// The dot product, near ±1 for close rotations.
    pub fn dot(self, o: Quaternion) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z + self.w * o.w
    }

    /// Spherical interpolation towards `other` (`t` from 0 to 1).
    pub fn slerp(self, other: Quaternion, t: f32) -> Quaternion {
        let mut cos = self.dot(other);
        let mut to = other;
        if cos < 0.0 {
            cos = -cos;
            to = Quaternion::from_xyzw(-to.x, -to.y, -to.z, -to.w);
        }
        if cos > 0.9995 {
            return Quaternion::from_xyzw(
                self.x + (to.x - self.x) * t,
                self.y + (to.y - self.y) * t,
                self.z + (to.z - self.z) * t,
                self.w + (to.w - self.w) * t,
            )
            .normalize();
        }
        let angle = cos.acos();
        let sin = angle.sin();
        let (a, b) = (((1.0 - t) * angle).sin() / sin, (t * angle).sin() / sin);
        Quaternion::from_xyzw(
            self.x * a + to.x * b,
            self.y * a + to.y * b,
            self.z * a + to.z * b,
            self.w * a + to.w * b,
        )
    }
}

impl Mul for Quaternion {
    type Output = Quaternion;
    /// `a * b` rotates by `b`, then by `a`.
    fn mul(self, b: Quaternion) -> Quaternion {
        let a = self;
        Quaternion::from_xyzw(
            a.x * b.w + a.w * b.x + a.y * b.z - a.z * b.y,
            a.y * b.w + a.w * b.y + a.z * b.x - a.x * b.z,
            a.z * b.w + a.w * b.z + a.x * b.y - a.y * b.x,
            a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
        )
    }
}

impl MulAssign for Quaternion {
    fn mul_assign(&mut self, b: Quaternion) {
        *self = *self * b;
    }
}

impl Mul<Vector3> for Quaternion {
    type Output = Vector3;
    fn mul(self, v: Vector3) -> Vector3 {
        // t = 2 q × v ; v' = v + w t + q × t
        let q = Vector3::new(self.x, self.y, self.z);
        let t = q.cross(v) * 2.0;
        v + t * self.w + q.cross(t)
    }
}

/// A 4×4 matrix in column-major order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix4 {
    /// The 16 numbers, column after column.
    pub elements: [f32; 16],
}

/// The short name of [`Matrix4`].
pub type Mat4 = Matrix4;

impl Default for Matrix4 {
    fn default() -> Matrix4 {
        Matrix4::IDENTITY
    }
}

impl Matrix4 {
    /// The identity.
    pub const IDENTITY: Matrix4 = Matrix4 {
        elements: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    };

    /// The matrix of a translation, a rotation and a scale (applied scale first).
    pub fn compose(position: Vector3, rotation: Quaternion, scale: Vector3) -> Matrix4 {
        let Quaternion { x, y, z, w } = rotation;
        let (x2, y2, z2) = (x + x, y + y, z + z);
        let (xx, xy, xz) = (x * x2, x * y2, x * z2);
        let (yy, yz, zz) = (y * y2, y * z2, z * z2);
        let (wx, wy, wz) = (w * x2, w * y2, w * z2);
        let (sx, sy, sz) = (scale.x, scale.y, scale.z);
        Matrix4 {
            elements: [
                (1.0 - (yy + zz)) * sx,
                (xy + wz) * sx,
                (xz - wy) * sx,
                0.0,
                (xy - wz) * sy,
                (1.0 - (xx + zz)) * sy,
                (yz + wx) * sy,
                0.0,
                (xz + wy) * sz,
                (yz - wx) * sz,
                (1.0 - (xx + yy)) * sz,
                0.0,
                position.x,
                position.y,
                position.z,
                1.0,
            ],
        }
    }

    /// The translation, the rotation and the scale of a matrix made by [`compose`](Self::compose).
    pub fn decompose(&self) -> (Vector3, Quaternion, Vector3) {
        let e = &self.elements;
        let mut sx = Vector3::new(e[0], e[1], e[2]).length();
        let sy = Vector3::new(e[4], e[5], e[6]).length();
        let sz = Vector3::new(e[8], e[9], e[10]).length();
        if self.determinant() < 0.0 {
            sx = -sx;
        }
        let mut m = *self;
        let (ix, iy, iz) = (1.0 / sx, 1.0 / sy, 1.0 / sz);
        for i in 0..3 {
            m.elements[i] *= ix;
            m.elements[4 + i] *= iy;
            m.elements[8 + i] *= iz;
        }
        (
            Vector3::new(e[12], e[13], e[14]),
            Quaternion::from_rotation_matrix(&m),
            Vector3::new(sx, sy, sz),
        )
    }

    /// The rotation matrix whose `-z` axis looks along `forward`.
    pub fn look_rotation(forward: Vector3, up: Vector3) -> Matrix4 {
        let z = (-forward).normalize();
        let z = if z == Vector3::ZERO { Vector3::Z } else { z };
        let mut x = up.cross(z);
        if x.length_squared() < 1e-12 {
            // `up` le long du regard : on en prend un autre.
            let alt = if z.z.abs() > 0.9 {
                Vector3::X
            } else {
                Vector3::Z
            };
            x = alt.cross(z);
        }
        let x = x.normalize();
        let y = z.cross(x);
        Matrix4 {
            elements: [
                x.x, x.y, x.z, 0.0, y.x, y.y, y.z, 0.0, z.x, z.y, z.z, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        }
    }

    /// A perspective projection (`fov_y` in radians), for a right-handed view space looking
    /// along `-z`, into WebGL's clip space (depth from -1 to 1).
    pub fn perspective(fov_y: f32, aspect: f32, near: f32, far: f32) -> Matrix4 {
        let f = 1.0 / (fov_y * 0.5).tan();
        let nf = 1.0 / (near - far);
        Matrix4 {
            elements: [
                f / aspect,
                0.0,
                0.0,
                0.0,
                0.0,
                f,
                0.0,
                0.0,
                0.0,
                0.0,
                (far + near) * nf,
                -1.0,
                0.0,
                0.0,
                2.0 * far * near * nf,
                0.0,
            ],
        }
    }

    /// An orthographic projection.
    pub fn orthographic(
        left: f32,
        right: f32,
        top: f32,
        bottom: f32,
        near: f32,
        far: f32,
    ) -> Matrix4 {
        let (w, h, d) = (
            1.0 / (right - left),
            1.0 / (top - bottom),
            1.0 / (far - near),
        );
        Matrix4 {
            elements: [
                2.0 * w,
                0.0,
                0.0,
                0.0,
                0.0,
                2.0 * h,
                0.0,
                0.0,
                0.0,
                0.0,
                -2.0 * d,
                0.0,
                -(right + left) * w,
                -(top + bottom) * h,
                -(far + near) * d,
                1.0,
            ],
        }
    }

    /// The determinant.
    pub fn determinant(&self) -> f32 {
        let m = &self.elements;
        let (a00, a01, a02, a03) = (m[0], m[1], m[2], m[3]);
        let (a10, a11, a12, a13) = (m[4], m[5], m[6], m[7]);
        let (a20, a21, a22, a23) = (m[8], m[9], m[10], m[11]);
        let (a30, a31, a32, a33) = (m[12], m[13], m[14], m[15]);
        let b00 = a00 * a11 - a01 * a10;
        let b01 = a00 * a12 - a02 * a10;
        let b02 = a00 * a13 - a03 * a10;
        let b03 = a01 * a12 - a02 * a11;
        let b04 = a01 * a13 - a03 * a11;
        let b05 = a02 * a13 - a03 * a12;
        let b06 = a20 * a31 - a21 * a30;
        let b07 = a20 * a32 - a22 * a30;
        let b08 = a20 * a33 - a23 * a30;
        let b09 = a21 * a32 - a22 * a31;
        let b10 = a21 * a33 - a23 * a31;
        let b11 = a22 * a33 - a23 * a32;
        b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06
    }

    /// The inverse (the identity if the matrix cannot be inverted).
    pub fn inverse(&self) -> Matrix4 {
        let m = &self.elements;
        let (a00, a01, a02, a03) = (m[0], m[1], m[2], m[3]);
        let (a10, a11, a12, a13) = (m[4], m[5], m[6], m[7]);
        let (a20, a21, a22, a23) = (m[8], m[9], m[10], m[11]);
        let (a30, a31, a32, a33) = (m[12], m[13], m[14], m[15]);
        let b00 = a00 * a11 - a01 * a10;
        let b01 = a00 * a12 - a02 * a10;
        let b02 = a00 * a13 - a03 * a10;
        let b03 = a01 * a12 - a02 * a11;
        let b04 = a01 * a13 - a03 * a11;
        let b05 = a02 * a13 - a03 * a12;
        let b06 = a20 * a31 - a21 * a30;
        let b07 = a20 * a32 - a22 * a30;
        let b08 = a20 * a33 - a23 * a30;
        let b09 = a21 * a32 - a22 * a31;
        let b10 = a21 * a33 - a23 * a31;
        let b11 = a22 * a33 - a23 * a32;
        let det = b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06;
        if det == 0.0 {
            return Matrix4::IDENTITY;
        }
        let d = 1.0 / det;
        Matrix4 {
            elements: [
                (a11 * b11 - a12 * b10 + a13 * b09) * d,
                (a02 * b10 - a01 * b11 - a03 * b09) * d,
                (a31 * b05 - a32 * b04 + a33 * b03) * d,
                (a22 * b04 - a21 * b05 - a23 * b03) * d,
                (a12 * b08 - a10 * b11 - a13 * b07) * d,
                (a00 * b11 - a02 * b08 + a03 * b07) * d,
                (a32 * b02 - a30 * b05 - a33 * b01) * d,
                (a20 * b05 - a22 * b02 + a23 * b01) * d,
                (a10 * b10 - a11 * b08 + a13 * b06) * d,
                (a01 * b08 - a00 * b10 - a03 * b06) * d,
                (a30 * b04 - a31 * b02 + a33 * b00) * d,
                (a21 * b02 - a20 * b04 - a23 * b00) * d,
                (a11 * b07 - a10 * b09 - a12 * b06) * d,
                (a00 * b09 - a01 * b07 + a02 * b06) * d,
                (a31 * b01 - a30 * b03 - a32 * b00) * d,
                (a20 * b03 - a21 * b01 + a22 * b00) * d,
            ],
        }
    }

    /// The matrix that turns normals: the inverse transpose of the 3×3 part, as a
    /// column-major 3×3 (for a `mat3` uniform).
    pub fn normal_matrix(&self) -> [f32; 9] {
        let i = self.inverse().elements;
        // Transposée de l'inverse : la ligne de l'une est la colonne de l'autre.
        [i[0], i[4], i[8], i[1], i[5], i[9], i[2], i[6], i[10]]
    }

    /// The translation part.
    pub fn translation(&self) -> Vector3 {
        Vector3::new(self.elements[12], self.elements[13], self.elements[14])
    }
}

impl Mul for Matrix4 {
    type Output = Matrix4;
    /// `a * b` applies `b`, then `a`.
    fn mul(self, b: Matrix4) -> Matrix4 {
        let (a, b) = (&self.elements, &b.elements);
        let mut out = [0.0; 16];
        for col in 0..4 {
            for row in 0..4 {
                out[col * 4 + row] = (0..4).map(|k| a[k * 4 + row] * b[col * 4 + k]).sum();
            }
        }
        Matrix4 { elements: out }
    }
}

impl Mul<Vector3> for Matrix4 {
    type Output = Vector3;
    fn mul(self, v: Vector3) -> Vector3 {
        v.apply_matrix4(&self)
    }
}

/// A color, in linear RGB (the working space of lighting).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    /// Red, from 0 to 1 (more for HDR).
    pub r: f32,
    /// Green.
    pub g: f32,
    /// Blue.
    pub b: f32,
}

impl Default for Color {
    fn default() -> Color {
        Color::WHITE
    }
}

impl Color {
    /// White.
    pub const WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
    };
    /// Black.
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
    };

    /// A color from a hexadecimal sRGB value, like `0x44aa88`.
    pub fn hex(hex: u32) -> Color {
        Color::srgb_u8((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
    }

    /// A color from sRGB components from 0 to 1, as CSS and image editors give them.
    pub fn srgb(r: f32, g: f32, b: f32) -> Color {
        Color {
            r: srgb_to_linear(r),
            g: srgb_to_linear(g),
            b: srgb_to_linear(b),
        }
    }

    /// A color from sRGB bytes.
    pub fn srgb_u8(r: u8, g: u8, b: u8) -> Color {
        Color::srgb(
            f32::from(r) / 255.0,
            f32::from(g) / 255.0,
            f32::from(b) / 255.0,
        )
    }

    /// A color from linear components.
    pub const fn linear_rgb(r: f32, g: f32, b: f32) -> Color {
        Color { r, g, b }
    }

    /// The color multiplied by a factor (intensity).
    pub fn scale(self, s: f32) -> Color {
        Color {
            r: self.r * s,
            g: self.g * s,
            b: self.b * s,
        }
    }

    /// The sRGB hexadecimal value.
    pub fn to_hex(self) -> u32 {
        let c = |v: f32| (linear_to_srgb(v).clamp(0.0, 1.0) * 255.0).round() as u32;
        (c(self.r) << 16) | (c(self.g) << 8) | c(self.b)
    }

    /// The linear components.
    pub fn to_array(self) -> [f32; 3] {
        [self.r, self.g, self.b]
    }
}

impl From<u32> for Color {
    fn from(hex: u32) -> Color {
        Color::hex(hex)
    }
}

/// sRGB to linear, for one component.
pub fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Linear to sRGB, for one component.
pub fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Degrees to radians.
pub fn deg_to_rad(degrees: f32) -> f32 {
    degrees.to_radians()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn close3(a: Vector3, b: Vector3) -> bool {
        close(a.x, b.x) && close(a.y, b.y) && close(a.z, b.z)
    }

    #[test]
    fn vectors() {
        let a = Vector3::new(1.0, 2.0, 3.0);
        assert_eq!(a + Vector3::ONE, Vector3::new(2.0, 3.0, 4.0));
        assert_eq!(a.dot(Vector3::X), 1.0);
        assert_eq!(Vector3::X.cross(Vector3::Y), Vector3::Z);
        assert!(close(Vector3::new(3.0, 4.0, 0.0).length(), 5.0));
        assert!(close(a.normalize().length(), 1.0));
        assert_eq!(Vector3::ZERO.normalize(), Vector3::ZERO);
        assert_eq!(Vector3::ZERO.lerp(Vector3::splat(2.0), 0.5), Vector3::ONE);
    }

    #[test]
    fn quaternions_rotate_like_three() {
        let q = Quaternion::from_rotation_y(std::f32::consts::FRAC_PI_2);
        // Un quart de tour autour de y : x devient -z.
        assert!(close3(q * Vector3::X, Vector3::NEG_Z));
        let r = Quaternion::from_rotation_x(0.3) * Quaternion::from_rotation_z(-1.1);
        assert!(close3(
            r * (r.inverse() * Vector3::new(1.0, 2.0, 3.0)),
            Vector3::new(1.0, 2.0, 3.0)
        ));
        let e = Quaternion::from_euler(Euler::new(0.3, 0.0, -1.1));
        assert!(close(e.dot(r).abs(), 1.0));
        let half = Quaternion::IDENTITY.slerp(q, 0.5);
        assert!(close3(
            half * Vector3::X,
            Quaternion::from_rotation_y(std::f32::consts::FRAC_PI_4) * Vector3::X
        ));
    }

    #[test]
    fn matrices_compose_invert_and_decompose() {
        let p = Vector3::new(1.0, -2.0, 3.0);
        let q = Quaternion::from_euler(Euler::new(0.4, -0.7, 1.2));
        let s = Vector3::new(2.0, 0.5, 1.5);
        let m = Matrix4::compose(p, q, s);
        let v = Vector3::new(0.3, 0.2, -0.9);
        assert!(close3(m * v, p + q * v.mul_vec(s)));
        assert!(close3(m.inverse() * (m * v), v));
        let (p2, q2, s2) = m.decompose();
        assert!(close3(p2, p) && close3(s2, s) && close(q2.dot(q).abs(), 1.0));
        assert_eq!(Matrix4::IDENTITY * m, m);
        assert!(close(
            Matrix4::compose(Vector3::ZERO, q, Vector3::splat(2.0)).determinant(),
            8.0
        ));
    }

    #[test]
    fn looking_and_projecting() {
        // Regarder vers +x depuis l'origine : -z local devient +x.
        let q = Quaternion::looking_to(Vector3::X, Vector3::Y);
        assert!(close3(q * Vector3::NEG_Z, Vector3::X));
        assert!(close3(q * Vector3::Y, Vector3::Y));
        // Regarder droit vers le bas ne tombe pas sur un axe nul.
        let down = Quaternion::looking_to(Vector3::NEG_Y, Vector3::Y);
        assert!(close3(down * Vector3::NEG_Z, Vector3::NEG_Y));
        // Un point sur le plan proche se projette à la profondeur -1, sur le plan lointain à 1.
        let proj = Matrix4::perspective(1.0, 1.5, 0.1, 100.0);
        assert!(close((proj * Vector3::new(0.0, 0.0, -0.1)).z, -1.0));
        assert!(close((proj * Vector3::new(0.0, 0.0, -100.0)).z, 1.0));
        let ortho = Matrix4::orthographic(-2.0, 2.0, 1.0, -1.0, 0.0, 10.0);
        assert!(close3(
            ortho * Vector3::new(2.0, 1.0, -10.0),
            Vector3::new(1.0, 1.0, 1.0)
        ));
    }

    #[test]
    fn colors_are_linear() {
        let c = Color::hex(0x44aa88);
        assert_eq!(c.to_hex(), 0x44aa88);
        assert!(c.r < 0.0666 && c.r > 0.05);
        assert_eq!(Color::srgb(1.0, 1.0, 1.0), Color::WHITE);
        assert_eq!(Color::from(0x000000), Color::BLACK);
    }
}
