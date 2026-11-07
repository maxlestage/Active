//! The browser side: [`Renderer`] draws a [`Frame`] with WebGL 2, [`animation_loop`]
//! calls a closure before each repaint, and the pointer listeners of
//! [`OrbitControls`](super::OrbitControls).
//!
//! One shader program draws every material: the kind is a uniform, the lights are arrays
//! (4 directional and 8 point lights at most, the others are ignored), and the lighting
//! follows a physically based model (Lambert diffuse, GGX specular, the same light
//! falloff), then the tone mapping and the sRGB output.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::JsValue;
use web_sys::{
    HtmlCanvasElement, WebGl2RenderingContext as Gl, WebGlBuffer, WebGlProgram, WebGlShader,
    WebGlUniformLocation, WebGlVertexArrayObject,
};

use super::controls::Input;
use super::math::{Color, Vector2, Vector3, linear_to_srgb};
use super::{BufferGeometry, Camera, Draw, Frame, Material, Scene, Side, ToneMapping};

const MAX_DIRECTIONAL: usize = 4;
const MAX_POINT: usize = 8;
/// A geometry not drawn for this many frames gives its GPU buffers back.
const KEEP_FRAMES: u64 = 120;

const VERTEX: &str = r#"#version 300 es
layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
uniform mat4 modelMatrix;
uniform mat4 viewMatrix;
uniform mat4 projectionMatrix;
uniform mat3 normalMatrix;
out vec3 vWorldPosition;
out vec3 vNormal;
void main() {
    vec4 world = modelMatrix * vec4(position, 1.0);
    vWorldPosition = world.xyz;
    vNormal = normalMatrix * normal;
    gl_Position = projectionMatrix * viewMatrix * world;
}
"#;

const FRAGMENT: &str = r#"#version 300 es
precision highp float;
#define RECIPROCAL_PI 0.3183098861837907
in vec3 vWorldPosition;
in vec3 vNormal;
uniform int kind; // 0 basic, 1 Lambert, 2 standard, 3 normal
uniform vec3 diffuse;
uniform float opacity;
uniform vec3 emissive;
uniform float roughness;
uniform float metalness;
uniform bool flatShading;
uniform mat4 viewMatrix;
uniform vec3 cameraPosition;
uniform bool orthographic;
uniform vec3 cameraBack;
uniform vec3 ambientLightColor;
uniform vec3 skyColor;
uniform vec3 groundColor;
uniform int numDirectional;
uniform vec3 directionalDirection[4];
uniform vec3 directionalColor[4];
uniform int numPoint;
uniform vec3 pointPosition[8];
uniform vec3 pointColor[8];
uniform vec2 pointFalloff[8]; // distance, decay
uniform float exposure;
uniform int toneMapping;
out vec4 fragColor;

float pow2(float x) { return x * x; }

vec3 fresnelSchlick(vec3 f0, float dotVH) {
    float fresnel = exp2((-5.55473 * dotVH - 6.98316) * dotVH);
    return f0 * (1.0 - fresnel) + vec3(fresnel);
}

float visibilityGgx(float alpha, float dotNL, float dotNV) {
    float a2 = pow2(alpha);
    float gv = dotNL * sqrt(a2 + (1.0 - a2) * pow2(dotNV));
    float gl = dotNV * sqrt(a2 + (1.0 - a2) * pow2(dotNL));
    return 0.5 / max(gv + gl, 1e-6);
}

float distributionGgx(float alpha, float dotNH) {
    float a2 = pow2(alpha);
    float denom = pow2(dotNH) * (a2 - 1.0) + 1.0;
    return RECIPROCAL_PI * a2 / pow2(denom);
}

vec3 specularGgx(vec3 l, vec3 v, vec3 n, vec3 f0, float rough) {
    float alpha = pow2(rough);
    vec3 h = normalize(l + v);
    float dotNL = clamp(dot(n, l), 0.0, 1.0);
    float dotNV = clamp(dot(n, v), 0.0, 1.0);
    float dotNH = clamp(dot(n, h), 0.0, 1.0);
    float dotVH = clamp(dot(v, h), 0.0, 1.0);
    return fresnelSchlick(f0, dotVH) * (visibilityGgx(alpha, dotNL, dotNV) * distributionGgx(alpha, dotNH));
}

vec3 acesFilmic(vec3 color) {
    const mat3 inputMat = mat3(
        vec3(0.59719, 0.07600, 0.02840), vec3(0.35458, 0.90834, 0.13383), vec3(0.04823, 0.01566, 0.83777));
    const mat3 outputMat = mat3(
        vec3(1.60475, -0.10208, -0.00327), vec3(-0.53108, 1.10813, -0.07276), vec3(-0.07367, -0.00605, 1.07602));
    color = inputMat * (color / 0.6);
    vec3 a = color * (color + 0.0245786) - 0.000090537;
    vec3 b = color * (0.983729 * color + 0.4329510) + 0.238081;
    return clamp(outputMat * (a / b), 0.0, 1.0);
}

vec3 linearToSrgb(vec3 c) {
    return mix(pow(c, vec3(0.41666)) * 1.055 - vec3(0.055), c * 12.92, vec3(lessThanEqual(c, vec3(0.0031308))));
}

void main() {
    vec3 n;
    if (flatShading) {
        n = normalize(cross(dFdx(vWorldPosition), dFdy(vWorldPosition)));
    } else {
        n = normalize(vNormal);
        if (!gl_FrontFacing) n = -n;
    }
    vec3 color;
    if (kind == 3) {
        color = normalize(mat3(viewMatrix) * n) * 0.5 + 0.5;
    } else if (kind == 0) {
        color = diffuse;
    } else {
        vec3 v = orthographic ? cameraBack : normalize(cameraPosition - vWorldPosition);
        vec3 diffuseColor = kind == 2 ? diffuse * (1.0 - metalness) : diffuse;
        vec3 f0 = mix(vec3(0.04), diffuse, metalness);
        vec3 dxy = max(abs(dFdx(n)), abs(dFdy(n)));
        float rough = min(max(roughness, 0.0525) + max(max(dxy.x, dxy.y), dxy.z), 1.0);
        vec3 irradiance = ambientLightColor + mix(groundColor, skyColor, 0.5 * n.y + 0.5);
        vec3 specular = vec3(0.0);
        for (int i = 0; i < 4; i++) {
            if (i >= numDirectional) break;
            vec3 l = directionalDirection[i];
            vec3 light = clamp(dot(n, l), 0.0, 1.0) * directionalColor[i];
            irradiance += light;
            if (kind == 2) specular += light * specularGgx(l, v, n, f0, rough);
        }
        for (int i = 0; i < 8; i++) {
            if (i >= numPoint) break;
            vec3 toLight = pointPosition[i] - vWorldPosition;
            float d = length(toLight);
            vec3 l = toLight / max(d, 1e-6);
            float falloff = 1.0 / max(pow(d, pointFalloff[i].y), 0.01);
            if (pointFalloff[i].x > 0.0) falloff *= pow2(clamp(1.0 - pow2(pow2(d / pointFalloff[i].x)), 0.0, 1.0));
            vec3 light = clamp(dot(n, l), 0.0, 1.0) * pointColor[i] * falloff;
            irradiance += light;
            if (kind == 2) specular += light * specularGgx(l, v, n, f0, rough);
        }
        color = (irradiance * RECIPROCAL_PI * diffuseColor + specular + emissive) * exposure;
    }
    if (toneMapping == 1) color = acesFilmic(color);
    color = linearToSrgb(clamp(color, 0.0, 1.0));
    // The canvas expects premultiplied alpha.
    fragColor = vec4(color * opacity, opacity);
}
"#;

/// Where the uniforms are.
struct Uniforms {
    model: Option<WebGlUniformLocation>,
    view: Option<WebGlUniformLocation>,
    projection: Option<WebGlUniformLocation>,
    normal_matrix: Option<WebGlUniformLocation>,
    kind: Option<WebGlUniformLocation>,
    diffuse: Option<WebGlUniformLocation>,
    opacity: Option<WebGlUniformLocation>,
    emissive: Option<WebGlUniformLocation>,
    roughness: Option<WebGlUniformLocation>,
    metalness: Option<WebGlUniformLocation>,
    flat_shading: Option<WebGlUniformLocation>,
    camera_position: Option<WebGlUniformLocation>,
    orthographic: Option<WebGlUniformLocation>,
    camera_back: Option<WebGlUniformLocation>,
    ambient: Option<WebGlUniformLocation>,
    sky: Option<WebGlUniformLocation>,
    ground: Option<WebGlUniformLocation>,
    num_directional: Option<WebGlUniformLocation>,
    directional_direction: Option<WebGlUniformLocation>,
    directional_color: Option<WebGlUniformLocation>,
    num_point: Option<WebGlUniformLocation>,
    point_position: Option<WebGlUniformLocation>,
    point_color: Option<WebGlUniformLocation>,
    point_falloff: Option<WebGlUniformLocation>,
    exposure: Option<WebGlUniformLocation>,
    tone_mapping: Option<WebGlUniformLocation>,
}

impl Uniforms {
    fn new(gl: &Gl, p: &WebGlProgram) -> Uniforms {
        let at = |name: &str| gl.get_uniform_location(p, name);
        Uniforms {
            model: at("modelMatrix"),
            view: at("viewMatrix"),
            projection: at("projectionMatrix"),
            normal_matrix: at("normalMatrix"),
            kind: at("kind"),
            diffuse: at("diffuse"),
            opacity: at("opacity"),
            emissive: at("emissive"),
            roughness: at("roughness"),
            metalness: at("metalness"),
            flat_shading: at("flatShading"),
            camera_position: at("cameraPosition"),
            orthographic: at("orthographic"),
            camera_back: at("cameraBack"),
            ambient: at("ambientLightColor"),
            sky: at("skyColor"),
            ground: at("groundColor"),
            num_directional: at("numDirectional"),
            directional_direction: at("directionalDirection[0]"),
            directional_color: at("directionalColor[0]"),
            num_point: at("numPoint"),
            point_position: at("pointPosition[0]"),
            point_color: at("pointColor[0]"),
            point_falloff: at("pointFalloff[0]"),
            exposure: at("exposure"),
            tone_mapping: at("toneMapping"),
        }
    }
}

/// A geometry's buffers on the GPU.
struct GpuGeometry {
    vao: Option<WebGlVertexArrayObject>,
    buffers: Vec<Option<WebGlBuffer>>,
    count: i32,
    /// The edges, for wireframes: made the first time one is drawn.
    wire: Option<(Option<WebGlVertexArrayObject>, Option<WebGlBuffer>, i32)>,
    /// The last frame that drew it.
    used: u64,
}

/// Draws scenes in a `<canvas>` with WebGL 2 (`new THREE.Renderer({ canvas })`).
///
/// Without [`set_size`](Self::set_size), the drawing follows the size the page gives the
/// canvas (its CSS size × the pixel ratio), checked at each render. Without a background
/// the canvas is transparent, and the page shows through.
///
/// ```ignore
/// let mut renderer = Renderer::new(&canvas)?;
/// renderer.set_pixel_ratio(web_sys::window().unwrap().device_pixel_ratio());
/// let looping = animation_loop(move |_time| {
///     scene[cube].rotation.y += 0.01;
///     renderer.render(&scene, &camera);
/// });
/// ```
pub struct Renderer {
    canvas: HtmlCanvasElement,
    gl: Gl,
    program: Option<WebGlProgram>,
    u: Uniforms,
    geometries: HashMap<u64, GpuGeometry>,
    frame_number: u64,
    pixel_ratio: f64,
    /// The CSS size given to `set_size`.
    size: Option<(f64, f64)>,
    clear_color: Color,
    clear_alpha: f32,
    /// How colors brighter than white are brought into range.
    pub tone_mapping: ToneMapping,
    /// The exposure, used with a tone mapping (`renderer.toneMappingExposure`).
    pub tone_mapping_exposure: f32,
}

impl Renderer {
    /// A renderer drawing in `canvas` (the element given to `on_mount` or `use_action`).
    /// Fails when it is not a canvas, or when the browser has no WebGL 2.
    pub fn new(canvas: &crate::event::Dom) -> Result<Renderer, String> {
        let canvas: HtmlCanvasElement = canvas
            .clone()
            .dyn_into()
            .map_err(|_| "Renderer::new needs a <canvas> element".to_owned())?;
        let options = js_sys::Object::new();
        for (key, value) in [
            ("antialias", true),
            ("alpha", true),
            ("premultipliedAlpha", true),
        ] {
            let _ = js_sys::Reflect::set(
                &options,
                &JsValue::from_str(key),
                &JsValue::from_bool(value),
            );
        }
        let gl: Gl = match canvas.get_context_with_context_options("webgl2", &options) {
            Ok(Some(context)) => context
                .dyn_into()
                .map_err(|_| "not a WebGL 2 context".to_owned())?,
            _ => return Err("WebGL 2 is not available in this browser".to_owned()),
        };
        let program = link(&gl)?;
        let u = Uniforms::new(&gl, &program);
        Ok(Renderer {
            canvas,
            gl,
            program: Some(program),
            u,
            geometries: HashMap::new(),
            frame_number: 0,
            pixel_ratio: 1.0,
            size: None,
            clear_color: Color::BLACK,
            clear_alpha: 0.0,
            tone_mapping: ToneMapping::None,
            tone_mapping_exposure: 1.0,
        })
    }

    /// The canvas (`renderer.domElement`).
    pub fn dom_element(&self) -> &HtmlCanvasElement {
        &self.canvas
    }

    /// The WebGL 2 context, for anything this renderer does not do.
    pub fn context(&self) -> &Gl {
        &self.gl
    }

    /// Device pixels per CSS pixel: pass `window.devicePixelRatio` for sharp images on
    /// high-density screens (1 by default).
    pub fn set_pixel_ratio(&mut self, ratio: f64) {
        self.pixel_ratio = if ratio > 0.0 { ratio } else { 1.0 };
        self.fit();
    }

    /// The pixel ratio.
    pub fn pixel_ratio(&self) -> f64 {
        self.pixel_ratio
    }

    /// Gives the canvas this CSS size, in pixels, and draws at that size × the pixel ratio.
    pub fn set_size(&mut self, width: f64, height: f64) {
        self.size = Some((width.max(1.0), height.max(1.0)));
        let style = self.canvas.style();
        let _ = style.set_property("width", &format!("{width}px"));
        let _ = style.set_property("height", &format!("{height}px"));
        self.fit();
    }

    /// The size of the drawing, in device pixels.
    pub fn drawing_buffer_size(&self) -> (u32, u32) {
        (self.canvas.width(), self.canvas.height())
    }

    /// The color the canvas is cleared with when the scene has no background, and its
    /// opacity (0 by default: transparent).
    pub fn set_clear_color(&mut self, color: impl Into<Color>, alpha: f32) {
        self.clear_color = color.into();
        self.clear_alpha = alpha.clamp(0.0, 1.0);
    }

    /// Draws `scene` seen by `camera` (`renderer.render(scene, camera)`).
    pub fn render(&mut self, scene: &Scene, camera: &impl Camera) {
        self.fit();
        let (width, height) = self.drawing_buffer_size();
        let mut frame = scene.frame(camera, width.max(1) as f32 / height.max(1) as f32);
        frame.tone_mapping = self.tone_mapping;
        if self.tone_mapping != ToneMapping::None {
            frame.exposure *= self.tone_mapping_exposure;
        }
        self.draw_frame(&frame);
    }

    /// The aspect ratio (width / height) of the drawing, after following the canvas's size.
    pub fn aspect(&mut self) -> f32 {
        self.fit();
        let (width, height) = self.drawing_buffer_size();
        width.max(1) as f32 / height.max(1) as f32
    }

    /// Draws a frame made by [`Scene::frame`] (or by anything else: the ECS draws its own).
    pub fn draw_frame(&mut self, frame: &Frame<'_>) {
        self.frame_number += 1;
        let (width, height) = self.drawing_buffer_size();
        let gl = &self.gl;
        gl.viewport(0, 0, width as i32, height as i32);
        let (clear, alpha) = match frame.background {
            Some(color) => (color, 1.0),
            None => (self.clear_color, self.clear_alpha),
        };
        let [r, g, b] = srgb(clear);
        gl.clear_color(r * alpha, g * alpha, b * alpha, alpha);
        gl.depth_mask(true);
        gl.clear(Gl::COLOR_BUFFER_BIT | Gl::DEPTH_BUFFER_BIT);
        gl.enable(Gl::DEPTH_TEST);
        gl.depth_func(Gl::LEQUAL);
        gl.use_program(self.program.as_ref());
        self.set_frame_uniforms(frame);

        // Opaque meshes first, nearest first; then the transparent ones, farthest first.
        let mut order: Vec<(bool, f32, usize)> = frame
            .meshes
            .iter()
            .enumerate()
            .map(|(i, draw)| {
                (
                    draw.material.surface().transparent,
                    (frame.view * draw.world.translation()).z,
                    i,
                )
            })
            .collect();
        order.sort_by(|a, b| match (a.0, b.0) {
            (false, false) => b.1.total_cmp(&a.1),
            (true, true) => a.1.total_cmp(&b.1),
            (x, y) => x.cmp(&y),
        });
        for (_, _, i) in order {
            self.draw_mesh(&frame.meshes[i]);
        }
        let gl = &self.gl;
        gl.bind_vertex_array(None);
        gl.disable(Gl::BLEND);
        gl.depth_mask(true);

        // Give back the buffers of the geometries no longer drawn.
        let now = self.frame_number;
        let stale: Vec<u64> = self
            .geometries
            .iter()
            .filter(|(_, g)| now - g.used > KEEP_FRAMES)
            .map(|(id, _)| *id)
            .collect();
        for id in stale {
            if let Some(g) = self.geometries.remove(&id) {
                delete(&self.gl, g);
            }
        }
    }

    fn set_frame_uniforms(&self, frame: &Frame<'_>) {
        let (gl, u) = (&self.gl, &self.u);
        gl.uniform_matrix4fv_with_f32_array(u.view.as_ref(), false, &frame.view.elements);
        gl.uniform_matrix4fv_with_f32_array(
            u.projection.as_ref(),
            false,
            &frame.projection.elements,
        );
        let eye = frame.camera.translation();
        gl.uniform3f(u.camera_position.as_ref(), eye.x, eye.y, eye.z);
        let e = &frame.camera.elements;
        let back = Vector3::new(e[8], e[9], e[10]).normalize();
        gl.uniform3f(u.camera_back.as_ref(), back.x, back.y, back.z);
        // A perspective projection has 0 in its last element, an orthographic one 1.
        gl.uniform1i(
            u.orthographic.as_ref(),
            i32::from(frame.projection.elements[15] == 1.0),
        );

        let lights = &frame.lights;
        set_color(gl, &u.ambient, lights.ambient);
        let (sky, ground) = lights.hemisphere.unwrap_or((Color::BLACK, Color::BLACK));
        set_color(gl, &u.sky, sky);
        set_color(gl, &u.ground, ground);
        let directional = &lights.directional[..lights.directional.len().min(MAX_DIRECTIONAL)];
        gl.uniform1i(u.num_directional.as_ref(), directional.len() as i32);
        let mut directions = [0.0; MAX_DIRECTIONAL * 3];
        let mut colors = [0.0; MAX_DIRECTIONAL * 3];
        for (i, (direction, color)) in directional.iter().enumerate() {
            directions[i * 3..i * 3 + 3].copy_from_slice(&direction.to_array());
            colors[i * 3..i * 3 + 3].copy_from_slice(&color.to_array());
        }
        gl.uniform3fv_with_f32_array(u.directional_direction.as_ref(), &directions);
        gl.uniform3fv_with_f32_array(u.directional_color.as_ref(), &colors);
        let point = &lights.point[..lights.point.len().min(MAX_POINT)];
        gl.uniform1i(u.num_point.as_ref(), point.len() as i32);
        let mut positions = [0.0; MAX_POINT * 3];
        let mut colors = [0.0; MAX_POINT * 3];
        let mut falloffs = [0.0; MAX_POINT * 2];
        for (i, light) in point.iter().enumerate() {
            positions[i * 3..i * 3 + 3].copy_from_slice(&light.position.to_array());
            colors[i * 3..i * 3 + 3].copy_from_slice(&light.color.to_array());
            falloffs[i * 2] = light.distance;
            falloffs[i * 2 + 1] = light.decay;
        }
        gl.uniform3fv_with_f32_array(u.point_position.as_ref(), &positions);
        gl.uniform3fv_with_f32_array(u.point_color.as_ref(), &colors);
        gl.uniform2fv_with_f32_array(u.point_falloff.as_ref(), &falloffs);
        gl.uniform1f(u.exposure.as_ref(), frame.exposure);
        gl.uniform1i(
            u.tone_mapping.as_ref(),
            i32::from(frame.tone_mapping == ToneMapping::AcesFilmic),
        );
    }

    fn draw_mesh(&mut self, draw: &Draw<'_>) {
        let id = draw.geometry.id();
        if !self.geometries.contains_key(&id) {
            let Some(uploaded) = upload(&self.gl, draw.geometry) else {
                return;
            };
            self.geometries.insert(id, uploaded);
        }
        let (gl, u) = (&self.gl, &self.u);
        let Some(g) = self.geometries.get_mut(&id) else {
            return;
        };
        g.used = self.frame_number;

        let surface = draw.material.surface();
        let (kind, emissive, roughness, metalness, flat) = match draw.material {
            Material::Basic(_) => (0, Color::BLACK, 1.0, 0.0, false),
            Material::Lambert(m) => (1, m.emissive, 1.0, 0.0, false),
            Material::Standard(m) => (2, m.emissive, m.roughness, m.metalness, m.flat_shading),
            Material::Normal(_) => (3, Color::BLACK, 1.0, 0.0, false),
        };
        gl.uniform1i(u.kind.as_ref(), kind);
        set_color(gl, &u.diffuse, surface.color);
        gl.uniform1f(
            u.opacity.as_ref(),
            if surface.transparent {
                surface.opacity.clamp(0.0, 1.0)
            } else {
                1.0
            },
        );
        set_color(gl, &u.emissive, emissive);
        gl.uniform1f(u.roughness.as_ref(), roughness);
        gl.uniform1f(u.metalness.as_ref(), metalness);
        gl.uniform1i(u.flat_shading.as_ref(), i32::from(flat));
        gl.uniform_matrix4fv_with_f32_array(u.model.as_ref(), false, &draw.world.elements);
        gl.uniform_matrix3fv_with_f32_array(
            u.normal_matrix.as_ref(),
            false,
            &draw.world.normal_matrix(),
        );

        match surface.side {
            Side::Front => {
                gl.enable(Gl::CULL_FACE);
                gl.cull_face(Gl::BACK);
            }
            Side::Back => {
                gl.enable(Gl::CULL_FACE);
                gl.cull_face(Gl::FRONT);
            }
            Side::Double => gl.disable(Gl::CULL_FACE),
        }
        // A mirrored object (a negative scale) turns its triangles clockwise.
        gl.front_face(if draw.world.determinant() < 0.0 {
            Gl::CW
        } else {
            Gl::CCW
        });
        if surface.transparent {
            gl.enable(Gl::BLEND);
            gl.blend_func(Gl::ONE, Gl::ONE_MINUS_SRC_ALPHA);
            gl.depth_mask(false);
        } else {
            gl.disable(Gl::BLEND);
            gl.depth_mask(true);
        }

        if surface.wireframe {
            if g.wire.is_none() {
                g.wire = Some(upload_edges(gl, g, draw.geometry));
            }
            if let Some((vao, _, count)) = &g.wire {
                gl.bind_vertex_array(vao.as_ref());
                gl.draw_elements_with_i32(Gl::LINES, *count, Gl::UNSIGNED_INT, 0);
            }
        } else {
            gl.bind_vertex_array(g.vao.as_ref());
            gl.draw_elements_with_i32(Gl::TRIANGLES, g.count, Gl::UNSIGNED_INT, 0);
        }
    }

    /// Follows the canvas's size (or the one given to `set_size`) × the pixel ratio.
    fn fit(&mut self) {
        let (width, height) = match self.size {
            Some(size) => size,
            None => (
                f64::from(self.canvas.client_width()),
                f64::from(self.canvas.client_height()),
            ),
        };
        if width <= 0.0 || height <= 0.0 {
            return; // not laid out yet: keep the size it has
        }
        let (w, h) = (
            (width * self.pixel_ratio).round() as u32,
            (height * self.pixel_ratio).round() as u32,
        );
        if self.canvas.width() != w {
            self.canvas.set_width(w);
        }
        if self.canvas.height() != h {
            self.canvas.set_height(h);
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        for (_, g) in self.geometries.drain() {
            delete(&self.gl, g);
        }
        self.gl.delete_program(self.program.take().as_ref());
    }
}

fn srgb(c: Color) -> [f32; 3] {
    [
        linear_to_srgb(c.r.clamp(0.0, 1.0)),
        linear_to_srgb(c.g.clamp(0.0, 1.0)),
        linear_to_srgb(c.b.clamp(0.0, 1.0)),
    ]
}

fn set_color(gl: &Gl, at: &Option<WebGlUniformLocation>, c: Color) {
    gl.uniform3f(at.as_ref(), c.r, c.g, c.b);
}

fn compile(gl: &Gl, kind: u32, source: &str) -> Result<WebGlShader, String> {
    let shader = gl.create_shader(kind).ok_or("cannot create a shader")?;
    gl.shader_source(&shader, source);
    gl.compile_shader(&shader);
    if gl
        .get_shader_parameter(&shader, Gl::COMPILE_STATUS)
        .as_bool()
        == Some(true)
    {
        Ok(shader)
    } else {
        let log = gl.get_shader_info_log(&shader).unwrap_or_default();
        gl.delete_shader(Some(&shader));
        Err(format!("shader compilation failed: {log}"))
    }
}

fn link(gl: &Gl) -> Result<WebGlProgram, String> {
    let vertex = compile(gl, Gl::VERTEX_SHADER, VERTEX)?;
    let fragment = compile(gl, Gl::FRAGMENT_SHADER, FRAGMENT)?;
    let program = gl.create_program().ok_or("cannot create a program")?;
    gl.attach_shader(&program, &vertex);
    gl.attach_shader(&program, &fragment);
    gl.link_program(&program);
    // Linked programs keep their code: the shaders can go.
    gl.delete_shader(Some(&vertex));
    gl.delete_shader(Some(&fragment));
    if gl
        .get_program_parameter(&program, Gl::LINK_STATUS)
        .as_bool()
        == Some(true)
    {
        Ok(program)
    } else {
        let log = gl.get_program_info_log(&program).unwrap_or_default();
        gl.delete_program(Some(&program));
        Err(format!("shader link failed: {log}"))
    }
}

fn array_buffer(gl: &Gl, location: u32, data: &[f32]) -> Option<WebGlBuffer> {
    let buffer = gl.create_buffer();
    gl.bind_buffer(Gl::ARRAY_BUFFER, buffer.as_ref());
    gl.buffer_data_with_array_buffer_view(
        Gl::ARRAY_BUFFER,
        &js_sys::Float32Array::from(data),
        Gl::STATIC_DRAW,
    );
    gl.enable_vertex_attrib_array(location);
    gl.vertex_attrib_pointer_with_i32(location, 3, Gl::FLOAT, false, 0, 0);
    buffer
}

fn element_buffer(gl: &Gl, indices: &[u32]) -> Option<WebGlBuffer> {
    let buffer = gl.create_buffer();
    gl.bind_buffer(Gl::ELEMENT_ARRAY_BUFFER, buffer.as_ref());
    gl.buffer_data_with_array_buffer_view(
        Gl::ELEMENT_ARRAY_BUFFER,
        &js_sys::Uint32Array::from(indices),
        Gl::STATIC_DRAW,
    );
    buffer
}

/// The triangles' vertex indices (0, 1, 2… for a geometry without indices).
fn indices(geometry: &BufferGeometry) -> Vec<u32> {
    if geometry.indices.is_empty() {
        (0..(geometry.positions.len() / 3) as u32).collect()
    } else {
        geometry.indices.clone()
    }
}

fn upload(gl: &Gl, geometry: &BufferGeometry) -> Option<GpuGeometry> {
    if geometry.positions.is_empty() {
        return None;
    }
    let vao = gl.create_vertex_array();
    gl.bind_vertex_array(vao.as_ref());
    let position = array_buffer(gl, 0, &geometry.positions);
    let normal = array_buffer(gl, 1, &geometry.normals);
    let indices = indices(geometry);
    let index = element_buffer(gl, &indices);
    gl.bind_vertex_array(None);
    Some(GpuGeometry {
        vao,
        buffers: vec![position, normal, index],
        count: indices.len() as i32,
        wire: None,
        used: 0,
    })
}

/// A second vertex array with the same vertices, and each edge of the triangles once.
fn upload_edges(
    gl: &Gl,
    g: &GpuGeometry,
    geometry: &BufferGeometry,
) -> (Option<WebGlVertexArrayObject>, Option<WebGlBuffer>, i32) {
    let mut seen = HashSet::new();
    let mut edges = Vec::new();
    for t in indices(geometry).chunks_exact(3) {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            if seen.insert((a.min(b), a.max(b))) {
                edges.extend([a, b]);
            }
        }
    }
    let vao = gl.create_vertex_array();
    gl.bind_vertex_array(vao.as_ref());
    for (location, buffer) in [(0, &g.buffers[0]), (1, &g.buffers[1])] {
        gl.bind_buffer(Gl::ARRAY_BUFFER, buffer.as_ref());
        gl.enable_vertex_attrib_array(location);
        gl.vertex_attrib_pointer_with_i32(location, 3, Gl::FLOAT, false, 0, 0);
    }
    let index = element_buffer(gl, &edges);
    gl.bind_vertex_array(None);
    (vao, index, edges.len() as i32)
}

fn delete(gl: &Gl, g: GpuGeometry) {
    gl.delete_vertex_array(g.vao.as_ref());
    for buffer in &g.buffers {
        gl.delete_buffer(buffer.as_ref());
    }
    if let Some((vao, index, _)) = g.wire {
        gl.delete_vertex_array(vao.as_ref());
        gl.delete_buffer(index.as_ref());
    }
}

/// The page's `window.devicePixelRatio` (1 when unknown): pass it to
/// [`Renderer::set_pixel_ratio`] for sharp images on high-density screens.
pub fn device_pixel_ratio() -> f64 {
    web_sys::window().map_or(1.0, |w| w.device_pixel_ratio())
}

// ----- The animation loop ---------------------------------------------------------------

struct LoopState {
    running: Cell<bool>,
    callback: RefCell<Box<dyn FnMut(f64)>>,
}

/// A running [`animation_loop`]: it stops when dropped, or with [`stop`](Self::stop).
#[must_use = "the loop stops when this handle is dropped"]
pub struct AnimationLoop {
    state: Rc<LoopState>,
}

impl AnimationLoop {
    /// Stops the loop.
    pub fn stop(&self) {
        self.state.running.set(false);
    }

    /// Whether it still runs.
    pub fn is_running(&self) -> bool {
        self.state.running.get()
    }
}

impl Drop for AnimationLoop {
    fn drop(&mut self) {
        self.stop();
    }
}

impl std::fmt::Debug for AnimationLoop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimationLoop")
            .field("running", &self.is_running())
            .finish()
    }
}

/// Calls `callback` before each repaint of the page with the time in milliseconds
/// (`renderer.setAnimationLoop(callback)`, `requestAnimationFrame`), until the returned
/// handle is dropped. Hidden tabs pause it.
///
/// In a component, drop the handle when the element goes: return it from
/// [`use_action`](crate::Element::use_action)'s cleanup.
pub fn animation_loop(callback: impl FnMut(f64) + 'static) -> AnimationLoop {
    let state = Rc::new(LoopState {
        running: Cell::new(true),
        callback: RefCell::new(Box::new(callback)),
    });
    next_frame(state.clone());
    AnimationLoop { state }
}

fn next_frame(state: Rc<LoopState>) {
    let Some(window) = web_sys::window() else {
        return;
    };
    // A closure for one frame, freed once called: a stopped loop runs one last empty frame
    // and lets everything go.
    let step = Closure::once_into_js(move |time: f64| {
        if !state.running.get() {
            return;
        }
        if let Ok(mut callback) = state.callback.try_borrow_mut() {
            callback(time);
        }
        if state.running.get() {
            next_frame(state);
        }
    });
    let _ = window.request_animation_frame(step.unchecked_ref());
}

// ----- The pointer, for OrbitControls ---------------------------------------------------

type Listener = Closure<dyn FnMut(web_sys::Event)>;

/// Event listeners on an element, removed when dropped.
pub(crate) struct Listeners {
    target: web_sys::EventTarget,
    closures: Vec<(&'static str, Listener)>,
}

impl Drop for Listeners {
    fn drop(&mut self) {
        for (name, closure) in &self.closures {
            let _ = self
                .target
                .remove_event_listener_with_callback(name, closure.as_ref().unchecked_ref());
        }
    }
}

#[derive(Default)]
struct Pointers {
    /// The pointers down: id and position.
    down: Vec<(i32, Vector2)>,
    /// A single pointer pans rather than rotates (right button, or a modifier key).
    panning: bool,
}

impl Pointers {
    fn center(&self) -> Vector2 {
        let n = self.down.len().max(1) as f32;
        let (x, y) = self
            .down
            .iter()
            .fold((0.0, 0.0), |(x, y), (_, p)| (x + p.x, y + p.y));
        Vector2::new(x / n, y / n)
    }

    fn spread(&self) -> f32 {
        match self.down.as_slice() {
            [(_, a), (_, b), ..] => Vector2::new(a.x - b.x, a.y - b.y).length(),
            _ => 0.0,
        }
    }
}

impl Listeners {
    pub(crate) fn orbit(element: &web_sys::Element, input: Rc<RefCell<Input>>) -> Listeners {
        if let Some(html) = element.dyn_ref::<web_sys::HtmlElement>() {
            // The page must not scroll or zoom under the fingers.
            let _ = html.style().set_property("touch-action", "none");
        }
        let pointers = Rc::new(RefCell::new(Pointers::default()));
        let mut listeners = Listeners::new(element);
        let height = {
            let element = element.clone();
            move || element.client_height() as f32
        };

        let (p, el) = (pointers.clone(), element.clone());
        listeners.on("pointerdown", move |e| {
            let Some(e) = e.dyn_ref::<web_sys::PointerEvent>() else {
                return;
            };
            let _ = el.set_pointer_capture(e.pointer_id());
            let mut p = p.borrow_mut();
            p.down.retain(|(id, _)| *id != e.pointer_id());
            p.down.push((
                e.pointer_id(),
                Vector2::new(e.client_x() as f32, e.client_y() as f32),
            ));
            if p.down.len() == 1 {
                p.panning = e.button() == 2 || e.shift_key() || e.ctrl_key() || e.meta_key();
            }
        });

        let (p, i, h) = (pointers.clone(), input.clone(), height.clone());
        listeners.on("pointermove", move |e| {
            let Some(e) = e.dyn_ref::<web_sys::PointerEvent>() else {
                return;
            };
            let mut p = p.borrow_mut();
            let Some(index) = p.down.iter().position(|(id, _)| *id == e.pointer_id()) else {
                return;
            };
            let (center, spread) = (p.center(), p.spread());
            let at = Vector2::new(e.client_x() as f32, e.client_y() as f32);
            let moved = Vector2::new(at.x - p.down[index].1.x, at.y - p.down[index].1.y);
            p.down[index].1 = at;
            let mut input = i.borrow_mut();
            input.height = h();
            if p.down.len() >= 2 {
                // Two fingers: pinch to zoom, move together to pan.
                let (new_center, new_spread) = (p.center(), p.spread());
                if spread > 0.0 && new_spread > 0.0 {
                    input.zoom *= spread / new_spread;
                }
                input.pan.x += new_center.x - center.x;
                input.pan.y += new_center.y - center.y;
            } else if p.panning {
                input.pan.x += moved.x;
                input.pan.y += moved.y;
            } else {
                input.rotate.x += moved.x;
                input.rotate.y += moved.y;
            }
        });

        for name in ["pointerup", "pointercancel"] {
            let p = pointers.clone();
            listeners.on(name, move |e| {
                let Some(e) = e.dyn_ref::<web_sys::PointerEvent>() else {
                    return;
                };
                let mut p = p.borrow_mut();
                p.down.retain(|(id, _)| *id != e.pointer_id());
                if p.down.len() <= 1 {
                    p.panning = false;
                }
            });
        }

        let (i, h) = (input, height);
        listeners.on_not_passive("wheel", move |e| {
            let Some(e) = e.dyn_ref::<web_sys::WheelEvent>() else {
                return;
            };
            e.prevent_default();
            // Lines and pages, in pixels.
            let pixels = match e.delta_mode() {
                1 => 16.0,
                2 => 100.0,
                _ => 1.0,
            };
            let mut input = i.borrow_mut();
            input.height = h();
            input.zoom *= 0.95f32.powf(-(e.delta_y() as f32) * pixels * 0.01);
        });
        listeners.on("contextmenu", |e| e.prevent_default());
        listeners
    }

    /// Listeners on an element, the window or the document.
    pub(crate) fn new(target: &web_sys::EventTarget) -> Listeners {
        Listeners {
            target: target.clone(),
            closures: Vec::new(),
        }
    }

    pub(crate) fn on(&mut self, name: &'static str, f: impl FnMut(web_sys::Event) + 'static) {
        let closure = Closure::<dyn FnMut(web_sys::Event)>::new(f);
        let _ = self
            .target
            .add_event_listener_with_callback(name, closure.as_ref().unchecked_ref());
        self.closures.push((name, closure));
    }

    /// For events whose default action is cancelled (the wheel scrolls the page otherwise).
    pub(crate) fn on_not_passive(
        &mut self,
        name: &'static str,
        f: impl FnMut(web_sys::Event) + 'static,
    ) {
        let closure = Closure::<dyn FnMut(web_sys::Event)>::new(f);
        let options = web_sys::AddEventListenerOptions::new();
        options.set_passive(false);
        let _ = self
            .target
            .add_event_listener_with_callback_and_add_event_listener_options(
                name,
                closure.as_ref().unchecked_ref(),
                &options,
            );
        self.closures.push((name, closure));
    }
}
