//! `OrbitControls`: turn around a point with the mouse or a finger, zoom with the wheel or
//! two fingers, slide with the right button.

use std::cell::RefCell;
use std::f32::consts::{PI, TAU};
use std::rc::Rc;

use super::camera::{OrthographicCamera, PerspectiveCamera};
use super::math::{Quaternion, Vector2, Vector3, deg_to_rad};
use super::{Camera, Matrix4};

/// A camera that [`OrbitControls`] can move.
pub trait OrbitCamera: Camera {
    /// Its position, to change.
    fn position_mut(&mut self) -> &mut Vector3;
    /// Its up direction.
    fn up(&self) -> Vector3;
    /// Turns it to look at a point.
    fn look_at_point(&mut self, target: Vector3);
    /// How many world units one pixel covers, at `distance` from the camera, in a view
    /// `height` pixels tall.
    fn units_per_pixel(&self, distance: f32, height: f32) -> f32;
    /// Zooms by making the view `scale` times bigger, when the camera zooms that way rather
    /// than by moving (an orthographic camera); `false` otherwise.
    fn zoom_view(&mut self, scale: f32) -> bool {
        let _ = scale;
        false
    }
}

impl OrbitCamera for PerspectiveCamera {
    fn position_mut(&mut self) -> &mut Vector3 {
        &mut self.position
    }
    fn up(&self) -> Vector3 {
        self.up
    }
    fn look_at_point(&mut self, target: Vector3) {
        self.look_at(target);
    }
    fn units_per_pixel(&self, distance: f32, height: f32) -> f32 {
        2.0 * distance * (deg_to_rad(self.fov) / 2.0).tan() / height.max(1.0)
    }
}

impl OrbitCamera for OrthographicCamera {
    fn position_mut(&mut self) -> &mut Vector3 {
        &mut self.position
    }
    fn up(&self) -> Vector3 {
        self.up
    }
    fn look_at_point(&mut self, target: Vector3) {
        self.look_at(target);
    }
    fn units_per_pixel(&self, _distance: f32, height: f32) -> f32 {
        (self.top - self.bottom).abs() / height.max(1.0)
    }
    fn zoom_view(&mut self, scale: f32) -> bool {
        if scale != 1.0 {
            self.left *= scale;
            self.right *= scale;
            self.top *= scale;
            self.bottom *= scale;
        }
        true
    }
}

/// What the pointer did since the last [`OrbitControls::update`], in pixels.
#[derive(Debug, Default)]
pub(crate) struct Input {
    pub(crate) rotate: Vector2,
    pub(crate) pan: Vector2,
    /// The wheel and the pinch, as a product of zoom factors (1: no zoom).
    pub(crate) zoom: f32,
    /// The height of the element, which the movements are relative to.
    pub(crate) height: f32,
}

/// Turns the camera around [`target`](Self::target): drag to orbit, wheel or pinch to
/// zoom, right button (or shift + drag, or two fingers) to pan. Call
/// [`update`](Self::update) each frame, before drawing.
///
/// ```
/// use active::scene::{OrbitControls, PerspectiveCamera, Vector3};
///
/// let mut camera = PerspectiveCamera::new(50.0, 0.0, 0.1, 100.0);
/// camera.position = Vector3::new(0.0, 0.0, 5.0);
/// let mut controls = OrbitControls::default(); // OrbitControls::new(&canvas) in a page
/// controls.enable_damping = true;
/// controls.rotate_left(0.5); // what a drag does
/// controls.update(&mut camera);
/// assert!(camera.position.x < 0.0);
/// ```
pub struct OrbitControls {
    /// The point the camera turns around and looks at.
    pub target: Vector3,
    /// Movements slow down smoothly instead of stopping at once (call `update` every
    /// frame).
    pub enable_damping: bool,
    /// How much of the movement is kept each frame, with damping (0.05).
    pub damping_factor: f32,
    /// Dragging turns the camera.
    pub enable_rotate: bool,
    /// How fast it turns (1).
    pub rotate_speed: f32,
    /// The wheel and pinching zoom.
    pub enable_zoom: bool,
    /// How fast it zooms (1).
    pub zoom_speed: f32,
    /// The right button slides the view.
    pub enable_pan: bool,
    /// How fast it slides (1).
    pub pan_speed: f32,
    /// The nearest the camera gets to the target.
    pub min_distance: f32,
    /// The farthest.
    pub max_distance: f32,
    /// The lowest vertical angle, in radians from straight up (0).
    pub min_polar_angle: f32,
    /// The highest (π: straight down).
    pub max_polar_angle: f32,
    /// Turns on its own around the target.
    pub auto_rotate: bool,
    /// How fast: 2 is one turn in 30 seconds at 60 frames a second.
    pub auto_rotate_speed: f32,
    delta_theta: f32,
    delta_phi: f32,
    scale: f32,
    pan_offset: Vector3,
    pub(crate) input: Rc<RefCell<Input>>,
    #[cfg(feature = "client")]
    _listeners: Option<super::renderer::Listeners>,
}

impl Default for OrbitControls {
    /// Controls listening to nothing: move them with [`rotate_left`](Self::rotate_left)
    /// and the other methods.
    fn default() -> OrbitControls {
        OrbitControls {
            target: Vector3::ZERO,
            enable_damping: false,
            damping_factor: 0.05,
            enable_rotate: true,
            rotate_speed: 1.0,
            enable_zoom: true,
            zoom_speed: 1.0,
            enable_pan: true,
            pan_speed: 1.0,
            min_distance: 0.0,
            max_distance: f32::INFINITY,
            min_polar_angle: 0.0,
            max_polar_angle: PI,
            auto_rotate: false,
            auto_rotate_speed: 2.0,
            delta_theta: 0.0,
            delta_phi: 0.0,
            scale: 1.0,
            pan_offset: Vector3::ZERO,
            input: Rc::new(RefCell::new(Input {
                zoom: 1.0,
                ..Input::default()
            })),
            #[cfg(feature = "client")]
            _listeners: None,
        }
    }
}

impl OrbitControls {
    /// Controls driven by the pointer on `element`, usually the canvas (`new
    /// OrbitControls(camera, renderer.domElement)`). They stop listening when dropped. On
    /// the server, they listen to nothing.
    pub fn new(element: &crate::event::Dom) -> OrbitControls {
        let controls = OrbitControls::default();
        #[cfg(feature = "client")]
        let controls = OrbitControls {
            _listeners: Some(super::renderer::Listeners::orbit(
                element,
                controls.input.clone(),
            )),
            ..controls
        };
        #[cfg(not(feature = "client"))]
        let _ = element;
        controls
    }

    /// Turns the camera around the target, to the left (radians).
    pub fn rotate_left(&mut self, angle: f32) {
        self.delta_theta -= angle;
    }

    /// Turns the camera over the target, upwards (radians).
    pub fn rotate_up(&mut self, angle: f32) {
        self.delta_phi -= angle;
    }

    /// Brings the camera `scale` times closer (more than 1 zooms in).
    pub fn dolly_in(&mut self, scale: f32) {
        if scale > 0.0 {
            self.scale /= scale;
        }
    }

    /// Takes the camera `scale` times farther.
    pub fn dolly_out(&mut self, scale: f32) {
        self.scale *= scale;
    }

    /// Slides the target and the camera by `offset`, in world units.
    pub fn pan_by(&mut self, offset: Vector3) {
        self.pan_offset += offset;
    }

    /// Applies what happened since the last call and moves the camera; returns whether it
    /// moved.
    pub fn update(&mut self, camera: &mut impl OrbitCamera) -> bool {
        let before = *camera.position_mut();
        self.take_input(camera);
        // Spherical coordinates around the target, with y up.
        let to_y_up = Quaternion::from_unit_vectors(camera.up().normalize(), Vector3::Y);
        let offset = (before - self.target).apply_quaternion(to_y_up);
        let mut radius = offset.length();
        let mut theta = offset.x.atan2(offset.z);
        let mut phi = if radius > 0.0 {
            (offset.y / radius).clamp(-1.0, 1.0).acos()
        } else {
            PI / 2.0
        };
        if self.auto_rotate {
            self.rotate_left(TAU / 60.0 / 60.0 * self.auto_rotate_speed);
        }
        let keep = if self.enable_damping {
            self.damping_factor
        } else {
            1.0
        };
        theta += self.delta_theta * keep;
        phi += self.delta_phi * keep;
        // Never exactly at a pole, where "up" has no meaning.
        const EPS: f32 = 1e-6;
        phi = phi
            .clamp(self.min_polar_angle, self.max_polar_angle)
            .clamp(EPS, PI - EPS);
        if !camera.zoom_view(self.scale) {
            radius = (radius * self.scale).clamp(self.min_distance, self.max_distance);
        }
        self.target += self.pan_offset * keep;
        let offset = Vector3::new(
            radius * phi.sin() * theta.sin(),
            radius * phi.cos(),
            radius * phi.sin() * theta.cos(),
        );
        *camera.position_mut() = self.target + offset.apply_quaternion(to_y_up.inverse());
        camera.look_at_point(self.target);
        if self.enable_damping {
            self.delta_theta *= 1.0 - self.damping_factor;
            self.delta_phi *= 1.0 - self.damping_factor;
            self.pan_offset *= 1.0 - self.damping_factor;
        } else {
            self.delta_theta = 0.0;
            self.delta_phi = 0.0;
            self.pan_offset = Vector3::ZERO;
        }
        self.scale = 1.0;
        (*camera.position_mut() - before).length_squared() > EPS
    }

    /// Turns what the pointer did into movements.
    fn take_input(&mut self, camera: &mut impl OrbitCamera) {
        let input = std::mem::replace(
            &mut *self.input.borrow_mut(),
            Input {
                zoom: 1.0,
                ..Input::default()
            },
        );
        let height = input.height.max(1.0);
        // Keep the element's height for the next frames.
        self.input.borrow_mut().height = input.height;
        if self.enable_rotate {
            self.rotate_left(TAU * input.rotate.x / height * self.rotate_speed);
            self.rotate_up(TAU * input.rotate.y / height * self.rotate_speed);
        }
        if self.enable_zoom && input.zoom != 1.0 {
            self.dolly_out(input.zoom.powf(self.zoom_speed));
        }
        if self.enable_pan && input.pan != Vector2::ZERO {
            let world = camera.world_matrix();
            let distance = (*camera.position_mut() - self.target).length();
            let units = camera.units_per_pixel(distance, height) * self.pan_speed;
            let (right, up) = (column(&world, 0), column(&world, 1));
            self.pan_by(right * (-input.pan.x * units) + up * (input.pan.y * units));
        }
    }
}

/// A column of a matrix, as a direction.
fn column(m: &Matrix4, i: usize) -> Vector3 {
    Vector3::new(
        m.elements[i * 4],
        m.elements[i * 4 + 1],
        m.elements[i * 4 + 2],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera() -> PerspectiveCamera {
        let mut camera = PerspectiveCamera::new(50.0, 1.0, 0.1, 100.0);
        camera.position = Vector3::new(0.0, 0.0, 5.0);
        camera
    }

    #[test]
    fn orbiting_keeps_the_distance_and_looks_at_the_target() {
        let mut camera = camera();
        let mut controls = OrbitControls::default();
        controls.rotate_left(PI / 2.0);
        assert!(controls.update(&mut camera));
        // A quarter turn to the left: from +z to -x.
        assert!(
            (camera.position - Vector3::new(-5.0, 0.0, 0.0)).length() < 1e-4,
            "{:?}",
            camera.position
        );
        let forward = Quaternion::from_euler(camera.rotation) * Vector3::NEG_Z;
        assert!((forward - Vector3::X).length() < 1e-4, "{forward:?}");
        // Nothing more to do.
        assert!(!controls.update(&mut camera));
    }

    #[test]
    fn the_poles_and_the_distances_are_limits() {
        let mut camera = camera();
        let mut controls = OrbitControls {
            min_distance: 2.0,
            max_distance: 8.0,
            ..OrbitControls::default()
        };
        controls.rotate_up(10.0);
        controls.update(&mut camera);
        assert!(
            (camera.position.y - 5.0).abs() < 0.01,
            "{:?}",
            camera.position
        );
        controls.dolly_out(10.0);
        controls.update(&mut camera);
        assert!((camera.position.length() - 8.0).abs() < 1e-3);
        controls.dolly_in(100.0);
        controls.update(&mut camera);
        assert!((camera.position.length() - 2.0).abs() < 1e-3);
    }

    #[test]
    fn damping_spreads_a_movement_over_frames() {
        let mut camera = camera();
        let mut controls = OrbitControls {
            enable_damping: true,
            ..OrbitControls::default()
        };
        controls.rotate_left(1.0);
        controls.update(&mut camera);
        let first = camera.position.x;
        assert!(controls.update(&mut camera), "it keeps moving");
        assert!(camera.position.x < first);
    }

    #[test]
    fn the_pointer_input_rotates_pans_and_zooms() {
        let mut camera = camera();
        let mut controls = OrbitControls::default();
        {
            let mut input = controls.input.borrow_mut();
            input.height = 500.0;
            input.pan = Vector2::new(100.0, 0.0);
            input.zoom = 0.5;
        }
        controls.update(&mut camera);
        // Dragging right slides the scene right: the target goes left.
        assert!(controls.target.x < 0.0, "{:?}", controls.target);
        assert!((camera.position - controls.target).length() < 2.6);
    }

    #[test]
    fn an_orthographic_camera_zooms_its_box() {
        let mut camera = OrthographicCamera::new(-2.0, 2.0, 1.0, -1.0, 0.1, 100.0);
        camera.position = Vector3::new(0.0, 0.0, 5.0);
        let mut controls = OrbitControls::default();
        controls.dolly_in(2.0);
        controls.update(&mut camera);
        assert_eq!((camera.left, camera.top), (-1.0, 0.5));
        assert!((camera.position.z - 5.0).abs() < 1e-4);
    }
}
