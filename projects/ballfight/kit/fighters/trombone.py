"""TROMBONE (mint), as built in #34 and drawn / played in #41: SLIDE LANCE (the slide shoots out at the enemy on every
beat, reach grows per landed hit, swats projectiles and, with Rules(swat_cobras=True), cobras), SAD TROMBONE
(shockwave + slow), POWER SLIDE (haste), PEDAL TONE (a giant bell stomps the predicted spot).

Look: the brass slide lance (two tubes, braces, the crook) with its thrust streak and lit lane on the canvas, the
crook's flare, SAD TROMBONE's ring and the slowed enemy's droopy wobble, PEDAL TONE's target, hovering bell and stomp,
brass sparks and embers, the solo played on the slide (lower note = longer slide), brass shards.
Sound: the circus bass and the smears (role 'bass'): every thrust is the bass line (root on 1 + 3, the fifth on 2 + 4)
with a smear up into it, POWER SLIDE walks 8ths, a landed lance smears up the chord, SAD TROMBONE is the clown's
plunger wah-wah down to the tonic, PEDAL TONE the pedal tone under the hovering bell. Opt-in (#48), role 'riff': over
someone else's bass, every thrust is a punchy stab of its riff line (config LINES['TROMBONE'], 8ths: a thrust a beat
plays the beat's note, POWER SLIDE every 8th), smeared up into it; the rest as before.
Draw code from projects/ballfight_trombone_vs_horn/styles.py (#34), copied here unchanged; sound from #41's audio.py."""
import math

import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, FPS, BAYER_FULL, ss, ease
from reelkit.shadows import proj_point
from reelkit import fx
import scene as S

from . import base

R = S.R_BALL
BRASS = np.array([232, 188, 92])
BRASS_HI = np.array([255, 238, 176])
BRASS_DK = np.array([150, 104, 40])

# ---------------------------------------------------------------- the slide lance (#34 styles.py)
TUBE_GAP, TUBE_W, OUTER_W = 0.075, 0.034, 0.05


def slide_geom(p, th, ext, y=R):
    """world quads [(Q, colour, kind)] of the slide from inside the ball to the crook at R + ext"""
    d = np.array([math.cos(th), math.sin(th)])
    n = np.array([-d[1], d[0]])
    r0, r1 = R * 0.8, R + ext
    Wp = lambda r, w: S.to_world(*(p + d * r + n * w), y)
    out = []
    r_out = max(r0 + 0.08, r1 - max(0.35, 0.62 * (r1 - r0)))     # the outer slide (thicker) rides on the inner tubes
    for side in (-1, 1):
        c = side * TUBE_GAP
        out.append((np.array([Wp(r0, c - TUBE_W / 2), Wp(r1, c - TUBE_W / 2), Wp(r1, c + TUBE_W / 2),
                              Wp(r0, c + TUBE_W / 2)]), BRASS, 'tube'))
        out.append((np.array([Wp(r_out, c - OUTER_W / 2), Wp(r1 - 0.02, c - OUTER_W / 2), Wp(r1 - 0.02, c + OUTER_W / 2),
                              Wp(r_out, c + OUTER_W / 2)]), BRASS * 1.04, 'outer'))
    for rb in (r0 + 0.14, r_out + 0.06):                          # the braces across the two tubes
        if rb < r1 - 0.1:
            out.append((np.array([Wp(rb - 0.025, -TUBE_GAP), Wp(rb + 0.025, -TUBE_GAP), Wp(rb + 0.025, TUBE_GAP),
                                  Wp(rb - 0.025, TUBE_GAP)]), BRASS_DK * 1.2, 'brace'))
    # the crook: a U-bend beyond the tube ends (8 segments)
    a = np.linspace(-math.pi / 2, math.pi / 2, 9)
    for j in range(8):
        pts = []
        for aa, rr in ((a[j], TUBE_GAP + OUTER_W / 2), (a[j + 1], TUBE_GAP + OUTER_W / 2),
                       (a[j + 1], TUBE_GAP - OUTER_W / 2), (a[j], TUBE_GAP - OUTER_W / 2)):
            pts.append(Wp(r1 + rr * math.cos(aa) * 0.9, rr * math.sin(aa)))
        out.append((np.array(pts), BRASS, 'crook'))
    tip = S.to_world(*(p + d * (r1 + TUBE_GAP)), y)
    return out, tip


def draw_slide(fr, cam, L, shade, st, glow_col=None, glow=0.0):
    quads, tip = slide_geom(st['p'], st['th'], st['ext'])
    up = np.array([[0, 1.0, 0]])
    for Q, col, kind in quads:
        c = shade(L, up, Q.mean(0)[None], col, rimk=0.45)[0]
        if glow_col is not None and glow > 0 and kind in ('outer', 'crook'):
            c = c * (1 - 0.45 * glow) + np.asarray(glow_col) * 0.45 * glow + 30 * glow
        fr.poly(cam, Q, np.clip(c, 0, 255))
    for Q, col, kind in quads:                                    # a glint line along each outer slide (round tubes)
        if kind == 'outer':
            s = cam.wp(Q)
            fr.draw.line([tuple((s[0] + s[3]) / 2), tuple((s[1] + s[2]) / 2)], fill=tuple(int(v) for v in BRASS_HI), width=1)
    return cam.wp(tip[None])[0]


def slide_trail_polys(cam, st, ext0):
    """the streak left by a fast thrust: a soft band from the previous tip position to the current one"""
    d = np.array([math.cos(st['th']), math.sin(st['th'])])
    n = np.array([-d[1], d[0]])
    p = st['p']
    a, b = R + ext0, R + st['ext'] + TUBE_GAP
    if b - a < 0.05:
        return []
    w = TUBE_GAP + OUTER_W
    pts = [p + d * a + n * w * 0.6, p + d * b + n * w, p + d * b - n * w, p + d * a - n * w * 0.6]
    return [cam.wp(np.array([S.to_world(q[0], q[1], R) for q in pts]))]


# ---------------------------------------------------------------- PEDAL TONE's giant bell (#34 styles.py)
def bell_height(t, t0, t_slam):
    """the bell drops in from high above, hovers (bobbing with the held pedal note) during the telegraph, slams down in
    the last 0.12 s, sits on the canvas for a moment, then flies back up out of frame"""
    if t < t0:
        return None
    if t < t_slam - 0.12:
        u = min(1.0, (t - t0) / 0.3)
        return 3.2 + 5.0 * (1 - u) ** 2 + 0.08 * math.sin(2 * math.pi * 3.0 * (t - t0))
    if t < t_slam:
        u = (t - (t_slam - 0.12)) / 0.12
        return 3.2 - 3.1 * u * u
    if t < t_slam + 0.3:
        return 0.10
    if t < t_slam + 0.75:
        return 0.10 + 8.0 * ((t - t_slam - 0.3) / 0.45) ** 2
    return None


BELL_PROFILE = [(0.0, 1.0), (0.04, 0.93), (0.13, 0.74), (0.28, 0.54), (0.48, 0.40), (0.78, 0.30), (1.15, 0.24),
                (1.6, 0.21)]       # (height above the rim, radius / rim radius): a flared bell, mouth down


def draw_giant_bell(fr, cam, L, shade, cx, cz, rad, h, col, nseg=18):
    """mouth down: a flat-shaded low-poly surface of revolution (painter-sorted quads), a bright lip, and the leadpipe
    rising out of frame"""
    rim = rad * 1.05
    a = np.linspace(0, 2 * math.pi, nseg + 1)
    rings = [S.to_world(cx + rim * rr * np.cos(a), cz + rim * rr * np.sin(a), h + dy) for dy, rr in BELL_PROFILE]
    base_ = BRASS * 0.42 + np.asarray(col) * 0.06       # dark enough that the spot, attack lights + bloom land it in gold
    quads = []
    for j in range(len(rings) - 1):
        lo, hi = rings[j], rings[j + 1]
        for i in range(nseg):
            Q = np.array([lo[i], lo[i + 1], hi[i + 1], hi[i]])
            nrm = np.cross(Q[1] - Q[0], Q[3] - Q[0])
            nrm = nrm / (np.linalg.norm(nrm) + 1e-9)
            if nrm[1] < 0:
                nrm = -nrm
            if np.dot(nrm, cam.pos - Q.mean(0)) <= 0:
                continue
            quads.append((cam.depth(Q.mean(0)[None])[0], Q, nrm, j))
    quads.sort(key=lambda q: -q[0])
    for dz, Q, nrm, j in quads:
        c = shade(L, nrm[None], Q.mean(0)[None], base_ * (0.92 + 0.03 * j), rimk=0.6)[0]
        fr.poly(cam, Q, np.clip(c, 0, 255))
    s = cam.wp(rings[0])
    fr.draw.line([tuple(q) for q in s], fill=tuple(int(v) for v in BRASS_HI), width=1)
    top = S.to_world(cx, cz, h + BELL_PROFILE[-1][0])
    up = S.to_world(cx + 0.6, cz - 0.6, h + 6.0)
    pw = rim * BELL_PROFILE[-1][1] * 0.7
    side = np.array([pw, 0.0, pw]) * 0.7
    Q = np.array([top - side, top + side, up + side, up - side])
    fr.poly(cam, Q, np.clip(shade(L, np.array([[0.3, 0.6, -0.7]]), Q.mean(0)[None], base_, rimk=0.6)[0], 0, 255))


class Look(base.Look):
    shards = ['slidebit'] * 4 + ['tube'] * 4 + ['bellbit'] * 6
    spark_flash = ('blade', 'slam')

    def setup(self):
        P, k = self.P, self.k
        self.THR_T = np.array([e[0] for e in P.EV if e[1] == 'thrust' and e[2] == k])
        self.solo_n = P.T.SOLO.get(self.name, [])
        self.solo_lo = min((m for b, m, d in self.solo_n), default=0)
        self.solo_hi = max((m for b, m, d in self.solo_n), default=1)

    def solo_ext(self, t):
        """the winning trombone plays its solo with the slide: a lower note = a longer slide (real slide positions run
        from 1st, closed, to 7th, out), gliding to each note's position as it starts; closed between phrases"""
        P, k = self.P, self.k
        rest = P.REST[k]
        reach = float(P.RUN['BL'][-1, k])
        past = [(P.T_SOLO + b * P.T.BEAT, m, d) for b, m, d in self.solo_n if P.T_SOLO + b * P.T.BEAT <= t]
        if not past:
            return rest
        lo, hi = self.solo_lo, self.solo_hi
        t0, m, d = past[-1]
        pos = 1 - (m - lo) / max(1, hi - lo)
        tgt = rest + 0.25 + (reach - rest - 0.25) * pos
        prev = past[-2] if len(past) > 1 else None
        p0 = rest if prev is None else rest + 0.25 + (reach - rest - 0.25) * (1 - (prev[1] - lo) / max(1, hi - lo))
        u = ease(min(1.0, (t - t0) / 0.07))
        return p0 + (tgt - p0) * u

    def state(self, kk, st, t, phase, gl=0.0):
        P, k = self.P, self.k
        if kk != k:
            return
        if phase == 'fight' and len(self.THR_T):                   # the trombone ball thumps on every thrust
            j = np.searchsorted(self.THR_T, min(t, P.T_KO), 'right') - 1
            if j >= 0:
                st['thr_k'] = math.exp(-(min(t, P.T_KO) - self.THR_T[j]) / 0.1)
                st['glow'] = 0.35 * st['thr_k']
        elif phase == 'tail' and k == P.WIN:                        # the solo on the slide, closing for the glide home
            rest = P.REST[k]
            e_ko = float(P.RUN['EXT'][-1, k])
            e = e_ko + (rest - e_ko) * ss(P.T_HS, P.T_HS + 0.4, t) if t < P.T_SOLO else self.solo_ext(t)
            st['ext'] = e + (rest - e) * ss(P.T.T_REASSEMBLE - 0.3, P.T.T_REASSEMBLE, t)

    def bell_down(self, t):
        P = self.P
        if not (P.T.T_FIGHT <= t < P.T_HS):
            return []
        return [(cx, cz, t0, t_slam, rad) for owner, cx, cz, t0, t_slam, rad in P.RUN['SLAMS'][P.sim_i(t)] if owner == self.k]

    def hides_hp(self, kk, t):
        """trapped under PEDAL TONE's bell"""
        P = self.P
        if not (P.T.T_FIGHT <= t < P.T_HS):
            return False
        p = P.fighter(kk, t)['p']
        for cx, cz, t0, t_slam, rad in self.bell_down(t):
            h = bell_height(t, t0, t_slam)
            if h is not None and h < 1.6 and math.hypot(p[0] - cx, p[1] - cz) < rad * 1.05:
                return True
        return False

    # ---------------------------------------------------------------- events
    def shake(self, e):
        if e[1] == 'hit' and e[7] in ('blade', 'slam', 'shockwave'):
            return {'blade': 0.55, 'slam': 2.5, 'shockwave': 1.2}[e[7]]
        if e[1] == 'swat':
            return 0.4
        return super().shake(e)

    def spark(self, e):
        if e[7] == 'blade':
            return self.P.COL[e[2]] * 0.6 + 100, 8, 2.4, 0.22
        return super().spark(e)

    def hit_glow(self, e):
        if e[7] in ('slam', 'shockwave'):
            return 14, 0.35
        if e[7] == 'blade':
            return 8, 0.22
        return None

    def fx_event(self, e):
        P = self.P
        t_ = e[0]
        up = P.up
        if e[1] == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            pos = up(x, z, R)
            if how == 'blade':                                     # the slide lance lands: brass sparks (#34)
                P.BU.add(t_, pos, P.COL[att], 0.8)
                P.FL.add(t_, pos, P.FL_COL[att], radius=1.4, peak=0.6, decay=0.22)
                th = P.fighter(att, t_)['th']
                d = up(math.cos(th), math.sin(th), 0.0)
                P.PA.emit(t_, pos, 'brass', n=9, dir=d + np.array([0, 0.4, 0]))
            elif how == 'shockwave':                               # SAD TROMBONE
                P.BU.add(t_, pos, P.COL[att], 1.0)
                P.FL.add(t_, pos, P.FL_COL[att], radius=1.6, peak=0.8, decay=0.3)
                P.PA.emit(t_, pos, 'brass', n=6)
            elif how == 'slam':
                P.BU.add(t_, pos, P.COL[att], 2.0)
        elif e[1] == 'slam':                                       # PEDAL TONE: the bell stomps (#34)
            cx, cz = e[3], e[4]
            P.FL.add(t_, up(cx, cz, 0.15), P.FL_COL[e[2]], radius=2.4, peak=0.9, decay=0.5)  # low: under the bell's rim
            P.PA.emit(t_, up(cx, cz, 0.06), 'dust', n=30, spread=180, speed=(0.6, 1.8), scale=1.3)
            P.PA.emit(t_, up(cx, cz, 0.2), 'brass', n=14, spread=180)
            P.MK.add(t_, 'crack', cx, cz, size=1.5, n=8, life=7.0, seed=int(t_ * 10))
            P.MK.add(t_, 'scorch', cx, cz, size=1.2, col=(70, 60, 30), life=7.0, seed=int(t_ * 10) + 1, strength=0.5)

    def fx_after(self):
        P = self.P
        for e in P.EV:                                             # PEDAL TONE's bell hovers mint-lit
            if e[1] == 'ability' and e[4] == 'slam' and e[2] == self.k and e[0] < P.T_KO:
                sl = [q for q in P.EV if q[1] == 'slam' and q[2] == e[2] and q[0] >= e[0]]
                if sl:
                    P.FL.add(e[0] + 0.15, P.up(sl[0][3], sl[0][4], 2.4), P.FL_COL[e[2]], radius=2.6, peak=0.65,
                             decay=max(0.3, sl[0][0] - e[0]))

    def ko_fx(self, pos):
        self.P.PA.emit(self.P.T_KO, pos, 'brass', n=30, spread=180, speed=(0.8, 2.2))

    # ---------------------------------------------------------------- canvas
    def shadows(self, st, spot):
        if st['shattered']:
            return []
        quads, _ = slide_geom(st['p'], st['th'], st['ext'])
        return [proj_point(Q, spot) for Q, c, kind in quads if kind in ('outer', 'crook')]

    def canvas(self, Ic, C):
        P, k = self.P, self.k
        st = C.fs[k ^ 1]                                           # SAD TROMBONE: the slowed enemy drags a droopy wobble
        if C.in_fight and st['slow'] > 0:
            sk = min(1.0, st['slow'] / 0.25)
            rd = np.hypot(C.sxp - st['p'][0], C.szp - st['p'][1])
            ang_ = np.arctan2(C.szp - st['p'][1], C.sxp - st['p'][0])
            for jj, rr in enumerate((R * 1.25, R * 1.6)):
                wob = rr + 0.07 * np.sin(5 * ang_ - 11 * C.t + jj * 2.0)
                band = np.clip(1 - np.abs(rd - wob) / 0.05, 0, 1)
                Ic += (band * 0.55 * sk * (1 - 0.35 * jj))[:, None] * (P.COL[k] / 255.0 + 0.15)[None]
        st = C.fs[k]                                               # the lance's line of attack, lit as it thrusts
        if C.in_fight and st['thr_k'] > 0.05:
            d_ = np.array([math.cos(st['th']), math.sin(st['th'])])
            rx, rz = C.sxp - st['p'][0], C.szp - st['p'][1]
            along = rx * d_[0] + rz * d_[1]
            across = np.abs(-rx * d_[1] + rz * d_[0])
            lane = np.clip(1 - across / 0.22, 0, 1) * (along > R) * (along < R + st['ext'] + 0.1)
            lane = np.floor(lane * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
            Ic += (lane * 0.30 * st['thr_k'])[:, None] * (P.COL[k] / 255.0 * 0.8 + 0.2)[None]
        return Ic

    def canvas_wave(self, Ic, C, w):
        owner, cx, cz, r, rmax, kind, t0 = w
        if kind != 'shockwave':
            return Ic
        rd = np.hypot(C.sxp - cx, C.szp - cz)                      # SAD TROMBONE's ring
        k_ = (1 - r / rmax) * 1.0
        band = np.clip(1 - np.abs(rd - r) / 0.16, 0, 1)
        band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
        Ic += (band * k_ * 0.9)[:, None] * (self.P.COL[owner] / 255.0 * 1.4 + 0.15)[None]
        return Ic

    def canvas_after(self, Ic, C):
        P = self.P
        sl = P.RUN['SLAMS'][C.i] if C.in_fight else []
        for owner, cx, cz, t0, t_slam, rad in sl:                  # PEDAL TONE's target + stomp ring (#34)
            if owner != self.k:
                continue
            rd = np.hypot(C.sxp - cx, C.szp - cz)
            if C.t < t_slam:
                u = (C.t - t0) / (t_slam - t0)
                m = rd < rad
                Ic[m] *= 1 - 0.55 * u
                ring = np.abs(rd - rad) < 0.05
                Ic[ring] += (P.COL[owner] / 255.0) * (0.4 + 0.5 * (math.sin(C.t * 40) > 0))
            else:
                u = (C.t - t_slam) / 0.6
                band = np.clip(1 - np.abs(rd - (rad + 1.6 * u)) / 0.2, 0, 1) * (1 - u)
                Ic += band[:, None] * np.array([0.9, 0.9, 1.0])[None]
        return Ic

    # ---------------------------------------------------------------- the lance, the bell, shards
    def items_ball(self, F, st):
        if st['shattered']:
            return []
        tip = st['p'] + (R + st['ext'] * 0.55) * np.array([math.cos(st['th']), math.sin(st['th'])])
        return [(F.cam.depth(S.to_world(tip[0], tip[1], R))[0] + 0.01, 'slide')]

    def draw_item(self, F, payload):
        P, k = self.P, self.k
        st = F.fs[k]
        fr, cam = F.fr, F.cam
        prev = P.fighter(k, max(0.0, F.t - 1.0 / FPS))
        arr, depth = fr.array(), fr.depth()
        if st['ext'] - prev['ext'] > 0.12 and abs(base.wrap(st['th'] - prev['th'])) < 0.3:   # a thrust going out: streak
            a_ = 0.55 if st['haste'] > 0 else 0.38
            P.layer_blend(arr, slide_trail_polys(cam, st, prev['ext']), P.COL[k] * 0.6 + 100, a_)
        fr.set_array(arr, depth)
        hk = 0.6 if st['haste'] > 0 else 0.0
        draw_slide(fr, cam, F.L, P.shade, st, P.COL[k], max(hk, 0.8 * st['thr_k']))

    def over_ring(self, F, arr, depth):
        P = self.P
        if not F.in_fight:
            return arr, depth
        fr = F.fr
        for owner, cx, cz, t0, t_slam, rad in P.RUN['SLAMS'][F.i]:
            h = bell_height(F.t, t0, t_slam) if owner == self.k else None   # PEDAL TONE: the giant bell stomps
            if h is not None:
                fr.set_array(arr, depth)
                draw_giant_bell(fr, F.cam, F.L, P.shade, cx, cz, rad, h, P.COL[owner])
                lip = F.cam.wp(S.to_world(cx, cz, h + 0.15)[None])[0]          # a mint halo under the lip
                fr.glow(lip[0], lip[1], 34, tuple(P.COL[owner] * 0.8 + 40), 0.32)
                arr, depth = fr.array(), fr.depth()
        return arr, depth

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        if kind == 'tube':                                         # a curl of brass tubing
            pts = [np.array([sx_, sz_]) + 0.16 * np.array([math.cos(rot + j * 0.5 * (1 + curl)), math.sin(rot + j * 0.5 * (1 + curl))])
                   for j in range(7)]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
            ImageDraw.Draw(im).line([tuple(q) for q in sp], fill=tuple(int(v) for v in BRASS), width=2)
            return np.asarray(im).astype(np.float32)
        if kind == 'slidebit':
            q4 = [np.array([sx_, sz_]) + d * 0.22 * a1 + nn * 0.025 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            P.layer_blend(arr, [sp], BRASS_HI, 1.0)
            return arr
        col = {'bellbit': BRASS, 'brass': BRASS}[kind]             # a curved chip of the bell, brass chips
        sz = size * (1.5 if kind == 'bellbit' else (1.3 if kind == 'brass' else 0.8))
        tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
               np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
        shade_k = 0.75 + 0.25 * math.cos(rot * 2)
        P.layer_blend(arr, [sp], np.clip(col * shade_k, 0, 255), 1.0)
        return arr

    # ---------------------------------------------------------------- lens, glows
    def shimmer(self, img, cam, t, w):
        owner, cx, cz, r, rmax, kind, t0 = w
        if kind == 'shockwave':
            img = fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r, 0.4, 3.0, t, k=1 - r / rmax, seed=owner)
        return img

    def shimmer_after(self, img, cam, t):
        P = self.P
        for e in P.EV:
            if e[1] == 'slam' and e[2] == self.k and 0 <= t - e[0] < 0.6 and t < P.T_HS:
                u = (t - e[0]) / 0.6
                rad = self.cfg['abilities']['low']['radius']
                img = fx.shimmer_world(img, cam, S.to_world(e[3], e[4], 0.05), rad + 1.6 * u, 0.35, 3.0, t, k=1 - u)
        return img

    def glows(self, F, drawn):
        st = F.fs[self.k]                                          # the crook flares as the lance strikes
        if st['thr_k'] > 0.2 and not st['shattered'] and st['vis']:
            tp = st['p'] + (R + st['ext'] + 0.07) * np.array([math.cos(st['th']), math.sin(st['th'])])
            s_ = F.cam.wp(S.to_world(tp[0], tp[1], R)[None])[0]
            F.fr.glow(s_[0], s_[1], 7, tuple(self.col * 0.6 + 90), 0.35 * st['thr_k'])

    def focus_points(self, t, st):
        P = self.P
        out = []
        if not st['shattered']:
            tip = st['p'] + (R + st.get('ext', st['blen'])) * np.array([math.cos(st['th']), math.sin(st['th'])])
            out.append((S.to_world(tip[0], tip[1], R), 'lance tip'))
        for cx, cz, t0, t_slam, rad in self.bell_down(t):          # the bell once it stomps onto the canvas (high
            h = bell_height(t, t0, t_slam)                         # above it, it is scenery, as in #34)
            if h is not None and h <= 1.0:
                out.append((S.to_world(cx, cz, h), 'pedal bell'))
        return out


# ===================================================================================================== sound (#41)
class Sound(base.Sound):
    bus_gain, send = 1.8, 0.2
    band = (70, 700)                                               # the bass line E2-C3 + its brass harmonics and smears

    def bone(self, m, dur=0.16, vel=0.8, gl=2.0, gl_t=0.035, cap=None):
        """a trombone note gliding up `gl` semitones into the pitch (the slide) (#34)"""
        M = self.M
        x = M.note('TROMBONE', m, round(dur, 3), cap or round(dur + 0.3, 2), round(vel, 1))
        return M.gliss(x, gl, gl_t) if gl else x

    # the bass role: the chords' roots and fifths in the trombone's register (E2 .. ), POWER SLIDE's walk
    def root(self, chord):
        return self.T.root_above(chord, self.M.CFG[self.k]['register'][0])

    def fifth(self, chord):
        r = self.root(chord)
        return r - 5 if r - 5 >= self.M.CFG[self.k]['register'][0] else r + 7

    def bass_note(self, t):
        """the circus bass on the beat: root on beats 1 + 3, the fifth on 2 + 4"""
        T = self.T
        c = T.chord_at(t)
        b = int(round(t / T.BEAT)) % 4
        return self.root(c) if b in (0, 2) else self.fifth(c)

    def walk_note(self, t):
        """POWER SLIDE walks 8ths: root, 3rd, 5th, a chromatic approach into the next bar's root (8 per bar)"""
        T = self.T
        lo, hi = self.M.CFG[self.k]['register']
        c = T.chord_at(t)
        i = int(round(t / (T.BEAT / 2))) % 8
        r = self.root(c)
        nxt = self.root(T.chord_at(t + T.BAR * 0.51)) if i >= 6 else r
        pcs = T.CHORD_PCS[c]
        third = r + ((pcs[1] - pcs[0]) % 12)
        fifth = r + 7
        table = [r, r, third, third, fifth, fifth, nxt + 2 if nxt + 2 <= hi else nxt - 1, nxt + 1]
        return max(lo, min(hi, table[i]))

    def title_blast(self, mix, t):
        hits = self.T.TITLE_HITS.get(self.name, (40, 52))
        mix.place(self.bus, self.bone(hits[0], 0.7, 1.0, 0, cap=1.0), t, 0.42)       # the low note + a big smear up
        mix.place(self.bus, self.bone(hits[-1], 0.6, 1.0, 12.0, 0.26, cap=0.95), t, 0.34)

    def count_in(self, mix):
        T = self.T
        ch = T.CHORDS[1]
        notes = (self.root(ch), self.fifth(ch), self.root(ch))
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            mix.place(self.bus, self.bone(notes[j], 0.25, 0.95, 3.0, 0.05), tc, 0.40)
        mix.place(self.bus, self.bone(self.root(ch), 0.4, 1.0, 7.0, 0.3), T.BAR + 3 * T.BEAT, 0.36)  # pickup smear

    def play(self, mix):
        M, T, k = self.M, self.T, self.k
        A, RUN, EV, q16 = M.A, M.RUN, M.EV, M.q16
        thin = M.Thinner(7)
        last_slot = None
        for e in EV:                                               # the bass line: a smear into every thrust
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'thrust' and e[2] == k:
                tq = q16(t)
                slot = round(tq / T.S16)
                if slot == last_slot:
                    continue
                last_slot = slot
                hp = M.hp_at(k, t) / 100.0
                if not thin.keep(slot, hp):
                    continue
                hasted = float(RUN['HA'][M.P.sim_i(t), k]) > 0
                on_beat = slot % 4 == 0
                if self.role == 'riff':                            # #48 (opt-in): a riff stab, smeared up into it
                    m = T.line_at(self.name, tq)
                    dur = 0.15 if hasted else 0.24
                    sig, g, lead = self.colour(self.bone(m, dur, 0.9 if on_beat else 0.75, 3.0 if on_beat else 1.5,
                                                         0.05), t, 0.8)
                    mix.place(self.bus, sig, tq, (0.42 if on_beat else 0.34) * g, pan=0.15)
                    if lead > 0.04 and on_beat:                    # the leader: doubled an octave down, fuller
                        mix.place(self.bus, self.colour(self.bone(m - 12, dur, 0.6, 2.0, 0.05), t, low=True)[0], tq,
                                  0.14 * g, pan=0.2)
                    continue
                m = self.walk_note(tq) if hasted else self.bass_note(tq)
                dur = 0.17 if hasted else 0.30
                sig, g, lead = self.colour(self.bone(m, dur, 0.85 if on_beat else 0.7, 3.0 if on_beat else 1.5, 0.05), t,
                                           0.8, low=True)
                mix.place(self.bus, sig, tq, (0.44 if on_beat else 0.34) * g, pan=0.15)
                if lead > 0.04 and on_beat:                        # the leader: doubled an octave up, fuller
                    mix.place(self.bus, self.colour(self.bone(m + 12, dur, 0.6, 2.0, 0.05), t)[0], tq, 0.14 * g, pan=0.2)
        for e in EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'hit' and e[2] == k and e[7] == 'blade':     # a landed lance: a big smear up the chord
                tq = q16(t)
                ct = T.chord_tones(52, 62, T.chord_at(tq + 0.01))
                m = ct[int(e[8]) % len(ct)]
                sig, g, _ = self.colour(self.bone(m, 0.22, 1.0, 7.0, 0.11), t)
                mix.place(self.bus, sig, tq, 0.26 * g, pan=float(np.clip(e[5] / 3.5, -0.6, 0.6)))
                mix.place('fx', A.thud(0.22, 150, 70, seed=int(t * 7)), tq, 0.08)
            elif kind == 'hit' and e[2] == k and e[7] == 'shockwave':
                mix.place('fx', A.thud(0.4, 120, 45), q16(t), 0.22)
            elif kind == 'ability' and e[2] == k:
                name, tq = e[3], q16(t)
                ab = self.M.CFG[k]['abilities']
                if name == ab['unlock1']['name']:                  # SAD TROMBONE: wah wah wah waaah, down to the tonic
                    for j, m in enumerate((55, 54, 53, 52)):
                        d = 0.5 * T.BEAT if j < 3 else 1.6 * T.BEAT
                        x = self.bone(m, d, 0.9, 1.0 if j else 0.0, 0.04, cap=round(d + 0.35, 2))
                        if j == 3:
                            x = M.vibrato(x, 0.5, 5.0, 0.2)
                        x = M.wah(x, [(0.0, min(0.18, d * 0.6))])
                        mix.place(self.bus, self.colour(x, t)[0], tq + j * 0.5 * T.BEAT, 0.42, pan=0.1)
                    mix.place('fx', A.whoosh(0.6, 200, 1500, rise=False), tq, 0.08)
                elif name == ab['unlock2']['name']:                # POWER SLIDE: a long rip up two octaves into the walk
                    mix.place(self.bus, self.colour(self.bone(64, 0.45, 1.0, 24.0, 0.32), t)[0], tq, 0.30)
                elif name == ab['low']['name']:                    # PEDAL TONE: the pedal E under the hovering bell
                    mix.place(self.bus, self.bone(28, 2 * T.BEAT, 1.0, 2.0, 0.1, cap=round(2 * T.BEAT + 0.3, 2)), tq, 0.40)
                    mix.place(self.bus, self.bone(40, 2 * T.BEAT, 0.9, 0.0, cap=round(2 * T.BEAT + 0.3, 2)), tq, 0.30)
                    mix.place('fx', A.whoosh(2 * T.BEAT, 150, 1800), tq, 0.10)
            elif kind == 'slam' and e[2] == k:                     # PEDAL TONE: the bell stomps
                ts = e[0]
                M.bed.boom(mix, ts, 0.8)
                mix.place('fx', A.thud(0.8, 90, 30), ts, 0.5)
                mix.place('fx', M.clank(64, 9), ts, 0.10)
                mix.place(self.bus, self.bone(40, 0.5, 1.0, 0), ts, 0.36)
                mix.place('drums', A.crash(1.6), ts, 0.10)

    def fall_apart(self, mix):                                     # the slide falls apart: a falling wail, clattering tubing
        M = self.M
        x = self.bone(52, 0.9, 1.0, 0, cap=1.3)
        mix.place(self.bus, M.gliss(x, -10, 0.9), M.P.T_HS, 0.26)
        for j, m in enumerate((80, 75, 85, 71)):
            mix.place('fx', M.clank(m, 30 + j), M.P.T_HS + 0.1 * j, 0.08, pan=(j - 1.5) / 4)

    def stinger(self, mix, tw):
        mix.place('solo', self.bone(40, 1.0, 1.0, 0, cap=1.5), tw, 0.40)
        for m, g in ((52, 0.30), (55, 0.24), (59, 0.20)):
            mix.place('solo', self.bone(m, 1.0, 1.0, 5.0, 0.12, cap=1.5), tw, g)

    def solo(self, mix, t0):
        """the solo on the slide (each note glides from the last), then back to the bass line for the reassembly"""
        M, T = self.M, self.T
        notes = T.SOLO[self.name]
        t_end = t0 + max(bt + d for bt, m, d in notes) * T.BEAT
        for j in range(int(math.ceil(t_end / T.BEAT)), int((T.N_BARS - 1) * T.BAR / T.BEAT + 0.5)):
            tb = j * T.BEAT
            m = T.line_at(self.name, tb) if self.role == 'riff' else self.bass_note(tb)     # #48: back to its riff
            mix.place('solo', self.bone(m, 0.3, 0.85, 3.0, 0.05), tb, 0.40)
        prev = None
        for bt, m, d in notes:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:                         # a late K.O. leaves less room: stop at the reassembly
                break
            gl = float(np.clip(m - prev, -9, 9)) if prev is not None else 5.0
            mix.place('solo', self.bone(m, d * T.BEAT * 0.95, 0.95, gl, 0.08, cap=round(d * T.BEAT + 0.35, 2)), ts, 0.50)
            prev = m

    def reassemble(self, mix):
        T, M = self.T, self.M
        for j, m in enumerate((40, 45, 47, 52, 55, 59)):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / 6)
            mix.place(self.bus, self.bone(m, 0.12, 0.7), tj, 0.20)

    def last_bar(self, mix):
        """the bass climbs the last chord (the dominant) into the title slam"""
        if self.role != 'bass':
            return
        T = self.T
        t_last = (T.N_BARS - 1) * T.BAR
        ch = T.CHORDS[-1]
        r = self.root(ch)
        climb = T.chord_tones(r, r + 11, ch)[:4]
        for bt in range(4):
            mix.place(self.bus, self.bone(climb[bt % len(climb)], 0.35, 0.9, 3.0, 0.06), t_last + bt * T.BEAT, 0.42)
