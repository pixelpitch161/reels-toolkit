"""Shadows, two complementary tools:

1. Planar shadow masks (receivers on a ground plane y = 0): project caster faces along a
   directional light (sun/moon) or from a point light (fire, lantern) onto the ground and
   rasterise them into a world-space XZ mask. Sample it per pixel (floor lighting) or per face.
2. Ray vs bounding-box occlusion (receivers anywhere: walls, props, sand on a plate):
   occ_vis(P, groups, d=... | lp=...) tests the ray from P toward the light against every
   caster's AABB, ignoring the receiver's own group (no self-shadow acne).

Typical floor pass: draw floor albedo into its own image + mask, compute per-pixel world XZ
with cam.ground_hit, then I = ambient + key*(1-mask) + fire*(1-fire_mask) ... (see the template)."""
import numpy as np
from PIL import Image, ImageDraw, ImageFilter


class MaskGrid:
    """world-space XZ raster: ext = (x0, x1, z0, z1, units_per_px)"""
    def __init__(self, ext):
        self.ext = ext
        x0, x1, z0, z1, res = ext
        self.W, self.H = int((x1 - x0) / res), int((z1 - z0) / res)

    def raster(self, polys_xz, blur=0.0):
        x0, x1, z0, z1, res = self.ext
        im = Image.new('L', (self.W, self.H), 0)
        dr = ImageDraw.Draw(im)
        for q in polys_xz:
            dr.polygon([((p[0] - x0) / res, (p[1] - z0) / res) for p in q], fill=255)
        if blur:
            im = im.filter(ImageFilter.GaussianBlur(blur))
        return np.asarray(im, np.float32) / 255.0

    def sample(self, mask, X, Z):
        x0, x1, z0, z1, res = self.ext
        i = np.floor((np.asarray(Z) - z0) / res).astype(np.int64)
        j = np.floor((np.asarray(X) - x0) / res).astype(np.int64)
        ok = (i >= 0) & (i < mask.shape[0]) & (j >= 0) & (j < mask.shape[1])
        out = np.zeros(np.shape(X), np.float32)
        out[ok] = mask[i[ok], j[ok]]
        return out


def proj_dir(Q, d):
    """project points along a directional light onto y = 0 -> XZ"""
    return (Q - (Q[:, 1:2] / d[1]) * d[None])[:, [0, 2]]


def proj_point(Q, lp, max_scale=0.93):
    """project from a point light onto y = 0 (points near/above the light are clamped far away)"""
    y = np.minimum(Q[:, 1], lp[1] * max_scale)
    sc = lp[1] / (lp[1] - y)
    return (lp[None] + (Q - lp[None]) * sc[:, None])[:, [0, 2]]


def directional_mask(grid, faces, d, blur=2.0, min_y=0.02):
    return grid.raster([proj_dir(Q, d) for Q in faces if Q[:, 1].max() > min_y], blur)


def point_mask(grid, faces, lp, blur=3.0, min_y=0.02):
    return grid.raster([proj_point(Q, lp) for Q in faces if Q[:, 1].max() > min_y], blur)


def ground_point(P, d):
    """where the light ray through P hits the ground (to test a raised point against a ground mask)"""
    P = np.atleast_2d(P)
    return P - (P[:, 1:2] / d[1]) * d[None]


class Occluders:
    """AABBs of caster objects (reelkit.geometry.Obj with .lo .hi .grp .cast)"""
    def __init__(self, objs):
        objs = [o for o in objs if getattr(o, 'cast', True)]
        self.lo = np.array([o.lo for o in objs])
        self.hi = np.array([o.hi for o in objs])
        self.grp = np.array([o.grp for o in objs])

    def vis(self, P, pg, d=None, lp=None, exclude=()):
        """1 where the ray from P toward the light (direction d, or point lp) is unblocked"""
        P = np.atleast_2d(np.asarray(P, float))
        N = len(P)
        if d is not None:
            D = np.broadcast_to(d, (N, 3))
            tmax = np.full(N, 1e9)
        else:
            V = np.asarray(lp, float)[None] - P
            tmax = np.linalg.norm(V, axis=1) + 1e-9
            D = V / tmax[:, None]
            tmax = tmax * 0.97
        inv = 1.0 / np.where(np.abs(D) < 1e-9, 1e-9, D)
        t1 = (self.lo[None] - P[:, None]) * inv[:, None]
        t2 = (self.hi[None] - P[:, None]) * inv[:, None]
        tn = np.minimum(t1, t2).max(-1)
        tf = np.maximum(t1, t2).min(-1)
        pg = np.broadcast_to(np.asarray(pg), (N,))
        hit = (tf >= np.maximum(tn, 1e-3)) & (tn < tmax[:, None]) & (self.grp[None] != pg[:, None])
        for g in exclude:
            hit &= self.grp[None] != g
        return (~hit.any(1)).astype(np.float32)


def contact_ao(X, Z, feet, falloff=0.32):
    """soft contact darkening around object footprints: feet = [(x, z, radius, strength)]"""
    ao = np.ones(np.shape(X), np.float32)
    for (cx, cz, r0, k) in feet:
        dd = np.sqrt((X - cx) ** 2 + (Z - cz) ** 2) - r0
        ao *= 1 - k * np.exp(-np.maximum(dd, 0) / falloff)
    return ao
