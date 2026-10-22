"""The Galton board itself: peg/bin layout, ball motion and colours. Shared by sim, render and audio.

Coordinates (world units ~ metres): the board stands upright facing +z (the audience), the
backplate is the plane z = ZB, balls roll in the plane z = ZBALL just in front of it.

12 rows of pegs. Row k (k = 0..11) has k + 1 pegs. A peg's column c is counted in half peg
spacings from the centre line: x = c * DX / 2, with c = -k, -k + 2, ..., k. Every bounce moves
a ball one column left or right (c -> c - 1 or c + 1), so after 12 bounces it lands in bin
b = (c + 12) / 2, b = 0..12. Paths into bin b: C(12, b) out of 2^12 = 4,096."""
import math
import numpy as np

ROWS = 12
NB = ROWS + 1
DX = 0.1                    # peg spacing in a row (= bin width)
DY = 0.09                   # row spacing
R_BALL = 0.0112
R_PEG = 0.0075
PEG_LEN = 0.034             # pegs stick out of the backplate this far
ZB = -0.6                   # backplate plane
ZBALL = ZB + 0.021          # ball centres
ZGLASS = ZB + 0.05          # glass front
DIV_T = 0.008               # bin divider thickness

Y_STAGE = 0.4               # top of the lecture-hall stage the machine stands on
Y_BF = 0.75                 # bin floor (trapdoors)
BIN_H = 1.45
Y_BT = Y_BF + BIN_H         # 2.20: top of the bin dividers
Y_ROW = np.array([Y_BT + 0.07 + (ROWS - 1 - k) * DY for k in range(ROWS)])     # peg centre heights, row 0 on top
BH = 0.38 * DY              # bounce height parameter of every hop parabola
Y_C = Y_ROW + R_PEG + R_BALL                                                    # ball centre at a contact
Y_OUT = Y_C[0] + 4 * BH     # hopper outlet: a ball dropped here reaches peg 0 after exactly one hop time
X0, X1 = -0.72, 0.72        # backplate extent (inside the walnut posts)
Y_TOP = 3.47                # top of the backplate (the hopper sits above)

PEGS = [(k, c) for k in range(ROWS) for c in range(-k, k + 1, 2)]               # 78 pegs
PEG_ID = {p: i for i, p in enumerate(PEGS)}
PEG_XY = np.array([[c * DX / 2, Y_ROW[k]] for k, c in PEGS])


def peg_xy(k, c):
    return np.array([c * DX / 2, Y_ROW[k]])


def bin_x(b):
    return (b - 6) * DX


# ---------------------------------------------------------------- bin packing (rows of 4 and 3)
ROW_H = R_BALL * math.sqrt(3) * 0.99
_PERM4 = [(1, 2, 0, 3), (2, 1, 3, 0), (0, 3, 1, 2), (3, 0, 2, 1), (1, 3, 2, 0), (2, 0, 1, 3)]
_PERM3 = [(1, 0, 2), (0, 2, 1), (2, 1, 0), (1, 2, 0)]


def slot_pos(b, s):
    """(x, y) of the s-th ball to land in bin b (hex-ish packing, a slightly random fill order)"""
    q, m = divmod(s, 7)
    if m < 4:
        r = 2 * q
        i = _PERM4[(b * 5 + r * 3) % len(_PERM4)][m]
        xo = (i - 1.5) * 2 * R_BALL
    else:
        r = 2 * q + 1
        i = _PERM3[(b * 7 + r) % len(_PERM3)][m - 4]
        xo = (i - 1) * 2 * R_BALL
    return bin_x(b) + xo, Y_BF + R_BALL + r * ROW_H


def stack_top(n):
    """height of the top of a bin holding n balls"""
    if n <= 0:
        return Y_BF
    rows = 2 * (n // 7) + (1 if n % 7 else 0) + (1 if n % 7 > 4 else 0)
    return Y_BF + 2 * R_BALL + (rows - 1) * ROW_H


# ---------------------------------------------------------------- colours
# bins left (cool) -> right (warm); balls start pearl and take on colour bounce by bounce
RAINBOW = np.array([(150, 100, 255), (112, 116, 255), (76, 150, 255), (56, 196, 242), (52, 222, 196), (104, 230, 112),
                    (240, 228, 76), (255, 192, 60), (255, 148, 56), (255, 108, 64), (255, 72, 88), (240, 66, 150),
                    (214, 78, 214)], float)
PEARL = np.array([236, 232, 222.0])
GOLD = np.array([255, 196, 52.0])


def rainbow(ri):
    """ri in [0, 12] (fractional = between two bins)"""
    ri = np.clip(np.asarray(ri, float), 0, 12)
    i0 = np.floor(ri).astype(int)
    i1 = np.minimum(i0 + 1, 12)
    u = (ri - i0)[..., None]
    return RAINBOW[i0] * (1 - u) + RAINBOW[i1] * u


def ball_color(c, k):
    """colour after k bounces at column c"""
    sat = (0.36 + 0.64 * np.asarray(k, float) / ROWS)[..., None]
    return PEARL * (1 - sat) + rainbow((np.asarray(c, float) + 12) / 2) * sat


# ---------------------------------------------------------------- ball motion
def gravity(h):
    """vertical acceleration of a hop parabola with hop time h"""
    return 8 * BH / (h * h)


def hop_y(y0, u):
    """height along a hop: starts at y0 (u = 0), lands DY lower at u = 1; u > 1 keeps falling"""
    return y0 - DY * u + 4 * BH * u * (1 - u)


def fall_u(y0, y1):
    """u > 1 at which a hop starting at y0 reaches height y1 < y0 - DY"""
    a, bq, cq = -4 * BH, 4 * BH - DY, y0 - y1
    return (-bq - math.sqrt(bq * bq - 4 * a * cq)) / (2 * a)
