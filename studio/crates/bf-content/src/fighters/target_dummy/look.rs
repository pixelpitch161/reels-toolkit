//! TARGET DUMMY's look (a developer fighter, drawn the Engine 1 way). The ball is its model (a straw training dummy
//! on a post with a bullseye on its chest); it has no weapon to draw. Its VS-card panel is rows of small bullseyes,
//! and it shatters into burlap, straw and wooden post pieces.

use bf_core::dmath;
use bf_core::roster::FighterCfg;
use bf_picture::camera::{Cam, V3};
use bf_picture::img::Img;
use bf_picture::look::Look;
use bf_picture::np;
use bf_picture::ring::{self, PictureCtx};
use bf_picture::scene::to_world;

/// The sack's burlap.
const BURLAP: V3 = [204.0, 170.0, 116.0];
/// A wisp of straw.
const STRAW: V3 = [236.0, 210.0, 120.0];
/// The post's wood.
const WOOD: V3 = [150.0, 102.0, 58.0];

/// TARGET DUMMY's look.
#[derive(Clone, Debug, Default)]
pub struct TargetDummyLook {
    /// Its side.
    k: usize,
}

impl Look for TargetDummyLook {
    /// Its own panel on the VS card ([`crate::fighters::shared::panels::target_dummy`]).
    fn title_pattern(&self, base: &mut bf_picture::img::RgbF) {
        crate::fighters::shared::panels::target_dummy(base);
    }

    /// Its shatter parts: burlap patches, wisps of straw and splinters of the post.
    fn shards(&self) -> Vec<String> {
        [("burlap", 6), ("straw", 6), ("wood", 2)]
            .iter()
            .flat_map(|&(kind, n)| std::iter::repeat_n(kind.to_string(), n))
            .collect()
    }

    fn setup(&mut self, p: &PictureCtx, k: usize) {
        self.setup_as(p, k, &p.cfg[k]);
    }

    fn setup_as(&mut self, _p: &PictureCtx, k: usize, _cfg: &FighterCfg) {
        self.k = k;
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_shard(
        &self,
        _p: &PictureCtx,
        arr: &mut Img<f32>,
        cam: &Cam,
        kind: &str,
        pos: V3,
        rot: f64,
        size: f64,
        _curl: f64,
    ) {
        let (sx, sz) = (pos[0], pos[2]);
        let d = [dmath::cos(rot), dmath::sin(rot)];
        let nn = [-d[1], d[0]];
        let at = |q: [f64; 2]| to_world(q[0], q[1], pos[1]);
        let (ln, wd, col) = match kind {
            "burlap" => (0.10, 0.08, BURLAP),
            "straw" => (0.16, 0.012, STRAW),
            _ => (0.14, 0.03, WOOD),
        };
        let q4: Vec<V3> = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
            .iter()
            .map(|&(a1, a2)| {
                at([
                    sx + d[0] * ln * a1 * size / 0.1 + nn[0] * wd * a2,
                    sz + d[1] * ln * a1 * size / 0.1 + nn[1] * wd * a2,
                ])
            })
            .collect();
        let sp = ring::screen(cam, &q4);
        let shade_k = 0.75 + 0.25 * dmath::cos(rot * 2.0);
        ring::layer_blend(arr, &[sp], col.map(|c| np::clip(c * shade_k, 0.0, 255.0)), 1.0, None);
    }
}
