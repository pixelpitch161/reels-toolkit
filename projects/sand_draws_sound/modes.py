"""Chladni mode shapes for a free square plate (classic cosine approximation).
Plate coordinates x, y in [-1, 1]."""
import numpy as np

PI = np.pi


def mode_val_grad(P, n, m, s, w=1.0):
    """A = cos(n pi u) cos(m pi v) + s*w*cos(m pi u) cos(n pi v),  u=(x+1)/2."""
    u = (P[:, 0] + 1) * 0.5
    v = (P[:, 1] + 1) * 0.5
    cnu, snu = np.cos(n * PI * u), np.sin(n * PI * u)
    cmu, smu = np.cos(m * PI * u), np.sin(m * PI * u)
    cnv, snv = np.cos(n * PI * v), np.sin(n * PI * v)
    cmv, smv = np.cos(m * PI * v), np.sin(m * PI * v)
    a = cnu * cmv + s * w * cmu * cnv
    dadu = -n * PI * snu * cmv - s * w * m * PI * smu * cnv
    dadv = -m * PI * cnu * smv - s * w * n * PI * cmu * snv
    return a / (1 + w), np.stack([dadu, dadv], 1) * 0.5 / (1 + w)


def field(n, m, s, w=1.0, res=200):
    xs = np.linspace(-1, 1, res)
    X, Y = np.meshgrid(xs, xs)
    P = np.stack([X.ravel(), Y.ravel()], 1)
    a, _ = mode_val_grad(P, n, m, s, w)
    return a.reshape(res, res)


def nodal_image(n, m, s, w=1.0, res=200, thr=0.035):
    a = field(n, m, s, w, res)
    return np.abs(a) / np.abs(a).max() < thr
