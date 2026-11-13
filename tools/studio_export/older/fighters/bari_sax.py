"""BARI SAX (coral), the reference kit module, moved from #39 v4 Bassoon vs Bari Sax
(projects/ballfight_bassoon_vs_barisax, Oct 27: the roster's BARI SAX with `steer`; owner default Q3). Plan m4 Task 9,
Template F' Step 0: proven against that episode by Gate O (studio/reference/fighters/bari_sax/origin.json) before any Rust is written.

Look (#39's styles.py, imported from the episode's scratch copy, never retyped; #39 render.py's BARI SAX layers moved
into the kit's hooks with their names): the ball wears a leather harness (straps + a steel ring on the side facing the
cable); every WRECKING BALL cycle a crane line drops from the spot's rig (VERT = 1.6: the pendulum drawn taller), its
hook block stops over the swing's bottom, the hook bites the harness ring, the ball is hoisted back and swung through a
pendulum arc (motion smear + a coral trail under the ball, a coral light at the bottom), smashing what is in front of
it (burst, dust, sparks, grit, a skid mark, shake), then unhooks and the block flies back up. Out over the ropes (v3)
the ropes are pressed down under it and spring back when it lifts off (a rope event of speed 3.2); up over a front rope
it is drawn after the front ropes. GROWL = a gravelly honk ring on the canvas, its inked front, heat shimmer, grit.
CABLE SNAP = hoisted high over the enemy's spot (its shadow and a pulsing danger ring on the canvas), the cable snaps
(steel sparks up high, the upper cable whipping away, a stub on the falling ball), the ball craters the canvas (dust,
grit, a crack and a scorch). The HUD holds SKANK lit for its 8 beats and CABLE SNAP until the crash. It shatters into
lacquer sticks, nickel key chips and bell chips.
When a hop of the enemy's sails over the swinging ball (#39's leap, `leap_k`), the crane is drawn under the balls (an
items_free item at an infinite depth: first in the painter pass, right after the back ropes).

Not moved (they draw or play nothing for today's BARI SAX): v1's WINDOW RATTLER (RUN['RATTLE'], `rope_amp`, the 'honk',
'rattle', 'sling' events and their sounds) and LOW A ('quake'). Not the BARI SAX's: the BASSOON's TIPTOE (its hops,
including the 1.45 high leap over the swing, drawn by the BASSOON's look), its BOCAL HOOK swirl on the bari.

Picture opt-ins (older/picture_patches.py): ball_after_sprite (the harness), ball_late (over a front rope), rope_press
and rope_events (the pressed ropes and their spring-back), and two this port adds: ball_under (the swing's smear and
trail, drawn into the ball's own frame array right before the glass: #39 drew them with no 8-bit round trip between) and
ability_hold (SKANK's and CABLE SNAP's HUD hold, kinds the kit's `_hold` does not list).

Sound (#39 audio.py's BARI SAX part, unchanged; bus 'bari sax', #39's 'bari': Gate O's harness renames it, M4-R9;
 gain 1.3, send 0.2,
#39's colour()):
every swing one huge honk on the bass root of the bar it bottoms out in (SKANK's swings up the octave, their bottoms on
the off-beats), let go a 16th early, swelling to the bottom and bent like a doppler pass, the leader's octave on top,
a rush of air; the winch whirrs as the cable drops and the hook clanks home; a smash = a heavy thud, a clank, grit
(a crash cymbal when fast); GROWL a bent growl + grit; SKANK a crash; CABLE SNAP: the winch strains up, the cable parts
(two clanks, a whoosh, a falling whistle), the crash (thunder, a thud, the bed's boom, the biggest bent honk); a rope
it pressed twangs as it springs back; dropped on the ropes a low twang and the slung-back whoosh; back in, a thud. The
notes follow the config's key: #39's honk table is the chord's bass root (35..46 in E) or, below the bari's low C (its
register's floor, 36), its fifth; #39's literal E2 / E3 are the key's root in the C2 octave and an octave up."""
PORTED = True

import importlib.util
import math
import os

import numpy as np
from reelkit.config import W, H, FPS, BAYER_FULL, ss
from reelkit import fx
from reelkit import audio as A
import scene as S

from . import base

EPISODE = 'ballfight_bassoon_vs_barisax'
R = S.R_BALL
VERT = 1.6        # #39: the crane's pendulum drawn 1.6x taller (the view looks down at 58 deg: heights read short)
LEAP_H = 1.45     # #39: a hop that sails over the swinging wrecking ball (drawn this high by the hopper)
WOOD = np.array([234, 188, 86], float)                   # #39's shatter lacquer (the bari as loser)
NICKEL = np.array([214, 216, 226], float)
MAPLE_DUST = [(214, 176, 140), (186, 140, 110), (240, 214, 186)]
CORAL_SP = [(255, 196, 170), (255, 118, 92), (255, 240, 220)]
GRIT = [(60, 44, 40), (110, 84, 70), (170, 130, 110)]
SIDES = ((1, 1), (0, 1), (1, -1), (0, -1))              # #39 _rope_wobbles: (axis, sign) of rope sides 0..3


def _episode_styles():
    """#39's styles.py, from the episode's scratch copy (series rule: styles live there so crossovers import them)"""
    from tools.studio_export.older import origin as O
    folder = O.scratch(EPISODE)
    spec = importlib.util.spec_from_file_location('ballfight_bassoon_vs_barisax_styles',
                                                  os.path.join(folder, 'styles.py'))
    m = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(m)
    return m


ST = _episode_styles()


def _w(p, y):
    return S.to_world(p[0], p[1], y)


# ===================================================================================================== look
class Look(base.Look):
    shards = ['wood'] * 6 + ['key'] * 8 + ['bell'] * 3          # #39's: lacquer sticks, nickel key chips, bell chips
    flash_hits = ('quake', 'sling', 'hook', 'wreck', 'crash')     # #39 FLASH_EV
    fringe_hits = ('quake', 'sling', 'hook', 'shockwave', 'wreck', 'crash')     # #39 post_fx
    speed_lift = True                                            # #39: speed lines at the ball's height (R + h)

    # ---------------------------------------------------------------- setup (#39 render.py's event lists)
    def setup(self):
        P, k = self.P, self.k
        EV = P.EV
        self.wrecker = self.cfg['weapon']['kind'] == 'wrecking'
        self.snapper = any(a['kind'] == 'snap' for a in self.cfg['abilities'].values())
        self.WRECK = np.array(P.RUN['WRECK']) if 'WRECK' in P.RUN else None    # (#39's sim.load makes them arrays;
        self.SNAP = np.array(P.RUN['SNAP']) if 'SNAP' in P.RUN else None       # the kit's story.load does not)
        self.SWINGS = [e for e in EV if e[1] == 'swing']
        self.SMASHES = [(e[0], e[2], e[3], e[4], e[5]) for e in EV if e[1] == 'smash']
        self.SNAPS = [(e[0], e[2], np.array([e[3], e[4]]), e[5], e[6], e[7]) for e in EV if e[1] == 'snap_aim']
        self.SNAPS = [q for q in self.SNAPS if q[1] == k]
        hops = [(e[0], e[2], np.array([e[3], e[4]]), np.array([e[5], e[6]]), e[7], e[8]) for e in EV if e[1] == 'hop']
        self.E_HOPS = [h for h in hops if h[1] == self.enemy]   # the enemy's hops (a leap over the swing: crane first)
        self.E_HOP_T = np.array([h[0] for h in self.E_HOPS])
        self.CYCLES = self._cycles()
        self._leap = {}
        self._over = {}
        self._cs = {}
        self._h_ko = None

    def _cycles(self):
        """every WRECKING BALL cycle: the cable drop, the hook, the hoist, the swing (from the run's events + the WRECK
        record), cut short where CABLE SNAP takes over (#39 _cycles)"""
        P, k, T = self.P, self.k, self.T
        EV, RUN = P.EV, P.RUN
        out = []
        lift = self.cfg['weapon']['lift16'] * T.S16 if self.wrecker else 0.0
        for e in EV:
            if e[1] != 'cable' or e[0] > P.T_KO or e[2] != k:
                continue
            t0, t_hook = e[0], e[3]
            sw = [q for q in self.SWINGS if q[2] == k and t_hook - 1e-6 <= q[0] <= t_hook + lift + 0.05]
            nxt = [q[0] for q in EV if q[1] in ('cable', 'snap_aim') and q[2] == k and q[0] > t0]
            cut = min(nxt + [P.T_KO + 1e-3])
            c = dict(k=k, t0=t0, t_hook=t_hook, sw=None, cut=cut, smash=None)
            if sw and sw[0][0] < cut:
                q = sw[0]
                ps, p1, bot = np.array([q[3], q[4]]), np.array([q[5], q[6]]), np.array([q[7], q[8]])
                d = p1 - ps
                u = d / (np.linalg.norm(d) + 1e-9)
                i = P.sim_i(q[0] - 0.5 / FPS)
                row = self.WRECK[i, k]
                p0 = row[5:7].copy() if row[0] in (2, 3) else ps.copy()
                c.update(sw=q, ps=ps, p1=p1, bot=bot, u=u, L=q[9], th=q[10], ts0=q[11], ts1=q[12], tl0=q[11] - lift,
                         p0=p0, skank=q[13])
                hits = [m for m in self.SMASHES if m[1] == k and q[11] - 1e-6 <= m[0] <= q[12] + 1e-6]
                c['smash'] = hits[0] if hits else None
                c['end'] = min(q[12], cut)
            else:
                c['end'] = min(t_hook, cut)
            out.append(c)
        return out

    # ---------------------------------------------------------------- clocks (#39 render.py)
    def cycle_at(self, t, tail=0.55):
        """the wrecking cycle on screen at t (from its cable drop until the block is back up), or None"""
        for c in self.CYCLES:
            if c['t0'] <= t < c['end'] + tail:
                return c
        return None

    @staticmethod
    def wreck_pose(c, t):
        """(p, h, phase) of the wrecking ball in cycle c at t: 'lift' (hoisted back), 'swing' (the pendulum, exact
        between frames), or None"""
        if c is None or t < c['t0'] or t >= c['end'] or c['sw'] is None:
            return None
        if t < c['tl0']:
            return None
        L, th, u = c['L'], c['th'], c['u']
        if t < c['ts0']:
            uu = (t - c['tl0']) / max(1e-6, c['ts0'] - c['tl0'])
            e = uu * uu * (3 - 2 * uu)
            return c['p0'] + (c['ps'] - c['p0']) * e, VERT * L * (1 - math.cos(th)) * e, 'lift'
        uu = min(1.0, (t - c['ts0']) / max(1e-6, c['ts1'] - c['ts0']))
        phi = -th * math.cos(math.pi * uu)
        return c['bot'] + u * (L * math.sin(phi)), VERT * L * (1 - math.cos(phi)), 'swing'

    def snap_at(self, t):
        """CABLE SNAP on at t: (snap, phase 'hoist' / 'fall', u 0..1) or (None, None, None)"""
        for q in self.SNAPS:
            t0, kk, spot, ts, t1, rad = q
            if t0 <= t < t1 and t0 <= self.P.T_KO:
                if t < ts:
                    return q, 'hoist', (t - t0) / max(1e-6, ts - t0)
                return q, 'fall', (t - ts) / max(1e-6, t1 - ts)
        return None, None, None

    def air_pose(self, t):
        """(p, h) of the bari off the canvas at t (fight time), or None: the swing / hoist (exact), CABLE SNAP (record),
        v3's drop on the ropes (record)"""
        P, k = self.P, self.k
        if not self.wrecker and not self.snapper:
            return None
        sn, ph, u = self.snap_at(t)
        if sn is not None:
            i = P.sim_i(t)
            return P.RUN['P'][i, k].copy(), float(self.SNAP[i, k, 1])
        if self.wrecker:
            pose = self.wreck_pose(self.cycle_at(t, 0.0), t)
            if pose is not None:
                return pose[0], pose[1]
            i = P.sim_i(t)
            if self.T.T_FIGHT <= t < P.T_KO and int(self.WRECK[i, k, 0]) == 4:   # dropped on the ropes, slung back
                return P.RUN['P'][i, k].copy(), VERT * float(self.WRECK[i, k, 1])
        return None

    def h_ko(self):
        """its height at the K.O. (caught mid-swing: it drops after the hit-stop)"""
        if self._h_ko is None:
            self._h_ko = (self.air_pose(self.P.T_KO - 1e-4) or (None, 0.0))[1]
        return self._h_ko

    def e_hop_at(self, t):
        """the enemy's hop in the air at t, or None (#39 hop_at)"""
        ts = self.E_HOP_T
        if not len(ts) or t >= self.P.T_KO or t < self.T.T_FIGHT:
            return None
        j = int(np.searchsorted(ts, t, 'right')) - 1
        if j < 0:
            return None
        h = self.E_HOPS[j]
        return h if h[0] <= t < h[4] else None

    def leaps(self, hop):
        """does one of the enemy's hops pass over the swinging wrecking ball? (#39 leap_k > 1)"""
        key = hop[0]
        if key not in self._leap:
            f = 1.0
            if self.wrecker:
                for u in np.linspace(0.1, 0.9, 9):
                    tt = hop[0] + (hop[4] - hop[0]) * u
                    pose = self.wreck_pose(self.cycle_at(tt, 0.0), tt)
                    if pose is not None and pose[2] == 'swing':
                        p = hop[2] + (hop[3] - hop[2]) * u
                        if np.hypot(*(p - pose[0])) < 2 * R * 1.1:
                            f = max(1.0, LEAP_H / ST.hop_height(float(np.hypot(*(hop[3] - hop[2])))))
                            break
            self._leap[key] = f
        return self._leap[key] > 1.0

    def crane_first(self, t):
        """#39: a leap over the swing passes in front of the cable: the crane goes under the balls"""
        if not (self.T.T_FIGHT <= t < self.P.T_HS):
            return False
        hop = self.e_hop_at(t)
        return hop is not None and self.leaps(hop)

    # ---------------------------------------------------------------- the ball: its height on the cable
    @staticmethod
    def _lift(st, h):
        """#39's ball height `h` in the kit's state: drawn at R + h (lift), painted at the resting height and, when
        airborne, painted and depth-written 0.6 nearer (it clears what it swings past)"""
        st['lift'] = h
        if h != 0:
            st['paint_h'] = R
        if h > 0.02:
            st['air_bias'] = 0.6

    def state(self, kk, st, t, phase, gl=0.0):
        if kk != self.k:
            return
        P = self.P
        if phase == 'fight':                                      # swung on the cable / hoisted for CABLE SNAP
            ap = self.air_pose(min(t, P.T_KO - 1e-4))
            if ap is not None:
                st['p'] = ap[0]
                self._lift(st, ap[1])
        elif phase == 'tail' and kk == P.WIN:                     # caught in the air at the K.O.: it drops
            self._lift(st, self.h_ko() * (1 - ss(P.T_HS, P.T_HS + 0.4, t)))

    def ball_under(self, kk, st, t, arr, cam):
        """(opt-in) the swing's smear and trail right under the ball, and the harness ring's side while hooked"""
        if kk != self.k or not self.wrecker or not (self.T.T_FIGHT <= t < self.P.T_HS):
            return
        self.draw_swing_fx(arr, cam, t)
        cs_ = self.crane_state(t)
        if cs_ is not None and cs_['attached']:
            a_ = cam.wp(S.to_world(st['p'][0], st['p'][1], R + st['lift'])[None])[0]
            b_ = cam.wp(cs_['block'][None])[0]
            d_ = b_ - a_
            st['ring_dir'] = d_ / (np.linalg.norm(d_) + 1e-9)

    def ball_after_sprite(self, kk, st, t, arr, s, rp):
        """(opt-in) the harness: two leather straps crossing at a steel ring, over the instrument"""
        if kk == self.k and self.wrecker and st['scale'] > 0.5:
            ST.harness(arr, s, rp, st.get('sx', 1.0), st.get('sy', 1.0), st.get('ring_dir'), self.col)

    def ball_late(self, kk, F, st):
        """(opt-in) up over a front rope: drawn after it"""
        return (kk == self.k and self.wrecker and F.in_fight and st['vis'] and st['lift'] > 0.02
                and max(st['p'][0], st['p'][1]) > S.A - R * 1.05)

    def ability_hold(self, slot):
        """(opt-in) how long its HUD row stays lit after a use: SKANK for its beats, CABLE SNAP until the crash"""
        ab = self.cfg['abilities'][slot]
        if ab['kind'] == 'offbeat':
            return ab['dur_beats'] * self.T.BEAT if 'dur_beats' in ab else ab['dur']
        if ab['kind'] == 'snap':
            return ab['tele16'] * self.T.BEAT / 4
        return None

    # ---------------------------------------------------------------- the ropes (v3: out over them)
    def bari_over(self, t):
        """[(ring-frame centre, underside height)] of the bari while it is up on the cable / slung back and its
        footprint reaches the rope line: the ropes are pressed down under it"""
        if t in self._over:
            return self._over[t]
        out = []
        if self.wrecker and self.T.T_FIGHT <= t < self.P.T_HS:
            st = self.P.fighter(self.k, t)
            if st['vis'] and st['lift'] > 0.02 and max(abs(st['p'][0]), abs(st['p'][1])) > S.A - R * 1.05:
                out.append((np.asarray(st['p'], float), float(st['lift'])))
        if len(self._over) > 64:
            self._over.clear()
        self._over[t] = out
        return out

    def rope_press(self, s, y, t, P, yy):
        """(opt-in) the ropes pressed down under the bari, easing back up either side of it"""
        for c, hb in self.bari_over(t):
            dd = np.hypot(P[:, 0] - c[0], P[:, 1] - c[1])
            under = hb + R - np.sqrt(np.clip(R * R - dd * dd, 0, None)) - 0.03    # the ball's underside above it
            on = dd < R
            yy = np.where(on, np.minimum(yy, under), yy)
            near = (dd >= R) & (dd < R * 1.7)
            if near.any():
                w = 1 - (dd - R) / (R * 0.7)
                yy = np.where(near, np.minimum(yy, yy - np.clip(yy - (hb - 0.03), 0, None) * w * w * 0.6), yy)
        return np.maximum(yy, 0.05)

    def rope_events(self):
        """(opt-in) when the bari lifts off a rope it pressed, the rope springs back: a rope event there (speed 3.2
        marks it for the twang), #39 _rope_wobbles"""
        out = []
        prev = {}
        f0, f1 = int(self.T.T_FIGHT * FPS), int(self.P.T_KO * FPS)
        for f in range(f0, f1 + 1):
            t = f / FPS
            cur = {}
            for c, hb in self.bari_over(t):
                for s_, (ax, sg) in enumerate(SIDES):
                    if c[ax] * sg > S.A - R * 1.05 and hb < S.ROPE_Y[-1]:
                        cur[s_] = c.copy()
            for s_, c in prev.items():
                if s_ not in cur:
                    out.append((t, s_, float(c[0]), float(c[1]), 3.2))
            prev = cur
        return out

    # ---------------------------------------------------------------- events -> camera, ropes, sparks, glows
    def shake(self, e):
        if e[0] > self.P.T_KO:
            return None
        if e[1] == 'hit' and e[7] in ('shockwave', 'quake', 'sling'):
            return {'shockwave': 1.6, 'quake': 2.2, 'sling': 1.4}[e[7]]
        if e[1] == 'smash' and e[2] == self.k:                    # a WRECKING BALL smash (harder the faster)
            return 1.0 + 1.2 * e[5]
        if e[1] == 'crater' and e[2] == self.k:                   # CABLE SNAP: the crash (and its echo: fx_setup)
            return 2.5
        return None

    def rope(self, e):
        if e[1] == 'ability' and e[4] in ('shockwave', 'quake'):
            return 1.0 if e[4] == 'quake' else 0.5
        return None

    def spark(self, e):
        how = e[7]
        n, sp, life = (16, 3.2, 0.3) if how in ('quake', 'sling', 'shockwave', 'hook', 'wreck', 'crash') \
            else (8, 2.4, 0.22)
        return self.P.COL[e[2]] * 0.6 + 100, n, sp, life

    def hit_glow(self, e):
        if e[7] in ('quake', 'sling', 'shockwave', 'rattle', 'wreck', 'crash'):
            big = e[7] in ('quake', 'sling', 'shockwave', 'wreck', 'crash')
            return (14, 0.35) if big else (8, 0.2)
        return None

    # ---------------------------------------------------------------- effects (reelkit.fx), in #39's order
    def fx_setup(self):
        """the crash's second jolt (#39 adds two shakes per crater: 2.5 at the crash, 1.2 at +0.09 s)"""
        P = self.P
        for e in P.EV:
            if e[1] == 'crater' and e[2] == self.k and e[0] <= P.T_KO:
                P.SHK.add(e[0] + 0.09, 1.2)

    def fx_event(self, e):
        P, k = self.P, self.k
        up = P.up
        t_, kind = e[0], e[1]
        if kind == 'hit':
            att, x, z, how = e[2], e[5], e[6], e[7]
            pos = up(x, z, R)
            if how == 'shockwave':                                 # GROWL lands: grit
                P.BU.add(t_, pos, P.COL[att], 1.2)
                P.FL.add(t_, pos, P.FL_COL[att], radius=1.8, peak=0.8, decay=0.35)
                P.PA.emit(t_, pos, 'chips', n=12, col=GRIT)
            elif how == 'wreck':                                   # WRECKING BALL smash: burst, dust, a skid mark
                sp_ = [m[4] for m in self.SMASHES if abs(m[0] - t_) < 1e-4]
                sp_ = sp_[0] if sp_ else 0.7
                P.BU.add(t_, pos, P.COL[att], 1.0 + 0.7 * sp_)
                P.FL.add(t_, pos, P.FL_COL[att], radius=2.0, peak=0.6 + 0.5 * sp_, decay=0.32)
                P.PA.emit(t_, up(x, z, 0.06), 'dust', n=6 + int(8 * sp_), col=MAPLE_DUST, scale=1.1)
                P.PA.emit(t_, pos, 'sparks', n=10, col=CORAL_SP)
                P.PA.emit(t_, pos, 'chips', n=5, col=GRIT + MAPLE_DUST[:1])
                P.MK.add(t_, 'scorch', x, z, size=0.32 + 0.2 * sp_, col=(46, 30, 30), life=3.0, seed=int(t_ * 100),
                         strength=0.45)
            elif how == 'crash':                                   # CABLE SNAP lands on its target
                P.BU.add(t_, pos, P.COL[att], 1.8)
                P.FL.add(t_, pos, P.FL_COL[att], radius=2.6, peak=1.1, decay=0.45)
        elif kind == 'swing' and e[2] == k:                        # the swing's bottom: a coral light
            tb = (e[11] + e[12]) / 2
            if tb <= P.T_KO:
                P.FL.add(tb, up(e[7], e[8], R), P.FL_COL[k], radius=2.0, peak=0.45, decay=0.25)
        elif kind == 'cable_snap' and e[2] == k:                   # the cable parts: steel sparks up high
            i_ = P.sim_i(t_)
            p_ = P.RUN['P'][i_, k]
            top = up(p_[0], p_[1], R + float(self.SNAP[i_, k, 1]) + 2 * R + 0.5)
            P.BU.add(t_, top, (230, 234, 245), 0.9, bright=False)
            P.PA.emit(t_, top, 'sparks', n=14, spread=180, col=[(255, 250, 230), (226, 230, 242), (255, 196, 120)])
            P.FL.add(t_, top, (255, 240, 220), radius=1.8, peak=0.6, decay=0.25)
        elif kind == 'crater' and e[2] == k:                       # the crash: a ring of dust, grit, the crater mark
            x, z = e[3], e[4]
            P.PA.emit(t_, up(x, z, 0.05), 'dust', n=26, spread=180, col=MAPLE_DUST, scale=1.4, speed=(1.2, 3.0))
            P.PA.emit(t_, up(x, z, R * 0.5), 'chips', n=14, spread=180, col=GRIT, speed=(1.5, 3.4))
            P.MK.add(t_, 'crack', x, z, size=1.5, life=6.0, n=7, seed=int(t_ * 10), strength=0.75)
            P.MK.add(t_, 'scorch', x, z, size=0.95, col=(34, 22, 24), life=6.0, seed=int(t_ * 10) + 1, strength=0.7)
        elif kind == 'ability' and e[4] == 'shockwave':            # GROWL leaves the bari: grit
            p = P.RUN['P'][P.sim_i(t_), e[2]]
            P.PA.emit(t_, up(p[0], p[1], R * 0.6), 'chips', n=14, spread=180, col=GRIT, speed=(1.5, 3.0))

    def ko_fx(self, pos):
        self.P.PA.emit(self.P.T_KO, pos, 'brass', n=26, spread=180, speed=(0.8, 2.2),
                       col=[tuple(WOOD), (214, 216, 226), (232, 208, 150)])

    # ---------------------------------------------------------------- the canvas
    def canvas_wave(self, Ic, C, w):
        """GROWL's gravelly, broken ring (#39 honk_light)"""
        owner, cx, cz, r, rmax, kind, t0 = w
        band = ST.honk_light(C.sxp, C.szp, cx, cz, r, rmax, C.t, kind)
        band = np.floor(band * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
        Ic += (band * (1.3 if kind == 'quake' else 1.0))[:, None] * (self.col / 255.0 * 1.3 + 0.2)[None]
        return Ic

    def canvas_after(self, Ic, C):
        """CABLE SNAP's target: the falling ball's shadow and a pulsing danger ring"""
        P, t = self.P, C.t
        tcn = min(t, P.T_KO)
        for t0_, k_s, spot_, ts_, t1_, rad_ in self.SNAPS:
            if t0_ <= tcn < t1_ and t < P.T_HS:
                u_ = (tcn - t0_) / max(1e-6, t1_ - t0_)
                shd, rng_ = ST.snap_target(C.sxp, C.szp, spot_[0], spot_[1], R, rad_, u_, t)
                Ic *= (1 - 0.8 * shd)[:, None]
                rng_ = np.floor(rng_ * 3 + BAYER_FULL[C.ys, C.xs] * 0.9) / 3
                Ic += (1.1 * rng_)[:, None] * (self.col / 255.0 * 1.3 + 0.2)[None]
        return Ic

    # ---------------------------------------------------------------- the crane (#39 crane_state, draw_crane)
    def crane_state(self, t):
        """world geometry of the crane at t: dict(top, block, end, attached, pieces=[(points, alpha)]) or None. Frozen
        through the hit-stop; after it any live crane flies back up to the rig"""
        if t not in self._cs:
            if len(self._cs) > 64:
                self._cs.clear()
            self._cs[t] = self._crane_state(t)
        return self._cs[t]

    def _crane_state(self, t):
        P, k, T = self.P, self.k, self.T
        RUN = P.RUN
        if (not self.wrecker and not self.snapper) or t < T.T_FIGHT:
            return None
        if t >= P.T_HS:
            st = self._crane_state(P.T_KO - 1e-4)
            if st is None:
                return None
            v = (t - P.T_HS) / 0.6
            if v >= 1.0:
                return None
            b = st['block'].copy()
            b[1] += (ST.Y_RIG - b[1]) * v * v
            st.update(block=b, end=b - np.array([0.0, 0.42, 0.0]), attached=False,
                      pieces=[(Q, a * (1 - v)) for Q, a in st['pieces']])
            return st
        tf = min(t, P.T_KO - 1e-4)
        sn, ph, u = self.snap_at(tf)
        if sn is None:                                       # the snapped cable's ends, just after the crash
            for q in self.SNAPS:
                if q[4] <= tf < q[4] + 0.6:
                    sn, ph, u = q, 'after', (tf - q[4]) / 0.6
        if sn is not None:
            t0, _, spot, ts, t1, rad = sn
            i = P.sim_i(tf)
            p = RUN['P'][i, k]
            h = float(self.SNAP[i, k, 1]) if ph != 'after' else 0.0
            C = _w(p, R + h)
            ring = C + np.array([0.0, R, 0.0])
            if ph == 'hoist':
                tgt = ring + np.array([0.0, 1.1, 0.0])
                prev = [c for c in self.CYCLES if c['sw'] is not None and abs(c['cut'] - t0) < 1e-3]
                if prev and u < 0.3:                          # out of a swing: the block slides over from its pivot
                    pv = _w(prev[0]['bot'], R + VERT * prev[0]['L'])
                    tgt = pv + (tgt - pv) * ss(0.0, 0.3, u)
                return dict(top=np.array([tgt[0], ST.Y_RIG, tgt[2]]), block=tgt, end=ring, attached=True, pieces=[])
            # snapped: the upper cable whips up and away, a stub flops on the falling ball
            tsn = tf - ts
            i_s = P.sim_i(ts)
            ps_ = RUN['P'][i_s, k]
            b0 = _w(ps_, R + float(self.SNAP[i_s, k, 1]) + R + 1.1)
            blk = b0 + np.array([0.35 * tsn, 7.0 * tsn + 6.0 * tsn * tsn, 0.0])
            brk = blk - np.array([0.0, 0.55, 0.0])
            up_piece = ST.cable(blk, brk + np.array([0.2 * math.sin(tsn * 30), 0.0, 0.0]), 6,
                                wig=0.12 * math.exp(-tsn / 0.3), ph=tsn * 40)
            stub_top = ring + np.array([0.15 * math.sin(tsn * 22), 0.5 * math.exp(-tsn / 0.4) + 0.08, 0.1])
            stub = ST.cable(ring, stub_top, 5, wig=0.06, ph=tsn * 25)
            fade = 1.0 - (u if ph == 'after' else 0.0)
            return dict(top=np.array([blk[0], ST.Y_RIG, blk[2]]), block=blk, end=None, attached=False,
                        pieces=[(up_piece, max(0.0, 1 - tsn / 0.7)), (stub, fade)])
        c = self.cycle_at(tf)
        if c is None:
            return None
        if c['sw'] is not None:
            Pv = _w(c['bot'], R + VERT * c['L'])
        else:
            Pv = _w(RUN['P'][P.sim_i(c['t0']), k], R + VERT * self.cfg['weapon']['base'])
        i = P.sim_i(tf)
        pose = self.wreck_pose(c, tf)
        p = pose[0] if pose is not None else RUN['P'][i, k]
        h = pose[1] if pose is not None else 0.0
        C = _w(p, R + h)
        if tf < c['t_hook'] and tf < c['end']:                   # the drop: the block comes down, the hook reaches
            u = (tf - c['t0']) / max(1e-6, c['t_hook'] - c['t0'])
            y = ST.Y_RIG + (Pv[1] - ST.Y_RIG) * (1 - (1 - u) ** 3)
            blk = np.array([Pv[0], y, Pv[2]])
            ring = C + np.array([0.0, R, 0.0])
            dang = blk - np.array([0.0, 0.42, 0.0])
            end = dang + (ring - dang) * ss(0.55, 1.0, u)
            return dict(top=np.array([Pv[0], ST.Y_RIG, Pv[2]]), block=blk, end=end, attached=False, pieces=[])
        if tf < c['end']:                                        # hooked: hoisted back, the swing
            d = Pv - C
            ring = C + d / (np.linalg.norm(d) + 1e-9) * R
            return dict(top=np.array([Pv[0], ST.Y_RIG, Pv[2]]), block=Pv, end=ring, attached=True, pieces=[])
        v = (tf - c['end']) / 0.55                               # unhooked: the block flies back up
        blk = Pv + np.array([0.0, (ST.Y_RIG - Pv[1]) * v * v, 0.0])
        end_c = self.cycle_end_ring(c)
        dang = blk - np.array([0.0, 0.42, 0.0]) + np.array([0.18 * math.sin(v * 9) * (1 - v), 0.0, 0.0])
        end = end_c + (dang - end_c) * ss(0.0, 0.3, v)
        return dict(top=np.array([Pv[0], ST.Y_RIG, Pv[2]]), block=blk, end=end, attached=False, pieces=[])

    def cycle_end_ring(self, c):
        """where the hook let go at the end of cycle c (the harness ring at the far end of the swing)"""
        P = self.P
        pose = self.wreck_pose(c, c['end'] - 1e-4)
        if pose is None:
            p = P.RUN['P'][P.sim_i(c['end']), self.k]
            return _w(p, 2 * R)
        Pv = _w(c['bot'], R + VERT * c['L']) if c['sw'] is not None else _w(pose[0], 2 * R + 1)
        C = _w(pose[0], R + pose[1])
        d = Pv - C
        return C + d / (np.linalg.norm(d) + 1e-9) * R

    @staticmethod
    def draw_crane(arr, cam, cs):
        """the crane line from the rig, the hook block, the swing cable and its hook, the snapped pieces"""
        if cs is None:
            return arr
        blk = cs['block']
        arr = ST.line3(arr, cam, ST.cable(cs['top'], blk + np.array([0.0, 0.17, 0.0]), 6), ST.STEEL * 0.75, 2, hi=0.3)
        arr = ST.hook_block(arr, cam, blk)
        if cs['end'] is not None:
            bot = blk - np.array([0.0, 0.2, 0.0])
            slack = 0.0 if cs['attached'] else 0.02
            arr = ST.line3(arr, cam, ST.cable(bot, cs['end'], 8, sag=slack), ST.STEEL, 2)
            tw = blk - cs['end']
            arr = ST.hook_shape(arr, cam, cs['end'], tw / (np.linalg.norm(tw) + 1e-9))
        for Q, a in cs['pieces']:
            if a > 0.02:
                arr = ST.line3(arr, cam, Q, ST.STEEL, 2, alpha=a)
        return arr

    def draw_swing_fx(self, arr, cam, t):
        """the swing's motion smear (ghost balls a few ms back) and its coral trail, under the ball"""
        P = self.P
        tf = min(t, P.T_KO - 1e-4)
        c = self.cycle_at(tf, 0.0)
        if c is None or c['sw'] is None or not (c['tl0'] <= tf < c['ts1'] + 0.12):
            return arr
        long_hoist = float(np.hypot(*(c['ps'] - c['p0']))) > 1.2         # v4: a long, fast yank (a turned swing)
        phases = ('swing', 'lift') if long_hoist else ('swing',)
        pts = []
        for j in range(12, -1, -1):
            tt = tf - j * 0.018
            pose = self.wreck_pose(c, tt)
            if pose is None or pose[2] not in phases:
                continue
            pts.append(cam.wp(_w(pose[0], R + pose[1])[None])[0])
        uu = min(1.0, max(0.0, (tf - c['ts0']) / max(1e-6, c['ts1'] - c['ts0'])))
        spd = math.sin(math.pi * uu) if tf >= c['ts0'] else 0.8
        if tf < c['ts0'] and not long_hoist:
            return arr
        if len(pts) >= 2:
            arr = ST.swing_trail(arr, pts, self.col, k=0.35 + 0.65 * spd)
        for j in (4, 3, 2, 1):                                   # ghosts, oldest first
            pose = self.wreck_pose(c, tf - j * 0.014)
            if pose is None or pose[2] not in phases:
                continue
            Pc = cam.to_cam(_w(pose[0], R + pose[1])[None])[0]
            s_ = cam.proj(Pc[None])[0]
            rp = R * cam.f / Pc[2]
            a = np.linspace(0, 2 * math.pi, 19)[:-1]
            circ = np.column_stack([s_[0] + rp * np.cos(a), s_[1] + rp * np.sin(a)])
            P.layer_blend(arr, [circ], np.clip(self.col * 0.7 + 70, 0, 255), 0.24 * (1 - j / 5) * (0.3 + 0.7 * spd))
        return arr

    # ---------------------------------------------------------------- the painter pass, over the ring
    def items_free(self, F):
        if self.wrecker and self.crane_first(F.t):               # the crane under the balls, after the back ropes
            return [(math.inf, 'crane')]
        return []

    def draw_item(self, F, payload):
        if payload != 'crane':
            return
        arr, depth = F.fr.array(), F.fr.depth()
        arr = self.draw_crane(arr, F.cam, self.crane_state(F.t))
        F.fr.set_array(arr, depth)

    def over_ring(self, F, arr, depth):
        """the crane over the ring (unless a leap put it under the balls), GROWL's inked fronts"""
        if self.wrecker and not self.crane_first(F.t):
            arr = self.draw_crane(arr, F.cam, self.crane_state(F.t))
        if F.in_fight:
            for owner, cx, cz, r, rmax, kind, t0 in self.P.RUN['WAVES'][F.i]:
                if owner == self.k:
                    arr = ST.draw_honk_front(arr, F.cam, cx, cz, r, rmax, self.col,
                                             kind='shockwave' if kind == 'shockwave' else 'rattle')
        return arr, depth

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        """#39's: a stick of the lacquered body, a nickel key chip, a bell chip"""
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        if kind == 'wood':
            ln_, wd_ = 0.26, 0.035
            q4 = [np.array([sx_, sz_]) + d * ln_ * a1 + nn * wd_ * a2
                  for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            P.layer_blend(arr, [sp], WOOD, 1.0)
        else:                                                      # key (silver chip), bell (a curved chip)
            col = NICKEL if kind == 'key' else WOOD * 1.3
            sz = size * (0.7 if kind == 'key' else 1.5)
            tri = [np.array([sx_, sz_]) + d * sz, np.array([sx_, sz_]) - d * sz * 0.7 + nn * sz * 0.8,
                   np.array([sx_, sz_]) - d * sz * 0.4 - nn * sz * 0.7]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
            shade_k = 0.75 + 0.25 * math.cos(rot * 2)
            P.layer_blend(arr, [sp], np.clip(col * shade_k, 0, 255), 1.0)
        return arr

    # ---------------------------------------------------------------- lens
    def shimmer(self, img, cam, t, w):
        """heat shimmer on GROWL's ring"""
        owner, cx, cz, r, rmax, kind, t0 = w
        big = kind == 'quake'
        return fx.shimmer_world(img, cam, S.to_world(cx, cz, 0.05), r, 0.45 if big else 0.3, 3.0 if big else 2.2, t,
                                k=1 - r / rmax, seed=owner)

    # ---------------------------------------------------------------- checks
    def focus_points(self, t, st):
        p = st['p']
        return [(S.to_world(p[0], p[1], 0.0), 'bari sax'), (S.to_world(p[0], p[1], 2 * R + st['lift']), 'bari sax')]


# ===================================================================================================== Gate O
def origin_harness(stage, mod):
    """the kit matched to #39 inside Gate O's window: the bari's bus under #39's name `bari` (the kit's is the
    fighter's name lowercased, `bari sax`: ruling M4-R9)"""
    if stage == 'music':
        for s in mod.SOUNDS:
            if isinstance(s, Sound):
                s.bus = 'bari'


# ===================================================================================================== sound
def thunder(dur=1.6, seed=0, crack=True):
    """#39's thunder (no rumble wobble)"""
    n = int(dur * A.SR)
    t = A.tt(n)
    r = np.random.default_rng(seed)
    rumble = A.lp(r.normal(0, 1, n), 140) * 3.0 * np.clip(t / 0.03, 0, 1) * np.exp(-t / (dur * 0.35))
    x = rumble
    if crack:
        x = x + A.bp(r.normal(0, 1, n), 1500, 9000) * np.exp(-t / 0.03) * 0.9
    return x


def clank(m, seed=0):
    """#39's clank: a key, a ferrule or the hook hitting something, a short metallic ring"""
    n = int(0.3 * A.SR)
    t = A.tt(n)
    f = A.mtof(m)
    x = sum(np.sin(2 * np.pi * f * r * t) * np.exp(-t / d) for r, d in ((1.0, 0.09), (2.76, 0.045), (5.4, 0.02)))
    return x + 0.3 * A.bp(np.random.default_rng(seed).normal(0, 1, n), 2000, 8000) * np.exp(-t / 0.005)


def grit(dur=0.5, seed=0):
    """GROWL's gravel: a rough, low, crackling noise burst"""
    n = int(dur * A.SR)
    t = A.tt(n)
    r = np.random.default_rng(seed)
    x = A.bp(r.normal(0, 1, n), 120, 900) * (0.6 + 0.4 * np.sign(np.sin(2 * np.pi * 31 * t)))
    return x * np.clip(t / 0.01, 0, 1) * np.exp(-t / (dur * 0.4))


def doppler(x, t_bot, up=1.6, down=2.6, width=0.10):
    """a pass: sharp (+up semitones) coming in, through the pitch at t_bot (s into x), sliding flat (-down) going away,
    and swelling to the pass (a time warp + an envelope)"""
    n = len(x)
    t = A.tt(n)
    z = (t - t_bot) / width
    off = np.where(z < 0, up, down) * -np.tanh(z)
    rate = 2 ** (off / 12)
    pos = np.cumsum(rate) - rate[0]
    y = np.interp(pos, np.arange(n), x, right=0.0)
    return y * (0.5 + 0.5 * np.exp(-((t - t_bot) / (2.2 * width)) ** 2))


def winch(dur, seed=0, rise=1.0):
    """the crane's winch: a geared whirr (a buzzy tone sweeping up `rise` octaves) + chain rattle"""
    n = int(dur * A.SR)
    t = A.tt(n)
    f = 110 * 2 ** (rise * t / dur)
    ph = 2 * np.pi * np.cumsum(f) / A.SR
    buzz = np.sign(np.sin(ph)) * 0.3 + np.sin(2 * ph) * 0.2
    r = np.random.default_rng(seed)
    rattle = A.bp(r.normal(0, 1, n), 2500, 8000) * (0.5 + 0.5 * np.sign(np.sin(2 * np.pi * 34 * t)))
    env = np.clip(t / 0.03, 0, 1) * np.clip((dur - t) / 0.03, 0, 1)
    return A.bp(buzz, 150, 3000) * env * 0.7 + rattle * env * 0.25


class Sound(base.Sound):
    bus_gain, send = 1.3, 0.2                                    # #39's mix ('bari')
    band = (60, 500)                                             # #39's band check: its honks
    colour_args = dict(cut0=40, cut_span=200, drive=4.0)         # #39's colour()

    def __init__(self, M, k):
        super().__init__(M, k)
        self.BXM = M.B.module('barisax_lowpoly')                 # the Pixel Pitch bari's honk (read-only)
        self._vc = {}
        self._bc = {}

    # ---------------------------------------------------------------- notes in the key (#39: E minor)
    def k0(self, off):
        """semitones above the key's root in the C2 octave (#39's literal E2 = 40)"""
        return self.T.KEY_ROOT + off

    def bass_root(self, ch):
        """#39 CHORD_ROOT: the chord's root in the octave from a fourth under the key's root (E minor: B1 .. D#3)"""
        return self.T.root_above(ch, self.T.KEY_ROOT - 5)

    def honk_note(self, ch):
        """#39 HONK_NOTE: the bass root, or its fifth when the root is below the bari's low C (its register's floor)"""
        r = self.bass_root(ch)
        return r if r >= self.M.CFG[self.k]['register'][0] else r + 7

    def bari(self, m, dur, vel=0.9, honk=0.0):
        """one bari note; honk 0..1 = the hard-blown, growled honk (#39's bari: fitted, 20 ms fade, level-matched)"""
        key = (round(m, 3), round(dur, 3), round(vel, 2), round(honk, 2))
        if key not in self._vc:
            M, BXM = self.M, self.BXM
            x = M.fit(M.B._resample(np.asarray(BXM.bari_tone(BXM._midi_hz(m), dur, vel, honk=honk), float)),
                      dur + 0.12, fade=0.02)
            self._vc[key] = x * (0.16 / (np.sqrt((x[:int(0.12 * M.SR)] ** 2).mean()) + 1e-9))
        return self._vc[key]

    def voice(self, m, dur, vel=0.85):
        return self.bari(m, dur, vel)

    def rope_twang(self, m, seed=0, dur=0.45):
        """a ring rope springing back from under the bari: a low plucked tone, quickly damped, a fibrous buzz (#39's
        cache: keyed by the seed modulo 3)"""
        key = (round(m, 2), seed % 3, round(dur, 2))
        if key not in self._bc:
            n = int(dur * A.SR)
            t = A.tt(n)
            f = A.mtof(m)
            x = sum((1.0 / k ** 1.2) * np.sin(2 * np.pi * f * k * t * (1 + .01 * np.exp(-t / .05)) + .3 * k) *
                    np.exp(-t * (6 + 4 * k)) for k in range(1, 7))
            buzz = A.bp(np.random.default_rng(seed + 31).normal(0, 1, n), 300, 2500) * np.exp(-t / .02) * .4
            self._bc[key] = np.tanh(1.5 * (x + buzz)) * np.clip(t / .002, 0, 1)
        return self._bc[key]

    # ---------------------------------------------------------------- the score, in the order kit.music calls it
    def title_blast(self, mix, t):
        """a huge honk on the root and its octave"""
        hits = self.T.TITLE_HITS.get(self.name, (self.k0(0), self.k0(12)))
        mix.place(self.bus, self.bari(hits[0], 0.5, 1.0, honk=1.0), t, 0.55)
        mix.place(self.bus, self.bari(hits[-1], 0.4, 0.8, honk=0.6), t + 0.02, 0.20)

    def play(self, mix):
        M, T, k = self.M, self.T, self.k
        q16, S16 = M.q16, T.S16
        ab = M.CFG[k]['abilities']
        for e in M.EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'cable' and e[2] == k:                    # the cable drops (the winch), the hook clanks home
                tq = q16(t)
                mix.place('fx', winch(e[3] - t, int(t * 10), rise=-0.8), tq, 0.10, pan=0.15)
                mix.place('fx', clank(79, int(t * 13)), q16(e[3]), 0.12, pan=0.15)
            elif kind == 'swing' and e[2] == k:                  # the swing honk: the bass on the swings
                ts0, ts1, sk = e[11], e[12], e[13]
                tb = q16((ts0 + ts1) / 2)                        # the bottom: on the beat (SKANK's: the off-beat)
                ch = T.chord_at(tb + 0.01)
                m = self.honk_note(ch) + (12 if sk else 0)       # never thinned (every swing on screen honks)
                t_on = q16(ts0 - S16)                            # it honks as it's let go, swelling to the bottom
                dur = ts1 - t_on + 0.06
                sig, g, lead = self.colour(self.bari(m, dur, 0.95, honk=0.85 if not sk else 0.55), t, 0.6)
                sig = doppler(sig, tb - t_on, up=1.6 if not sk else 1.0, down=2.6 if not sk else 1.6, width=0.09)
                mix.place(self.bus, sig, t_on, (1.35 if not sk else 1.15) * g, pan=0.2)
                if lead > 0.05 and not sk:                       # the leader: the octave honked on top (fuller)
                    up8 = doppler(self.colour(self.bari(m + 12, dur, 0.7, honk=0.4), t)[0], tb - t_on, 1.6, 2.6, 0.09)
                    mix.place(self.bus, up8, t_on, 0.20 * g, pan=0.25)
                mix.place('fx', A.whoosh(ts1 - ts0, 250, 3500), ts0, 0.10 if not sk else 0.07, pan=0.1)
            elif kind == 'smash' and e[2] == k:                  # the wrecking ball connects
                tq = q16(t)
                sp = e[5]
                mix.place('fx', A.thud(0.45, 120, 38, seed=int(t * 10)), tq, 0.22 + 0.22 * sp)
                mix.place('fx', clank(67, int(t * 11)), tq, 0.10 + 0.08 * sp, pan=0.1)
                mix.place('fx', grit(0.25, int(t * 7)), tq, 0.12 + 0.12 * sp)
                if sp > 0.6:
                    mix.place('drums', A.crash(1.0), tq, 0.06)
            elif kind == 'snap_aim' and e[2] == k:               # CABLE SNAP: the winch strains it up high
                mix.place('fx', winch(e[5] - t, 41, rise=1.3), q16(t), 0.16, pan=0.1)
            elif kind == 'cable_snap' and e[2] == k:             # the cable parts: a twang, the whip, a falling whistle
                tq = q16(t)
                mix.place('fx', clank(98, 5) + clank(86, 6) * 0.6, tq, 0.22)
                mix.place('fx', A.whoosh(0.25, 3000, 9000), tq, 0.12)
                fall = [q for q in M.EV if q[1] == 'crater' and q[2] == k and q[0] >= t]
                tf_ = (fall[0][0] - t) if fall else 0.4
                nf = int(tf_ * A.SR)
                tt_ = A.tt(nf)
                whistle = np.sin(2 * np.pi * np.cumsum(1800 * 2 ** (-1.6 * tt_ / tf_)) / A.SR) * \
                    (tt_ / tf_) ** 1.5 * 0.5
                mix.place('fx', whistle, tq, 0.10)
            elif kind == 'crater' and e[2] == k:                 # the crash
                tq = q16(t)
                mix.place('fx', thunder(2.0, 77), tq, 0.55)
                mix.place('fx', A.thud(0.7, 100, 32, seed=17), tq, 0.40)
                M.bed.boom(mix, tq, 0.9)                         # (#39: its bed's unison band hit)
                ch = T.chord_at(tq + 0.01)
                sig, g, _ = self.colour(self.bari(self.honk_note(ch), 0.7, 1.0, honk=1.0), t, 0.5)
                mix.place(self.bus, M.bend(sig, -1.5, 0.3, 0.7), tq, 0.70 * g, pan=0.0)
                mix.place(self.bus, self.colour(self.bari(self.honk_note(ch) + 12, 0.5, 0.7, honk=0.5), t)[0], tq,
                          0.18 * g)
            elif kind == 'ability' and e[2] == k:
                tq = q16(t)
                if e[3] == ab['unlock1']['name'] and ab['unlock1']['kind'] == 'shockwave':   # GROWL: growl + grit
                    sig, g, _ = self.colour(self.bari(self.honk_note(T.chord_at(tq)), 0.6, 1.0, honk=1.0), t, 0.8)
                    mix.place(self.bus, M.bend(sig, -1.0, 0.1, 0.6), tq, 0.50 * g, pan=0.2)
                    mix.place('fx', grit(0.6, int(t * 10)), tq, 0.30)
                elif e[3] == ab['unlock2']['name'] and ab['unlock2']['kind'] == 'offbeat':   # SKANK: a crash
                    mix.place('drums', A.crash(1.2), tq, 0.08)
        for te, side, x, z, spd in M.P.ROPE_EV:                 # a rope it pressed springs back: a low twang
            if spd == 3.2 and te <= M.T_KO:
                mix.place('fx', self.rope_twang(self.bass_root(T.chord_at(te)) + 12, seed=int(te * 10)), q16(te),
                          0.20, pan=0.2)
        for e in M.EV:                                           # dropped on the ropes, slung back in
            if e[0] > M.T_KO:
                break
            if e[1] == 'wreck_out' and e[2] == k:
                mix.place('fx', self.rope_twang(self.bass_root(T.chord_at(e[0])), seed=7, dur=0.6), q16(e[0] + 0.06),
                          0.30, pan=0.25)
                mix.place('fx', A.whoosh(e[8] - e[7], 400, 3000, rise=False), e[0], 0.08)
            elif e[1] == 'wreck_in' and e[2] == k:
                mix.place('fx', A.thud(0.35, 140, 50, seed=int(e[0] * 10)), q16(e[0]), 0.22)

    def fall_apart(self, mix):                                   # a falling honk, keys clattering
        M = self.M
        x = self.bari(self.k0(0), 1.0, 1.0, honk=0.8)
        mix.place(self.bus, M.bend(x, -7.0, 0.05, 1.0) * np.linspace(1, 0.4, len(x)), M.P.T_HS, 0.42)
        for j, m in enumerate((86, 81, 91, 77, 84)):
            mix.place('fx', clank(m, 20 + j), M.P.T_HS + 0.08 * j, 0.07, pan=(j - 2) / 5)

    def stinger(self, mix, tw):
        mix.place('solo', self.bari(self.k0(0), 0.8, 1.0, honk=1.0), tw, 0.50)
        mix.place('solo', self.bari(self.k0(12), 0.8, 0.8, honk=0.6), tw + 0.02, 0.22)

    def solo(self, mix, t0):
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            mix.place('solo', self.bari(m, d * T.BEAT * 0.9, 0.95, honk=0.7), ts, 0.55)

    def reassemble(self, mix):
        T, M = self.T, self.M
        run = [self.k0(o) for o in (0, 3, 7, 10, 12, 15)]       # #39's 64 67 71 74 76 79, two octaves down
        for j, m in enumerate(run):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
            mix.place(self.bus, self.bari(m, 0.10, 0.7), tj, 0.24)
