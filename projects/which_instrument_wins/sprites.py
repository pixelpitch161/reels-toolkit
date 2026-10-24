"""Pixel-art sprites of the 16 Pixel Pitch instrument models, rendered with the series' own rasteriser (flat banded
shading + ink outline, from piano_lowpoly) at the size they appear inside the glass balls.

    from sprites import sprite
    rgba = sprite('TUBA', radius_px=11, yaw_deg=30)      # (h, w, 4) uint8, centred

Sprites are cached per (radius, yaw step) in memory and in sprites_r<radius>.npz (git-ignored).
"""
import math
import os
import numpy as np
import pp_bridge as B

YAW_STEPS = 48
ELEV = 16.0                       # degrees the camera looks down on the model
_MEM = {}
_FACES = {}
HERE = os.path.dirname(os.path.abspath(__file__))


class _Cam:
    """duck-typed camera for piano_lowpoly.render_geo: a narrow perspective looking at the origin"""
    def __init__(self, yaw_deg, radius_px, cx, cy, roll_deg=0.0):
        self.scale = 1.0
        a = math.radians(yaw_deg)
        c, s = math.cos(a), math.sin(a)
        Ry = np.array([[c, 0, s], [0, 1, 0], [-s, 0, c]])
        b = math.radians(roll_deg)
        cb, sb = math.cos(b), math.sin(b)
        Rz = np.array([[cb, -sb, 0], [sb, cb, 0], [0, 0, 1]])
        self.R = Rz @ Ry
        dist = 9.0
        e = math.radians(ELEV)
        self.eye = np.array([0.0, math.sin(e) * dist, math.cos(e) * dist])
        f = -self.eye / dist
        r = np.cross(f, [0, 1, 0])
        r /= np.linalg.norm(r)
        self.fwd, self.right, self.up = f, r, np.cross(r, f)
        self.f = radius_px * dist
        self.cx, self.cy = cx, cy

    def world(self, P, scale=None):
        return np.asarray(P, float) @ self.R.T

    def proj(self, W):
        Q = W - self.eye
        d = Q @ self.fwd
        return np.stack([self.cx + self.f * (Q @ self.right) / d, self.cy - self.f * (Q @ self.up) / d], -1), d

    def screen(self, p, scale=None):
        S, d = self.proj(self.world(np.array([p])))
        return S[0]


def _geo(name):
    import piano_lowpoly as P
    if name not in _FACES:
        g = P.Geo()
        for Q, n, col, two, emit, gid, spec in B.model_faces(name):
            g.F.append((Q, n, tuple(int(c) for c in col), two, emit, 2 if gid != 1 else 1, spec))
        _FACES[name] = g
    return _FACES[name]


def render_sprite(name, radius_px, yaw_deg, roll_deg=0.0):
    import piano_lowpoly as P
    RW, RH = P.RW, P.RH
    cx = cy = radius_px + 3
    img = np.zeros((RH, RW, 3), np.uint8)
    zb = np.zeros((RH, RW))
    gb = np.zeros((RH, RW), np.int8)
    cam = _Cam(yaw_deg, radius_px, cx + 0.5, cy + 0.5, roll_deg)
    P.render_geo(_geo(name), cam, img, zb, gb, scale=1.0)
    P.outline(img, zb, gb)
    n = 2 * radius_px + 7
    rgb = img[:n, :n]
    a = (gb[:n, :n] > 0).astype(np.uint8) * 255
    return np.dstack([rgb, a])


def _cache_path(radius_px):
    return os.path.join(HERE, f'sprites_r{radius_px}.npz')


def atlas(radius_px):
    """all sprites at this radius: {name: (YAW_STEPS, n, n, 4)}"""
    if radius_px in _MEM:
        return _MEM[radius_px]
    path = _cache_path(radius_px)
    if os.path.exists(path):
        z = np.load(path)
        _MEM[radius_px] = {k: z[k] for k in z.files}
        return _MEM[radius_px]
    out = {}
    for name in B.NAMES:
        out[name] = np.stack([render_sprite(name, radius_px, k * 360.0 / YAW_STEPS) for k in range(YAW_STEPS)])
    try:
        tmp = path + f'.{os.getpid()}.tmp.npz'
        np.savez_compressed(tmp, **out)
        os.replace(tmp, path)
    except OSError:
        pass
    _MEM[radius_px] = out
    return out


def sprite(name, radius_px, yaw_deg):
    a = atlas(int(radius_px))[name]
    k = int(round((yaw_deg % 360.0) / 360.0 * YAW_STEPS)) % YAW_STEPS
    return a[k]


if __name__ == '__main__':
    import sys
    from PIL import Image
    r = int(sys.argv[1]) if len(sys.argv) > 1 else 11
    yaws = [0, 30, 60, 90, 135, 180]
    n = 2 * r + 7
    sheet = np.full((len(B.NAMES) * (n + 2), len(yaws) * (n + 2), 3), (40, 30, 50), np.uint8)
    for i, nm in enumerate(B.NAMES):
        for j, y in enumerate(yaws):
            s = render_sprite(nm, r, y)
            reg = sheet[i * (n + 2):i * (n + 2) + n, j * (n + 2):j * (n + 2) + n]
            m = s[..., 3] > 0
            reg[m] = s[..., :3][m]
    Image.fromarray(sheet).resize((sheet.shape[1] * 4, sheet.shape[0] * 4), Image.NEAREST).save(f'check/sprites_r{r}.png')
    print('check/sprites_r%d.png' % r)


def yaw_sheet(r=12, out='check/yaws.png', roll=None):
    """every instrument at 12 yaws (0, 30, ... 330) to pick each one's most readable view"""
    from PIL import Image
    yaws = list(range(0, 360, 30))
    n = 2 * r + 7
    sheet = np.full((len(B.NAMES) * (n + 2), len(yaws) * (n + 2), 3), (200, 220, 230), np.uint8)
    for i, nm in enumerate(B.NAMES):
        for j, y in enumerate(yaws):
            s = render_sprite(nm, r, y, (roll or {}).get(nm, 0.0))
            reg = sheet[i * (n + 2):i * (n + 2) + n, j * (n + 2):j * (n + 2) + n]
            m = s[..., 3] > 0
            reg[m] = s[..., :3][m]
    Image.fromarray(sheet).resize((sheet.shape[1] * 3, sheet.shape[0] * 3), Image.NEAREST).save(out)
    return out
