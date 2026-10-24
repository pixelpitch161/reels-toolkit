"""#30 v2 draw functions for the BALLFIGHT v2 redesign (user, Oct 24), kept in the episode's folder (series rule) so later
episodes and crossovers can import them:

    'balloon'       BAG INFLATE: the bag's tartan stretched over the glass as the ball swells (tartan_rgb), the
                    squealing flight's ribbon trail (draw_trail)
    'jaws'          BELLOWS BITE: the two end boards fly apart with the pleated bellows fanned out between them like
                    jaws, then snap shut (bite_open, draw_jaws)
    'clamp'         FULL BELLOWS: the boards fly to the ropes with a giant pleated bellows wall across the ring, then slam
                    together on the enemy (clamp_state, draw_clamp)
    'squeezebox'    SQUEEZEBOX: a closed bellows collar folded around the ball, its buttons glowing
    tartan_ring     WAR CRY: the shockwave as a tartan band on the canvas

v1 (drone aura, bellows rings, giant bellows slam) is in styles_v1_backup.py. Everything works in the ring frame
(sx, sz) + height, through scene.to_world, like #27 / #28's renderer."""
import math
import numpy as np
from PIL import Image, ImageDraw
import scene as S
from reelkit.config import W, H, BAYER_FULL

R = S.R_BALL
PLEAT_DK = np.array([46, 16, 44])
PLEAT_EDGE = np.array([255, 236, 250])
BOARD = np.array([34, 24, 40])
BUTTON = np.array([248, 242, 228])
TARTAN = dict(dk=np.array([26, 96, 46]), lt=np.array([84, 206, 98]), red=np.array([214, 48, 44]),
              gold=np.array([246, 204, 72]))


# ---------------------------------------------------------------- helpers
def annulus_polys(cam, cx, cz, r_in, r_out, y, n=36, a0=0.0, a1=2 * math.pi):
    """screen quads of a flat ring (or an arc a0..a1), one quad per segment"""
    a = np.linspace(a0, a1, n + 1)
    oi = S.to_world(cx + r_out * np.cos(a), cz + r_out * np.sin(a), y)
    ii = S.to_world(cx + r_in * np.cos(a), cz + r_in * np.sin(a), y)
    so, si = cam.wp(oi), cam.wp(ii)
    return [[so[j], so[j + 1], si[j + 1], si[j]] for j in range(n)]


def mask_of(polys):
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        d.polygon([tuple(q) for q in pts], fill=255)
    return np.asarray(m, np.float32) / 255.0


def blend(arr, m, col, k):
    arr += (np.array(col, float)[None, None] - arr) * (m * k)[..., None]


def dots(arr, pts, col, k=1.0, size=1):
    for x, y in pts:
        xi, yi = int(round(x)), int(round(y))
        if 0 <= xi < W - size and 0 <= yi < H - size:
            arr[yi:yi + size + 1, xi:xi + size + 1] = arr[yi:yi + size + 1, xi:xi + size + 1] * (1 - k) + np.array(col) * k


def quad_ring(cx, cz, u, v, a, b, y):
    """a world quad in the ring frame: centre (cx, cz), half-extents a along unit u and b along unit v, height y"""
    pts = [(cx + u[0] * a * i + v[0] * b * j, cz + u[1] * a * i + v[1] * b * j) for i, j in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
    return np.array([S.to_world(p[0], p[1], y) for p in pts])


def box_faces(cam, cx, cz, u, v, a, b, y0, y1):
    """the camera-facing faces of a box (ring frame centre, half-extents a along u, b along v, heights y0..y1), back to
    front: [(screen quad, shade 0..1)]"""
    lo, hi = quad_ring(cx, cz, u, v, a, b, y0), quad_ring(cx, cz, u, v, a, b, y1)
    c = (lo.mean(0) + hi.mean(0)) / 2
    faces = []
    for e in range(4):
        Q = np.array([lo[e], lo[(e + 1) % 4], hi[(e + 1) % 4], hi[e]])
        mid = Q.mean(0)
        if np.dot(mid - c, cam.pos - mid) > 0:
            faces.append((cam.depth(mid[None])[0], cam.wp(Q), 0.72 + 0.12 * (e % 2)))
    faces.sort(key=lambda f: -f[0])
    out = [(q, k) for d, q, k in faces]
    out.append((cam.wp(hi), 1.0))                                 # the top last
    return out


# ---------------------------------------------------------------- BAG INFLATE: the tartan bag over the swelling ball
def tartan_rgb(dx, dy, stretch=1.0, t=0.0):
    """tartan at the ball's local unit coords (dx, dy in -1..1): green blocks, a red over-check, gold pin lines; the
    sett stretches as the bag swells"""
    k = 2.6 / stretch
    u, v = dx * k + 0.37, dy * k + 0.11
    fu, fv = u % 1.0, v % 1.0
    blk = ((np.floor(u * 2) + np.floor(v * 2)) % 2).astype(float)
    c = TARTAN['dk'][None, None] * (1 - blk)[..., None] + TARTAN['lt'][None, None] * blk[..., None]
    red = (np.abs(fu - 0.5) < 0.07) | (np.abs(fv - 0.5) < 0.07)
    gold = (np.abs(fu - 0.12) < 0.03) | (np.abs(fv - 0.12) < 0.03)
    c = np.where(red[..., None], TARTAN['red'][None, None], c)
    c = np.where(gold[..., None], TARTAN['gold'][None, None], c)
    return c


def draw_trail(arr, cam, pts_ring, col, width=3):
    """the flight's ribbon: a fading polyline through the recent positions (ring frame, oldest first)"""
    if len(pts_ring) < 2:
        return
    P = cam.wp(np.array([S.to_world(p[0], p[1], R * 0.9) for p in pts_ring]))
    n = len(P)
    for j in range(n - 1):
        u = (j + 1) / n
        im = Image.new('L', (W, H), 0)
        ImageDraw.Draw(im).line([tuple(P[j]), tuple(P[j + 1])], fill=255, width=max(1, int(round(width * u))))
        m = np.asarray(im, np.float32) / 255.0
        blend(arr, m, np.clip(np.array(col) * 0.45 + 140 * u + 20, 0, 255), 0.9 * u ** 0.7)


# ---------------------------------------------------------------- BELLOWS BITE
def bite_open(t, t0, t_snap):
    """(openness 0..1, plate extension 0..1) of a bite: the boards fly out and the jaws gape over the opening, snap
    shut in 60 ms at t_snap, stay shut a beat-fraction, then the boards fly home; None when idle"""
    if t < t0:
        return None
    if t < t_snap - 0.06:
        u = (t - t0) / max(1e-6, t_snap - 0.06 - t0)
        e = 1 - (1 - min(1.0, u)) ** 2
        return e, min(1.0, u * 1.6)
    if t < t_snap:
        u = (t - (t_snap - 0.06)) / 0.06
        return 1 - u * u, 1.0
    if t < t_snap + 0.10:
        return 0.0, 1.0
    if t < t_snap + 0.24:
        return 0.0, 1 - (t - t_snap - 0.10) / 0.14
    return None


def draw_jaws(arr, cam, p, aim, half_deg, reach, opn, ext, col, y=R * 0.95):
    """the open bellows: a fan of pleats between the two boards (angles aim +- the opening), the boards (dark lacquer,
    a magenta trim, three buttons) at the jaw edges; ext pulls everything in toward the ball"""
    a = math.radians(half_deg) * 1.15 * opn
    r0 = R * 0.9
    r1 = R + reach * ext
    col = np.array(col, float)
    if a > 0.02 and r1 > R * 1.05:                                # the pleated fan
        n = max(6, int(a * 12))
        polys = annulus_polys(cam, p[0], p[1], r0, r1 * 0.97, y, n=n * 2, a0=aim - a, a1=aim + a)
        fold = [q for j, q in enumerate(polys) if j % 2 == 0]
        crease = [q for j, q in enumerate(polys) if j % 2 == 1]
        blend(arr, mask_of(fold), np.clip(col * 1.05 + 22, 0, 255), 0.92)
        blend(arr, mask_of(crease), PLEAT_DK + col * 0.22, 0.92)
        # pale ridges on the fold tips (the outer arc)
        aa = np.linspace(aim - a, aim + a, n * 2 + 1)
        P = cam.wp(S.to_world(p[0] + r1 * 0.97 * np.cos(aa), p[1] + r1 * 0.97 * np.sin(aa), y + 0.01))
        dots(arr, P[::2], PLEAT_EDGE, 0.85, 0)
    for side in (-1, 1):                                          # the two boards
        ang = aim + side * a
        d = np.array([math.cos(ang), math.sin(ang)])
        nrm = np.array([-d[1], d[0]])
        rm = (R * 1.02 + r1) / 2
        hl = max(0.05, (r1 - R * 1.02) / 2)
        c = p + d * rm
        for (q, k) in box_faces(cam, c[0], c[1], d, nrm, hl, 0.07, y - 0.10, y + 0.06):
            blend(arr, mask_of([q]), np.clip(BOARD * (0.8 + 0.5 * k) + col * 0.10 * k, 0, 255), 1.0)
        trim = quad_ring(c[0], c[1], d, nrm, hl, 0.025, y + 0.065)
        blend(arr, mask_of([cam.wp(trim)]), np.clip(col * 1.1 + 30, 0, 255), 1.0)
        bpts = [S.to_world(*(p + d * (r1 - 0.06 - 0.1 * j) - nrm * 0.0), y + 0.07) for j in range(3) if r1 - 0.06 - 0.1 * j > R]
        if bpts:
            dots(arr, cam.wp(np.array(bpts)), BUTTON, 1.0, 0)


# ---------------------------------------------------------------- FULL BELLOWS (the ring-wide clamp)
def rope_dist(c, d, A):
    """distance from c along unit d to the ropes (|x|, |z| <= A)"""
    ts = []
    for ax in (0, 1):
        if abs(d[ax]) > 1e-9:
            ts.append(((A if d[ax] > 0 else -A) - c[ax]) / d[ax])
    return max(0.2, min(ts))


def clamp_state(t, t0, t_snap, c, axis, q, A):
    """(far board pos, near board pos, alpha) of FULL BELLOWS: the boards fly from the accordion to the ropes on both
    sides along the axis (the telegraph), then slam together on q (the enemy at the snap) in 0.12 s, hold, and fly
    home; None when idle"""
    d = np.array([math.cos(axis), math.sin(axis)])
    c = np.asarray(c, float)
    far, near = c + d * (rope_dist(c, d, A) - 0.12), c - d * (rope_dist(c, -d, A) - 0.12)
    q = np.asarray(q, float)
    qa = c + d * float((q - c) @ d)                               # the enemy's spot projected on the axis
    if t < t0:
        return None
    if t < t_snap - 0.12:
        u = min(1.0, (t - t0) / max(1e-6, t_snap - 0.12 - t0) * 1.4)
        e = 1 - (1 - u) ** 3
        return c + (far - c) * e, c + (near - c) * e, min(1.0, (t - t0) / 0.1)
    if t < t_snap:
        u = (t - (t_snap - 0.12)) / 0.12
        e = u * u
        return far + (qa + d * 0.32 - far) * e, near + (qa - d * 0.32 - near) * e, 1.0
    if t < t_snap + 0.35:
        return qa + d * 0.32, qa - d * 0.32, 1.0
    if t < t_snap + 0.75:
        u = (t - t_snap - 0.35) / 0.4
        return qa + d * 0.32 + (c - qa) * u, qa - d * 0.32 + (c - qa) * u, 1 - u
    return None


def draw_clamp(arr, cam, a_pos, b_pos, axis, col, alpha=1.0, width=1.5, h=1.1):
    """a giant pleated bellows wall between the two boards (zig-zag ridges across the ring), the boards as big standing
    end plates with keyboards / button rows"""
    d = np.array([math.cos(axis), math.sin(axis)])
    nrm = np.array([-d[1], d[0]])
    a_pos, b_pos = np.asarray(a_pos, float), np.asarray(b_pos, float)
    L = float((a_pos - b_pos) @ d)
    col = np.array(col, float)
    n = max(2, int(abs(L) / 0.22))
    items = []
    for j in range(n):                                            # pleat segments (alternating folds / creases)
        s0, s1 = j / n, (j + 1) / n
        cA = b_pos + (a_pos - b_pos) * (s0 + s1) / 2
        hl = abs(L) / n / 2
        wj = width * (1.0 if j % 2 == 0 else 0.86)
        items.append((cam.depth(S.to_world(cA[0], cA[1], h * 0.5)[None])[0], 'pleat', j, cA, hl, wj))
    for k, pos in ((0, a_pos), (1, b_pos)):
        items.append((cam.depth(S.to_world(pos[0], pos[1], h * 0.5)[None])[0], 'board', k, pos, 0.08, width * 1.12))
    items.sort(key=lambda it: -it[0])
    for dz, kind, j, cc, hl, wj in items:
        if kind == 'pleat':
            base = np.clip(col * 1.05 + 20, 0, 255) if j % 2 == 0 else PLEAT_DK + col * 0.22
            for q, k in box_faces(cam, cc[0], cc[1], d, nrm, hl, wj / 2, 0.02, h * (0.96 if j % 2 else 1.0)):
                blend(arr, mask_of([q]), np.clip(base * (0.75 + 0.35 * k), 0, 255), alpha)
        else:
            for q, k in box_faces(cam, cc[0], cc[1], d, nrm, hl, wj / 2, 0.0, max(0.9, h * 1.08)):
                blend(arr, mask_of([q]), np.clip(BOARD * (0.8 + 0.5 * k) + col * 0.12, 0, 255), alpha)
            # a row of glowing buttons along the top edge
            pts = [S.to_world(*(cc + nrm * (wj / 2 * (u * 1.6 - 0.8))), max(0.9, h * 1.08) + 0.01) for u in np.linspace(0, 1, 7)]
            dots(arr, cam.wp(np.array(pts)), BUTTON, alpha, 1)
    # pale ridges along the top of the folds
    im = Image.new('L', (W, H), 0)
    dr = ImageDraw.Draw(im)
    for j in range(0, n, 2):
        cc = b_pos + (a_pos - b_pos) * (j + 0.5) / n
        e0 = S.to_world(*(cc - nrm * width / 2), h + 0.01)
        e1 = S.to_world(*(cc + nrm * width / 2), h + 0.01)
        s0, s1 = cam.wp(np.array([e0, e1]))
        dr.line([tuple(s0), tuple(s1)], fill=255)
    blend(arr, np.asarray(im, np.float32) / 255.0, PLEAT_EDGE, 0.7 * alpha)


# ---------------------------------------------------------------- SQUEEZEBOX (shield): the folded collar, buttons glowing
def collar_polys(cam, cx, cz, r_in, r_out, y, half, n=32):
    a = np.linspace(0, 2 * math.pi, n + 1)
    oi = S.to_world(cx + r_out * np.cos(a), cz + r_out * np.sin(a), y)
    ii = S.to_world(cx + r_in * np.cos(a), cz + r_in * np.sin(a), y)
    so, si = cam.wp(oi), cam.wp(ii)
    dz_o = cam.depth(oi)
    c = cam.depth(S.to_world(cx, cz, y)[None])[0]
    out = []
    for j in range(n):
        if ((dz_o[j] + dz_o[j + 1]) / 2 < c) == bool(half):
            out.append((j, [so[j], so[j + 1], si[j + 1], si[j]]))
    return out


def draw_squeezebox(arr, cam, cx, cz, t, k, half, col, close, glow):
    """the collar (three stacked pleat rings folded shut around the ball) and two rings of buttons that glow (glow 0..1
    pulses on the beat)"""
    col = np.array(col, float)
    for layer in range(3):
        y = R * (0.62 + 0.38 * layer) * (1 - 0.15 * close) + 0.05
        r_out = R * (1.85 - 0.45 * close) - 0.04 * layer
        items = collar_polys(cam, cx, cz, R * 0.96, r_out, y, half)
        fold = [p for j, p in items if (j + layer) % 2 == 0]
        crease = [p for j, p in items if (j + layer) % 2 == 1]
        blend(arr, mask_of(fold), np.clip(col * (0.95 + 0.1 * layer) + 20, 0, 255), 0.86 * k)
        blend(arr, mask_of(crease), PLEAT_DK + col * 0.2, 0.86 * k)
    rb = R * (1.62 - 0.42 * close)
    c0 = cam.depth(S.to_world(cx, cz, R)[None])[0]
    for ring, (yy, nb) in enumerate(((R * 1.05, 10), (R * 0.72, 12))):
        for j in range(nb):
            ang = j * 2 * math.pi / nb + 0.3 * ring + t * 1.5
            pw = S.to_world(cx + rb * math.cos(ang), cz + rb * math.sin(ang), yy)
            if (cam.depth(pw[None])[0] < c0) != bool(half):
                continue
            s_ = cam.wp(pw[None])[0]
            g = np.clip(BUTTON * (0.75 + 0.4 * glow) + col * 0.25 * glow, 0, 255)
            dots(arr, [s_], g, k, 1 if glow > 0.5 else 0)


# ---------------------------------------------------------------- WAR CRY: a tartan shockwave band on the canvas
def tartan_ring(sxp, szp, bayer, cx, cz, r, width, k, t):
    """(paint (n, 3), weight (n,)) of a tartan band of radius r around (cx, cz)"""
    rd = np.hypot(sxp - cx, szp - cz)
    u = (rd - (r - width)) / width
    on = (u >= 0) & (u <= 1)
    ang = np.arctan2(szp - cz, sxp - cx)
    s_ = (ang * max(r, 0.3) / 0.24 + 0.5 * t) % 1.0
    c = np.where((s_ < 0.5)[:, None], TARTAN['dk'][None], TARTAN['lt'][None]).astype(float)
    red = (np.abs(s_ - 0.25) < 0.06) | (np.abs(u - 0.5) < 0.10)
    gold = np.abs(s_ - 0.75) < 0.035
    c = np.where(red[:, None], TARTAN['red'][None], c)
    c = np.where(gold[:, None], TARTAN['gold'][None], c)
    edge = np.clip(1 - np.abs(u - 0.5) * 2, 0, 1) ** 0.3
    wgt = np.floor(edge * k * 4 + bayer * 0.9) / 4 * on
    return c, np.clip(wgt, 0, 1)
