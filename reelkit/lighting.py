"""Lights + flat shading.

Lights holds:
    amb      ambient RGB multiplier
    sky      extra light on upward faces (hemisphere)
    keys     [(name, dir_to_light, rgb)]  - main lights (sun / moon). Shadowed.
    fills    [(dir, rgb)]                 - soft directional fills, never shadowed
    points   [(pos, rgb, radius)]         - lanterns, candles, fireflies (falloff 1/(1+(d/R)^2))
    view     unit vector scene -> camera (for the rim term); set it each frame from cam.view

shade() returns colours quantised to steps of 5 (keeps the flat low-poly look)."""
import math
import numpy as np


class Lights:
    def __init__(self, amb=(0.3, 0.3, 0.4), sky=(0.1, 0.1, 0.15)):
        self.amb = np.array(amb, float)
        self.sky = np.array(sky, float)
        self.keys = []
        self.fills = []
        self.points = []
        self.view = np.array([0, 0, 1.0])

    def key(self, name, d, rgb):
        self.keys.append((name, np.asarray(d, float) / np.linalg.norm(d), np.array(rgb, float)))

    def fill(self, d, rgb):
        self.fills.append((np.asarray(d, float) / np.linalg.norm(d), np.array(rgb, float)))

    def point(self, pos, rgb, radius):
        self.points.append((np.asarray(pos, float), np.array(rgb, float), float(radius)))


def shade(L, nrm, ctr, base, kvis=None, rimk=0.55, trans=0.0, emit=0.0, step=5):
    """vectorised flat shading.
    nrm (N,3) unit normals, ctr (N,3) face centres, base (N,3) or (3,) albedo 0-255.
    kvis (N, len(L.keys)) visibility per key light (1 lit, 0 shadow) - from shadows.occ_vis / masks.
    rimk: strength of the silhouette rim from each key (backlight sparkle).
    trans: thin leaves/cloth glow when a key light shines through them toward the camera.
    emit: self-illumination (dials, screens)."""
    nrm = np.atleast_2d(np.asarray(nrm, float))
    ctr = np.atleast_2d(np.asarray(ctr, float))
    N = len(nrm)
    I = np.tile(L.amb, (N, 1)) + np.clip(nrm[:, 1:2], 0, None) * L.sky[None] + emit
    vw = L.view
    facing = np.abs(nrm @ vw)
    for ki, (name, d, c) in enumerate(L.keys):
        vis = 1.0 if kvis is None else kvis[:, ki]
        I += (np.clip(nrm @ d, 0, None) * vis)[:, None] * c[None]
        kp = d - np.dot(d, vw) * vw
        kp = kp / (np.linalg.norm(kp) + 1e-9)
        I += (rimk * (1 - facing) ** 2 * np.clip(nrm @ kp, 0, None) * vis)[:, None] * c[None]
        if trans:
            I += (trans * vis * max(0.0, -float(np.dot(vw, d))))[..., None] * c[None] * np.ones((N, 1))
    for d, c in L.fills:
        I += np.clip(nrm @ d, 0, None)[:, None] * c[None]
    for lp, lc, R in L.points:
        v = lp[None] - ctr
        dist = np.linalg.norm(v, axis=1) + 1e-6
        lam = 0.3 + 0.7 * np.clip((nrm * v).sum(1) / dist, 0, None)
        I += (lam / (1 + (dist / R) ** 2))[:, None] * lc[None]
    m = I.max(1, keepdims=True)                         # soft highlight roll-off
    I = np.where(m > 0.95, I * (0.95 + 0.6 * (1 - np.exp(-(m - 0.95) / 0.6))) / np.maximum(m, 1e-6), I)
    col = np.broadcast_to(np.asarray(base, float), (N, 3)) * I
    return np.clip(np.round(col / step) * step, 0, 255)


def fog(col, dist, fog_col, d0, d1, mx=0.85):
    """distance fog toward fog_col (N,3) or (3,)"""
    k = np.clip((np.asarray(dist) - d0) / (d1 - d0), 0, mx)[..., None]
    return np.asarray(col) * (1 - k) + np.asarray(fog_col) * k


def sky_gradient(cam, zenith, horizon, horizon2=None, glow_dir=None, glow_col=None, glow_k=0.0, step=7.0):
    """banded, dithered sky; optional glow around a direction (sun/moon)"""
    from .config import XX, YY, BAYER_FULL
    d = np.stack([(XX - cam.cx) / cam.f, -(YY - cam.cy) / cam.f, np.ones_like(XX)], -1) @ cam.M
    d /= np.linalg.norm(d, axis=-1, keepdims=True)
    el = np.arcsin(np.clip(d[..., 1], -1, 1))
    k = np.clip(el / 0.75, 0, 1) ** 0.6
    hor = np.array(horizon, float)
    if horizon2 is not None:
        k2 = np.clip(el / 0.25, 0, 1)
        hor = np.array(horizon, float)[None, None] * (1 - k2[..., None]) + np.array(horizon2, float)[None, None] * k2[..., None]
    col = hor * (1 - k[..., None]) + np.array(zenith, float)[None, None] * k[..., None]
    if glow_dir is not None and glow_k > 0:
        cosang = (d * np.asarray(glow_dir)[None, None]).sum(-1)
        g = np.clip((cosang - 0.8) / 0.2, 0, 1) ** 3
        col = col + g[..., None] * np.array(glow_col, float) * glow_k
    col = np.floor(col / step + BAYER_FULL[..., None]) * step
    return np.clip(col, 0, 255)
