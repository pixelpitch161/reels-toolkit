"""Reusable backdrops (so sessions don't rebuild them). Each is loop-safe: nothing drifts, and anything that
moves completes whole cycles in the video (see config.whole_period).

    from reelkit.backdrops import VibrantVoid
    VOID = VibrantVoid(moon_dir=(0.3, 0.45, -0.85))     # build once at import
    img = VOID.sky(cam)                                   # (H, W, 3) float background, dithered
    VOID.stars(img, cam, t)                               # twinkling stars on top (in place)

VibrantVoid: the "vibrant void of space" look (user request for #18; also used by #20): an indigo -> magenta ->
pink elevation gradient, two layers of static noise nebula, optional moon glow, 700 twinkling stars. Lifted
from projects/battle_of_the_bands/render.py; pass your own palette / seed for a different sky."""
import math
import numpy as np
from PIL import Image
from .config import W, H, XX, YY, BAYER_FULL, DUR


def _noise(h, w, octaves, rng):
    out = np.zeros((h, w))
    for k, amp in octaves:
        g = rng.normal(0, 1, (k, 2 * k))
        g = np.concatenate([g, g[:, :1]], 1)                    # wraps around in azimuth
        im = Image.fromarray(((g - g.min()) / (np.ptp(g) + 1e-9) * 255).astype(np.uint8)).resize(
            (w + w // (2 * k), h), Image.BICUBIC)
        out += (np.asarray(im, float)[:, :w] / 255 - 0.5) * amp
    return out


class VibrantVoid:
    def __init__(self, seed=31, moon_dir=None, top=(22, 18, 70), mid=(92, 34, 132), low=(196, 70, 150),
                 neb1=(60, 30, 90), neb2=(20, 70, 110), n_stars=700, twinkles=19):
        rng = np.random.default_rng(seed)
        self.top, self.mid, self.low = (np.array(c, float) for c in (top, mid, low))
        self.neb1, self.neb2 = np.array(neb1, float), np.array(neb2, float)
        self.NEB = np.clip(_noise(120, 240, [(4, 1.0), (9, 0.6), (20, 0.3)], rng) * 1.8 + 0.05, 0, 1)
        self.NEB2 = np.clip(_noise(120, 240, [(5, 1.0), (12, 0.5)], rng) * 1.8 - 0.1, 0, 1)
        st = rng.normal(0, 1, (n_stars, 3))
        st /= np.linalg.norm(st, axis=1, keepdims=True)
        st[:, 2] = -np.abs(st[:, 2])
        self.STARS = st
        self.STAR_B = rng.uniform(0.25, 1.0, n_stars) ** 2
        self.STAR_PH = rng.uniform(0, 2 * math.pi, n_stars)
        self.moon = None if moon_dir is None else np.asarray(moon_dir, float) / np.linalg.norm(moon_dir)
        self.twinkles = int(twinkles)                           # whole twinkle cycles per video: loop-safe

    def sky(self, cam):
        d = np.stack([(XX - cam.cx) / cam.f, -(YY - cam.cy) / cam.f, np.ones_like(XX)], -1) @ cam.M
        d /= np.linalg.norm(d, axis=-1, keepdims=True)
        el = np.arcsin(np.clip(d[..., 1], -1, 1))
        az = np.arctan2(d[..., 0], -d[..., 2])
        k = np.clip((el + 0.05) / 0.55, 0, 1)[..., None]
        col = np.where(k < 0.5, self.low * (1 - 2 * k) + self.mid * 2 * k, self.mid * (2 - 2 * k) + self.top * (2 * k - 1))
        u = ((az / (2 * math.pi) + 0.5) * 240).astype(int) % 240          # static nebula: seamless loop
        v = np.clip(((0.6 - el) / 1.2 * 120).astype(int), 0, 119)
        col = col + self.NEB[v, u][..., None] * self.neb1 * 0.8 + self.NEB2[v, (u + 60) % 240][..., None] * self.neb2 * 0.6
        if self.moon is not None:
            cg = (d * self.moon[None, None]).sum(-1)
            col += (np.clip((cg - 0.9) / 0.1, 0, 1) ** 3)[..., None] * np.array([120, 120, 170.0]) * 0.6
        col = np.floor(col / 7 + BAYER_FULL[..., None]) * 7
        return np.clip(col, 0, 255)

    def stars(self, img, cam, t, dur=DUR):
        Pc = (self.STARS * 1e4) @ cam.M.T
        ok = Pc[:, 2] > 1
        s = cam.proj(Pc[ok])
        b = self.STAR_B[ok] * (0.75 + 0.25 * np.sin(2 * math.pi * self.twinkles * t / dur + self.STAR_PH[ok]))
        for (x, y), bb in zip(s, b):
            xi, yi = int(x), int(y)
            if 0 <= xi < W and 0 <= yi < H:
                img[yi, xi] = np.maximum(img[yi, xi], 120 + 135 * bb)
        return img
