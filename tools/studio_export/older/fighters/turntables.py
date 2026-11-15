"""TURNTABLES (lime), as built in #31 v3 (Turntables vs Synth, projects/ballfight_turntables_vs_synth): THE RING IS THE
RECORD (weapon kind 'scratch_floor': every `every` beats the DJ scratches the record the ring sits on, the floor jerks
forward then back and flings the opponent into the ropes; a rope slam within `window` s is a 'scratch' hit), SCRATCH
(haste: scratches twice as often), REWIND (rewind: undoes the last 2 s of damage taken, zipping back along its own path,
intangible while it zips), BASS DROP (slam: a giant record drops onto the predicted spot).

Look (#31's render.py layers, moved into the kit's hooks with their names): the canvas IS the record (slate vinyl,
grooves, sheen, strobe dots, the lime label), turning with the floor angle (a slow spin + every scratch's jerk, the
curve the scratch audio plays; the champion's solo scratches turn it too) through the opt-in `canvas_albedo`; the
vinyl's gloss under the spot (`canvas_first`); the decks thump on every jerk (`state`: glow); the instrument tipped
toward the camera (its `pitch`) and wiggling while it scratches (`ball_sprite`); the tonearm from its pivot on the apron
down onto the DJ's ball, with its shadow (`items_free`, `shadows_free`); REWIND's ghost rings along the zip
(`items_free`) and the tape look around the zipping ball with its rewind glyph (the opt-in `after_post`); BASS DROP's
darkening target (`canvas_after`), the giant record hanging, dropping (`over_ring`) and lying under the balls
(`items_free`), its shimmer (`shimmer_after`), crack and scorch; every scratch hit's burst, lime light, shards and
crack, every jerk's floor light and streaks (`fx_event`); its sparks, flashbulbs, glows, fringes and shakes; the
champion's groove rings of light (`canvas`); its shatter (vinyl, label, tonearm, plinth chips); the VS card's record
grooves (`title_pattern`) and its tipped close-up (`title_closeup`).

Sound (#31's audio.py, the decks' part, moved into the hooks): every scratch IS the floor's motion (the record under
the needle follows the picture's scratch curve, played through a chopped F-minor stab) with a thump under each jerk;
every rope slam an organ stab chord in the decks' Pixel Pitch voice (the root doubled when leading); SCRATCH a scribble,
REWIND a backspin with the tape whirring backwards, BASS DROP a whoosh and then the sub drop, a thud and a stab; the
title blast (a flare over a stab chord), the count-in chirps, the tape-stopped baby scratch when it falls apart, the
stinger (a backspin into a stab chord), the scratch routine solo (config SOLO entries (beat, kind, beats), kind a
scratch or 'stab'), the reassembly run. Its bus is `turntables` (#31's `decks`).

Draw functions come from the episode's own styles.py, imported from its scratch copy (older/origin.py `scratch`), never
retyped; the scratch and stab synths, the harmony and the thinning hash are #31's audio.py functions, moved here."""
import importlib.util
import math
import os
import sys

import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, BAYER_FULL, SAFE, ss, ease
from reelkit.lighting import shade
from reelkit.shadows import proj_point
from reelkit import fx
from reelkit import ui
import scene as S

from . import base

PORTED = True
EPISODE = 'ballfight_turntables_vs_synth'
R = S.R_BALL
GHOST_Z = 2e9                                  # items_free depths, drawn first in the painter pass (#31: under the balls):
UNDER_Z = 5e8                                  # REWIND's ghosts, then (SYNTH's cables at 1e9), the lying record, the arm


# ---------------------------------------------------------------- #31's styles.py, from its scratch copy
def styles():
    """the episode's styles.py (its draw functions), imported from its scratch copy under a module name of its own"""
    name = f'older_styles_{EPISODE}'
    if name in sys.modules:
        return sys.modules[name]
    from tools.studio_export.older import origin as O
    src = os.path.join(O.PROJECTS, EPISODE, 'styles.py')
    path = os.path.join(O.ROOT, EPISODE, 'styles.py')
    if not (os.path.isfile(path) and open(path, 'rb').read() == open(src, 'rb').read()):
        O.scratch(EPISODE)                     # (only when missing: a running episode keeps its folder as it is)
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


ST = styles()

# ---------------------------------------------------------------- #31 render.py: the floor angle (picture + audio)
ANG_PER_SEC = 2 * math.pi / 1.8       # record audio seconds -> floor angle (33 1/3 rpm: 1.8 s a turn)
SCR_AMP = 0.40                        # floor angle (rad) of a scratch at strength 1.3 (grows with the strength)
_PP = None


def pp_turntables():
    """the decks' own Pixel Pitch module (its scratch motion curves), read-only"""
    global _PP
    if _PP is None:
        import pp_bridge as B
        _PP = B.module('turntables60_lowpoly')
    return _PP


def scratch_kind(name):
    TT = pp_turntables()
    return {'baby': TT.BABY, 'chirp': TT.CHIRP, 'trans': TT.TRANS, 'flare': TT.FLARE, 'tear': TT.TEAR,
            'scrib': TT.SCRIB, 'back': TT.BACKSPIN, 'fwd': TT.FWD}[name]


class Record:
    """#31's floor: the scratches of side k (from the run) and its solo, as the curves render.py's scratch_q, solo_q and
    floor_angle give them (P: kit.picture)"""

    def __init__(self, P, k):
        self.P, self.k, T = P, k, P.T
        self.spin_p = 5 * T.BAR                # the record turns once every 5 bars: 7 whole turns in 61 s (loop-safe)
        self.scr_ev = [(e[0], e[3], e[4], e[5]) for e in P.EV if e[1] == 'scratch' and e[2] == k]
        self.scr_t = np.array([s[0] for s in self.scr_ev]) if self.scr_ev else np.zeros(0)
        name = P.NAMES[k]
        self.solo_ev = [(P.T_SOLO + bt * T.BEAT,) + tuple(x) for bt, *x in T.SOLO.get(name, [])] if P.WIN == k else []

    def scratch_q(self, t):
        """the record's position under the DJ's hand (seconds of record audio) from the fight's scratches: a baby
        scratch (forward, then back) over each scratch's length; 0 between scratches"""
        P = self.P
        tc = P.fx_clock(t)
        j = np.searchsorted(self.scr_t, tc, 'right') - 1
        if j < 0 or tc >= P.T_KO + 0.01 and t >= P.T_HS:
            return 0.0
        t0, d, stg, sgn = self.scr_ev[j]
        u = (tc - t0) / d
        if not 0 <= u < 1:
            return 0.0
        a = SCR_AMP * stg / 1.3 / ANG_PER_SEC
        return sgn * a * (1 - math.cos(2 * math.pi * u)) / 2

    def solo_q(self, t):
        """the champion's solo scratches move the record too (the deck's own motion curves, same as the audio)"""
        P = self.P
        if P.WIN != self.k or t < P.T_SOLO:
            return 0.0
        for e in self.solo_ev:
            t0, kind, ln = e[0], e[1], e[2]
            d = ln * P.T.BEAT * 0.95
            if isinstance(kind, str) and kind != 'stab' and t0 <= t < t0 + d:     # (a note: a stab)
                q, g = pp_turntables().qmotion(scratch_kind(kind), np.array([t - t0]), d)
                return float(q[0]) * (0.35 if kind == 'back' else 1.0)
        return 0.0

    def floor_angle(self, t):
        return 2 * math.pi * t / self.spin_p + (self.scratch_q(t) + self.solo_q(t)) * ANG_PER_SEC


# ---------------------------------------------------------------- #31 render.py: drawing helpers
def layer_blend(arr, polys, col, alpha):
    if not polys:
        return
    m = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(m)
    for pts in polys:
        if len(pts) >= 3:
            d.polygon([tuple(p) for p in pts], fill=255)
    mk = np.asarray(m, np.float32) / 255.0 * alpha
    arr += (np.array(col, float)[None, None] - arr) * mk[..., None]


def line_blend(arr, pts, col, alpha, width=1):
    """alpha-blend a screen polyline onto arr"""
    if len(pts) < 2 or alpha <= 0:
        return
    m = Image.new('L', (W, H), 0)
    ImageDraw.Draw(m).line([tuple(p) for p in pts], fill=255, width=width)
    mk = np.asarray(m, np.float32) / 255.0 * alpha
    arr += (np.array(col, float)[None, None] - arr) * mk[..., None]


def screen(cam, P):
    return cam.wp(np.atleast_2d(P))


def ring_outline(arr, cam, cx, cz, r, y, col, alpha, width=1):
    s = cam.wp(ST.circle_world(cx, cz, r, y, 28))
    line_blend(arr, list(s) + [s[0]], col, alpha, width)


# ---------------------------------------------------------------- #31 render.py: the tonearm
PIVOT = np.array([-3.42, 1.55])                 # on the apron, by the DJ's (left) corner
REST_TIP = np.array([-3.38, -0.55])             # parked along the apron


class Look(base.Look):
    spark_big = ('slam', 'wave', 'shockwave', 'cannon', 'scratch')      # #31's big spark spray (+ its rope slams)
    flash_hits = ('slam', 'wave', 'shockwave', 'scratch')               # #31's flashbulbs (+ its rope slams)
    fringe_hits = ('slam', 'wave', 'shockwave', 'scratch')              # #31's chromatic fringes (+ its rope slams)
    shards = ['vinyl'] * 8 + ['label'] * 3 + ['arm'] * 2 + ['plinth'] * 3   # #31's decks shatter (after the glass)

    def setup(self):
        P, k = self.P, self.k
        T = P.T
        self.rec = Record(P, k)
        self.pitch = float(self.cfg.get('pitch', 0))
        self.rw_win = []                               # REWIND windows: (t0, t1, heal) from the run's own events
        for e in P.EV:
            if e[1] == 'heal' and e[2] == k:
                t1 = next((f[0] for f in P.EV if f[1] == 'rewound' and f[2] == e[2] and f[0] >= e[0]), e[0] + T.BEAT)
                self.rw_win.append((e[0], t1, e[3]))
        self.jerk_t = np.array([e[0] for e in P.EV if e[1] == 'jerk' and e[2] == k])
        self.slam_ev = [e for e in P.EV if e[1] == 'slam' and e[2] == k]
        self.slam_rad = self.cfg['abilities']['low'].get('radius', 1.35)
        self.t_drop = (T.T_FIGHT - 0.42, T.T_FIGHT)     # the arm swings over and drops on FIGHT!
        self._spr = {}
        self._on_rec = None

    # ---------------------------------------------------------------- the VS card
    def title_pattern(self, base_):
        """#31's _grooves: record grooves around the far corner, track gaps, a label arc in its colour"""
        import title as TT
        r = np.hypot(TT.XX, TT.YY)
        g = 0.5 + 0.5 * np.sin(r * 2 * math.pi / 6.0)
        base_ = base_ * (0.88 + 0.14 * g[..., None])
        gap = np.abs((r % 84.0) - 42.0) < 1.6
        base_ = np.where(gap[..., None], base_ * 1.2, base_)
        lab = r < 46
        return np.where(lab[..., None], base_ * 0.55 + np.clip(self.col, 0, 255) * 0.45, base_)

    def title_closeup(self, r):
        """the card's big ball: the model tipped toward us (#31: render_sprite with its pitch)"""
        yc, sw, roll = self.cfg['view']
        return ST.render_sprite(self.cfg['pp'], r, float(yc), float(roll), self.pitch)

    # ---------------------------------------------------------------- the ball
    def sprite(self, r, yaw, roll):
        """the instrument tipped toward the camera (#31 render.sprite, its `pitch`), radius r px, yaw a 6 deg step"""
        key = (r, int(yaw) % 360, int(roll))
        if key not in self._spr:
            self._spr[key] = ST.render_sprite(self.cfg['pp'], r, float(yaw), float(roll), self.pitch)
        return self._spr[key]

    def ball_sprite(self, kk, st, t, rp, yaw):
        """#31 draw_ball: the decks wiggle while they scratch (SCRATCH's haste, and the solo)"""
        P = self.P
        if kk != self.k:
            return None
        if st['haste'] > 0 or (self.k == P.WIN and P.solo_env(t) > 0.2 and t > P.T_SOLO):
            yaw += 12 * math.sin(2 * math.pi * t / (P.T.BEAT / 2))
        roll = self.cfg['view'][2]
        r = int(np.clip(round(rp * self.sprite_k), 12, 30))
        yk = int(round(yaw / 6.0)) * 6
        return self.sprite(r, yk, roll), None

    def state(self, kk, st, t, phase, gl=0.0):
        """the decks thump on every jerk of the floor (#31 fighter())"""
        P = self.P
        if kk != self.k or phase != 'fight' or not len(self.jerk_t):
            return
        j = np.searchsorted(self.jerk_t, min(t, P.T_KO), 'right') - 1
        if j >= 0:
            st['glow'] = 0.5 * math.exp(-(min(t, P.T_KO) - self.jerk_t[j]) / 0.1)

    # ---------------------------------------------------------------- events -> camera, ropes, sparks, glows
    def shake(self, e):
        if e[1] == 'hit' and e[7] == 'scratch':                     # a rope slam
            return 1.4
        if e[1] == 'hit' and e[7] == 'slam':
            return 2.5
        return super().shake(e)

    def rope(self, e):
        if e[1] == 'jerk':                                          # every jerk of the floor shakes the ropes a little
            return 0.35
        return super().rope(e)

    def hit_glow(self, e):
        return (12, 0.35) if e[7] in ('slam', 'scratch') else None

    def fringe_size(self, e):
        return (26, 2.5) if e[7] in ('slam', 'wave', 'shockwave') else (16, 1.4)

    # ---------------------------------------------------------------- effects
    def fx_event(self, e):
        P = self.P
        t0, kind = e[0], e[1]
        if kind == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            pos = S.to_world(x, z, R)
            if how == 'scratch':                                    # a rope slam
                P.BU.add(t0, pos, P.COL[att], 1.2)
                P.FL.add(t0, pos, P.COL[att], radius=1.8, peak=0.8, decay=0.3)
                P.PA.emit(t0, pos, 'shards', n=8)
                P.MK.add(t0, 'crack', x, z, size=0.45, n=4, life=4.0, seed=int(t0 * 100))
            elif how == 'slam':
                P.BU.add(t0, pos, P.COL[att], 1.6)
        elif kind == 'jerk':                                        # the whole floor lights up on every jerk
            P.FL.add(t0, S.to_world(0.0, 0.0, 1.0), P.COL[e[2]], radius=2.4, peak=0.32, decay=0.22)
            x, z, sgn = e[4], e[5], e[3]
            rn = math.hypot(x, z) + 1e-6
            tang = S.to_world(-z / rn * sgn, x / rn * sgn, 0.0)     # streaks fly along the fling
            P.PA.emit(t0, S.to_world(x, z, 0.3), 'streaks', n=3, dir=tuple(tang), col=[tuple(P.COL[e[2]] * 0.6 + 100)])
        elif kind == 'slam':
            ts, owner, cx, cz = e[0], e[2], e[3], e[4]
            c = S.to_world(cx, cz, 0.1)
            P.FL.add(ts, S.to_world(cx, cz, 1.2), P.COL[owner], radius=2.6, peak=1.2, decay=0.5)
            P.PA.emit(ts, c, 'shards', n=24, spread=180, speed=(1.2, 3.6), scale=1.3)
            P.MK.add(ts, 'crack', cx, cz, size=1.4, n=8, life=7.0, seed=int(ts * 10))
            P.MK.add(ts, 'scorch', cx, cz, size=1.1, col=(60, 70, 30), life=7.0, seed=int(ts * 10) + 1, strength=0.6)

    def ko_fx(self, pos):
        self.P.PA.emit(self.P.T_KO, pos, 'shards', n=24, spread=180, speed=(0.8, 2.6))

    # ---------------------------------------------------------------- the canvas: THE RECORD
    def canvas_albedo(self, alb, C):
        """(older opt-in) the record floor turning with the floor angle (#31 render 1184: ST.record_albedo)"""
        alb, self._on_rec = ST.record_albedo(alb, C.sxp, C.szp, self.rec.floor_angle(C.t), self.col)
        return alb

    def canvas_first(self, Ic, C):
        """the vinyl's gloss under the spot (#31 render 1197)"""
        if self._on_rec is not None:
            Ic[self._on_rec] *= 1.12
            self._on_rec = None
        return Ic

    def solo_rings(self, t):
        P = self.P
        out = []
        for e in self.rec.solo_ev:
            t0 = e[0]
            if t0 <= t < t0 + 0.6:
                out.append((t0, 1.9 if not isinstance(e[1], str) or e[1] == 'stab' else 1.1))   # (a note: a stab)
        return out

    def canvas(self, Ic, C):
        """the champion's solo: groove rings of light (#31 render 1198-1206)"""
        P = self.P
        t = C.t
        if P.WIN != self.k or not t >= P.T_SOLO - 0.01:
            return Ic
        sxp, szp = C.sxp, C.szp
        for t0, reach in self.solo_rings(t):
            st = C.fs[self.k]
            u = t - t0
            r = R + min(reach, 6.0 * u)
            a = 1.0 if u < reach / 6.0 else max(0.0, 1 - (u - reach / 6.0) / 0.25)
            rd = np.hypot(sxp - st['p'][0], szp - st['p'][1])
            band = np.clip(1 - np.abs(rd - r) / 0.16, 0, 1) * (rd < r + 0.05)
            band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
            Ic += (band * a * 0.5)[:, None] * (self.col / 255.0 * 0.9 + 0.25)[None]
        return Ic

    def canvas_after(self, Ic, C):
        """BASS DROP's target darkening with a flashing rim, then the stomp ring (#31 render 1222-1234)"""
        P = self.P
        t = C.t
        sxp, szp = C.sxp, C.szp
        for owner, cx, cz, t0, t_slam, rad in (P.RUN['SLAMS'][C.i] if C.in_fight else []):
            if owner != self.k:
                continue
            rd = np.hypot(sxp - cx, szp - cz)
            if t < t_slam:
                u = (t - t0) / (t_slam - t0)
                m = rd < rad * 1.1
                Ic[m] *= 1 - 0.6 * u
                ring = np.abs(rd - rad) < 0.05
                Ic[ring] += (P.COL[owner] / 255.0) * (0.4 + 0.5 * (math.sin(t * 40) > 0))
            else:
                u = (t - t_slam) / 0.6
                band = np.clip(1 - np.abs(rd - (rad + 1.6 * u)) / 0.2, 0, 1) * (1 - u)
                Ic += band[:, None] * np.array([0.9, 0.9, 1.0])[None]
        return Ic

    # ---------------------------------------------------------------- the tonearm (the DJ's needle anchors its ball)
    def arm_state(self, t):
        """(tip in ring frame, tip height, on the ball?) of the tonearm at time t"""
        P, k = self.P, self.k
        T = P.T

        def on_ball(tt):
            p = P.fighter(k, tt)['p']
            d = p - PIVOT
            return p - d / (np.hypot(*d) + 1e-9) * R * 0.92
        t_drop0, t_drop1 = self.t_drop
        if t < t_drop0:
            return REST_TIP, 0.85, False
        if t < t_drop1:
            u = ease((t - t_drop0) / (t_drop1 - t_drop0))
            tip = REST_TIP + (on_ball(t) - REST_TIP) * u
            return tip, 0.85 + (ST.ARM_Y - 0.85) * u ** 2, u > 0.95
        lift_t = T.T_GLIDE if k == P.WIN else P.T_HS
        if t < lift_t:
            return on_ball(t), ST.ARM_Y, True
        u = ease((t - lift_t) / 0.9)
        tip0 = on_ball(lift_t) if k == P.WIN else on_ball(min(t, lift_t))
        return tip0 + (REST_TIP - tip0) * u, ST.ARM_Y + (0.85 - ST.ARM_Y) * math.sin(math.pi * 0.5 * min(1.0, u * 1.6)), \
            False

    def shadows_free(self, F, spot):
        tip, h, on = self.arm_state(F.t)
        return [proj_point(Q, spot) for kind, Q in ST.tonearm(PIVOT, tip, h) if kind in ('arm', 'shell', 'weight')]

    def draw_tonearm(self, fr, cam, L, arm):
        tip, h, on = arm
        pieces = ST.tonearm(PIVOT, tip, h)
        for kind, Q in pieces:                                      # a dark outline so the silver arm reads
            if kind == 'arm':
                s = cam.wp(Q)
                fr.draw.polygon([tuple(p) for p in s], outline=(16, 14, 24))
                fr.draw.line([tuple((s[0] + s[3]) / 2), tuple((s[1] + s[2]) / 2)], fill=(16, 14, 24), width=4)
        for kind, Q in pieces:
            if kind == 'base':
                c = shade(L, np.array([[0, 1.0, 0]]), Q.mean(0)[None], np.array([40, 38, 50]), rimk=0.4)[0]
            elif kind == 'base2':
                c = shade(L, np.array([[0, 1.0, 0]]), Q.mean(0)[None], np.array([200, 204, 216]), rimk=0.6)[0]
            elif kind == 'weight':
                c = shade(L, np.array([[0, 1.0, 0]]), Q.mean(0)[None], np.array([150, 154, 168]), rimk=0.5)[0]
            elif kind == 'arm':
                c = shade(L, np.array([[0, 1.0, 0]]), Q.mean(0)[None], np.array([214, 218, 230]), rimk=0.7)[0]
            else:
                c = shade(L, np.array([[0, 1.0, 0]]), Q.mean(0)[None], np.array([30, 30, 38]), rimk=0.5)[0]
            fr.poly(cam, Q, c)
            if kind == 'shell':                                     # its colour on the headshell
                s = cam.wp(Q)
                fr.draw.line([tuple(s[0]), tuple(s[1])], fill=tuple(int(v) for v in self.col), width=1)

    # ---------------------------------------------------------------- REWIND, BASS DROP
    def rewinding(self, t):
        for t0, t1, heal in self.rw_win:
            if t0 - 0.06 <= t < t1 + 0.3:
                return ss(t0 - 0.06, t0 + 0.04, t) * (1 - ss(t1, t1 + 0.3, t)), (t0, t1)
        return 0.0, None

    def draw_drop(self, arr, cam, t, i, in_fight, falling):
        """BASS DROP: the giant record hangs high, drops onto the predicted spot, lies there a moment and fades"""
        P = self.P
        if not in_fight:
            return
        for owner, cx, cz, t0, t_slam, rad in P.RUN['SLAMS'][i]:
            if owner != self.k:
                continue
            h = ST.drop_height(t, t0, t_slam)
            if h is None or (h > 0.3) != falling:
                continue
            a = min(1.0, (t - t0) / 0.2) * (1.0 if t < t_slam + 0.35 else max(0.0, 1 - (t - t_slam - 0.35) / 0.2))
            for s_, c in ST.drop_polys(cam, cx, cz, rad, h, P.COL[owner], t * 3.0):
                layer_blend(arr, [s_], np.clip(c, 0, 255), a)

    def items_free(self, F):
        """under the balls, after the back ropes (#31 render 1256-1270, in its order): REWIND's ghosts, (SYNTH's
        patch cables at depth 1e9,) the record lying after BASS DROP, then the tonearm"""
        return [(GHOST_Z, 'ghosts'), (UNDER_Z, 'drop'), (UNDER_Z, 'arm')]

    def draw_item(self, F, payload):
        P, k = self.P, self.k
        t, cam = F.t, F.cam
        if payload == 'ghosts':
            rk, win_ = self.rewinding(t)
            if rk > 0 and win_ is not None and F.fs[k]['vis']:
                arr, depth = F.fr.array(), F.fr.depth()
                for g in range(1, 6):
                    tg = max(win_[0], t - 0.035 * g)
                    if tg >= t - 1e-6:
                        continue
                    pg = P.RUN['P'][P.sim_i(tg), k]
                    ring_outline(arr, cam, pg[0], pg[1], R * (1 - 0.06 * g), R, P.COL[k] * 0.7 + 70,
                                 0.55 * rk * (1 - g / 6.5), 2)
                F.fr.set_array(arr, depth)
        elif payload == 'drop':                                     # once down, the record lies under the balls
            arr, depth = F.fr.array(), F.fr.depth()
            self.draw_drop(arr, cam, t, F.i, F.in_fight, falling=False)
            F.fr.set_array(arr, depth)
        elif payload == 'arm':
            self.draw_tonearm(F.fr, cam, F.L, self.arm_state(t))

    def over_ring(self, F, arr, depth):
        """BASS DROP while it falls (over everything, before the shards)"""
        self.draw_drop(arr, F.cam, F.t, F.i, F.in_fight, falling=True)
        return arr, depth

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        """#31 draw_shards: the tonearm's pieces, and vinyl / label / plinth chips tumbling flat"""
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        c0 = np.array([sx_, sz_])
        if kind == 'arm':
            Lk, Wk = 0.46, 0.04
            c = np.array([206, 210, 222])
            q4 = [c0 + d * Lk / 2 * a1 + nn * Wk / 2 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            layer_blend(arr, [sp], c * (0.8 + 0.2 * math.cos(rot * 2)), 1.0)
        elif kind in ('vinyl', 'label', 'plinth'):
            col = {'vinyl': ST.VINYL * 1.3, 'label': P.COL[self.k] * 0.95, 'plinth': np.array([58, 56, 66])}[kind]
            sz = size * (1.5 if kind == 'vinyl' else 1.0)
            tri = [c0 + d * sz, c0 - d * sz * 0.7 + nn * sz * 0.8, c0 - d * sz * 0.4 - nn * sz * 0.7]
            sp = screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
            shade_k = 0.75 + 0.25 * math.cos(rot * 2)
            layer_blend(arr, [sp], np.clip(col * shade_k, 0, 255), 1.0)
            if kind == 'vinyl':
                g = (sp[0] * 0.5 + sp[1] * 0.25 + sp[2] * 0.25).astype(int)
                if 0 <= g[0] < W and 0 <= g[1] < H:
                    arr[g[1], g[0]] = ST.GROOVE * 1.4
        return arr

    # ---------------------------------------------------------------- lens, post
    def shimmer_after(self, img, cam, t):
        """BASS DROP's impact ring (#31 shimmers)"""
        P = self.P
        tc = P.fx_clock(t)
        for e in self.slam_ev:
            u = (tc - e[0]) / 0.6
            if 0 <= u < 1:
                img = fx.shimmer_world(img, cam, S.to_world(e[3], e[4], 0.05), self.slam_rad + 1.6 * u, 0.35, 3.0, t,
                                       k=1 - u)
        return img

    def after_post(self, img, F, drawn):
        """(older opt-in) REWIND: the tape look around the zipping ball + the rewind glyph (#31 render 1341-1348)"""
        k = self.k
        if k not in drawn:
            return img
        s_, rp = drawn[k]
        rk, win_ = self.rewinding(F.t)
        if rk > 0:
            img = ST.vhs(img, s_[0], s_[1], rk, F.t, seed=k)
            gx, gy = int(round(s_[0] - 6)), int(round(s_[1] - rp - 9))
            if SAFE[0] <= gx and gx + 13 <= SAFE[2] and SAFE[1] <= gy - 4 and gy + 4 <= SAFE[3]:
                ST.rewind_glyph(img, gx, gy, self.col, rk)
                ui._note(gx, gy - 4, gx + 13, gy + 4, 'label', 'rewind glyph')
        return img


# ================================================================ the sound (#31 audio.py, the decks' part)
def h01(*xs):
    """#31's deterministic hash in [0, 1) (its solo's pans)"""
    v = 0
    for x in xs:
        v = (v * 1000003 + int(x * 1000) + 7) % 2147483647
    return ((v * 48271) % 2147483647) / 2147483647.0


class Sound(base.Sound):
    bus_gain, send = 3.2, 0.3                                    # #31's decks bus

    def __init__(self, M, k):
        super().__init__(M, k)
        self._vc, self._gain, self._scr, self._src = {}, {}, {}, None
        self.rec = Record(M.P, k)
        ab = M.CFG[k]['abilities']
        self.names = {slot: ab[slot]['name'] for slot in ab}
        self.rewind = next((a for a in ab.values() if a['kind'] == 'rewind'), None)

    # ---------------------------------------------------------------- voices (#31's note / _fit: a 30 ms fade)
    note_helper = dict(note='note', gain='inst_gain', fade=0.03)   # (voices.NoteLog logs its notes as the kit's)

    def inst_gain(self, name):
        """#31's inst_gain: the voice's level match, measured once, on the part's own table"""
        import pp_bridge as B
        from reelkit import audio as A
        if name not in self._gain:
            lo, hi = B.entrant(name)[3]
            x = B.voice(name, (lo + hi) // 2, 0.8, 0.2)[:int(0.3 * A.SR)]
            self._gain[name] = min(4.0, 0.16 / (np.sqrt((A.hp(x, 120) ** 2).mean()) + 1e-6))
        return self._gain[name]

    def note(self, name, midi, dur=0.18, cap=0.5, vel=0.85):
        """#31's note: _fit (a 30 ms fade) of the Pixel Pitch voice, times inst_gain, cached"""
        import pp_bridge as B
        from reelkit import audio as A
        key = (name, int(midi), round(dur, 3), cap, round(vel, 2))
        if key not in self._vc:
            x = B.voice(name, int(midi), vel, dur)
            n = int(cap * A.SR)
            x = np.asarray(x, float)[:n].copy()
            if len(x) < n:
                x = np.pad(x, (0, n - len(x)))
            fd = min(len(x) // 3, int(0.03 * A.SR))
            x[-fd:] *= np.linspace(1, 0, fd)
            self._vc[key] = x * self.inst_gain(name)
        return self._vc[key]

    def stab(self, m, vel=0.85, dur=0.12):
        """an organ stab in the decks' voice"""
        return self.note(self.M.CFG[self.k]['pp'], m, dur, 0.4, vel)

    # ---------------------------------------------------------------- scratches (the decks' motions, a stab)
    def src(self):
        """the 'record' under the needle: a chopped F-minor synth-brass stab, then a held tail (long enough to
        backspin) (#31 _stab_src)"""
        from reelkit import audio as A
        if self._src is None:
            SR = A.SR
            n = int(2.4 * SR)
            t = A.tt(n)
            x = np.zeros(n)
            for m in (53, 56, 60, 65, 68):
                x += A.saw(A.mtof(m), n, 0.004, 3)
            env = np.exp(-t / 0.5) * np.clip(t / 0.004, 0, 1)
            y = A.lp(x * env, 2600) + 0.6 * A.bp(x * np.exp(-t / 0.08), 1500, 5000)
            self._src = y / (np.abs(y).max() + 1e-9)
        return self._src

    def scratch(self, kind, d, s0=0.06, friction=0.05, seed=0):
        """the record pushed back and forth under the needle (the Pixel Pitch deck's own motion curves, crossfader cuts
        included) playing the stab; backspins start further into the record"""
        from reelkit import audio as A
        key = (kind, round(d, 3), seed)
        if key in self._scr:
            return self._scr[key]
        SR = A.SR
        SRC = self.src()
        n = int((d + 0.03) * SR)
        t = A.tt(n)
        q, g = pp_turntables().qmotion(scratch_kind(kind), np.minimum(t, d), d)
        if kind == 'back':
            s0 = 1.75
        vel = np.gradient(q, 1 / SR)
        idx = (s0 + q) * SR
        y = np.interp(idx, np.arange(len(SRC)), SRC, left=0.0, right=0.0) * np.clip(np.abs(vel), 0, 1.6) ** 0.7
        y = y + A.bp(np.random.default_rng(seed + 11).normal(0, 1, n), 400, 3000) * np.clip(np.abs(vel), 0, 4) * friction
        from scipy.signal import lfilter
        a = math.exp(-1 / (0.0025 * SR))
        y *= lfilter([1 - a], [1, -a], g)
        fd = int(0.015 * SR)
        y[-fd:] *= np.linspace(1, 0, fd)
        self._scr[key] = y
        return y

    def floor_scratch(self, t0, d):
        """the scratch the picture shows: the record position under the hand is the floor's own curve (scratch_q),
        sampled at audio rate and played through the stab; backward-first scratches hold further into the record"""
        from reelkit import audio as A
        SR = A.SR
        SRC = self.src()
        step = 1.0 / SR
        n = int((d + 0.03) * SR)
        tt_ = t0 + np.arange(n) * step
        q = np.array([self.rec.scratch_q(x) for x in tt_])
        s0 = 0.03 + max(0.0, -float(q.min()))
        vel = np.gradient(q, step)
        y = np.interp((s0 + q) * SR, np.arange(len(SRC)), SRC, left=0.0, right=0.0) * np.clip(np.abs(vel), 0, 1.6) ** 0.7
        y = y + A.bp(np.random.default_rng(int(t0 * 100)).normal(0, 1, n), 400, 3000) * np.clip(np.abs(vel), 0, 4) * 0.05
        fd = int(0.012 * SR)
        y[-fd:] *= np.linspace(1, 0, fd)
        return y

    @staticmethod
    def tick(seed=0):
        """a vinyl tick: a ricochet off the rope"""
        from reelkit import audio as A
        n = int(0.03 * A.SR)
        t = A.tt(n)
        return A.bp(np.random.default_rng(seed).normal(0, 1, n), 2500, 9000) * np.exp(-t / 0.004)

    @staticmethod
    def sub_drop(dur=1.2, f0=95, f1=28):
        from reelkit import audio as A
        n = int(dur * A.SR)
        t = A.tt(n)
        f = f1 + (f0 - f1) * np.exp(-t / 0.25)
        x = np.sin(2 * np.pi * np.cumsum(f) / A.SR)
        return np.tanh(2.2 * x) * np.exp(-t / (dur * 0.6)) * np.clip(t / 0.002, 0, 1)

    @staticmethod
    def rewind_whirr(dur):
        """tape whirring backwards: a chirpy, falling-then-rising band of noise + a pitched squeal"""
        from reelkit import audio as A
        n = int(dur * A.SR)
        t = A.tt(n)
        u = t / dur
        x = np.random.default_rng(31).normal(0, 1, n)
        out = np.zeros(n)
        nb = 16
        for k in range(nb):
            a, b = k * n // nb, (k + 1) * n // nb
            fc = 900 + 5000 * (1 - abs(2 * (k + 0.5) / nb - 1))
            out[a:b] = A.bp(x[max(0, a - 1024):b], fc * 0.6, min(fc * 1.5, 20000))[-(b - a):]
        f = 600 + 2400 * (1 - abs(2 * u - 1)) + 300 * np.sin(2 * np.pi * 23 * t)
        sq = np.sin(2 * np.pi * np.cumsum(f) / A.SR) * 0.4
        return (out * 0.6 + sq) * np.sin(np.pi * np.clip(u, 0, 1)) ** 0.5

    # ---------------------------------------------------------------- the score
    def title_blast(self, mix, t):
        """the DJ's blast: a scratch flare over an organ-stab chord (the decks' own voice)"""
        mix.place(self.bus, self.scratch('flare', 0.4, seed=11), t, 0.40, pan=-0.2)
        for m in (53, 60, 65, 68):
            mix.place(self.bus, self.stab(m, 1.0, 0.35), t, 0.22, pan=-0.15)

    def count_in(self, mix):
        """a chirp per count"""
        for j, (tc, txt) in enumerate(self.T.COUNT[:3]):
            mix.place(self.bus, self.scratch('chirp', 0.22, seed=j), tc, 0.30, pan=-0.2)

    def play(self, mix):
        """the scratch IS the floor's motion; a thump under each jerk; rope slams = stabs; its abilities, its slam"""
        from reelkit import audio as A
        M, k, T = self.M, self.k, self.T
        T_KO, EV, q16, S16 = M.T_KO, M.EV, M.q16, T.S16
        for e in EV:
            if e[1] == 'scratch' and e[2] == k and e[0] <= T_KO + 0.01:
                t0, d = e[0], e[3]
                sig, g, lead = self.colour(self.floor_scratch(t0, d), t0, 0.8)
                mix.place(self.bus, sig, t0, 0.34 * g, pan=-0.2)
            elif e[1] == 'jerk' and e[2] == k and e[0] <= T_KO + 0.01:
                mix.place('fx', A.thud(0.18, 110, 50, seed=int(e[0] * 13)), e[0], 0.07 * min(1.4, e[6] / 1.3))
        slots = set()
        for e in EV:
            if e[0] > T_KO + 0.01:
                break
            if e[1] == 'hit' and e[2] == k and e[7] == 'scratch':     # a rope slam: an organ stab on the chord
                tq = q16(e[0])
                s_ = round(tq / S16)
                if s_ in slots:
                    continue
                slots.add(s_)
                ch = T.chord_at(tq)
                tones = [m for m in T.scale_notes(60, 79, ch) if m % 12 in T.CHORD_PCS[ch]]
                pan = float(np.clip(e[5] / 3.5, -0.6, 0.6))
                for j, m in enumerate(tones[:3]):
                    sig, g, lead = self.colour(self.stab(m, 0.95), e[0], 1.0)
                    mix.place(self.bus, sig, tq, 0.16 * g, pan=pan)
                if lead > 0.04:                                     # the leader: the root doubled an octave down
                    mix.place(self.bus, self.colour(self.stab(tones[0] - 12, 0.8), e[0])[0], tq, 0.12 * g, pan=pan)
                mix.place('fx', A.thud(0.3, 130, 55, seed=int(e[0] * 7)), tq, 0.14)
                mix.place('fx', self.tick(int(e[0] * 100) % 50), tq, 0.08, pan=pan)
        for e in EV:
            t, kind = e[0], e[1]
            if t > T_KO + 0.01:
                break
            if kind == 'ability' and e[2] == k:
                name = e[3]
                tq = q16(t)
                if name == self.names.get('unlock1') and e[4] == 'haste':      # SCRATCH
                    mix.place(self.bus, self.scratch('scrib', 0.4, seed=3), tq, 0.20, pan=-0.2)
                elif e[4] == 'rewind' and self.rewind is not None:            # REWIND
                    d = T.BEAT * self.rewind['zip_beats']
                    mix.place(self.bus, self.scratch('back', round(d + 0.1, 3), seed=7), t, 0.40, pan=-0.1)
                    mix.place('fx', self.rewind_whirr(d + 0.1), t, 0.20)
                elif e[4] == 'slam':                                           # BASS DROP
                    mix.place('fx', A.whoosh(2 * T.BEAT, 6000, 300, rise=False), tq, 0.12)
            elif kind == 'slam' and e[2] == k:
                ts = e[0]
                mix.place('bass', self.sub_drop(1.3), ts, 0.62)
                mix.place('fx', A.thud(0.6, 90, 32), ts, 0.50)
                mix.place(self.bus, self.stab(53, 1.0, 0.3), ts, 0.24)
                mix.place('fx', A.crash(1.5), ts, 0.10)

    def fall_apart(self, mix):
        """the decks run down: a baby scratch tape-stopped"""
        from reelkit import audio as A
        mix.place(self.bus, A.tape_stop(np.stack([self.scratch('baby', 1.2, seed=9)] * 2, 1), 0.0, 1.0)[:, 0],
                  self.M.P.T_HS, 0.3)

    def stinger(self, mix, tw):
        """a backspin into a stab chord on "(WINNER) WINS" """
        mix.place('solo', self.scratch('back', 0.6, seed=2), tw - 0.6, 0.30)
        for m in (53, 60, 65, 68):
            mix.place('solo', self.stab(m, 1.0, 0.4), tw, 0.20)

    def solo(self, mix, t0):
        """the scratch routine (config SOLO: (beat, kind, beats), kind a scratch or 'stab' (an organ stab chord); a
        MIDI note instead of a kind, as the kit's default solo gives, is one organ stab of that note)"""
        T = self.T
        for bt, kind, ln in T.SOLO.get(self.name, []):
            ts = t0 + bt * T.BEAT
            if not isinstance(kind, str):
                mix.place('solo', self.stab(int(kind), 0.95, min(0.3, ln * T.BEAT)), ts, 0.16)
            elif kind == 'stab':
                ch = T.chord_at(ts)
                for m in sorted(T.scale_notes(60, 72, ch), key=lambda x: (x % 12 not in T.CHORD_PCS[ch], x))[:3]:
                    mix.place('solo', self.stab(m, 0.95, min(0.3, ln * T.BEAT)), ts, 0.16)
            else:
                mix.place('solo', self.scratch(kind, round(ln * T.BEAT * 0.95, 3), seed=int(bt * 4)), ts, 0.36,
                          pan=-0.15 + 0.3 * h01(bt))

    def reassemble(self, mix):
        """its pieces fly home as a rising run of stabs"""
        M, T = self.M, self.T
        run = [65, 68, 72, 75, 77, 80, 84, 89]
        for j, m in enumerate(run):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
            mix.place(self.bus, self.stab(m - 12, 0.7), tj, 0.16)


# ---------------------------------------------------------------- Gate O's harness (tools/studio_export/fighter_turntables.py)
def origin_harness(stage, mod):
    """the kit matched to #31's own series rules inside Gate O's window: no ability list in the HUD (#31 predates it,
    Oct 25), and the decks' bus under #31's name `decks` (the kit's is `turntables`)"""
    if stage == 'picture':
        real = mod.fight_hud

        def fight_hud(img, left, right, *a, abilities=None, **kw):
            return real(img, left, right, *a, **kw)
        mod.fight_hud = fight_hud
    elif stage == 'music':
        for s in mod.SOUNDS:
            if isinstance(s, Sound):
                s.bus = 'decks'
