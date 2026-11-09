//! The browser side: [`Renderer`] draws a [`Frame`] with WebGL 2, [`animation_loop`]
//! calls a closure before each repaint, and the pointer listeners of
//! [`OrbitControls`](super::OrbitControls).
//!
//! One shader program draws every material: the kind is a uniform, the lights are arrays
//! (4 directional, 16 point and 8 spot lights at most: the point lights nearest the
//! camera are kept), and the lighting follows a physically based model (Lambert diffuse,
//! GGX specular, the same light falloff), then the tone mapping and the sRGB output.
//! Textures are read in linear light (sRGB textures), with mipmaps and anisotropic
//! filtering; normal maps need no tangents (they are computed from the screen-space
//! derivatives). Shadows are depth maps drawn from each light casting them (4 at most),
//! compared by the hardware and softened over 3 × 3 samples.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::JsValue;
use web_sys::{
    HtmlCanvasElement, HtmlImageElement, WebGl2RenderingContext as Gl, WebGlBuffer,
    WebGlFramebuffer, WebGlProgram, WebGlSampler, WebGlShader, WebGlTexture, WebGlUniformLocation,
    WebGlVertexArrayObject,
};

use super::controls::Input;
use super::graph::{MAX_SHADOWS, ShadowCamera, ShadowLight};
use super::math::{Color, Vector2, Vector3, linear_to_srgb};
use super::texture::{ColorSpace, Filter, Texture, TextureSource, Wrapping};
use super::{BufferGeometry, Camera, Draw, Frame, Material, Scene, Side, ToneMapping};

const MAX_DIRECTIONAL: usize = 4;
const MAX_POINT: usize = 16;
const MAX_SPOT: usize = 8;
/// A geometry or a texture not drawn for this many frames gives its GPU memory back.
const KEEP_FRAMES: u64 = 120;
/// The texture units: the material's maps, then the shadow maps.
const UNIT_MAP: u32 = 0;
const UNIT_EMISSIVE: u32 = 1;
const UNIT_NORMAL: u32 = 2;
const UNIT_ROUGHNESS_METALNESS: u32 = 3;
const UNIT_SHADOW: u32 = 4;
/// `EXT_texture_filter_anisotropic`.
const TEXTURE_MAX_ANISOTROPY: u32 = 0x84FE;
const MAX_TEXTURE_MAX_ANISOTROPY: u32 = 0x84FF;

const VERTEX: &str = r#"#version 300 es
layout(location = 0) in vec3 position;
layout(location = 1) in vec3 normal;
layout(location = 2) in vec2 uv;
uniform mat4 modelMatrix;
uniform mat4 viewMatrix;
uniform mat4 projectionMatrix;
uniform mat3 normalMatrix;
out vec3 vWorldPosition;
out vec3 vNormal;
out vec2 vUv;
void main() {
    vec4 world = modelMatrix * vec4(position, 1.0);
    vWorldPosition = world.xyz;
    vNormal = normalMatrix * normal;
    vUv = uv;
    gl_Position = projectionMatrix * viewMatrix * world;
}
"#;

const FRAGMENT: &str = r#"#version 300 es
precision highp float;
precision highp sampler2DShadow;
#define RECIPROCAL_PI 0.3183098861837907
in vec3 vWorldPosition;
in vec3 vNormal;
in vec2 vUv;
uniform int kind; // 0 basic, 1 Lambert, 2 standard, 3 normal
uniform vec3 diffuse;
uniform float opacity;
uniform bool blend;
uniform float alphaTest;
uniform vec3 emissive;
uniform float roughness;
uniform float metalness;
uniform bool flatShading;
uniform bool hasMap;
uniform sampler2D map;
uniform mat3 mapTransform;
uniform bool hasEmissiveMap;
uniform sampler2D emissiveMap;
uniform mat3 emissiveMapTransform;
uniform bool hasNormalMap;
uniform sampler2D normalMap;
uniform mat3 normalMapTransform;
uniform vec2 normalScale;
uniform bool hasRoughnessMetalnessMap;
uniform sampler2D roughnessMetalnessMap;
uniform mat3 roughnessMetalnessMapTransform;
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
uniform int directionalShadow[4];
uniform int numPoint;
uniform vec3 pointPosition[16];
uniform vec3 pointColor[16];
uniform vec2 pointFalloff[16]; // distance, decay
uniform int numSpot;
uniform vec3 spotPosition[8];
uniform vec3 spotDirection[8];
uniform vec3 spotColor[8];
uniform vec4 spotParams[8]; // distance, decay, cos of the cone, cos of the full light
uniform int spotShadow[8];
uniform bool receiveShadow;
uniform mat4 shadowMatrix[4];
uniform vec4 shadowParams[4]; // bias, normal bias, radius, 1 / map size
uniform sampler2DShadow shadowMap0;
uniform sampler2DShadow shadowMap1;
uniform sampler2DShadow shadowMap2;
uniform sampler2DShadow shadowMap3;
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

vec2 transformed(mat3 m) {
    return (m * vec3(vUv, 1.0)).xy;
}

// A normal map without tangents: the frame of the texture from the derivatives on screen.
vec3 perturbNormal(vec3 n, vec3 mapN, vec2 uv, float faceDirection) {
    vec3 q0 = dFdx(vWorldPosition);
    vec3 q1 = dFdy(vWorldPosition);
    vec2 st0 = dFdx(uv);
    vec2 st1 = dFdy(uv);
    vec3 q1perp = cross(q1, n);
    vec3 q0perp = cross(n, q0);
    vec3 t = q1perp * st0.x + q0perp * st1.x;
    vec3 b = q1perp * st0.y + q0perp * st1.y;
    float det = max(dot(t, t), dot(b, b));
    float scale = det == 0.0 ? 0.0 : faceDirection * inversesqrt(det);
    return normalize(t * (mapN.x * scale) + b * (mapN.y * scale) + n * mapN.z);
}

float shadowSample(int i, vec3 c) {
    if (i == 0) return texture(shadowMap0, c);
    if (i == 1) return texture(shadowMap1, c);
    if (i == 2) return texture(shadowMap2, c);
    return texture(shadowMap3, c);
}

float shadowFactor(int i, vec3 n) {
    if (i < 0 || !receiveShadow) return 1.0;
    vec4 p = shadowMatrix[i] * vec4(vWorldPosition + n * shadowParams[i].y, 1.0);
    vec3 c = p.xyz / p.w * 0.5 + 0.5;
    if (p.w <= 0.0 || c.x < 0.0 || c.x > 1.0 || c.y < 0.0 || c.y > 1.0 || c.z > 1.0) return 1.0;
    c.z -= shadowParams[i].x;
    vec2 r = vec2(shadowParams[i].z * shadowParams[i].w);
    float lit = 0.0;
    for (int x = -1; x <= 1; x++) {
        for (int y = -1; y <= 1; y++) {
            lit += shadowSample(i, vec3(c.xy + vec2(float(x), float(y)) * r, c.z));
        }
    }
    return lit / 9.0;
}

void main() {
    vec3 base = diffuse;
    float alpha = opacity;
    if (hasMap) {
        vec4 texel = texture(map, transformed(mapTransform));
        base *= texel.rgb;
        alpha *= texel.a;
    }
    if (alpha < alphaTest) discard;
    float faceDirection = gl_FrontFacing ? 1.0 : -1.0;
    vec3 n;
    if (flatShading) {
        n = normalize(cross(dFdx(vWorldPosition), dFdy(vWorldPosition)));
    } else {
        n = normalize(vNormal) * faceDirection;
    }
    vec3 geometryNormal = n;
    vec3 color;
    if (kind == 3) {
        color = normalize(mat3(viewMatrix) * n) * 0.5 + 0.5;
    } else if (kind == 0) {
        color = base;
    } else {
        if (hasNormalMap) {
            vec2 uv = transformed(normalMapTransform);
            vec3 mapN = texture(normalMap, uv).xyz * 2.0 - 1.0;
            mapN.xy *= normalScale;
            n = perturbNormal(n, mapN, uv, faceDirection);
        }
        float rough0 = roughness;
        float metal = metalness;
        if (hasRoughnessMetalnessMap) {
            vec4 rm = texture(roughnessMetalnessMap, transformed(roughnessMetalnessMapTransform));
            rough0 *= rm.g;
            metal *= rm.b;
        }
        vec3 glow = emissive;
        if (hasEmissiveMap) glow *= texture(emissiveMap, transformed(emissiveMapTransform)).rgb;
        vec3 v = orthographic ? cameraBack : normalize(cameraPosition - vWorldPosition);
        vec3 diffuseColor = kind == 2 ? base * (1.0 - metal) : base;
        vec3 f0 = mix(vec3(0.04), base, metal);
        vec3 dxy = max(abs(dFdx(n)), abs(dFdy(n)));
        float rough = min(max(rough0, 0.0525) + max(max(dxy.x, dxy.y), dxy.z), 1.0);
        vec3 irradiance = ambientLightColor + mix(groundColor, skyColor, 0.5 * n.y + 0.5);
        vec3 specular = vec3(0.0);
        for (int i = 0; i < 4; i++) {
            if (i >= numDirectional) break;
            vec3 l = directionalDirection[i];
            float shadow = shadowFactor(directionalShadow[i], geometryNormal);
            vec3 light = clamp(dot(n, l), 0.0, 1.0) * directionalColor[i] * shadow;
            irradiance += light;
            if (kind == 2) specular += light * specularGgx(l, v, n, f0, rough);
        }
        for (int i = 0; i < 16; i++) {
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
        for (int i = 0; i < 8; i++) {
            if (i >= numSpot) break;
            vec3 toLight = spotPosition[i] - vWorldPosition;
            float d = length(toLight);
            vec3 l = toLight / max(d, 1e-6);
            float cone = smoothstep(spotParams[i].z, spotParams[i].w, dot(l, -spotDirection[i]));
            if (cone <= 0.0) continue;
            float falloff = cone / max(pow(d, spotParams[i].y), 0.01);
            if (spotParams[i].x > 0.0) falloff *= pow2(clamp(1.0 - pow2(pow2(d / spotParams[i].x)), 0.0, 1.0));
            float shadow = shadowFactor(spotShadow[i], geometryNormal);
            vec3 light = clamp(dot(n, l), 0.0, 1.0) * spotColor[i] * falloff * shadow;
            irradiance += light;
            if (kind == 2) specular += light * specularGgx(l, v, n, f0, rough);
        }
        color = (irradiance * RECIPROCAL_PI * diffuseColor + specular + glow) * exposure;
    }
    if (toneMapping == 1) color = acesFilmic(color);
    color = linearToSrgb(clamp(color, 0.0, 1.0));
    float outAlpha = blend ? clamp(alpha, 0.0, 1.0) : 1.0;
    // The canvas expects premultiplied alpha.
    fragColor = vec4(color * outAlpha, outAlpha);
}
"#;

const SHADOW_VERTEX: &str = r#"#version 300 es
layout(location = 0) in vec3 position;
uniform mat4 lightMatrix;
uniform mat4 modelMatrix;
void main() {
    gl_Position = lightMatrix * modelMatrix * vec4(position, 1.0);
}
"#;

const SHADOW_FRAGMENT: &str = r#"#version 300 es
precision mediump float;
void main() {}
"#;

/// Where the uniforms are, found by name once.
struct Uniforms {
    at: HashMap<&'static str, WebGlUniformLocation>,
}

impl Uniforms {
    fn new(gl: &Gl, p: &WebGlProgram, names: &[&'static str]) -> Uniforms {
        let at = names
            .iter()
            .filter_map(|name| Some((*name, gl.get_uniform_location(p, name)?)))
            .collect();
        Uniforms { at }
    }

    fn get(&self, name: &str) -> Option<&WebGlUniformLocation> {
        self.at.get(name)
    }
}

const UNIFORMS: &[&str] = &[
    "modelMatrix",
    "viewMatrix",
    "projectionMatrix",
    "normalMatrix",
    "kind",
    "diffuse",
    "opacity",
    "blend",
    "alphaTest",
    "emissive",
    "roughness",
    "metalness",
    "flatShading",
    "hasMap",
    "map",
    "mapTransform",
    "hasEmissiveMap",
    "emissiveMap",
    "emissiveMapTransform",
    "hasNormalMap",
    "normalMap",
    "normalMapTransform",
    "normalScale",
    "hasRoughnessMetalnessMap",
    "roughnessMetalnessMap",
    "roughnessMetalnessMapTransform",
    "cameraPosition",
    "orthographic",
    "cameraBack",
    "ambientLightColor",
    "skyColor",
    "groundColor",
    "numDirectional",
    "directionalDirection[0]",
    "directionalColor[0]",
    "directionalShadow[0]",
    "numPoint",
    "pointPosition[0]",
    "pointColor[0]",
    "pointFalloff[0]",
    "numSpot",
    "spotPosition[0]",
    "spotDirection[0]",
    "spotColor[0]",
    "spotParams[0]",
    "spotShadow[0]",
    "receiveShadow",
    "shadowMatrix[0]",
    "shadowParams[0]",
    "shadowMap0",
    "shadowMap1",
    "shadowMap2",
    "shadowMap3",
    "exposure",
    "toneMapping",
];

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

/// A texture on the GPU (one per image, color space and orientation).
struct GpuTexture {
    texture: Option<WebGlTexture>,
    /// The image being downloaded, until it is uploaded.
    image: Option<HtmlImageElement>,
    ready: bool,
    version: u32,
    used: u64,
}

/// A shadow map: a depth texture and the framebuffer that draws into it.
struct ShadowTarget {
    texture: Option<WebGlTexture>,
    framebuffer: Option<WebGlFramebuffer>,
    size: u32,
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
    shadow_program: Option<WebGlProgram>,
    shadow_u: Uniforms,
    geometries: HashMap<u64, GpuGeometry>,
    textures: HashMap<(u64, bool, bool), GpuTexture>,
    samplers: HashMap<(Wrapping, Wrapping, Filter, Filter), Option<WebGlSampler>>,
    shadow_targets: Vec<ShadowTarget>,
    /// Bound to the units of missing textures: one white pixel, a depth of 1.
    white: Option<WebGlTexture>,
    no_shadow: Option<WebGlTexture>,
    anisotropy: f32,
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
    /// Whether shadows are drawn (`renderer.shadowMap.enabled`; true by default: a scene
    /// has shadows as soon as a light and meshes cast them).
    pub shadows: bool,
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
        let program = link(&gl, VERTEX, FRAGMENT)?;
        let u = Uniforms::new(&gl, &program, UNIFORMS);
        // Each sampler on its own unit (they all start on unit 0).
        gl.use_program(Some(&program));
        for (name, unit) in [
            ("map", UNIT_MAP),
            ("emissiveMap", UNIT_EMISSIVE),
            ("normalMap", UNIT_NORMAL),
            ("roughnessMetalnessMap", UNIT_ROUGHNESS_METALNESS),
            ("shadowMap0", UNIT_SHADOW),
            ("shadowMap1", UNIT_SHADOW + 1),
            ("shadowMap2", UNIT_SHADOW + 2),
            ("shadowMap3", UNIT_SHADOW + 3),
        ] {
            gl.uniform1i(u.get(name), unit as i32);
        }
        let shadow_program = link(&gl, SHADOW_VERTEX, SHADOW_FRAGMENT)?;
        let shadow_u = Uniforms::new(&gl, &shadow_program, &["lightMatrix", "modelMatrix"]);
        let anisotropy = match gl.get_extension("EXT_texture_filter_anisotropic") {
            Ok(Some(_)) => gl
                .get_parameter(MAX_TEXTURE_MAX_ANISOTROPY)
                .ok()
                .and_then(|v| v.as_f64())
                .map_or(1.0, |v| v.min(8.0) as f32),
            _ => 1.0,
        };
        let white = gl.create_texture();
        gl.bind_texture(Gl::TEXTURE_2D, white.as_ref());
        let _ = gl.tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
            Gl::TEXTURE_2D,
            0,
            Gl::RGBA8 as i32,
            1,
            1,
            0,
            Gl::RGBA,
            Gl::UNSIGNED_BYTE,
            Some(&[255, 255, 255, 255]),
        );
        let no_shadow = gl.create_texture();
        gl.bind_texture(Gl::TEXTURE_2D, no_shadow.as_ref());
        let depth = js_sys::Uint32Array::from(&[u32::MAX][..]);
        let _ = gl
            .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_array_buffer_view(
                Gl::TEXTURE_2D,
                0,
                Gl::DEPTH_COMPONENT24 as i32,
                1,
                1,
                0,
                Gl::DEPTH_COMPONENT,
                Gl::UNSIGNED_INT,
                Some(&depth),
            );
        shadow_texture_parameters(&gl);
        gl.bind_texture(Gl::TEXTURE_2D, None);
        Ok(Renderer {
            canvas,
            gl,
            program: Some(program),
            u,
            shadow_program: Some(shadow_program),
            shadow_u,
            geometries: HashMap::new(),
            textures: HashMap::new(),
            samplers: HashMap::new(),
            shadow_targets: Vec::new(),
            white,
            no_shadow,
            anisotropy,
            frame_number: 0,
            pixel_ratio: 1.0,
            size: None,
            clear_color: Color::BLACK,
            clear_alpha: 0.0,
            tone_mapping: ToneMapping::None,
            tone_mapping_exposure: 1.0,
            shadows: true,
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
        let shadows = if self.shadows {
            frame.shadow_cameras()
        } else {
            Vec::new()
        };
        self.draw_shadow_maps(frame, &shadows);

        let (width, height) = self.drawing_buffer_size();
        let gl = &self.gl;
        gl.bind_framebuffer(Gl::FRAMEBUFFER, None);
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
        self.set_frame_uniforms(frame, &shadows);

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

        // Give back the memory of the geometries and textures no longer drawn.
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
        let gl = &self.gl;
        self.textures.retain(|_, t| {
            let keep = now - t.used <= KEEP_FRAMES;
            if !keep {
                gl.delete_texture(t.texture.as_ref());
            }
            keep
        });
    }

    /// Draws the depth of the meshes casting shadows, from each light casting them.
    fn draw_shadow_maps(&mut self, frame: &Frame<'_>, cameras: &[ShadowCamera]) {
        if cameras.is_empty() {
            return;
        }
        self.gl.active_texture(Gl::TEXTURE0 + UNIT_SHADOW);
        for (i, camera) in cameras.iter().enumerate() {
            let size = camera.settings.map_size.clamp(16, 8192);
            if self.shadow_targets.len() <= i {
                self.shadow_targets.push(ShadowTarget {
                    texture: None,
                    framebuffer: None,
                    size: 0,
                });
            }
            if self.shadow_targets[i].size != size {
                let target = &mut self.shadow_targets[i];
                let gl = &self.gl;
                gl.delete_texture(target.texture.as_ref());
                gl.delete_framebuffer(target.framebuffer.as_ref());
                target.texture = gl.create_texture();
                gl.bind_texture(Gl::TEXTURE_2D, target.texture.as_ref());
                gl.tex_storage_2d(
                    Gl::TEXTURE_2D,
                    1,
                    Gl::DEPTH_COMPONENT24,
                    size as i32,
                    size as i32,
                );
                shadow_texture_parameters(gl);
                target.framebuffer = gl.create_framebuffer();
                gl.bind_framebuffer(Gl::FRAMEBUFFER, target.framebuffer.as_ref());
                gl.framebuffer_texture_2d(
                    Gl::FRAMEBUFFER,
                    Gl::DEPTH_ATTACHMENT,
                    Gl::TEXTURE_2D,
                    target.texture.as_ref(),
                    0,
                );
                gl.bind_texture(Gl::TEXTURE_2D, None);
                target.size = size;
            }
        }
        // The shadow maps are drawn into: none of them may stay bound for reading.
        for unit in 0..MAX_SHADOWS as u32 {
            self.gl.active_texture(Gl::TEXTURE0 + UNIT_SHADOW + unit);
            self.gl
                .bind_texture(Gl::TEXTURE_2D, self.no_shadow.as_ref());
        }
        self.gl.use_program(self.shadow_program.as_ref());
        self.gl.enable(Gl::DEPTH_TEST);
        self.gl.depth_func(Gl::LEQUAL);
        self.gl.depth_mask(true);
        self.gl.disable(Gl::BLEND);
        self.gl.disable(Gl::CULL_FACE);
        self.gl.enable(Gl::POLYGON_OFFSET_FILL);
        self.gl.polygon_offset(1.0, 1.0);
        for (i, camera) in cameras.iter().enumerate() {
            let (framebuffer, size) = (
                self.shadow_targets[i].framebuffer.clone(),
                self.shadow_targets[i].size,
            );
            self.gl
                .bind_framebuffer(Gl::FRAMEBUFFER, framebuffer.as_ref());
            self.gl.viewport(0, 0, size as i32, size as i32);
            self.gl.clear(Gl::DEPTH_BUFFER_BIT);
            self.gl.uniform_matrix4fv_with_f32_array(
                self.shadow_u.get("lightMatrix"),
                false,
                &camera.view_projection.elements,
            );
            for draw in frame.meshes.iter().filter(|d| d.cast_shadow) {
                let Some(g) = self.geometry(draw.geometry) else {
                    continue;
                };
                let (vao, count) = (g.vao.clone(), g.count);
                self.gl.uniform_matrix4fv_with_f32_array(
                    self.shadow_u.get("modelMatrix"),
                    false,
                    &draw.world.elements,
                );
                self.gl.bind_vertex_array(vao.as_ref());
                self.gl
                    .draw_elements_with_i32(Gl::TRIANGLES, count, Gl::UNSIGNED_INT, 0);
            }
        }
        self.gl.disable(Gl::POLYGON_OFFSET_FILL);
        self.gl.bind_vertex_array(None);
        self.gl.bind_framebuffer(Gl::FRAMEBUFFER, None);
    }

    /// The buffers of a geometry, uploaded the first time.
    fn geometry(&mut self, geometry: &BufferGeometry) -> Option<&GpuGeometry> {
        let id = geometry.id();
        if !self.geometries.contains_key(&id) {
            let uploaded = upload(&self.gl, geometry)?;
            self.geometries.insert(id, uploaded);
        }
        let g = self.geometries.get_mut(&id)?;
        g.used = self.frame_number;
        Some(g)
    }

    fn set_frame_uniforms(&mut self, frame: &Frame<'_>, shadows: &[ShadowCamera]) {
        let (gl, u) = (&self.gl, &self.u);
        gl.uniform_matrix4fv_with_f32_array(u.get("viewMatrix"), false, &frame.view.elements);
        gl.uniform_matrix4fv_with_f32_array(
            u.get("projectionMatrix"),
            false,
            &frame.projection.elements,
        );
        let eye = frame.camera.translation();
        gl.uniform3f(u.get("cameraPosition"), eye.x, eye.y, eye.z);
        let e = &frame.camera.elements;
        let back = Vector3::new(e[8], e[9], e[10]).normalize();
        gl.uniform3f(u.get("cameraBack"), back.x, back.y, back.z);
        // A perspective projection has 0 in its last element, an orthographic one 1.
        gl.uniform1i(
            u.get("orthographic"),
            i32::from(frame.projection.elements[15] == 1.0),
        );

        let lights = &frame.lights;
        set_color(gl, u.get("ambientLightColor"), lights.ambient);
        let (sky, ground) = lights.hemisphere.unwrap_or((Color::BLACK, Color::BLACK));
        set_color(gl, u.get("skyColor"), sky);
        set_color(gl, u.get("groundColor"), ground);

        // Which shadow map each light reads (-1: none).
        let mut directional_shadow = [-1i32; MAX_DIRECTIONAL];
        let mut spot_shadow = [-1i32; MAX_SPOT];
        let mut matrices = [0.0; MAX_SHADOWS * 16];
        let mut params = [0.0; MAX_SHADOWS * 4];
        for (i, camera) in shadows.iter().enumerate() {
            match camera.light {
                ShadowLight::Directional(l) if l < MAX_DIRECTIONAL => {
                    directional_shadow[l] = i as i32
                }
                ShadowLight::Spot(l) if l < MAX_SPOT => spot_shadow[l] = i as i32,
                _ => {}
            }
            matrices[i * 16..i * 16 + 16].copy_from_slice(&camera.view_projection.elements);
            let s = camera.settings;
            params[i * 4..i * 4 + 4].copy_from_slice(&[
                // A perspective depth is not linear: the bias is smaller there.
                if camera.orthographic {
                    s.bias
                } else {
                    s.bias * 0.1
                },
                s.normal_bias,
                s.radius.max(0.0),
                1.0 / s.map_size.clamp(16, 8192) as f32,
            ]);
        }
        gl.uniform1iv_with_i32_array(u.get("directionalShadow[0]"), &directional_shadow);
        gl.uniform1iv_with_i32_array(u.get("spotShadow[0]"), &spot_shadow);
        gl.uniform_matrix4fv_with_f32_array(u.get("shadowMatrix[0]"), false, &matrices);
        gl.uniform4fv_with_f32_array(u.get("shadowParams[0]"), &params);
        for unit in 0..MAX_SHADOWS {
            gl.active_texture(Gl::TEXTURE0 + UNIT_SHADOW + unit as u32);
            gl.bind_sampler(UNIT_SHADOW + unit as u32, None);
            let texture = if unit < shadows.len() {
                self.shadow_targets[unit].texture.as_ref()
            } else {
                self.no_shadow.as_ref()
            };
            gl.bind_texture(Gl::TEXTURE_2D, texture);
        }

        let directional = &lights.directional[..lights.directional.len().min(MAX_DIRECTIONAL)];
        gl.uniform1i(u.get("numDirectional"), directional.len() as i32);
        let mut directions = [0.0; MAX_DIRECTIONAL * 3];
        let mut colors = [0.0; MAX_DIRECTIONAL * 3];
        for (i, (direction, color)) in directional.iter().enumerate() {
            directions[i * 3..i * 3 + 3].copy_from_slice(&direction.to_array());
            colors[i * 3..i * 3 + 3].copy_from_slice(&color.to_array());
        }
        gl.uniform3fv_with_f32_array(u.get("directionalDirection[0]"), &directions);
        gl.uniform3fv_with_f32_array(u.get("directionalColor[0]"), &colors);

        // Too many point lights: those that light the camera's surroundings most.
        let mut point: Vec<_> = lights.point.iter().collect();
        if point.len() > MAX_POINT {
            let strength = |l: &super::PointLightData| {
                let d2 = (l.position - eye).length_squared();
                (l.color.r + l.color.g + l.color.b) / (1.0 + d2)
            };
            point.sort_by(|a, b| strength(b).total_cmp(&strength(a)));
            point.truncate(MAX_POINT);
        }
        gl.uniform1i(u.get("numPoint"), point.len() as i32);
        let mut positions = [0.0; MAX_POINT * 3];
        let mut colors = [0.0; MAX_POINT * 3];
        let mut falloffs = [0.0; MAX_POINT * 2];
        for (i, light) in point.iter().enumerate() {
            positions[i * 3..i * 3 + 3].copy_from_slice(&light.position.to_array());
            colors[i * 3..i * 3 + 3].copy_from_slice(&light.color.to_array());
            falloffs[i * 2] = light.distance;
            falloffs[i * 2 + 1] = light.decay;
        }
        gl.uniform3fv_with_f32_array(u.get("pointPosition[0]"), &positions);
        gl.uniform3fv_with_f32_array(u.get("pointColor[0]"), &colors);
        gl.uniform2fv_with_f32_array(u.get("pointFalloff[0]"), &falloffs);

        let spot = &lights.spot[..lights.spot.len().min(MAX_SPOT)];
        gl.uniform1i(u.get("numSpot"), spot.len() as i32);
        let mut positions = [0.0; MAX_SPOT * 3];
        let mut directions = [0.0; MAX_SPOT * 3];
        let mut colors = [0.0; MAX_SPOT * 3];
        let mut params = [0.0; MAX_SPOT * 4];
        for (i, light) in spot.iter().enumerate() {
            positions[i * 3..i * 3 + 3].copy_from_slice(&light.position.to_array());
            directions[i * 3..i * 3 + 3].copy_from_slice(&light.direction.normalize().to_array());
            colors[i * 3..i * 3 + 3].copy_from_slice(&light.color.to_array());
            let cone = light.angle.cos();
            let full = (light.angle * (1.0 - light.penumbra))
                .cos()
                .max(cone + 1e-4);
            params[i * 4..i * 4 + 4].copy_from_slice(&[light.distance, light.decay, cone, full]);
        }
        gl.uniform3fv_with_f32_array(u.get("spotPosition[0]"), &positions);
        gl.uniform3fv_with_f32_array(u.get("spotDirection[0]"), &directions);
        gl.uniform3fv_with_f32_array(u.get("spotColor[0]"), &colors);
        gl.uniform4fv_with_f32_array(u.get("spotParams[0]"), &params);

        gl.uniform1f(u.get("exposure"), frame.exposure);
        gl.uniform1i(
            u.get("toneMapping"),
            i32::from(frame.tone_mapping == ToneMapping::AcesFilmic),
        );
    }

    /// Binds a texture (or the white pixel) to a unit; whether it is there to read.
    fn bind_texture(&mut self, unit: u32, texture: Option<&Texture>) -> bool {
        // First the unit: a texture being made is bound to it, not to a shadow map's.
        self.gl.active_texture(Gl::TEXTURE0 + unit);
        let gpu = texture.and_then(|t| self.texture(t).map(|g| (g, t)));
        let gl = &self.gl;
        match gpu {
            Some((gpu, t)) => {
                gl.bind_texture(Gl::TEXTURE_2D, gpu.as_ref());
                let key = (t.wrap_s, t.wrap_t, t.mag_filter, t.min_filter);
                let anisotropy = self.anisotropy;
                let sampler = self
                    .samplers
                    .entry(key)
                    .or_insert_with(|| make_sampler(gl, key, anisotropy));
                gl.bind_sampler(unit, sampler.as_ref());
                true
            }
            None => {
                gl.bind_texture(Gl::TEXTURE_2D, self.white.as_ref());
                gl.bind_sampler(unit, None);
                false
            }
        }
    }

    /// The GPU texture of a texture, once its pixels are there.
    fn texture(&mut self, texture: &Texture) -> Option<Option<WebGlTexture>> {
        let key = (
            texture.id(),
            texture.color_space == ColorSpace::Srgb,
            texture.flip_y,
        );
        let now = self.frame_number;
        let stale = self
            .textures
            .get(&key)
            .is_some_and(|t| t.version != texture.version());
        if stale && let Some(old) = self.textures.remove(&key) {
            self.gl.delete_texture(old.texture.as_ref());
        }
        let gl = &self.gl;
        let entry = self.textures.entry(key).or_insert_with(|| {
            let mut gpu = GpuTexture {
                texture: None,
                image: None,
                ready: false,
                version: texture.version(),
                used: now,
            };
            match texture.source() {
                TextureSource::Rgba8 {
                    width,
                    height,
                    data,
                } => {
                    gpu.texture = gl.create_texture();
                    gl.bind_texture(Gl::TEXTURE_2D, gpu.texture.as_ref());
                    gl.pixel_storei(Gl::UNPACK_FLIP_Y_WEBGL, i32::from(key.2));
                    gl.pixel_storei(Gl::UNPACK_ALIGNMENT, 1);
                    let _ = gl
                        .tex_image_2d_with_i32_and_i32_and_i32_and_format_and_type_and_opt_u8_array(
                            Gl::TEXTURE_2D,
                            0,
                            internal_format(key.1),
                            *width as i32,
                            *height as i32,
                            0,
                            Gl::RGBA,
                            Gl::UNSIGNED_BYTE,
                            Some(&data[..]),
                        );
                    gl.generate_mipmap(Gl::TEXTURE_2D);
                    gl.pixel_storei(Gl::UNPACK_FLIP_Y_WEBGL, 0);
                    gpu.ready = true;
                }
                TextureSource::Url(url) => {
                    if let Ok(image) = HtmlImageElement::new() {
                        // Images of other sites, when they allow it.
                        if !url.starts_with('/') && !url.starts_with("data:") {
                            image.set_cross_origin(Some("anonymous"));
                        }
                        image.set_src(url);
                        gpu.image = Some(image);
                    }
                }
            }
            gpu
        });
        entry.used = now;
        if !entry.ready {
            let image = entry.image.as_ref()?;
            if !image.complete() || image.natural_width() == 0 {
                return None;
            }
            entry.texture = gl.create_texture();
            gl.bind_texture(Gl::TEXTURE_2D, entry.texture.as_ref());
            gl.pixel_storei(Gl::UNPACK_FLIP_Y_WEBGL, i32::from(key.2));
            let uploaded = gl.tex_image_2d_with_u32_and_u32_and_html_image_element(
                Gl::TEXTURE_2D,
                0,
                internal_format(key.1),
                Gl::RGBA,
                Gl::UNSIGNED_BYTE,
                image,
            );
            gl.pixel_storei(Gl::UNPACK_FLIP_Y_WEBGL, 0);
            if uploaded.is_err() {
                // Refused (another site without CORS): drawn without it.
                gl.delete_texture(entry.texture.take().as_ref());
                entry.image = None;
                return None;
            }
            gl.generate_mipmap(Gl::TEXTURE_2D);
            texture.set_size(image.natural_width(), image.natural_height());
            entry.image = None;
            entry.ready = true;
        }
        Some(entry.texture.clone())
    }

    fn draw_mesh(&mut self, draw: &Draw<'_>) {
        let Some(g) = self.geometry(draw.geometry) else {
            return;
        };
        let (vao, count) = (g.vao.clone(), g.count);
        let material = draw.material;
        let surface = material.surface();
        let (kind, emissive, roughness, metalness, flat) = match material {
            Material::Basic(_) => (0, Color::BLACK, 1.0, 0.0, false),
            Material::Lambert(m) => (1, m.emissive, 1.0, 0.0, false),
            Material::Standard(m) => (2, m.emissive, m.roughness, m.metalness, m.flat_shading),
            Material::Normal(_) => (3, Color::BLACK, 1.0, 0.0, false),
        };
        let (emissive_map, normal_map, rm_map, normal_scale) = match material {
            Material::Standard(m) => (
                m.emissive_map.as_ref(),
                m.normal_map.as_ref(),
                m.roughness_metalness_map.as_ref(),
                m.normal_scale,
            ),
            _ => (None, None, None, Vector2::new(1.0, 1.0)),
        };
        let maps = [
            ("hasMap", "mapTransform", UNIT_MAP, material.map()),
            (
                "hasEmissiveMap",
                "emissiveMapTransform",
                UNIT_EMISSIVE,
                emissive_map,
            ),
            (
                "hasNormalMap",
                "normalMapTransform",
                UNIT_NORMAL,
                normal_map,
            ),
            (
                "hasRoughnessMetalnessMap",
                "roughnessMetalnessMapTransform",
                UNIT_ROUGHNESS_METALNESS,
                rm_map,
            ),
        ];
        for (has, transform, unit, texture) in maps {
            let there = self.bind_texture(unit, texture);
            let u = &self.u;
            self.gl.uniform1i(u.get(has), i32::from(there));
            if let (true, Some(t)) = (there, texture) {
                self.gl
                    .uniform_matrix3fv_with_f32_array(u.get(transform), false, &t.uv_matrix());
            }
        }

        let (gl, u) = (&self.gl, &self.u);
        gl.uniform1i(u.get("kind"), kind);
        set_color(gl, u.get("diffuse"), surface.color);
        gl.uniform1f(u.get("opacity"), surface.opacity.clamp(0.0, 1.0));
        gl.uniform1i(u.get("blend"), i32::from(surface.transparent));
        gl.uniform1f(u.get("alphaTest"), material.alpha_test());
        set_color(gl, u.get("emissive"), emissive);
        gl.uniform1f(u.get("roughness"), roughness);
        gl.uniform1f(u.get("metalness"), metalness);
        gl.uniform1i(u.get("flatShading"), i32::from(flat));
        gl.uniform2f(u.get("normalScale"), normal_scale.x, normal_scale.y);
        gl.uniform1i(u.get("receiveShadow"), i32::from(draw.receive_shadow));
        gl.uniform_matrix4fv_with_f32_array(u.get("modelMatrix"), false, &draw.world.elements);
        gl.uniform_matrix3fv_with_f32_array(
            u.get("normalMatrix"),
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
            let id = draw.geometry.id();
            let Some(g) = self.geometries.get_mut(&id) else {
                return;
            };
            if g.wire.is_none() {
                g.wire = Some(upload_edges(&self.gl, g, draw.geometry));
            }
            if let Some((vao, _, count)) = &g.wire {
                self.gl.bind_vertex_array(vao.as_ref());
                self.gl
                    .draw_elements_with_i32(Gl::LINES, *count, Gl::UNSIGNED_INT, 0);
            }
        } else {
            gl.bind_vertex_array(vao.as_ref());
            gl.draw_elements_with_i32(Gl::TRIANGLES, count, Gl::UNSIGNED_INT, 0);
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
        let gl = &self.gl;
        for (_, g) in self.geometries.drain() {
            delete(gl, g);
        }
        for (_, t) in self.textures.drain() {
            gl.delete_texture(t.texture.as_ref());
        }
        for (_, s) in self.samplers.drain() {
            gl.delete_sampler(s.as_ref());
        }
        for target in self.shadow_targets.drain(..) {
            gl.delete_texture(target.texture.as_ref());
            gl.delete_framebuffer(target.framebuffer.as_ref());
        }
        gl.delete_texture(self.white.take().as_ref());
        gl.delete_texture(self.no_shadow.take().as_ref());
        gl.delete_program(self.program.take().as_ref());
        gl.delete_program(self.shadow_program.take().as_ref());
    }
}

fn internal_format(srgb: bool) -> i32 {
    if srgb {
        Gl::SRGB8_ALPHA8 as i32
    } else {
        Gl::RGBA8 as i32
    }
}

/// The parameters of a depth texture read as a shadow: compared, filtered, clamped.
fn shadow_texture_parameters(gl: &Gl) {
    gl.tex_parameteri(
        Gl::TEXTURE_2D,
        Gl::TEXTURE_COMPARE_MODE,
        Gl::COMPARE_REF_TO_TEXTURE as i32,
    );
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_COMPARE_FUNC, Gl::LEQUAL as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MIN_FILTER, Gl::LINEAR as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_MAG_FILTER, Gl::LINEAR as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_S, Gl::CLAMP_TO_EDGE as i32);
    gl.tex_parameteri(Gl::TEXTURE_2D, Gl::TEXTURE_WRAP_T, Gl::CLAMP_TO_EDGE as i32);
}

fn make_sampler(
    gl: &Gl,
    (wrap_s, wrap_t, mag, min): (Wrapping, Wrapping, Filter, Filter),
    anisotropy: f32,
) -> Option<WebGlSampler> {
    let sampler = gl.create_sampler()?;
    let wrap = |w: Wrapping| match w {
        Wrapping::ClampToEdge => Gl::CLAMP_TO_EDGE,
        Wrapping::Repeat => Gl::REPEAT,
        Wrapping::MirroredRepeat => Gl::MIRRORED_REPEAT,
    } as i32;
    gl.sampler_parameteri(&sampler, Gl::TEXTURE_WRAP_S, wrap(wrap_s));
    gl.sampler_parameteri(&sampler, Gl::TEXTURE_WRAP_T, wrap(wrap_t));
    gl.sampler_parameteri(
        &sampler,
        Gl::TEXTURE_MAG_FILTER,
        match mag {
            Filter::Linear => Gl::LINEAR,
            Filter::Nearest => Gl::NEAREST,
        } as i32,
    );
    gl.sampler_parameteri(
        &sampler,
        Gl::TEXTURE_MIN_FILTER,
        match min {
            Filter::Linear => Gl::LINEAR_MIPMAP_LINEAR,
            Filter::Nearest => Gl::NEAREST_MIPMAP_NEAREST,
        } as i32,
    );
    if anisotropy > 1.0 && min == Filter::Linear {
        gl.sampler_parameterf(&sampler, TEXTURE_MAX_ANISOTROPY, anisotropy);
    }
    Some(sampler)
}

fn srgb(c: Color) -> [f32; 3] {
    [
        linear_to_srgb(c.r.clamp(0.0, 1.0)),
        linear_to_srgb(c.g.clamp(0.0, 1.0)),
        linear_to_srgb(c.b.clamp(0.0, 1.0)),
    ]
}

fn set_color(gl: &Gl, at: Option<&WebGlUniformLocation>, c: Color) {
    gl.uniform3f(at, c.r, c.g, c.b);
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

fn link(gl: &Gl, vertex: &str, fragment: &str) -> Result<WebGlProgram, String> {
    let vertex = compile(gl, Gl::VERTEX_SHADER, vertex)?;
    let fragment = compile(gl, Gl::FRAGMENT_SHADER, fragment)?;
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

fn array_buffer(gl: &Gl, location: u32, size: i32, data: &[f32]) -> Option<WebGlBuffer> {
    let buffer = gl.create_buffer();
    gl.bind_buffer(Gl::ARRAY_BUFFER, buffer.as_ref());
    gl.buffer_data_with_array_buffer_view(
        Gl::ARRAY_BUFFER,
        &js_sys::Float32Array::from(data),
        Gl::STATIC_DRAW,
    );
    gl.enable_vertex_attrib_array(location);
    gl.vertex_attrib_pointer_with_i32(location, size, Gl::FLOAT, false, 0, 0);
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
    let position = array_buffer(gl, 0, 3, &geometry.positions);
    let normal = array_buffer(gl, 1, 3, &geometry.normals);
    let uv = if geometry.uvs.len() / 2 == geometry.positions.len() / 3 {
        array_buffer(gl, 2, 2, &geometry.uvs)
    } else {
        array_buffer(gl, 2, 2, &vec![0.0; geometry.positions.len() / 3 * 2])
    };
    let indices = indices(geometry);
    let index = element_buffer(gl, &indices);
    gl.bind_vertex_array(None);
    Some(GpuGeometry {
        vao,
        buffers: vec![position, normal, uv, index],
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
    for t in indices(geometry).as_chunks::<3>().0 {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            if seen.insert((a.min(b), a.max(b))) {
                edges.extend([a, b]);
            }
        }
    }
    let vao = gl.create_vertex_array();
    gl.bind_vertex_array(vao.as_ref());
    for (location, size, buffer) in [
        (0, 3, &g.buffers[0]),
        (1, 3, &g.buffers[1]),
        (2, 2, &g.buffers[2]),
    ] {
        gl.bind_buffer(Gl::ARRAY_BUFFER, buffer.as_ref());
        gl.enable_vertex_attrib_array(location);
        gl.vertex_attrib_pointer_with_i32(location, size, Gl::FLOAT, false, 0, 0);
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
