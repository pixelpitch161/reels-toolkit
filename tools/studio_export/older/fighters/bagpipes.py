"""BAGPIPES (tartan green), as built in #30 v3 (projects/ballfight_bagpipes_vs_accordion: BALLFIGHT #4, Bagpipes vs
Accordion): BAG INFLATE (weapon kind 'inflate': every bar the bag swells over 2 beats to about 1.6x size and mass,
heavy and slow, then lets go and rockets round the ring like a released balloon, zig-zagging as it deflates), WAR CRY
(shockwave: a tartan ring), HIGHLAND FLING (haste: two flights a bar), BAG BURST (wave: the bag pops).

Look (#30 render.py's BAGPIPES layers, moved into the kit's hooks with their names): the ball drawn at the run's own
radius (RUN['RAD']: `state` sets `scale`, as #30's fighter() did) with the bag's tartan stretched over the glass while a
cycle runs (opt-in `ball_glass`), a shiver just before it lets go, its glow through the swell and the flight; the
flight's ribbon trail behind the ball (`items_ball`); WAR CRY's tartan band rolling out on the canvas (`canvas_wave`,
the band painted over the lit canvas through opt-in `canvas_paint`) and BAG BURST's gust, both with #30's heat
shimmer; its hits' sparks, impact points, camera shakes, flashbulbs, glows and chromatic fringes as #30 drew them;
the effects (reelkit.fx) of its hits, its releases, WAR CRY and BAG BURST, the flight's puff trail and its K.O.
shreds; its shards (drone pipes, tartan, reeds). The draw functions (`tartan_rgb`, `draw_trail`, `tartan_ring`) are
#30's styles.py, imported from the episode's scratch copy (tools/studio_export/older/origin.py), never retyped.

Sound (#30 audio.py's BAGPIPES part, moved with its names): the Highland pipe's own synth from its Pixel Pitch module
(bagpipes60_lowpoly: `chanter`, `wt`, the drones), no pp_bridge voice call. The drone is the bag's air (A2 + two A3
drones, whole cycles in the file): low at rest, swelling as the bag fills, souring as its HP drops, stopping at its
K.O.; the flight squeals up about an octave; a landed flight is a grace-note squeal and a thud, a body slam a drone
blat, a stomp and a thud; WAR CRY skirls up to high A, HIGHLAND FLING throws a reel flourish (and the choir's 'oh'),
BAG BURST pops and deflates; it falls apart deflating, its stinger and solo on the chanter, its pieces fly home as a
chanter run. Buses as #30 placed them: 'chanter' (its own bus), 'drone', and the episode's shared 'fx', 'stomp',
'choir' and 'solo'.

#30's renderer drew both fighters inline; Gate O (tools/studio_export/fighter_bagpipes.py, SPEC.origin) proves this
module against it, layer by layer and bus by bus."""
PORTED = True

import importlib.util
import math
import sys

import numpy as np
from reelkit.config import W, H, FPS, BAYER_FULL
from reelkit import audio as A
from reelkit import fx
import scene as S

from . import base

R = S.R_BALL
EPISODE = 'ballfight_bagpipes_vs_accordion'


def _episode_styles():
    """#30's styles.py, from the episode's scratch copy (never from projects/: nothing is written there)"""
    name = '_older_styles_' + EPISODE
    if name not in sys.modules:
        import os
        from tools.studio_export.older import origin as O
        path = os.path.join(O.scratch(EPISODE), 'styles.py')
        spec = importlib.util.spec_from_file_location(name, path)
        mod = importlib.util.module_from_spec(spec)
        sys.modules[name] = mod
        spec.loader.exec_module(mod)
    return sys.modules[name]


ST = _episode_styles()

# #30 render.py: the hit kinds of its layers
BIG = ('flight', 'bodyslam', 'clamp', 'wave', 'shockwave')        # #30's big spark spray
SHAKE = {'bite': 0.7, 'flight': 1.3, 'bodyslam': 1.6, 'clamp': 2.5, 'wave': 2.2, 'shockwave': 1.4}


# ===================================================================================================== look (#30)
class Look(base.Look):
    shards = ('pipe',) * 5 + ('tartan',) * 7 + ('reed',) * 2       # #30 _shards: drone pipes, tartan, reeds
    spark_flash = ('flight', 'bodyslam')                          # #30 sparks(): the 3 px white impact point
    flash_hits = ('flight', 'bodyslam', 'wave', 'shockwave')      # #30 FLASH_EV
    fringe_hits = ()                                              # #30's fringe: after_grade (0.1 s, its own sizes)

    def setup(self):
        P = self.P
        self.rad = np.array(P.RUN['RAD'], float)                  # #30 sim.py REC_KEYS: RAD (the radius per frame)
        self.cycles = self._cycles()
        self.cyc_t = np.array([c['t0'] for c in self.cycles]) if self.cycles else np.zeros(0)

    # ---------------------------------------------------------------- #30 render.py: _cycles, cycle_at
    def _cycles(self):
        """BAG INFLATE cycles: [dict(k, t0, t_rel, t_land, s, dur_i, dur_f)] from the run's 'inflate' / 'release' /
        'land' events"""
        P = self.P
        out, cur = [], None
        for e in P.EV:
            if e[1] == 'inflate' and e[2] == self.k:
                cur = dict(k=e[2], t0=e[0], s=e[3], dur_i=e[4], dur_f=e[5], t_rel=None, t_land=None)
                out.append(cur)
            elif e[1] == 'release' and e[2] == self.k and cur is not None:
                cur['t_rel'] = e[0]
            elif e[1] == 'land' and e[2] == self.k and cur is not None:
                cur['t_land'] = e[0]
        for c in out:                                             # a cycle the K.O. interrupted ends there
            c['t_rel'] = c['t_rel'] if c['t_rel'] is not None else P.T_KO + 10
            c['t_land'] = c['t_land'] if c['t_land'] is not None else P.T_KO + 10
        return out

    def cycle_at(self, t):
        """(cycle, phase) at t: phase 'inflate' / 'fly' / None"""
        if not len(self.cyc_t):
            return None, None
        j = np.searchsorted(self.cyc_t, t, 'right') - 1
        if j < 0:
            return None, None
        c = self.cycles[j]
        if t < c['t_rel']:
            return c, 'inflate'
        if t < c['t_land']:
            return c, 'fly'
        return None, None

    # ---------------------------------------------------------------- the ball (#30 fighter(), draw_ball)
    def state(self, kk, st, t, phase, gl=0.0):
        """BAG INFLATE: the size comes from the run; while a cycle runs, how full the bag is (infl), a shiver just
        before it lets go, and its glow"""
        if kk != self.k or phase != 'fight':
            return
        P = self.P
        i = P.sim_i(min(t, P.T_KO))
        tc = min(t, P.T_KO)
        st['scale'] = float(self.rad[i, kk]) / R
        c, ph = self.cycle_at(tc)
        st['phase'] = ph
        if ph is not None:
            st['infl'] = (st['scale'] - 1) / max(1e-6, c['s'] - 1)
            if ph == 'inflate':                                   # a shiver just before it lets go
                u = (tc - c['t0']) / max(1e-6, c['t_rel'] - c['t0'])
                if u > 0.8:
                    st['p'] = st['p'] + 0.025 * math.sin(tc * 2 * math.pi * 31) * np.array([1.0, -0.6])
                st['glow'] = 0.15 + 0.25 * u
            else:
                st['glow'] = 0.55 * math.exp(-(tc - c['t_rel']) / 0.25) + 0.15

    def ball_glass(self, kk, st, t, inner, geo):
        """(opt-in) BAG INFLATE: the bag's tartan stretched over the glass as the ball swells"""
        if kk != self.k:
            return inner
        infl = st.get('infl', 0.0) if st.get('phase') else 0.0
        if infl > 0.02:
            dx, dy, nz, band = geo['dx'], geo['dy'], geo['nz'], geo['band']
            tart = ST.tartan_rgb(dx, dy, 1 + 0.5 * infl) * (0.62 + 0.42 * band[..., None])
            w_t = np.clip(infl * 1.15, 0, 0.92) * (0.75 + 0.25 * nz)
            inner = inner * (1 - w_t[..., None]) + tart * w_t[..., None]
        return inner

    # ---------------------------------------------------------------- events -> camera, sparks, glows
    def shake(self, e):
        if e[1] == 'hit' and e[2] == self.k and e[7] in SHAKE:
            return SHAKE[e[7]]
        return super().shake(e)

    def spark(self, e):
        how = e[7]
        n, sp, life = (16, 3.2, 0.3) if how in BIG else ((8, 2.2, 0.22) if how == 'bite' else (3, 1.4, 0.16))
        return self.P.COL[e[2]] * 0.6 + 100, n, sp, life

    def hit_glow(self, e):
        """#30: halved (the bursts carry the impacts)"""
        if e[7] in ('flight', 'bodyslam', 'clamp', 'wave', 'shockwave', 'bite'):
            big = e[7] != 'bite'
            return (12 if big else 7), (0.35 if big else 0.22)
        return None

    # ---------------------------------------------------------------- effects (reelkit.fx), as #30 built them
    def fx_event(self, e):
        P = self.P
        t0, kind = e[0], e[1]
        if kind == 'hit' and e[2] == self.k:
            att, x, z, how = e[2], e[5], e[6], e[7]
            pos = S.to_world(x, z, R)
            c = P.COL[att]
            if how in ('flight', 'bodyslam'):
                P.BU.add(t0, pos, c, 1.3 if how == 'flight' else 1.5)
                P.FL.add(t0, S.to_world(x, z, 1.0), c, radius=1.8, peak=0.9, decay=0.28)
                P.PA.emit(t0, pos, 'puffs', n=10, speed=(1.4, 2.8))
                P.PA.emit(t0, pos, 'shreds', n=4 if how == 'flight' else 6, speed=(1.0, 2.6))
                P.MK.add(t0, 'crack', x, z, size=0.45 if how == 'flight' else 0.65, n=4, life=4.0, seed=int(t0 * 100))
            elif how == 'wave':
                P.BU.add(t0, pos, c, 1.6)
                P.FL.add(t0, S.to_world(x, z, 1.0), c, radius=2.0, peak=1.0, decay=0.35)
            elif how == 'shockwave':
                P.BU.add(t0, pos, c, 1.0)
                P.FL.add(t0, S.to_world(x, z, 1.0), c * 0.9, radius=1.6, peak=0.7, decay=0.3)
        elif kind == 'release' and e[2] == self.k:                # the bag lets go: a blast of air out of the nozzle
            P.PA.emit(t0, S.to_world(e[5], e[6], R), 'puffs', n=10, spread=60, speed=(1.6, 3.0))
            P.FL.add(t0, S.to_world(e[5], e[6], 1.0), P.COL[self.k] * 0.9, radius=1.6, peak=0.6, decay=0.25)
        elif kind == 'ability' and e[2] == self.k:
            who = e[2]
            i = int(min(P.NSIM - 1, max(0, round((t0 - P.T.T_FIGHT) * FPS))))
            p = P.RUN['P'][i, who]
            if e[3] == 'BAG BURST':                               # the bag pops: a tartan-shred explosion
                P.PA.emit(t0, S.to_world(p[0], p[1], R), 'shreds', n=48, spread=180, speed=(2.0, 4.8), scale=1.2)
                P.PA.emit(t0, S.to_world(p[0], p[1], R), 'puffs', n=16, spread=180, speed=(1.5, 3.0), scale=1.4)
                P.FL.add(t0, S.to_world(p[0], p[1], 1.2), P.COL[who], radius=2.6, peak=1.1, decay=0.5)
                P.MK.add(t0, 'smoke_ring', p[0], p[1], size=1.6, life=5.0, seed=47, strength=0.7)
            elif e[4] == 'shockwave':
                P.FL.add(t0, S.to_world(p[0], p[1], 1.0), P.COL[who], radius=2.2, peak=0.8, decay=0.4)
                P.PA.emit(t0, S.to_world(p[0], p[1], R), 'shreds', n=10, spread=180, speed=(1.5, 3.0))

    def fx_after(self):
        """the flight's puff trail: one puff every other frame (after the event pass, as #30)"""
        P = self.P
        for c in self.cycles:
            if c['t_land'] > P.T_KO + 1:
                continue
            for j in range(P.sim_i(c['t_rel']), P.sim_i(c['t_land']), 2):
                k = c['k']
                p, v = P.RUN['P'][j, k], P.RUN['V'][j, k]
                tj = P.T.T_FIGHT + j / FPS
                back = p - v / (np.linalg.norm(v) + 1e-9) * float(self.rad[j, k]) * 0.9
                P.PA.emit(tj, S.to_world(back[0], back[1], R * 0.8), 'puffs', n=1,
                          vel=S.to_world(*(-0.4 * v), 0.0) - S.to_world(0, 0, 0))

    def ko_fx(self, pos):
        """the bag bursts at its K.O.: tartan shreds"""
        self.P.PA.emit(self.P.T_KO, pos, 'shreds', n=36, spread=180, speed=(1.5, 4.0))

    # ---------------------------------------------------------------- the canvas: WAR CRY's band, BAG BURST's gust
    def canvas_wave(self, Ic, C, w):
        P = self.P
        owner, cx, cz, r, rmax, kind, t0 = w
        sxp, szp = C.sxp, C.szp
        rd = np.hypot(sxp - cx, szp - cz)
        k_ = (1 - r / rmax)
        if kind == 'shockwave':                                   # WAR CRY: a tartan ring rolling out
            paint = C.__dict__.setdefault('bagpipes_paint', [])
            paint.append(ST.tartan_ring(sxp, szp, BAYER_FULL[C.ys, C.xs], cx, cz, r, 0.26, 0.95 * k_ + 0.05, C.t))
            band = np.clip(1 - np.abs(rd - r) / 0.3, 0, 1)
            Ic += (band * k_ * 0.5)[:, None] * (P.COL[owner] / 255.0 + 0.15)[None]
            return Ic
        ang_ = np.arctan2(szp - cz, sxp - cx)                     # BAG BURST: a thick gust with puffs of air
        rd = rd + 0.08 * np.sin(5 * ang_ + 9 * C.t) + 0.05 * np.sin(13 * ang_ - 17 * C.t)
        band = np.clip(1 - np.abs(rd - r) / 0.36, 0, 1)
        band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
        Ic += (band * k_ * 1.2)[:, None] * (P.COL[owner] / 255.0 * 1.4 + 0.35)[None]
        return Ic

    def canvas_paint(self, col, Ic, C):
        """(opt-in) WAR CRY's tartan bands, lit by the spot like the canvas"""
        for dpaint, dw in C.__dict__.get('bagpipes_paint', ()):
            lit = np.clip(Ic.mean(1), 0.45, 1.6)[:, None] * 0.82
            col = col * (1 - dw[:, None]) + dpaint * lit * dw[:, None]
        return col

    # ---------------------------------------------------------------- the painter: the flight's ribbon trail
    def items_ball(self, F, st):
        if st.get('phase') == 'fly' and not st['shattered']:      # the balloon's ribbon trail (behind it)
            dz = F.cam.depth(S.to_world(st['p'][0], st['p'][1], R))[0]
            return [(dz + 0.05, ('trail',))]
        return []

    def draw_item(self, F, payload):
        P = self.P
        if payload[0] == 'trail':
            tc = min(F.t, P.T_KO)
            j1 = P.sim_i(tc)
            c_, ph = self.cycle_at(tc)
            j0 = max(P.sim_i(c_['t_rel']) if c_ else j1 - 12, j1 - 14)
            pts = [P.RUN['P'][j, self.k] for j in range(j0, j1 + 1)]
            arr, depth = F.fr.array(), F.fr.depth()
            ST.draw_trail(arr, F.cam, pts, P.COL[self.k], width=7)
            F.fr.set_array(arr, depth)

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        """#30's shards: a drone pipe (black wood, an ivory ferrule), a cane reed, a chip of tartan bag cloth"""
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        if kind in ('pipe', 'reed'):
            Lk, Wk, c = {'pipe': (0.48, 0.07, np.array([34, 28, 30])), 'reed': (0.22, 0.03, np.array([222, 196, 120]))}[kind]
            q4 = [np.array([sx_, sz_]) + d * Lk / 2 * a1 + nn * Wk / 2 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            P.layer_blend(arr, [sp], c, 1.0)
            if kind == 'pipe':                                    # the ivory ferrule at one end
                q2 = [np.array([sx_, sz_]) + d * Lk / 2 * a1 + nn * Wk * 0.7 * a2 for a1, a2 in ((0.75, -1), (1, -1), (1, 1), (0.75, 1))]
                P.layer_blend(arr, [P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q2]))], (240, 232, 214), 1.0)
            return arr
        col = P.COL[P.LOSE] * 0.8 if (int(size * 100) % 3) else np.array([196, 40, 44])   # tartan: tumbling flat chips
        sz = size
        tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
               np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
        shade_k = 0.75 + 0.25 * math.cos(rot * 2)
        P.layer_blend(arr, [sp], np.clip(col * shade_k, 0, 255), 1.0)
        return arr

    # ---------------------------------------------------------------- lens, post
    def shimmer(self, img, cam, t, w):
        """heat shimmer on WAR CRY / BAG BURST fronts"""
        owner, cx, cz, r, rmax, kind, t0 = w
        return fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r, 0.4 if kind == 'wave' else 0.25,
                                3.0 if kind == 'wave' else 2.0, t, k=1 - r / rmax, seed=owner)

    def after_grade(self, img, F, drawn):
        """chromatic fringing on its big hits (local), as #30: 0.1 s, (30, 2.6) on BAG BURST, (20, 1.8) on a flight or
        a body slam (before the K.O.'s fringe, the ability moments and the bursts, as #30's renderer)"""
        P, t = self.P, F.t
        for e_i, e in P.events_between(t - 0.1, t, ('hit',)):
            if e[2] == self.k and e[7] in ('flight', 'bodyslam', 'clamp', 'wave'):
                s_ = F.cam.wp(S.to_world(e[5], e[6], R)[None])[0]
                img = fx.fringe(img, s_[0], s_[1], 30 if e[7] in ('clamp', 'wave') else 20,
                                2.6 if e[7] in ('clamp', 'wave') else 1.8, k=1 - (t - e[0]) / 0.1)
        return img


# ===================================================================================================== sound (#30)
SR = A.SR
_VC = {}
_G = {}


def _bag():
    import pp_bridge as B
    return B, B.module('bagpipes60_lowpoly')


def _fit(x, cap):
    n = int(cap * SR)
    x = np.asarray(x, float)[:n].copy()
    if len(x) < n:
        x = np.pad(x, (0, n - len(x)))
    fd = min(len(x) // 3, int(0.03 * SR))
    x[-fd:] *= np.linspace(1, 0, fd)
    return x


def chanter(m, dur=0.2, vel=0.85, grace=False):
    """one chanter note (the Highland pipe's own synth), optionally with the high-G grace squeal in front"""
    B, BAG = _bag()
    HL = BAG.TONES['ghb']
    key = ('ch', int(m), round(dur, 3), round(vel, 2), grace)
    if key not in _VC:
        x = BAG.chanter(BAG.midi_hz(m), dur, vel, HL, BAG.midi_hz(79) if grace else None, seed=int(m) * 7)
        x = _fit(B._resample(np.asarray(x, float)), dur + 0.12)
        if 'ch' not in _G:
            ref = B._resample(np.asarray(BAG.chanter(BAG.midi_hz(72), 0.2, 0.85, HL, None, 1), float))
            _G['ch'] = min(6.0, 0.16 / (np.sqrt((A.hp(ref, 120) ** 2).mean()) + 1e-6))
        _VC[key] = x * _G['ch']
    return _VC[key]


def drone_layer(ms_amps, n):
    """the Highland drones as wavetables at frequencies nudged (< 0.01 Hz) to whole cycles in n samples, so the drone
    at the file's last sample flows into its first; ms_amps: [(midi, amp, cents)]"""
    B, BAG = _bag()
    dc = BAG.DRONES['ghb']
    L = n / SR
    x = np.zeros(n)
    t = np.arange(n) / SR
    for j, (m, a, cents) in enumerate(ms_amps):
        f = BAG.midi_hz(m) * 2 ** (cents / 1200)
        fq = round(f * L) / L
        tb = BAG.wt(f, dc['tilt'], dc['form'], dc['fbw'], dc['fgain'], dc['odd'])
        ph = (fq * t + 0.137 * (j + 1)) % 1.0
        x += a * tb[(ph * 2048).astype(int) & 2047]
    return A.lp(x, dc['lpf']) * 0.3


def stomp(vel=1.0, seed=0):
    """a boot on a wooden deck: a low thump + the board's knock (#30's bed synth: a body slam stamps it)"""
    n = int(0.35 * SR)
    t = A.tt(n)
    f = 52 + 70 * np.exp(-t / 0.03)
    body = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / 0.13)
    knock = A.bp(np.random.default_rng(seed).normal(0, 1, n), 180, 1400) * np.exp(-t / 0.022)
    return (np.tanh(1.8 * body) * 0.9 + 0.55 * knock) * vel


def choir(ms, dur, att=0.35, rel=0.4, vowel='m'):
    """a low male choir (#30's bed synth: HIGHLAND FLING's 'oh')"""
    n = int((dur + rel) * SR)
    t = A.tt(n)
    x = np.zeros(n)
    for m in ms:
        f = A.mtof(m)
        for d, ph0 in ((-0.004, 0.1), (0.0, 0.5), (0.005, 0.8)):
            fr = f * (1 + d) * (1 + 0.004 * np.sin(2 * np.pi * (4.6 + 3 * d * 100) * t + ph0 * 6))
            ph = (np.cumsum(fr) / SR + ph0) % 1.0
            x += 2 * ph - 1
    x /= 3 * max(1, len(ms))
    if vowel == 'm':
        y = A.bp(x, 180, 420) * 1.0 + A.bp(x, 700, 1100) * 0.25
    else:
        y = A.bp(x, 300, 650) * 1.0 + A.bp(x, 750, 1250) * 0.45
    env = np.clip(t / att, 0, 1) * np.clip((dur + rel - t) / rel, 0, 1)
    return A.lp(y, 1600) * env


def deflate(dur=1.6, f0=440.0, f1=150.0, seed=21):
    """the bag giving out: the chanter's reed sliding down and choking, with the air hissing out"""
    B, BAG = _bag()
    HL = BAG.TONES['ghb']
    n = int(dur * SR)
    t = A.tt(n)
    u = t / dur
    f = f0 * (f1 / f0) ** (u ** 0.7) * (1 + 0.03 * np.sin(2 * np.pi * 7 * t))
    tb = BAG.wt(f0, HL['tilt'], HL['form'], HL['fbw'], HL['fgain'], HL['odd'])
    ph = np.cumsum(f) / SR
    x = tb[(ph * 2048).astype(int) & 2047] * (1 - u) ** 1.2
    air = A.bp(np.random.default_rng(seed).normal(0, 1, n), 1500, 7000) * np.exp(-u * 2.5) * 0.5
    return A.lp(x, 4500) + air


def squeal(dur, f0, t0):
    """the released balloon: the chanter's reed squealing up about an octave over the flight, wobbling hard at every
    zig-zag (a vibrato that speeds up), the air hissing out as it empties"""
    B, BAG = _bag()
    HL = BAG.TONES['ghb']
    n = int((dur + 0.06) * SR)
    t = A.tt(n)
    u = np.clip(t / dur, 0, 1)
    f = f0 * 2 ** (0.95 * u ** 0.8) * (1 + 0.035 * np.sin(2 * np.pi * (7 + 9 * u) * t))
    tb = BAG.wt(f0, HL['tilt'], HL['form'], HL['fbw'], HL['fgain'], HL['odd'])
    ph = np.cumsum(f) / SR
    x = tb[(ph * 2048).astype(int) & 2047]
    env = np.clip(t / 0.02, 0, 1) * (1 - 0.55 * u) * np.clip((dur + 0.05 - t) / 0.05, 0, 1)
    air = A.bp(np.random.default_rng(int(t0 * 100)).normal(0, 1, n), 2000, 8000) * 0.35 * u
    return A.lp(x * env, 5200) + air * env


class Sound(base.Sound):
    bus_gain, send = 1.0, 0.45                                     # #30's chanter bus (BUS_GAIN, the reverb send)

    def __init__(self, M, k):
        super().__init__(M, k)
        self.bus = 'chanter'                                       # #30's bus names: 'chanter', 'drone'

    def melody_at(self, t):
        """#30's melody_at: the chanter line (config LINES) at t"""
        return self.T.line_at(self.name, t)

    def title_blast(self, mix, t):
        """a drone blast (all three drones + the chanter's high A with its grace note)"""
        B, BAG = _bag()
        blast = B._resample(np.asarray(BAG.drone_voice('ghb', 0.75), float)) * 3.2
        mix.place('drone', np.stack([blast, blast], 1), t, 0.55)
        mix.place('chanter', chanter(81, 0.62, 1.0, grace=True), t, 0.46)
        mix.place('chanter', chanter(69, 0.62, 0.9), t, 0.26)

    def count_in(self, mix):
        T = self.T
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            mix.place('chanter', chanter((71, 74, 76)[j], 0.30, 0.9, grace=True), tc, 0.40)

    def play(self, mix):
        """the drone is the bag's air; the flight squeals; its hits and abilities (#30 main(), in its order)"""
        M, T, k = self.M, self.T, self.k
        B, BAG = _bag()
        q16 = M.q16
        RUN, EV, T_KO, LOSE = M.RUN, M.EV, M.T_KO, M.LOSE
        rad = np.array(RUN['RAD'], float)
        n = M.NA
        tt_ = np.arange(n) / SR
        from scipy.signal import lfilter
        drone = drone_layer(((57, 1.0, -1.5), (57, 1.0, 1.5), (45, 1.1, 0.0)), n)
        tframes = T.T_FIGHT + np.arange(len(rad)) / T.FPS
        full = np.clip((rad[:, k] / R - 1) / 0.6, 0, 1)                     # how full the bag is (0 at rest .. 1)
        full_t = np.interp(tt_, tframes, full, left=0, right=0)
        full_t = lfilter([0.003], [1, -0.997], full_t)
        alive = np.ones(n)                                                   # the drone stops at a bagpipes K.O. and
        if LOSE == k:                                                        # comes back with the reassembly
            alive = np.where(tt_ < T_KO, 1.0, 0.0) + np.clip((tt_ - T.T_GLIDE) / 0.6, 0, 1) * (tt_ >= T.T_GLIDE)
            alive = lfilter([0.002], [1, -0.998], alive)
        hp_p = np.array([M.hp_at(k, x) for x in np.linspace(0, T.DUR, 400)]) / 100.0
        hp_curve = np.interp(tt_, np.linspace(0, T.DUR, 400), hp_p)
        champ = (tt_ >= T_KO) * np.clip((T.T_REV - tt_) / (T.T_REV - T.T_GLIDE), 0, 1) * (LOSE != k)
        g = (0.11 + 0.10 * champ + 0.42 * full_t) * alive                   # under the mix, up with the bag
        sour = np.sin(2 * np.pi * 0.37 * tt_) * 0.5 + 0.5
        drone_sig = drone * g * (1 - 0.35 * (1 - hp_curve) * sour)
        mix.bus('drone')[:n] += np.stack([drone_sig * 0.95, drone_sig * 1.0], 1)
        for c in self._cycles():
            if c['t_rel'] > T_KO:
                continue
            mix.place('fx', A.whoosh(c['dur_i'], 200, 1400) * 0.6, c['t0'], 0.05)          # the bag filling
            t1 = min(c['t_land'], T_KO)
            d = t1 - c['t_rel']
            if d <= 0.05:
                continue
            m0 = self.melody_at(q16(c['t_rel']))
            sig = squeal(d, BAG.midi_hz(m0), c['t_rel'])
            sg, gg, _ = self.colour(sig, c['t_rel'], 1.0)
            mix.place('chanter', sg, c['t_rel'], 0.24 * gg, pan=-0.15)
        for e in EV:
            t, kind = e[0], e[1]
            if t > T_KO + 0.01:
                break
            if kind == 'hit' and e[2] == k and e[7] in ('flight', 'bodyslam'):
                tq = q16(t)
                pan = float(np.clip(e[5] / 3.5, -0.5, 0.5))
                if e[7] == 'flight':                                   # a squealing grace note + the hit
                    m = self.melody_at(tq) + 12 if self.melody_at(tq) < 72 else self.melody_at(tq)
                    sig, gg, lead = self.colour(chanter(m, 0.22, 1.0, grace=True), tq, 1.0)
                    mix.place('chanter', sig, tq, 0.34 * gg, pan=pan)
                    mix.place('fx', A.thud(0.32, 150, 55, seed=int(t * 7)), tq, 0.18)
                else:                                                  # a body slam: a drone blat + a stomp
                    mix.place('chanter', A.lp(self.colour(chanter(57, 0.3, 1.0), tq)[0], 1800), tq, 0.30, pan=pan)
                    mix.place('stomp', stomp(1.0, int(t * 3)), tq, 0.40)
                    mix.place('fx', A.thud(0.4, 110, 40, seed=int(t * 5)), tq, 0.22)
        for e in EV:
            t, kind = e[0], e[1]
            if t > T_KO + 0.01:
                break
            if kind == 'hit' and e[2] == k and e[7] in ('shockwave', 'wave'):
                mix.place('fx', A.thud(0.45, 110, 40), q16(t), 0.30)
            elif kind == 'ability' and e[2] == k:
                name = e[3]
                tq = q16(t)
                if name == 'WAR CRY':                                  # the skirl: a run up to high A, held
                    for j, m in enumerate((69, 71, 72, 74, 76, 79)):
                        mix.place('chanter', self.colour(chanter(m, T.S16 * 0.9, 1.0, grace=(j == 0)), t)[0],
                                  tq + j * T.S16 / 2, 0.26)
                    mix.place('chanter', self.colour(chanter(81, 2 * T.BEAT, 1.0, grace=True), t)[0], tq + 3 * T.S16,
                              0.34)
                elif name == 'HIGHLAND FLING':                         # a reel flourish
                    for j, m in enumerate((76, 74, 72, 74, 76, 79, 81, 79)):
                        mix.place('chanter', self.colour(chanter(m, T.S16 * 0.85, 0.95, grace=(j % 4 == 0)), t)[0],
                                  tq + j * T.S16, 0.22)
                    mix.place('choir', choir((57, 60, 64), 0.18, att=0.01, rel=0.12, vowel='o'), tq, 0.14)
                elif name == 'BAG BURST':                              # the bag pops, then deflates
                    pop = A.bp(np.random.default_rng(29).normal(0, 1, int(0.12 * SR)), 300, 9000) * \
                        np.exp(-A.tt(int(0.12 * SR)) / 0.02)
                    mix.place('fx', pop, tq, 0.55)
                    mix.place('chanter', deflate(1.8, BAG.midi_hz(81), 160.0), tq + 0.03, 0.34)
                    mix.place('fx', A.thud(0.7, 100, 32), tq, 0.45)

    def _cycles(self):
        """#30 render.py's _cycles (the BAG INFLATE cycles), from the run's events"""
        M = self.M
        out, cur = [], None
        for e in M.EV:
            if e[1] == 'inflate' and e[2] == self.k:
                cur = dict(k=e[2], t0=e[0], s=e[3], dur_i=e[4], dur_f=e[5], t_rel=None, t_land=None)
                out.append(cur)
            elif e[1] == 'release' and e[2] == self.k and cur is not None:
                cur['t_rel'] = e[0]
            elif e[1] == 'land' and e[2] == self.k and cur is not None:
                cur['t_land'] = e[0]
        for c in out:
            c['t_rel'] = c['t_rel'] if c['t_rel'] is not None else M.T_KO + 10
            c['t_land'] = c['t_land'] if c['t_land'] is not None else M.T_KO + 10
        return out

    def fall_apart(self, mix):
        """the bag gives out: the chanter sliding down and choking"""
        B, BAG = _bag()
        mix.place('chanter', deflate(2.2, BAG.midi_hz(76), 120.0, 27), self.M.P.T_HS, 0.36)

    def stinger(self, mix, tw):
        """#30's stinger lands where #30's main() placed it: its `tw` (q16(T_KO + 1.3)) is overwritten by the title
        card's loop (`for tw in (t1, t2, t3)`), so the stinger sounds on the card's third word, 2 beats in, not on
        "(WINNER) WINS" (as delivered; the kit's `tw` is not used)"""
        t3 = 2 * self.T.BEAT
        mix.place('solo', chanter(81, 1.2, 1.0, grace=True), t3, 0.36)
        mix.place('solo', chanter(69, 1.2, 0.8), t3, 0.18)

    def solo(self, mix, t0):
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts >= T.T_REASSEMBLE:
                break
            dur = d * T.BEAT * 0.95
            mix.place('solo', chanter(m, dur, 0.95, grace=(d >= 0.75)), ts, 0.46)

    def reassemble(self, mix):
        """its pieces fly home as a rising chanter run"""
        T, q16 = self.T, self.M.q16
        run = [57, 60, 62, 64, 67, 69, 72, 76]
        for j, m in enumerate(run):
            tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
            mix.place('chanter', chanter(m + 12 if m < 67 else m, T.S16 * 1.6, 0.6), tj, 0.16)
