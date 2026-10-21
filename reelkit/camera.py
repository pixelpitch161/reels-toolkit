"""Look-at perspective camera. World: y up; a camera with yaw 0 sits on +z looking toward -z."""
import math
import numpy as np
from .config import W, H

NEAR = 0.1


class Cam:
    def __init__(self, target, dist, yaw, pitch, fov, sy=None, roll=0.0, near=NEAR):
        """target: look-at point; dist: distance; yaw/pitch in degrees (pitch > 0 = above);
        fov: vertical field of view in degrees; sy: screen y of the optical centre (default H/2)."""
        yr, pr = math.radians(yaw), math.radians(pitch)
        tgt = np.asarray(target, float)
        self.pos = tgt + dist * np.array([math.sin(yr) * math.cos(pr), math.sin(pr), math.cos(yr) * math.cos(pr)])
        fwd = tgt - self.pos
        fwd /= np.linalg.norm(fwd)
        right = np.cross(fwd, [0, 1, 0])
        right /= np.linalg.norm(right)
        up = np.cross(right, fwd)
        if roll:
            c, s = math.cos(math.radians(roll)), math.sin(math.radians(roll))
            right, up = right * c + up * s, -right * s + up * c
        self.M = np.stack([right, up, fwd])
        self.f = (H / 2) / math.tan(math.radians(fov) / 2)
        self.cx, self.cy = W / 2, (H / 2 if sy is None else sy)
        self.near = near
        self.target = tgt

    @property
    def view(self):
        """unit vector from the scene toward the camera"""
        return -self.M[2]

    def to_cam(self, P):
        return (np.asarray(P, float) - self.pos) @ self.M.T

    def proj(self, Pc):
        z = np.maximum(Pc[..., 2], 1e-3)
        return np.stack([self.cx + self.f * Pc[..., 0] / z, self.cy - self.f * Pc[..., 1] / z], -1)

    def wp(self, P):
        """world -> screen"""
        return self.proj(self.to_cam(P))

    def depth(self, P):
        return self.to_cam(np.atleast_2d(P))[:, 2]

    def screen_of_dir(self, d, dist=1e5):
        """screen position of a direction at infinity (sun, moon), or None if behind"""
        Pc = self.M @ (np.asarray(d, float) * dist)
        if Pc[2] <= 0.1:
            return None
        return self.proj(Pc[None])[0]

    def pixel_rays(self, xs, ys):
        """world-space ray directions (camera-z component = 1) for pixel coords"""
        d = np.stack([(xs - self.cx) / self.f, -(ys - self.cy) / self.f, np.ones(np.shape(xs))], -1)
        return d @ self.M

    def ground_hit(self, xs, ys, y0=0.0):
        """intersect pixel rays with the plane y = y0 -> (X, Z, camera depth)"""
        dw = self.pixel_rays(xs, ys)
        t = (y0 - self.pos[1]) / np.minimum(dw[..., 1], -1e-6)
        return self.pos[0] + t * dw[..., 0], self.pos[2] + t * dw[..., 2], t


def clip_near(Pc, near=NEAR):
    """Sutherland-Hodgman clip of a camera-space polygon against z = near"""
    out = []
    n = len(Pc)
    for k in range(n):
        a, b = Pc[k], Pc[(k + 1) % n]
        ia, ib = a[2] >= near, b[2] >= near
        if ia:
            out.append(a)
        if ia != ib:
            u = (near - a[2]) / (b[2] - a[2])
            out.append(a + (b - a) * u)
    return np.array(out) if len(out) >= 3 else None


def interp_keys(keys, t, ease_fn=None):
    """camera keyframes: list of (t, target(3), dist, yaw, pitch, fov, sy). Returns a Cam."""
    from .config import ease
    ease_fn = ease_fn or ease
    if t <= keys[0][0]:
        k = keys[0]
    elif t >= keys[-1][0]:
        k = keys[-1]
    else:
        for a, b in zip(keys, keys[1:]):
            if t <= b[0]:
                u = ease_fn((t - a[0]) / (b[0] - a[0]))
                k = [t] + [np.asarray(a[i], float) + (np.asarray(b[i], float) - np.asarray(a[i], float)) * u for i in range(1, 7)]
                break
    _, tgt, d, yaw, pitch, fov, sy = k
    return Cam(tgt, float(d), float(yaw), float(pitch), float(fov), float(sy))
