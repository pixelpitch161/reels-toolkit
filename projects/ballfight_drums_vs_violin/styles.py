"""#28 draw functions for the new BALLFIGHT weapon styles and ability looks, kept in the episode's folder (series rule)
so later episodes and crossovers can import them:

    style 'bow'       the violin's bow spun like a rapier: dark stick, pale horsehair ribbon, ebony frog, hatchet tip
    style 'drumhead'  a drumhead ring (cream skin band + coloured hoop) blasting out from the ball on every pulse
    'dart'            a pizzicato dart: a plucked string segment with a bright head (arc projectiles)
    'cymbal'          the CRASH CYMBAL shield: a golden ride around the ball with lathe grooves and a sweeping glint
    'beater'          DOUBLE KICK: a giant kick-pedal beater stomping the predicted spot (taps, then the hit)

Everything works in the ring frame (sx, sz) + height, through scene.to_world, like #27's renderer."""
import math
import numpy as np
from PIL import Image, ImageDraw
import scene as S
from reelkit.config import W, H, BAYER_FULL

R = S.R_BALL

# ---------------------------------------------------------------- the bow (a blade)
STICK = np.array([92, 44, 30])          # pernambuco
STICK_HI = np.array([150, 86, 52])
HAIR = np.array([236, 230, 212])
FROG = np.array([26, 22, 30])
PEARL = np.array([226, 236, 255])
TIP = np.array([240, 236, 226])
STICK_W, HAIR_W, HAIR_GAP = 0.055, 0.045, 0.085


def bow_geom(p, th, blen, sd=1.0, y=R):
    """world quads [(Q, colour, kind)] of the bow from the ball's surface to the tip; the hair rides on the leading side"""
    d = np.array([math.cos(th), math.sin(th)])
    nrm = np.array([-d[1], d[0]]) * sd
    r0, r1 = R * 0.86, R + blen
    Wp = lambda r, w: p + d * r + nrm * w
    stick = [Wp(r0, -STICK_W / 2), Wp(r1, -STICK_W / 2 * 0.7), Wp(r1, STICK_W / 2 * 0.7), Wp(r0, STICK_W / 2)]
    hair = [Wp(r0 + 0.16, HAIR_GAP - HAIR_W / 2), Wp(r1 - 0.06, HAIR_GAP * 0.55 - HAIR_W / 2),
            Wp(r1 - 0.06, HAIR_GAP * 0.55 + HAIR_W / 2), Wp(r0 + 0.16, HAIR_GAP + HAIR_W / 2)]
    frog = [Wp(r0 + 0.02, -0.03), Wp(r0 + 0.22, -0.03), Wp(r0 + 0.20, HAIR_GAP + 0.04), Wp(r0 + 0.04, HAIR_GAP + 0.04)]
    tip = [Wp(r1 - 0.13, -0.04), Wp(r1 + 0.02, -0.04), Wp(r1 + 0.02, HAIR_GAP * 0.55 + 0.03), Wp(r1 - 0.08, HAIR_GAP * 0.55 + 0.03)]
    out = []
    for Q, c, kind in ((hair, HAIR, 'hair'), (stick, STICK, 'stick'), (frog, FROG, 'frog'), (tip, TIP, 'tip')):
        out.append((np.array([S.to_world(q[0], q[1], y + (0.012 if kind == 'stick' else 0.0)) for q in Q]), c, kind))
    pearl = S.to_world(*(p + d * (r0 + 0.12) + nrm * (HAIR_GAP * 0.5)), y + 0.02)
    return out, pearl


def draw_bow(fr, cam, L, shade, st):
    quads, pearl = bow_geom(st['p'], st['th'], st['blen'], st.get('sd', 1.0))
    for Q, col, kind in quads:
        c = shade(L, np.array([[0, 1.0, 0]]), Q.mean(0)[None], col, rimk=0.35)[0]
        fr.poly(cam, Q, c)
        if kind == 'stick':                                   # a highlight line along the stick (it's round)
            s = cam.wp(Q)
            fr.draw.line([tuple((s[0] + s[3]) / 2), tuple((s[1] + s[2]) / 2)],
                         fill=tuple(int(v) for v in np.clip(shade(L, np.array([[0, 1.0, 0]]), Q.mean(0)[None],
                                                                        STICK_HI, rimk=0.6)[0], 0, 255)), width=1)
    sp = cam.wp(pearl[None])[0]
    fr.draw.point([(sp[0], sp[1])], fill=tuple(int(v) for v in PEARL))


def bow_trail_polys(cam, st, dth, nseg=6):
    """the swept wedge behind the spinning bow (motion blur) as screen polygons, tip-heavy (a thin blade blurs as a fan)"""
    p, th, blen = st['p'], st['th'], st['blen']
    r0, r1 = R + blen * 0.35, R + blen
    out = []
    for j in range(nseg):
        a0 = th - dth * j / nseg
        a1 = th - dth * (j + 1) / nseg
        pts = []
        for a, r in ((a0, r0), (a0, r1), (a1, r1), (a1, r0)):
            q = p + r * np.array([math.cos(a), math.sin(a)])
            pts.append(S.to_world(q[0], q[1], R))
        out.append((cam.wp(np.array(pts)), j / nseg))
    return out


# ---------------------------------------------------------------- drumhead rings (pulses)
SKIN = np.array([246, 238, 216])
CHROME = np.array([214, 220, 236])


def annulus_polys(cam, cx, cz, r_in, r_out, y, n=36):
    """screen quads of a flat ring (no holes needed: one quad per segment)"""
    a = np.linspace(0, 2 * math.pi, n + 1)
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


def pulse_state(t, t0, reach, speed):
    """(radius beyond the ball's centre, alpha, expanding?) of a pulse fired at t0 that stops at R + reach"""
    u = t - t0
    if u < 0:
        return None
    r = R + min(reach, speed * u)
    t_full = reach / speed
    a = 1.0 if u <= t_full else max(0.0, 1 - (u - t_full) / 0.22)
    if a <= 0:
        return None
    return r, a, u <= t_full


def draw_drumhead(arr, cam, cx, cz, r, a, col, main=True, y=0.34):
    """one pulse ring: hoop (fighter colour) on the outside, the cream skin band inside, chrome lugs on the rim"""
    w_hoop, w_skin = 0.07, 0.12 if main else 0.07
    k = a * (1.0 if main else 0.65)
    m_skin = mask_of(annulus_polys(cam, cx, cz, max(R * 0.9, r - w_hoop - w_skin), r - w_hoop, y))
    m_hoop = mask_of(annulus_polys(cam, cx, cz, r - w_hoop, r, y))
    dz = np.floor(BAYER_FULL * 0.9 + 0.55 * k * 2) / 2 if k < 0.6 else 1.0
    arr += (SKIN[None, None] - arr) * (m_skin * 0.62 * k * dz)[..., None]
    arr += (np.clip(np.array(col) * 1.1 + 30, 0, 255)[None, None] - arr) * (m_hoop * 0.92 * k)[..., None]
    if main and k > 0.3:                                      # six lugs on the hoop
        for j in range(6):
            ang = j * math.pi / 3 + 0.3
            s = cam.wp(S.to_world(cx + r * math.cos(ang), cz + r * math.sin(ang), y + 0.02)[None])[0]
            xi, yi = int(round(s[0])), int(round(s[1]))
            if 0 <= xi < W and 0 <= yi < H:
                arr[yi, xi] = arr[yi, xi] * (1 - k) + CHROME * k


# ---------------------------------------------------------------- pizzicato darts (arc projectiles)
DART_COL = np.array([214, 232, 255])


def dart_quads(x, z, ang, y=0.48, L_=0.34, Wd=0.07):
    d = np.array([math.cos(ang), math.sin(ang)])
    n = np.array([-d[1], d[0]])
    c = np.array([x, z])
    head = [c + d * L_ * 0.5, c + d * L_ * 0.15 - n * Wd, c + d * L_ * 0.05, c + d * L_ * 0.15 + n * Wd]
    body = [c + d * L_ * 0.1 - n * 0.012, c - d * L_ * 0.5 - n * 0.012, c - d * L_ * 0.5 + n * 0.012, c + d * L_ * 0.1 + n * 0.012]
    return (np.array([S.to_world(q[0], q[1], y) for q in head]), np.array([S.to_world(q[0], q[1], y) for q in body]))


def draw_dart(fr, cam, L, shade, x, z, ang, col):
    head, body = dart_quads(x, z, ang)
    fr.poly(cam, body, np.clip(np.array(col) * 0.5 + 120, 0, 255))
    c = shade(L, np.array([[0, 1.0, 0]]), head.mean(0)[None], DART_COL, rimk=0.4)[0]
    fr.poly(cam, head, c + 20)


# ---------------------------------------------------------------- CRASH CYMBAL (shield)
BRONZE = np.array([226, 176, 70])
BRONZE_DK = np.array([150, 104, 34])


def cymbal_polys(cam, cx, cz, rp_world, y, half):
    """the cymbal's ring (inner 0.92 R .. outer 1.62 R) split in the back half (half=0) and front half (half=1) as seen by
    the camera, so the ball can sit inside it in painter order"""
    n = 40
    r_in, r_out = R * 0.92, R * 1.62
    a = np.linspace(0, 2 * math.pi, n + 1)
    oi = S.to_world(cx + r_out * np.cos(a), cz + r_out * np.sin(a), y)
    ii = S.to_world(cx + r_in * np.cos(a), cz + r_in * np.sin(a), y)
    so, si = cam.wp(oi), cam.wp(ii)
    dz_o = cam.depth(oi)
    c = cam.depth(S.to_world(cx, cz, y)[None])[0]
    polys, ang = [], []
    for j in range(n):
        front = (dz_o[j] + dz_o[j + 1]) / 2 < c
        if front == bool(half):
            polys.append([so[j], so[j + 1], si[j + 1], si[j]])
            ang.append((a[j] + a[j + 1]) / 2)
    return polys, ang


def draw_cymbal(arr, cam, cx, cz, t, k, half, y=R * 1.02):
    """k: 0..1 strength (fades in / out); a glint sweeps around as it spins"""
    polys, ang = cymbal_polys(cam, cx, cz, None, y, half)
    if not polys:
        return
    spin = t * 9.0
    for pts, a in zip(polys, ang):
        g = 0.5 + 0.5 * math.cos(a - spin)
        g2 = max(0.0, math.cos(a - spin)) ** 18
        col = BRONZE_DK + (BRONZE - BRONZE_DK) * g + np.array([255, 250, 230]) * 0.35 * g2
        m = mask_of([pts])
        arr += (np.clip(col, 0, 255)[None, None] - arr) * (m * 0.88 * k)[..., None]
    # lathe grooves: two thin darker rings
    for rr in (1.15, 1.4):
        a = np.linspace(0, 2 * math.pi, 41)
        P = S.to_world(cx + R * rr * np.cos(a), cz + R * rr * np.sin(a), y + 0.005)
        s = cam.wp(P)
        dz = cam.depth(P)
        c = cam.depth(S.to_world(cx, cz, y)[None])[0]
        im = Image.new('L', (W, H), 0)
        d = ImageDraw.Draw(im)
        for j in range(40):
            if ((dz[j] + dz[j + 1]) / 2 < c) == bool(half):
                d.line([tuple(s[j]), tuple(s[j + 1])], fill=255)
        mk = np.asarray(im, np.float32) / 255.0
        arr += ((BRONZE_DK * 0.8)[None, None] - arr) * (mk * 0.6 * k)[..., None]


# ---------------------------------------------------------------- DOUBLE KICK (slam): the beater
FELT = np.array([240, 234, 220])
SHAFT = np.array([200, 206, 220])


def beater_height(t, t0, t_slam, beat):
    """the beater's head height: hovers high during the telegraph, a tap an 8th before the hit (the 'double'), then the
    real stomp; stays down briefly, then lifts away"""
    tap = t_slam - beat / 2
    if t < t0:
        return None
    if t < tap - 0.10:
        return 3.2 - 0.4 * min(1.0, (t - t0) / 0.3)
    if t < tap:
        u = (t - (tap - 0.10)) / 0.10
        return 2.8 - 1.9 * u * u
    if t < tap + 0.08:
        return 0.9 + 1.4 * ((t - tap) / 0.08)
    if t < t_slam - 0.08:
        return 2.3 + 0.6 * min(1.0, (t - tap - 0.08) / 0.1)
    if t < t_slam:
        u = (t - (t_slam - 0.08)) / 0.08
        return 2.9 - 2.5 * u * u
    if t < t_slam + 0.25:
        return 0.40
    if t < t_slam + 0.6:
        return 0.40 + 3.0 * ((t - t_slam - 0.25) / 0.35) ** 2
    return None


def draw_beater(arr, cam, cx, cz, rad, h, alpha=1.0):
    """felt head (a squat disc seen from above) + chrome shaft up toward the pedal's pivot (off to the back)"""
    head_r = rad * 0.55
    top = S.to_world(cx, cz, h + 0.25)
    piv = S.to_world(cx - 0.9, cz + 0.9, h + 3.4)
    s_top, s_piv = cam.wp(np.array([top, piv]))
    im = Image.new('L', (W, H), 0)
    ImageDraw.Draw(im).line([tuple(s_top), tuple(s_piv)], fill=255, width=3)
    mk = np.asarray(im, np.float32) / 255.0
    arr += (SHAFT[None, None] - arr) * (mk * alpha)[..., None]
    a = np.linspace(0, 2 * math.pi, 25)[:-1]
    for yy, col, rr in ((h, FELT * 0.72, head_r), (h + 0.24, FELT, head_r * 0.96)):
        P = S.to_world(cx + rr * np.cos(a), cz + rr * np.sin(a), yy)
        m = mask_of([cam.wp(P)])
        arr += (col[None, None] - arr) * (m * alpha)[..., None]
    P = S.to_world(cx + head_r * 0.45 * np.cos(a), cz + head_r * 0.45 * np.sin(a), h + 0.245)
    m = mask_of([cam.wp(P)])
    arr += (np.array([200, 60, 52])[None, None] - arr) * (m * 0.5 * alpha)[..., None]      # the felt's worn red spot
