"""SYNTH (cyan), as built in #31 v3 (BALLFIGHT #5: Turntables vs Synth, projects/ballfight_turntables_vs_synth): PATCH
CABLES (weapon kind 'tether': every second beat a plug flies at the enemy while fewer cables are in than its slots; a
landed plug jabs and plugs a cable in for 8 beats; voltage pulses run down every cable on the 8ths, one arp note each;
the leash keeps the enemy within 2.4 unless it is flung harder than 0.15 x the cables, then they all snap), LFO (haste:
pulses at double rate, the waveform swells), SAWTOOTH (ability kind 'lash': the cables go saw-wave and lash side to
side, cutting), WHITE NOISE (wave).

Look (#31 render.py, the SYNTH's layers moved into the kit's hooks with their names): the patch cables between the two
balls carrying the waveform of the synth's level (sine / square / saw, scrolling, swelling under LFO), unplugging
cables sliding home, snapped cables whipping back, plugs in flight trailing their cable, voltage pulses running down
the cables (a bright dot and a glow), SAWTOOTH's lashing saw arms (a glow at each tip), the plugged jacks over the
balls; the effects of its hits and events (plug: a burst + cyan sparks; volt: a spark; lash: a burst, a flare,
glitch, a claw mark, a shake; WHITE NOISE's hit: a burst, glitch, a shake; a snap: sparks, glitch, a burst, a flare, a
shake; a leash bounce: sparks; WHITE NOISE itself: a flare and glitch), its spark sprays (lash and plug sprays, no
spray for a pulse, a spray where a plug misses), its flashbulbs and hit glows, the snap's chromatic fringe, WHITE NOISE's
static ring on the canvas and its heat shimmer, the champion's groove rings of light on the canvas, its pitched Pixel
Pitch sprite (the keys tipped toward the camera), its K.O. spray and shards (keys, black keys, knobs, jacks), its
VS-card panel (#31's circuit board) and close-up. A pulse does not flash the enemy's HP bar.
Sound (#31 audio.py, the SYNTH's part): every voltage pulse an arpeggio note in the synth's own Pixel Pitch voice (the
first cable plays the arp, the others the notes a third above, quieter), toned like the waveform on the cables (sine,
square, saw by level), skipping notes when hurt; a landed plug clicks in with an accent, a SAWTOOTH cut an accent and a
thud, WHITE NOISE a thud, a noise blast and a high note; the cable unspooling, a snapped cable's crack, the leash's
thrum; LFO's wobble bass, SAWTOOTH's supersaw sweep; the title blast (a saw chord + a sweep), the count-in stabs, the
falling arp when it shatters, the stinger, the solo and the rising run of its reassembly.

Draw functions come from the episode's own styles.py (read from its scratch copy under studio/reference/.cache/m4/
origin/, never retyped: the episodes' rule "styles live there so crossovers can import them"). Nothing under
projects/ is read for writing. Plan m4 Task 10; Gate O: python -m tools.studio_export origin fighters/synth.

The arpeggio table (#31 timeline.py ARP) and #31's fixed note tables (title blast, count-in, stinger, fall-apart, the
reassembly run) are #31's for #31's chords and key; for another chord the arp is the chord's tones climbing from E4, and
the fixed tables move with the tonic chord's root (a whole-step rule that gives #31's notes for F minor)."""
PORTED = True

import importlib.util
import math
import os
import sys

import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, BAYER_FULL, ease
from reelkit import audio as A
import scene as S

from . import base

EPISODE = 'ballfight_turntables_vs_synth'
R = S.R_BALL


def _episode_styles():
    """#31's styles.py from its scratch copy (one module per process, shared with any other look that imports it)"""
    name = f'older_styles_{EPISODE}'
    if name in sys.modules:
        return sys.modules[name]
    from tools.studio_export.older import origin as O
    folder = O.scratch(EPISODE)
    spec = importlib.util.spec_from_file_location(name, os.path.join(folder, 'styles.py'))
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


ST = _episode_styles()

# ================================================================ #31 render.py: its pieces, moved here with their names
CYAN_SPARK = [(150, 240, 255), (230, 252, 255), (70, 228, 255)]
WAVEFORM = {1: 'sine', 2: 'square', 3: 'saw'}
CABLES_DEPTH = 1.0e9                     # the cables lie under everything of the painter pass (#31: drawn before it)
JACKS_DEPTH = -1.0e9                     # the plugged jacks over everything of it (#31: drawn right after it)


def line_blend(arr, pts, col, alpha, width=1):
    """alpha-blend a screen polyline onto arr (#31 render.py)"""
    if len(pts) < 2 or alpha <= 0:
        return
    m = Image.new('L', (W, H), 0)
    ImageDraw.Draw(m).line([tuple(p) for p in pts], fill=255, width=width)
    mk = np.asarray(m, np.float32) / 255.0 * alpha
    arr += (np.array(col, float)[None, None] - arr) * mk[..., None]


def _circuit(base_):
    """SYNTH's panel on the VS card (bottom-right): circuit traces (right-angle runs) and solder pads (#31 render.py)"""
    import title as TT
    gx, gy = np.floor(TT.XX / 24.0), np.floor(TT.YY / 24.0)
    h = (gx * 7.13 + gy * 3.71) % 1.0
    fx_, fy_ = TT.XX % 24.0, TT.YY % 24.0
    trace = ((h < 0.5) & (np.abs(fy_ - 12) < 1.0)) | ((h >= 0.5) & (np.abs(fx_ - 12) < 1.0))
    pad = (np.hypot(fx_ - 12, fy_ - 12) < 2.6) & (h > 0.72)
    base_ = np.where(trace[..., None], base_ * 1.22, base_)
    return np.where(pad[..., None], base_ * 1.4, base_)


class Look(base.Look):
    shards = tuple(['key'] * 7 + ['bkey'] * 4 + ['knob'] * 4 + ['jack'] * 4)
    flash_hits = ('slam', 'wave', 'scratch', 'lash')
    fringe_hits = ()                 # its fringes are drawn in after_grade, hits and snaps in event order (#31)
    own_fringe_hits = ('slam', 'wave', 'scratch')
    quiet_hits = ('volt',)
    title_pattern = staticmethod(_circuit)

    # ---------------------------------------------------------------- setup (the cables, plugs, pulses, lashes)
    def setup(self):
        self.setup_as(self.P, self.k, self.cfg)

    def setup_as(self, P, k, cfg):
        """the run's cables, plugs, pulses and lashes for side k, with `cfg`'s numbers (#31 render.py 293-321)"""
        self.cfg = cfg
        self.volt_speed = cfg['weapon']['volt_speed']
        self.pp, self.view, self.pitch = cfg['pp'], cfg['view'], cfg.get('pitch', 0)
        EV, RUN, T_KO = P.EV, P.RUN, P.T_KO
        cables, plugs = {}, {}
        for e in EV:
            if e[1] == 'plug_fire' and e[2] == k:
                plugs[e[3]] = dict(t0=e[0], o=np.array([e[4], e[5]]), ang=e[6], speed=e[7], t1=None, landed=False)
            elif e[1] in ('plug', 'plug_miss') and e[2] == k and e[3] in plugs:
                plugs[e[3]]['t1'] = e[0]
                plugs[e[3]]['landed'] = e[1] == 'plug'
                if e[1] == 'plug':
                    cables[e[3]] = dict(t0=e[0], t1=T_KO, reason=2)
            elif e[1] == 'unplug' and e[2] == k and e[3] in cables:
                cables[e[3]]['t1'] = e[0]
                cables[e[3]]['reason'] = e[4]
        for P_ in plugs.values():
            if P_['t1'] is None:
                P_['t1'] = T_KO
        volts = []                                    # (t0, cable id, n, travel time)
        for e in EV:
            if e[1] == 'volt' and e[2] == k:
                i = P.sim_i(e[0])
                dist = float(np.hypot(*(RUN['P'][i, 1 - k] - RUN['P'][i, k])))
                volts.append((e[0], e[3], e[4], dist / self.volt_speed))
        self.CABLES, self.PLUGS, self.VOLTS = cables, plugs, volts
        self.VOLT_T = np.array([v[0] for v in volts]) if volts else np.zeros(0)
        self.LASHES = [dict(t0=e[0], t1=e[0] + e[6], aim=e[3], n=e[4], len=e[5], period=e[7], sweep=e[8],
                            spread=e[9], lag=e[10]) for e in EV if e[1] == 'lash' and e[2] == k]
        self._frame = (None, [], [])                  # (frame, jacks, glows) of the frame being drawn

    def title_closeup(self, r):
        """the card's big ball: the model tipped toward the camera (#31 render.py 104-107)"""
        yc, sw, roll = self.view
        return ST.render_sprite(self.pp, r, float(yc), float(roll), float(self.pitch))

    # ---------------------------------------------------------------- the ball
    def sprite(self, r, yaw, roll):
        """the Pixel Pitch synth tipped toward the camera by its `pitch` (#31 render.py sprite())"""
        return ST.render_sprite(self.pp, r, float(yaw), float(roll), float(self.pitch))

    # ---------------------------------------------------------------- events -> camera, sparks, glows
    def shake(self, e):
        """#31 _build_fx's shakes on its events"""
        if e[1] == 'hit':
            return {'scratch': 1.4, 'lash': 0.8, 'slam': 2.5, 'wave': 2.0}.get(e[7])
        if e[1] == 'snap':
            return 1.0
        if e[1] == 'slam':
            return 1.6
        return None

    def spark(self, e):
        """#31 sparks(): no spray for a pulse; lash and plug sprays mid-sized"""
        P = self.P
        how = e[7]
        if how == 'volt':
            return P.COL[e[2]] * 0.6 + 100, 0, 1.8, 0.18
        n, sp, life = (16, 3.2, 0.3) if how in ('slam', 'wave', 'scratch') else \
            ((9, 2.6, 0.24) if how in ('lash', 'plug') else (5, 1.8, 0.18))
        return P.COL[e[2]] * 0.6 + 100, n, sp, life

    def hit_glow(self, e):
        if e[7] in ('slam', 'wave', 'scratch', 'lash'):
            return 12, 0.35
        return None

    # ---------------------------------------------------------------- effects (#31 _build_fx, its events)
    def fx_event(self, e):
        P = self.P
        S_, BU, FL, PA, MK = S, P.BU, P.FL, P.PA, P.MK
        COL = P.COL
        t0, kind = e[0], e[1]
        if kind == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            pos = S_.to_world(x, z, R)
            if how == 'scratch':                                       # a rope slam
                BU.add(t0, pos, COL[att], 1.2)
                FL.add(t0, pos, COL[att], radius=1.8, peak=0.8, decay=0.3)
                PA.emit(t0, pos, 'shards', n=8)
                MK.add(t0, 'crack', x, z, size=0.45, n=4, life=4.0, seed=int(t0 * 100))
            elif how == 'plug':
                BU.add(t0, pos, COL[att], 0.5, bright=False)
                PA.emit(t0, pos, 'sparks', n=5, col=CYAN_SPARK)
            elif how == 'volt':
                PA.emit(t0, pos, 'sparks', n=1, col=CYAN_SPARK, scale=0.7)
            elif how == 'lash':
                BU.add(t0, pos, COL[att], 0.8)
                FL.add(t0, pos, COL[att], radius=1.4, peak=0.6, decay=0.24)
                PA.emit(t0, pos, 'glitch', n=6)
                MK.add(t0, 'claw', x, z, size=0.4, angle=math.atan2(z, x) + 1.2, n=1, col=(150, 236, 255), life=3.0,
                       seed=int(t0 * 100))
            elif how == 'slam':
                BU.add(t0, pos, COL[att], 1.6)
            elif how == 'wave':
                BU.add(t0, pos, COL[att], 1.1)
                PA.emit(t0, pos, 'glitch', n=10)
        elif kind == 'snap':
            pos = S_.to_world(e[4], e[5], R)
            PA.emit(t0, pos, 'sparks', n=14, col=CYAN_SPARK)
            PA.emit(t0, pos, 'glitch', n=6)
            BU.add(t0, pos, COL[e[2]], 0.9)
            FL.add(t0, pos, COL[e[2]], radius=1.6, peak=0.8, decay=0.3)
        elif kind == 'leash':
            PA.emit(t0, S_.to_world(e[3], e[4], R), 'sparks', n=2, col=CYAN_SPARK, scale=0.8)
        elif kind == 'ability' and e[4] == 'wave':
            i = P.sim_i(t0)
            p = P.RUN['P'][i, e[2]]
            FL.add(t0, S_.to_world(p[0], p[1], 1.0), COL[e[2]] * 0.9, radius=2.6, peak=0.75, decay=0.5)
            PA.emit(t0, S_.to_world(p[0], p[1], R), 'glitch', n=20, spread=180, speed=(0.8, 2.0))

    def ko_fx(self, pos):
        """#31: the synth shatters into glitch and cyan sparks"""
        P = self.P
        P.PA.emit(P.T_KO, pos, 'glitch', n=24, spread=180, speed=(0.8, 2.2))
        P.PA.emit(P.T_KO, pos, 'sparks', n=20, col=CYAN_SPARK)

    # ---------------------------------------------------------------- the canvas (per pixel)
    def canvas(self, Ic, C):
        """the champion's solo: groove rings of light round it on the canvas (#31 render.py solo_rings, 1198-1206)"""
        P = self.P
        if P.WIN != self.k or not C.t >= P.T_SOLO - 0.01:
            return Ic
        T = self.T
        t = C.t
        st = C.fs[P.WIN]
        for bt, m, d in T.SOLO.get(P.NAMES[P.WIN], []):
            t0 = P.T_SOLO + bt * T.BEAT
            if not t0 <= t < t0 + 0.6:
                continue
            reach = 1.9 if d >= 0.75 else 1.1
            u = t - t0
            r = R + min(reach, 6.0 * u)
            a = 1.0 if u < reach / 6.0 else max(0.0, 1 - (u - reach / 6.0) / 0.25)
            rd = np.hypot(C.sxp - st['p'][0], C.szp - st['p'][1])
            band = np.clip(1 - np.abs(rd - r) / 0.16, 0, 1) * (rd < r + 0.05)
            band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
            Ic += (band * a * 0.5)[:, None] * (P.COL[P.WIN] / 255.0 * 0.9 + 0.25)[None]
        return Ic

    def canvas_wave(self, Ic, C, w):
        """one of its rings on the canvas; WHITE NOISE's a ring of static (#31 render.py 1207-1221)"""
        owner, cx, cz, r, rmax, kind, t0 = w
        t = C.t
        sxp, szp = C.sxp, C.szp
        rd = np.hypot(sxp - cx, szp - cz)
        wdt = 0.16 if kind == 'shockwave' else 0.32
        k_ = (1 - r / rmax) * (1.0 if kind == 'shockwave' else 1.3)
        if kind == 'wave':
            ang_ = np.arctan2(szp - cz, sxp - cx)
            rd = rd + 0.08 * np.sin(31 * ang_ + 70 * t) + 0.05 * np.sin(13 * ang_ - 40 * t)
        band = np.clip(1 - np.abs(rd - r) / wdt, 0, 1)
        band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
        Ic += (band * k_ * 0.9)[:, None] * (self.P.COL[owner] / 255.0 * 1.4 + (0.35 if kind == 'wave' else 0.15))[None]
        if kind == 'wave':
            h = (np.floor(sxp * 23) * 7.3 + np.floor(szp * 23) * 3.1 + math.floor(t * 30) * 1.7) % 1.0
            snow = (h > 0.72) & (np.abs(rd - r) < wdt * 1.6)
            Ic[snow] += 0.9 * k_
        return Ic

    # ---------------------------------------------------------------- the cables (#31 render.py draw_cables, draw_jacks)
    def live_cables(self, t):
        """[(id, state, k)] state: 'in' (plugged), 'retract' (unplugged: sliding home), 'snap' (snapped: two halves
        recoiling); k = 0..1 progress of the retract / snap"""
        P = self.P
        tc = P.fx_clock(t)
        out = []
        for cid, C in sorted(self.CABLES.items()):
            if C['t0'] <= tc < C['t1']:
                out.append((cid, 'in', 0.0))
            elif C['t1'] <= tc < C['t1'] + (0.3 if C['reason'] == 1 else 0.2) and t < P.T_HS + 0.3:
                out.append((cid, 'snap' if C['reason'] == 1 else 'retract',
                            (tc - C['t1']) / (0.3 if C['reason'] == 1 else 0.2)))
        return out

    def items_free(self, F):
        return [(CABLES_DEPTH, 'cables'), (JACKS_DEPTH, 'jacks')]

    def draw_item(self, F, payload):
        if payload == 'cables':
            arr = F.fr.array()
            glows = []
            jacks = self.draw_cables(arr, F.cam, F.t, F.fs, glows)
            F.fr.set_array(arr)
            self._frame = (F.f, jacks, glows)
        elif payload == 'jacks':
            f, jacks, _ = self._frame
            if f == F.f:
                self.draw_jacks(F.fr, F.cam, jacks)

    def draw_cables(self, arr, cam, t, fs, glows):
        """plugged cables (waveform by the synth's level, scrolling toward the enemy), plugs in flight with their trailing
        cable, unplugging cables sliding home, snapped cables recoiling, voltage pulses running down them, SAWTOOTH arms"""
        from fight import Fight
        P, SY = self.P, self.k
        COL = P.COL
        if not (self.T.T_FIGHT <= t < P.T_HS + 0.3) or fs[SY]['shattered'] or not fs[SY]['vis']:
            return []
        ps, pe = fs[SY]['p'], fs[1 - SY]['p']
        d = pe - ps
        dist = float(np.hypot(*d)) + 1e-9
        u = d / dist
        lvl = fs[SY]['level']
        kind = WAVEFORM.get(lvl, 'sine')
        wob = 0.6 if fs[SY]['haste'] > 0 else 0.0
        tc = P.fx_clock(t)
        phase = tc * 3.2
        cab = self.live_cables(t)
        n_in = sum(1 for c in cab if c[1] == 'in')
        base_ = np.clip(COL[SY] * 0.85 + 40, 0, 255)
        core = np.array([236, 252, 255])
        jacks = []
        j_in = 0
        a_s = ps + u * R * 0.95
        a_e = pe - u * R * 0.92
        for cid, state, k in cab:
            bow = 0.0
            if state == 'in':
                bow = (j_in - (n_in - 1) / 2) * 0.24
                j_in += 1
                Pp = ST.cable_points(a_s, a_e, kind, phase + cid * 0.37, amp=0.1, bow=bow, wobble=wob, t=tc)
                jacks.append((a_e, math.atan2(d[1], d[0]), bow))
            elif state == 'retract':
                e_ = a_e + (a_s - a_e) * ease(k)
                Pp = ST.cable_points(a_s, e_, kind, phase, amp=0.05)
            else:                                                     # snapped: both halves whip back
                mid = (a_s + a_e) / 2
                P1 = ST.cable_points(a_s, mid + (a_s - mid) * ease(k), kind, phase, amp=0.1 * (1 - k))
                P2 = ST.cable_points(mid + (a_e - mid) * ease(k), a_e, kind, phase, amp=0.1 * (1 - k))
                for Pq in (P1, P2):
                    s = cam.wp(Pq)
                    line_blend(arr, s, (10, 26, 40), 0.8 * (1 - k), 3)
                    line_blend(arr, s, base_, 1 - k, 1)
                continue
            s = cam.wp(Pp)
            line_blend(arr, s, (10, 26, 40), 0.9, 4)                 # the cable's dark sheath
            line_blend(arr, s, base_, 1.0, 2)                         # its lit waveform
            line_blend(arr, s, core, 0.55, 1)
        # plugs in flight, trailing their cable from the synth
        for pid, P_ in self.PLUGS.items():
            if P_['t0'] <= tc < P_['t1']:
                v = np.array([math.cos(P_['ang']), math.sin(P_['ang'])])
                p = P_['o'] + v * P_['speed'] * (tc - P_['t0'])
                s = cam.wp(ST.cable_points(a_s, p, kind, phase, amp=0.05))
                line_blend(arr, s, (10, 26, 40), 0.8, 3)
                line_blend(arr, s, base_, 0.9, 1)
                jacks.append((p, P_['ang'], None))
        # voltage pulses: bright dots running down the plugged cables (they reach the enemy as it is hit)
        if len(self.VOLT_T):
            j0, j1 = np.searchsorted(self.VOLT_T, tc - 0.6), np.searchsorted(self.VOLT_T, tc, 'right')
            ids_in = [c[0] for c in cab if c[1] == 'in']
            for t0, cid, n, tr in self.VOLTS[j0:j1]:
                uu = (tc - t0) / max(tr, 1e-3)
                if 0 <= uu <= 1 and cid in ids_in:
                    jj = ids_in.index(cid)
                    bow = (jj - (len(ids_in) - 1) / 2) * 0.24
                    Pp = ST.cable_points(a_s, a_e, kind, phase + cid * 0.37, amp=0.1, bow=bow, wobble=wob, t=tc)
                    q = Pp[min(len(Pp) - 1, int(uu * (len(Pp) - 1)))]
                    sq = cam.wp(q[None])[0]
                    xi, yi = int(sq[0]), int(sq[1])
                    if 1 <= xi < W - 2 and 1 <= yi < H - 2:
                        arr[yi - 1:yi + 2, xi - 1:xi + 2] = arr[yi - 1:yi + 2, xi - 1:xi + 2] * 0.3 + core * 0.7
                        arr[yi, xi] = 255
                    glows.append((sq[0], sq[1], 6, tuple(COL[SY] * 0.6 + 100), 0.55))
        # SAWTOOTH: the cables go saw-wave and lash side to side
        for L in self.LASHES:
            if L['t0'] <= tc < L['t1']:
                for j in range(L['n']):
                    a, ph = Fight.lash_angle(L, j, tc)
                    s = cam.wp(ST.saw_arm_points(ps, a, R + L['len']))
                    line_blend(arr, s, (10, 26, 40), 0.9, 4)
                    line_blend(arr, s, np.clip(COL[SY] * 0.6 + 110, 0, 255), 1.0, 2)
                    line_blend(arr, s, core, 0.6, 1)
                    tip = ps + (R + L['len']) * np.array([math.cos(a), math.sin(a)])
                    st_ = cam.wp(S.to_world(tip[0], tip[1], 0.5)[None])[0]
                    glows.append((st_[0], st_[1], 7, tuple(COL[SY] * 0.7 + 70), 0.5))
        return jacks

    def draw_jacks(self, fr, cam, jacks):
        COL = self.P.COL
        for p, ang, bow in jacks:
            barrel, tip = ST.jack_quad(p, ang)
            fr.poly(cam, barrel, np.array([40, 44, 56]))
            fr.poly(cam, tip, np.clip(COL[self.k] * 0.7 + 70, 0, 255))

    # ---------------------------------------------------------------- effects over the ring: a plug that misses
    def over_ring(self, F, arr, depth):
        """the sparks where a plug misses (#31 sparks(): its 'plug_miss' events, after the shards' frame)"""
        P = self.P
        t, cam = F.t, F.cam
        for i, e in P.events_between(t - 0.32, t, ('plug_miss',)):
            if e[2] != self.k:
                continue
            dt = t - e[0]
            x, z = e[4], e[5]
            col, n, sp, life = np.array([180, 230, 245]), 4, 1.2, 0.16
            if dt > life:
                continue
            rng = np.random.default_rng(i)
            fade = 1 - dt / life
            for j in range(n):
                a = rng.uniform(0, 2 * math.pi)
                v = sp * rng.uniform(0.4, 1.0)
                q = np.array([x + v * dt * math.cos(a), z + v * dt * math.sin(a)])
                hgt = R + rng.uniform(-0.2, 0.3) + 2.5 * dt - 6 * dt * dt
                s = cam.wp(S.to_world(q[0], q[1], max(0.02, hgt))[None])[0]
                xi, yi = int(s[0]), int(s[1])
                if 0 <= xi < W - 1 and 0 <= yi < H - 1:
                    arr[yi, xi] = np.clip(arr[yi, xi] * (1 - fade) + np.clip(col + 60, 0, 255) * fade, 0, 255)
        return arr, depth

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        """#31 draw_shards: keys, black keys (flat bars), knobs and jacks (dots)"""
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        c0 = np.array([sx_, sz_])
        if kind in ('key', 'bkey'):
            Lk, Wk = {'key': (0.30, 0.09), 'bkey': (0.22, 0.06)}[kind]
            c = {'key': np.array([240, 238, 230]), 'bkey': np.array([34, 32, 40])}[kind]
            q4 = [c0 + d * Lk / 2 * a1 + nn * Wk / 2 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            P.layer_blend(arr, [sp], c * (0.8 + 0.2 * math.cos(rot * 2)), 1.0)
        elif kind in ('knob', 'jack'):
            sp = P.screen(cam, S.to_world(sx_, sz_, p[1])[None])[0]
            xi, yi = int(sp[0]), int(sp[1])
            c = self.col * 0.7 + 70 if kind == 'jack' else np.array([60, 58, 70])
            arr[max(0, yi - 1):yi + 1, max(0, xi - 1):xi + 1] = c
            if kind == 'knob' and 0 <= yi < H and 0 <= xi < W:
                arr[yi, xi] = self.col * 0.8 + 50
        return arr

    # ---------------------------------------------------------------- lens, glows, post
    def shimmer(self, img, cam, t, w):
        """WHITE NOISE's front (#31 shimmers())"""
        owner, cx, cz, r, rmax, kind, t0 = w
        from reelkit import fx
        return fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r, 0.4, 3.0, t, k=1 - r / rmax, seed=owner)

    def glows(self, F, drawn):
        """the voltage pulses' and the lash tips' glows (#31 draw_cables' `glows`)"""
        f, _, glows = self._frame
        if f == F.f:
            for g in glows:
                F.fr.glow(*g)

    def after_grade(self, img, F, drawn):
        """its chromatic fringes, its big hits' and a snapped cable's, in event order (#31 post_fx; the kit's post_fx
        fringes only hits, so a snap between two hits would land out of order there)"""
        P = self.P
        from reelkit import fx
        t = F.t
        for _, e in P.events_between(t - 0.12, t, ('hit', 'snap')):
            if e[2] != self.k or (e[1] == 'hit' and e[7] not in self.own_fringe_hits):
                continue
            x, z = (e[4], e[5]) if e[1] == 'snap' else (e[5], e[6])
            s_ = F.cam.wp(S.to_world(x, z, R)[None])[0]
            big = e[1] == 'hit' and e[7] in ('slam', 'wave')
            img = fx.fringe(img, s_[0], s_[1], 26 if big else 16, 2.5 if big else 1.4, k=1 - (t - e[0]) / 0.12)
        return img


# ================================================================ #31 audio.py: the SYNTH's part
def h01_31(*xs):
    """#31's deterministic hash in [0, 1) (its thinning decisions)"""
    v = 0
    for x in xs:
        v = (v * 1000003 + int(x * 1000) + 7) % 2147483647
    return ((v * 48271) % 2147483647) / 2147483647.0


# #31 timeline.py ARP: satellite / pulse j plays ARP[chord][j] (chord tones over two octaves, a 9th / 11th on top)
ARP_31 = {
    'Fm': (65, 72, 68, 77, 75, 80, 79, 84),
    'Db': (65, 68, 73, 72, 77, 80, 75, 85),
    'Bbm': (65, 70, 73, 77, 72, 82, 80, 85),
    'C': (64, 67, 72, 70, 76, 79, 74, 82),
}
TITLE_31 = (65, 72, 77, 80)                       # the saw-chord blast and the stinger (#31 audio.py 422, 567)
COUNT_31 = (60, 64, 67)                           # a stab per count (433)
FALL_31 = (84, 79, 75, 72, 68, 63, 60, 56)        # the arp falling apart (554)
RUN_31 = (65, 68, 72, 75, 77, 80, 84, 89)         # its pieces flying home (591)


class Sound(base.Sound):
    bus_gain = 2.0                   # #31 BUS_GAIN['synth']
    send = 0.5                       # #31's reverb send for 'synth'

    def __init__(self, M, k):
        super().__init__(M, k)
        self._vc, self._gain, self._lv = {}, {}, {}

    # ---------------------------------------------------------------- tones (#31 audio.py)
    def _fit(self, x, cap):
        SR = A.SR
        n = int(cap * SR)
        x = np.asarray(x, float)[:n].copy()
        if len(x) < n:
            x = np.pad(x, (0, n - len(x)))
        fd = min(len(x) // 3, int(0.03 * SR))
        x[-fd:] *= np.linspace(1, 0, fd)
        return x

    def _inst_gain(self, name):
        import pp_bridge as B
        if name not in self._gain:
            lo, hi = B.entrant(name)[3]
            x = B.voice(name, (lo + hi) // 2, 0.8, 0.2)[:int(0.3 * A.SR)]
            self._gain[name] = min(4.0, 0.16 / (np.sqrt((A.hp(x, 120) ** 2).mean()) + 1e-6))
        return self._gain[name]

    def _note(self, name, midi, dur=0.18, cap=0.5, vel=0.85):
        import pp_bridge as B
        key = (name, int(midi), round(dur, 3), cap, round(vel, 2))
        if key not in self._vc:
            self._vc[key] = self._fit(B.voice(name, int(midi), vel, dur), cap) * self._inst_gain(name)
        return self._vc[key]

    def arp_note(self, m, vel=0.8, dur=0.12):
        """one arpeggiator note in the synth's voice, cut before its built-in echo (which sits off this tempo)"""
        return self._note(self.M.CFG[self.k]['pp'], m, dur, 0.34, vel)

    def voice(self, m, dur, vel=0.85):
        return self.arp_note(m, vel, dur)

    def arp_lv(self, m, lvl, vel=0.8):
        """an arp note toned like the waveform on its cables: sine (LV1), square (LV2), saw (LV3)"""
        key = (int(m), int(lvl), round(vel, 2))
        if key not in self._lv:
            x = self.arp_note(m, vel)
            n = len(x)
            f = A.mtof(m)
            env = np.abs(x).max() * A.expenv(n, 0.12, 0.003)
            if lvl <= 1:
                y = A.lp(x, 1100) * 0.8 + np.sin(2 * np.pi * f * A.tt(n)) * env * 0.5
            elif lvl == 2:
                y = x * 0.85 + A.lp(A.square(f, n), 3500) * env * 0.35
            else:
                y = x * 0.85 + A.lp(A.saw(f, n, 0.006, 2), 5000) * env * 0.4
            self._lv[key] = y
        return self._lv[key]

    @staticmethod
    def jack_click(seed=0):
        """a patch cable plugging in: a metallic click and a tiny hum"""
        n = int(0.08 * A.SR)
        t = A.tt(n)
        x = A.bp(np.random.default_rng(seed).normal(0, 1, n), 2000, 9000) * np.exp(-t / 0.005)
        x += 0.4 * np.sin(2 * np.pi * 120 * t) * np.exp(-t / 0.03)
        return x

    @staticmethod
    def zip_out(dur=0.16, seed=0):
        """the cable unspooling behind a plug: a short rising zip"""
        SR = A.SR
        n = int(dur * SR)
        t = A.tt(n)
        f = 800 + 3000 * t / dur
        x = np.sign(np.sin(2 * np.pi * np.cumsum(f) / SR)) * 0.3 + \
            A.bp(np.random.default_rng(seed).normal(0, 1, n), 1500, 6000)
        return x * np.sin(np.pi * t / dur) * 0.6

    @staticmethod
    def cable_snap(seed=0):
        """a snapped patch cable: a crack, then a falling electric zap"""
        SR = A.SR
        n = int(0.4 * SR)
        t = A.tt(n)
        crack = A.bp(np.random.default_rng(seed).normal(0, 1, n), 1500, 10000) * np.exp(-t / 0.02)
        f = 1800 * np.exp(-t / 0.08) + 90
        zap = np.sign(np.sin(2 * np.pi * np.cumsum(f) / SR)) * np.exp(-t / 0.12) * 0.5
        return crack + A.lp(zap, 5000)

    def wobble(self, m, dur, rate_beats=0.5):
        """LFO: a wobble bass, the filter swinging once per half beat"""
        SR = A.SR
        n = int(dur * SR)
        f = A.mtof(m)
        t = A.tt(n)
        x = A.saw(f, n, 0.01, 2)
        lfo = 0.5 - 0.5 * np.cos(2 * np.pi * t / (rate_beats * self.T.BEAT))
        out = np.zeros(n)
        nb = int(dur / 0.02)
        for k in range(nb):
            a, b = k * n // nb, (k + 1) * n // nb
            out[a:b] = A.lp(x[max(0, a - 512):b], 150 + 1800 * float(lfo[(a + b) // 2]), 2)[-(b - a):]
        return np.tanh(2.0 * out) * A.adsr(n, 0.01, 0.1, 0.9, 0.05)

    @staticmethod
    def supersaw_sweep(dur, m0=53, m1=89):
        n = int(dur * A.SR)
        t = A.tt(n)
        f = A.mtof(m0) * (A.mtof(m1) / A.mtof(m0)) ** (t / dur)
        ph = np.cumsum(f) / A.SR
        x = sum(2 * ((ph * (1 + d) + d * 3) % 1.0) - 1 for d in (-0.01, -0.004, 0.0, 0.004, 0.01)) / 5
        return A.hp(x, 300) * np.clip(t / 0.02, 0, 1) * np.exp(-np.maximum(t - dur * 0.6, 0) / (dur * 0.2))

    @staticmethod
    def noise_blast(dur=1.6):
        n = int(dur * A.SR)
        t = A.tt(n)
        x = np.random.default_rng(21).normal(0, 1, n)
        y = A.hp(x, 1200) * 0.7 + A.bp(x, 300, 3000) * 0.5
        return y * np.clip(t / 0.01, 0, 1) * np.exp(-t / (dur * 0.35)) * (1 + 0.5 * np.sign(np.sin(2 * np.pi * 9 * t)))

    # ---------------------------------------------------------------- harmony (#31's tables; a rule for other chords)
    def arp(self, ch):
        """the arp of chord `ch`: #31's table for its four chords, else the chord's tones climbing from E4"""
        if ch in ARP_31:
            return ARP_31[ch]
        tones = self.T.chord_tones(64, 100, ch)
        return tuple(tones[:8])

    def shift(self):
        """semitones from #31's tonic (F) to this config's tonic chord's root, in -6..5 (0 for #31)"""
        from ballfight.kit.episode import parse_chord
        r = parse_chord(self.T.CHORDS[0])[0]
        return (r - 5 + 6) % 12 - 6

    def bass_root(self, ch):
        """the chord's root in C1..B1 (#31's BASS_ROOT for its chords)"""
        return self.T.root_above(ch, 24)

    # ---------------------------------------------------------------- the score, in the order kit.music calls it
    def title_blast(self, mix, t):
        """a saw-chord blast (the synth's voice at its saw level) + a quick sweep"""
        sh = self.shift()
        for m in TITLE_31:
            mix.place(self.bus, self.arp_lv(m + sh, 3, 1.0), t, 0.16, pan=0.2)
        mix.place(self.bus, self.supersaw_sweep(0.45, 65 + sh, 89 + sh), t, 0.14, pan=0.2)

    def count_in(self, mix):
        """a synth stab per count"""
        sh = self.shift()
        for j, (tc, txt) in enumerate(self.T.COUNT[:3]):
            for m in COUNT_31:
                mix.place(self.bus, self.arp_note(m + sh, 0.9, 0.18), tc, 0.20, pan=0.2)

    def play(self, mix):
        """a voltage pulse per arp note, the cable waveform's tone; plugs, cuts, the noise, the cables' own sounds"""
        M, T, k = self.M, self.T, self.k
        RUN, EV, T_KO, S16 = M.RUN, M.EV, M.T_KO, T.S16
        cfg = M.CFG[k]
        slot_names = {s: cfg['abilities'][s]['name'] for s in ('unlock1', 'unlock2', 'low')}
        q16 = M.q16
        by_tick = {}
        for e in EV:
            if e[1] == 'volt' and e[2] == k and e[0] <= T_KO + 0.01:
                by_tick.setdefault(round(q16(e[0]) / S16), []).append(e)
        for slot, evs in sorted(by_tick.items()):
            tq = slot * S16
            hp = M.hp_at(k, tq) / 100.0
            if h01_31(tq, 3) < 0.55 * (1 - hp) ** 1.6:                # skips notes when hurt
                continue
            n = evs[0][4]
            ch = T.chord_at(tq)
            lvl = int(RUN['LV'][M.P.sim_i(tq), k])
            for j, e in enumerate(evs):
                m = self.arp(ch)[(n + 2 * j) % 8]
                sig, g, lead = self.colour(self.arp_lv(m, lvl, 0.8 if j == 0 else 0.6), tq, 1.0)
                mix.place(self.bus, sig, tq, (0.22 if j == 0 else 0.10) * g, pan=0.2 + 0.15 * math.sin(j * 1.7))
        for e in EV:
            t, kind = e[0], e[1]
            if t > T_KO + 0.01:
                break
            if kind == 'hit' and e[2] == k:
                tq = q16(t)
                how = e[7]
                pan = float(np.clip(e[5] / 3.5, -0.6, 0.6))
                ch = T.chord_at(tq)
                lvl = int(RUN['LV'][M.P.sim_i(tq), k])
                if how == 'plug':                                      # a plug lands: a jack click + an accent
                    mix.place('fx', self.jack_click(int(t * 31)), t, 0.16, pan=pan)
                    mix.place(self.bus, self.colour(self.arp_lv(self.arp(ch)[0] + 12, lvl, 1.0), t)[0], tq, 0.22,
                              pan=pan)
                elif how == 'lash':                                    # a SAWTOOTH cut
                    m = self.arp(ch)[int(e[8]) % 8] + 12
                    mix.place(self.bus, self.colour(self.arp_lv(m, 3, 1.0), t)[0], tq, 0.26, pan=pan)
                    mix.place('fx', A.thud(0.2, 160, 60, seed=int(t * 7)), tq, 0.10)
                elif how == 'wave':
                    mix.place('fx', A.thud(0.45, 110, 40), tq, 0.36)
            elif kind == 'plug_fire' and e[2] == k:                    # the cable unspools
                mix.place('fx', self.zip_out(0.16, int(t * 17)), t, 0.05, pan=0.2)
            elif kind == 'snap' and e[2] == k:
                mix.place('fx', self.cable_snap(int(t * 23)), t, 0.30, pan=float(np.clip(e[4] / 3.5, -0.6, 0.6)))
            elif kind == 'leash' and e[2] == k:
                mix.place('fx', A.thud(0.14, 75, 45, seed=int(t * 9)), t, 0.035,
                          pan=float(np.clip(e[3] / 3.5, -0.6, 0.6)))
            elif kind == 'ability' and e[2] == k:
                name = e[3]
                tq = q16(t)
                if name == slot_names['unlock1']:                      # LFO: a wobble bass joins
                    dur = cfg['abilities']['unlock1']['dur']
                    mix.place('bass', self.wobble(self.bass_root(T.chord_at(tq)) + 12, dur), tq, 0.20)
                elif name == slot_names['unlock2']:                    # SAWTOOTH: a supersaw sweep
                    mix.place(self.bus, self.supersaw_sweep(T.BEAT * 1.0), tq, 0.16)
                elif name == slot_names['low']:                        # WHITE NOISE: a noise blast + a high note
                    mix.place('fx', self.noise_blast(1.6), tq, 0.20)
                    mix.place(self.bus, self.colour(self.arp_note(89, 1.0, 0.3), t, 2.0)[0], tq, 0.16)

    def fall_apart(self, mix):
        """the arp falls apart, note by note"""
        sh = self.shift()
        for j, m in enumerate(FALL_31):
            x = np.tanh(3.0 * self.arp_note(m + sh, 0.9, 0.1))
            mix.place(self.bus, x, self.M.P.T_HS + 0.07 * j, 0.20 * (1 - j / 10), pan=(j - 3.5) / 8)

    def stinger(self, mix, tw):
        sh = self.shift()
        for m in TITLE_31:
            mix.place('solo', self.arp_note(m + sh, 1.0, 0.4), tw, 0.20)

    def solo(self, mix, t0):
        """a synth lead (config SOLO) from t0"""
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            mix.place('solo', self.arp_note(m, 0.95, max(0.1, d * T.BEAT * 0.9)), t0 + bt * T.BEAT, 0.34)

    def reassemble(self, mix):
        """its pieces fly home as a rising run"""
        T = self.T
        sh = self.shift()
        for j, m in enumerate(RUN_31):
            tj = self.M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(RUN_31))
            mix.place(self.bus, self.arp_note(m + sh, 0.7), tj, 0.18)
