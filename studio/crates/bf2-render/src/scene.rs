//! A frame as plain CPU data: the camera, the lights and the instances. Content builds it; the renderer draws it.

use crate::maths::{self, M4, V3};
use crate::mesh::MeshId;
use crate::texture::TexId;

/// The most point lights a frame carries to the GPU.
pub const MAX_LIGHTS: usize = 64;

/// A sim point (x, y on the floor, z height) in render space (Y up): (x, z, −y). Used for every sim position.
pub fn to_render(v: [f64; 3]) -> [f32; 3] {
    [v[0] as f32, v[2] as f32, -v[1] as f32]
}

/// How an instance is shaded.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Material {
    /// Banded diffuse, a stepped specular from the vertex `spec`, and a rim light. `bands: 0` shades smoothly
    /// (the set: canvas, floor, stands).
    Toon { bands: u32, rim: [f32; 3], rim_power: f32 },
    /// Transparent: tinted, mostly see-through face on, more opaque and reflective at grazing angles.
    Glass { tint: [f32; 3], opacity: f32, fresnel: f32 },
    /// The colour times `strength`, unlit (HDR: above 1 glows once bloom exists).
    Emissive { strength: f32 },
    /// The colour as it is.
    Unlit,
}

impl Material {
    /// Toon with four bands and a soft white rim.
    pub fn toon() -> Material {
        Material::Toon {
            bands: 4,
            rim: [0.35, 0.35, 0.35],
            rim_power: 4.0,
        }
    }

    /// Smooth shading with no rim: the set (canvas, floor, stands), which should not read as cel bands.
    pub fn matte() -> Material {
        Material::Toon {
            bands: 0,
            rim: [0.0; 3],
            rim_power: 1.0,
        }
    }

    /// Glass of the given tint, with the default opacity (0.18) and fresnel (0.9).
    pub fn glass(tint: [f32; 3]) -> Material {
        Material::Glass {
            tint,
            opacity: 0.18,
            fresnel: 0.9,
        }
    }

    pub fn is_transparent(&self) -> bool {
        matches!(self, Material::Glass { .. })
    }
}

/// A material's optional maps and glass extras (E7). The default has none: the instance draws as it always has.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    /// An albedo map (sRGB) multiplying the vertex and instance colour: Toon only.
    pub albedo: Option<TexId>,
    /// A tangent-space normal map, lit by the spot and the point lights: Toon and Glass.
    pub normal: Option<TexId>,
    /// The maps repeat this many times across the mesh's uvs.
    pub uv_scale: f32,
    /// How far the normal map tilts the normal (0: flat, 1: as the map says).
    pub normal_strength: f32,
    /// Glass: the far side's own tint (linear) and opacity, shaded in four bands from the spot as if it faced the
    /// camera (the current engine's ball fill, behind what the ball holds). `None`: the near side's tint at half
    /// its opacity.
    pub back: Option<([f32; 3], f32)>,
}

impl Default for Surface {
    fn default() -> Self {
        Surface {
            albedo: None,
            normal: None,
            uv_scale: 1.0,
            normal_strength: 1.0,
            back: None,
        }
    }
}

/// One mesh drawn once.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Instance {
    pub mesh: MeshId,
    pub material: Material,
    /// Model to render space.
    pub model: M4,
    /// Multiplies the vertex colour; `a` is the alpha.
    pub colour: [f32; 4],
    /// Maps and extras; the default has none.
    pub surface: Surface,
}

impl Instance {
    pub fn new(mesh: MeshId, material: Material, model: M4) -> Instance {
        Instance {
            mesh,
            material,
            model,
            colour: [1.0; 4],
            surface: Surface::default(),
        }
    }

    pub fn with_surface(mut self, surface: Surface) -> Instance {
        self.surface = surface;
        self
    }

    pub fn coloured(mut self, rgb: [f32; 3]) -> Instance {
        self.colour = [rgb[0], rgb[1], rgb[2], self.colour[3]];
        self
    }
}

/// A point light.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    pub pos: V3,
    /// Linear HDR.
    pub colour: [f32; 3],
    /// Where its light ends.
    pub range: f32,
    /// Lights of a higher priority are kept first when a frame has more than [`MAX_LIGHTS`]: the stage's are 0,
    /// effect lights 1.
    pub priority: u8,
}

impl Light {
    /// Its brightness for ranking: the colour's luminance.
    pub fn luminance(&self) -> f32 {
        0.2126 * self.colour[0] + 0.7152 * self.colour[1] + 0.0722 * self.colour[2]
    }
}

/// The spotlight, which may cast the frame's shadow map.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spot {
    pub pos: V3,
    pub target: V3,
    /// Linear HDR.
    pub colour: [f32; 3],
    /// The cone's half angle.
    pub angle_deg: f32,
    pub shadow: bool,
}

/// The camera.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub eye: V3,
    pub target: V3,
    /// Radians about the view direction.
    pub roll: f32,
    /// The vertical field of view.
    pub fov_deg: f32,
}

/// The near and far planes of every camera.
pub const NEAR: f32 = 0.5;
pub const FAR: f32 = 400.0;

impl Camera {
    /// Today's ring framing: the ring centre from 27.5 away, 58 degrees down, a 36 degree field of view.
    pub fn default_ring() -> Camera {
        let (dist, pitch) = (27.5f32, 58f32.to_radians());
        let target = [0.0, 0.0, 0.0];
        Camera {
            eye: [0.0, dist * pitch.sin(), dist * pitch.cos()],
            target,
            roll: 0.0,
            fov_deg: 36.0,
        }
    }

    pub fn view(&self) -> M4 {
        maths::look_at(self.eye, self.target, [0.0, 1.0, 0.0], self.roll)
    }

    pub fn proj(&self, aspect: f32) -> M4 {
        maths::perspective(self.fov_deg.to_radians(), aspect, NEAR, FAR)
    }

    pub fn view_proj(&self, aspect: f32) -> M4 {
        maths::mul(&self.proj(aspect), &self.view())
    }

    /// Where `p` lands on a `w x h` picture, in pixels from the top left; `None` behind the camera.
    pub fn project(&self, p: V3, w: u32, h: u32) -> Option<[f32; 2]> {
        let c = maths::transform_point(&self.view_proj(w as f32 / h as f32), p);
        if c[3] <= 0.0 {
            return None;
        }
        let (x, y) = (c[0] / c[3], c[1] / c[3]);
        Some([(x + 1.0) / 2.0 * w as f32, (1.0 - y) / 2.0 * h as f32])
    }
}

impl From<bf2_direct::CamPose> for Camera {
    /// The director's pose (render space, `f64`) as the renderer's camera.
    fn from(c: bf2_direct::CamPose) -> Camera {
        let v = |p: [f64; 3]| p.map(|x| x as f32);
        Camera {
            eye: v(c.eye),
            target: v(c.target),
            roll: c.roll as f32,
            fov_deg: c.fov_deg as f32,
        }
    }
}

impl Camera {
    /// The camera slid sideways so the picture moves by `(dx, dy)` pixels of an `h` pixel high frame at the
    /// target's distance (x right, y down): the director's shake.
    pub fn shifted_px(&self, dx: f64, dy: f64, h: f64) -> Camera {
        let view = self.view();
        let row = |r: usize| [view[0][r], view[1][r], view[2][r]];
        let dist = maths::length(maths::sub(self.target, self.eye));
        let per_px = 2.0 * dist * (self.fov_deg.to_radians() / 2.0).tan() / h as f32;
        // The picture moves right when the camera moves left, and down when it moves up.
        let off = maths::add(
            maths::scale(row(0), -(dx as f32) * per_px),
            maths::scale(row(1), dy as f32 * per_px),
        );
        Camera {
            eye: maths::add(self.eye, off),
            target: maths::add(self.target, off),
            ..*self
        }
    }
}

/// One frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub camera: Camera,
    /// Linear HDR light from everywhere.
    pub ambient: [f32; 3],
    pub spot: Option<Spot>,
    /// At most [`MAX_LIGHTS`] reach the GPU; the dimmest of any extra are dropped.
    pub lights: Vec<Light>,
    pub instances: Vec<Instance>,
}

impl Default for Scene {
    fn default() -> Self {
        Scene {
            camera: Camera::default_ring(),
            ambient: [0.0; 3],
            spot: None,
            lights: Vec::new(),
            instances: Vec::new(),
        }
    }
}

impl Scene {
    /// The lights the GPU gets: all of them if there are at most [`MAX_LIGHTS`], otherwise the [`MAX_LIGHTS`]
    /// of highest priority, then brightest (by luminance, ties to the earlier), in the order they were added.
    pub fn lights_for_gpu(&self) -> Vec<Light> {
        if self.lights.len() <= MAX_LIGHTS {
            return self.lights.clone();
        }
        let mut order: Vec<usize> = (0..self.lights.len()).collect();
        order.sort_by(|&a, &b| {
            let (la, lb) = (&self.lights[a], &self.lights[b]);
            lb.priority
                .cmp(&la.priority)
                .then(lb.luminance().total_cmp(&la.luminance()))
                .then(a.cmp(&b))
        });
        order.truncate(MAX_LIGHTS);
        order.sort_unstable();
        order.into_iter().map(|i| self.lights[i]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifted_px_moves_the_picture_by_pixels() {
        let c = Camera::default_ring();
        let p = [0.0, 0.0, 0.0];
        let a = c.project(p, 1080, 1920).unwrap();
        let b = c.shifted_px(12.0, -7.0, 1920.0).project(p, 1080, 1920).unwrap();
        assert!(
            (b[0] - a[0] - 12.0).abs() < 0.05 && (b[1] - a[1] + 7.0).abs() < 0.05,
            "{a:?} -> {b:?}"
        );
        let pose = bf2_direct::CamPose {
            eye: [1.0, 2.0, 3.0],
            target: [0.0, 0.5, 0.0],
            roll: 0.1,
            fov_deg: 30.0,
        };
        let cam = Camera::from(pose);
        assert_eq!(
            (cam.eye, cam.target, cam.roll, cam.fov_deg),
            ([1.0, 2.0, 3.0], [0.0, 0.5, 0.0], 0.1, 30.0)
        );
    }

    #[test]
    fn to_render_swaps_axes() {
        assert_eq!(to_render([1.0, 2.0, 3.0]), [1.0, 3.0, -2.0]);
    }

    #[test]
    fn lights_over_64_drop_dimmest() {
        // 70 lights, brightness shuffled: light i has brightness (i * 37) % 70.
        let lights: Vec<Light> = (0..70)
            .map(|i| Light {
                pos: [i as f32, 0.0, 0.0],
                colour: [((i * 37) % 70) as f32; 3],
                range: 5.0,
                priority: 0,
            })
            .collect();
        let s = Scene {
            lights: lights.clone(),
            ..Scene::default()
        };
        let kept = s.lights_for_gpu();
        assert_eq!(kept.len(), MAX_LIGHTS);
        // The six dimmest (brightness 0..=5) are gone; the rest keep their order.
        assert!(kept.iter().all(|l| l.colour[0] >= 6.0));
        assert!(kept.windows(2).all(|w| w[0].pos[0] < w[1].pos[0]));
        let few = Scene {
            lights: lights[..10].to_vec(),
            ..Scene::default()
        };
        assert_eq!(few.lights_for_gpu(), lights[..10].to_vec());
    }

    #[test]
    fn default_camera_looks_at_the_ring_centre() {
        let c = Camera::default_ring();
        assert!((maths::length(c.eye) - 27.5).abs() < 1e-4);
        let p = c.project([0.0; 3], 1080, 1920).unwrap();
        assert!((p[0] - 540.0).abs() < 0.01 && (p[1] - 960.0).abs() < 0.01, "{p:?}");
        // Sim +y (away from the camera) is up the picture.
        let far = c.project(to_render([0.0, 3.0, 0.0]), 1080, 1920).unwrap();
        assert!(far[1] < 960.0);
    }
}
