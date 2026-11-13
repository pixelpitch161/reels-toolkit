"""VIOLA (teal), as built in #42 (Viola vs Violin, projects/ballfight_viola_vs_violin): COUNTING RESTS (weapon kind
'rests': it rests for 3 beats, taking x guard damage, then on beat 4 a giant whole note appears and rolls at the enemy
like a heavy wheel for a whole bar, crushing it once and flattening its projectiles; every landed note grows the next),
DOUBLE STOP (two notes at once), MELODY AT LAST (for 2 bars the rests shrink to one beat), VIOLA JOKE (a rimshot, the
crowd laughs, the enemy freezes for a beat while the biggest whole note drops on it).

Look (#42 render.py's VIOLA layers, moved into the kit's hooks): the whole notes (styles.draw_note: the hollow oval
standing up as a wheel, ink with a teal-lit rim, rolling and bobbing; their shadows from the spot; a soft teal glow; a
teal flash + motes as one appears; dust, ink chips, a crack where one crushes, VIOLA JOKE's also a scorch); the resting
viola's pulsing teal pool on the canvas; the giant quarter rest over the resting ball and the count "1 2 3" above it, the
eighth notes rising off it during MELODY AT LAST (and over the champion's solo), after the post effects (opt-in
after_post); MELODY AT LAST's teal halo (the ball's glow); VIOLA JOKE's target ring under the falling note, its landing
ring and shimmer, the frozen enemy greyed (opt-ins ball_tint, ball_lit; the glass through the `ball` hook), the crowd's
laugh (opt-in crowd_laugh), its flashbulbs on the big note only (opt-in flash_hit); a flattened dart's spark (opt-in
spark_events); the instrument drawn with #42's wider crop (render_padded) in the ball and on the VS card; shards of
string, wood in its varnish, the bridge, the scroll.
Sound (#42 audio.py's VIOLA part): the Pixel Pitch viola a fifth (or so) under the enemy's line (#42: under the
violin's): a soft muted pluck on each counted rest, one long bowed note per whole note (DOUBLE STOP a second string a
sixth above, the leader doubled an octave down), a crush's thud, cello crunch and rosin scratch (VIOLA JOKE's also a
crash and a sub drop), DOUBLE STOP's scratch, VIOLA JOKE's rimshot, the crowd's laugh ('laugh' bus) and the falling
whoosh, MELODY AT LAST's lead (VIOLA_LEAD); the title chord on the C string, the count-in plucks, the groaning slide down
when it falls apart, the stinger and the solo when it wins, the pizzicato run home, the climb into the title slam.

Draw functions are #42's styles.py, imported from the episode's scratch copy (studio/reference/.cache/m4/origin/), never
retyped; render.py's layer code and audio.py's part are moved here with their names.

Two series-frame differences from #42, accepted by the controller (Gate O review): #42's VIOLA JOKE shakes the ropes at
t = the victim's index in seconds (`WAVE_EV += [(e[3], 0.9) ...]`: about 1.15 to 2.2 s, before FIGHT!; the kit's rope
hook times a wobble at its event, so this module leaves it out); the 'laugh' bus's reverb send (0.3 in #42's master) is
lost, since the kit's master reads levels for a Sound's own bus only."""
PORTED = True

import importlib.util
import math
import os

import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import SAFE, ss, ease
from reelkit.lighting import shade
from reelkit.shadows import proj_point
from reelkit.ui import label
from reelkit.font import ADV
from reelkit import fx
import scene as S

from . import base

EPISODE = 'ballfight_viola_vs_violin'
R = S.R_BALL


# ================================================================ #42 styles.py, through the episode's scratch copy
def _episode_styles():
    """#42's styles.py, loaded from its scratch copy (it loads #28's styles.py from beside it, so that episode's scratch
    copy must exist too); each copy is made only when it is missing or differs from projects/ (never written there)"""
    from tools.studio_export.older import origin as O
    for ep in (EPISODE, 'ballfight_drums_vs_violin'):
        src = os.path.join(O.episode_dir(ep), 'styles.py')
        dst = os.path.join(O.ROOT, ep, 'styles.py')
        with open(src, 'rb') as fh:
            want = fh.read()
        if not (os.path.exists(dst) and open(dst, 'rb').read() == want):
            O.scratch(ep)
    spec = importlib.util.spec_from_file_location('ballfight_viola_vs_violin_styles',
                                                  os.path.join(O.ROOT, EPISODE, 'styles.py'))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


ST = _episode_styles()
WOOD_COL = np.array([120, 50, 28])          # render.py 464: the viola's varnish (its wood shards, its K.O. chips)
INKY = [(40, 40, 60), (70, 76, 96), (24, 22, 34)]       # render.py 954: ink flecks off a crushing note
LAUGH = 1.9                                 # render.py 159: s the crowd laughs after the rimshot
NOTE_MEAN = (ST.NOTE_A + ST.NOTE_B) / 2
REST_G = ST.glyph(ST.REST_GLYPH, 2)
EIGHTH_G = ST.glyph(ST.EIGHTH_GLYPH, 1)


# ================================================================ #42 render.py, moved
def render_padded(name, radius_px, yaw_deg, roll_deg=0.0, pad=1.2):
    """sprites.render_sprite with a wider crop: the VIOLA's bridge size (x1.12, so it reads bigger than the violin)
    reaches past the standard crop at close-up sizes; same camera, rasteriser and outline"""
    import sprites as SP
    import piano_lowpoly as P
    RW, RH = P.RW, P.RH
    m = int(math.ceil(radius_px * pad)) + 3
    img = np.zeros((RH, RW, 3), np.uint8)
    zb = np.zeros((RH, RW))
    gb = np.zeros((RH, RW), np.int8)
    cam = SP._Cam(yaw_deg, radius_px, m + 0.5, m + 0.5, roll_deg)
    P.render_geo(SP._geo(name), cam, img, zb, gb, scale=1.0)
    P.outline(img, zb, gb)
    n = 2 * m + 1
    return np.dstack([img[:n, :n], (gb[:n, :n] > 0).astype(np.uint8) * 255])


def _safe_x(x, w, y, h):
    """keep a small label inside the safe box and out of the button column"""
    x1 = 300 if y + h > 290 else SAFE[2]
    return int(min(max(x, SAFE[0] + 1), x1 - w - 1))


class Look(base.Look):
    shards = ('string',) * 4 + ('wood',) * 7 + ('bridge',) * 2 + ('scroll',)   # render.py 431 (after 24 glass)
    spark_flash = ('blade', 'note')          # render.py 934: a 3 px white impact point
    flash_hits = ()                          # (flash_hit decides: render.py 779)
    spark_events = ('crushed',)              # render.py 907: a flattened dart sparks

    def setup(self):
        P, k = self.P, self.k
        EV = P.EV
        self.STUNS = [(e[0], e[2], e[4]) for e in EV if e[1] == 'stun' and e[3] == k]          # t, victim, dur
        self.JOKES = [(e[0], e[2], e[3], e[4], e[5], e[6]) for e in EV if e[1] == 'joke' and e[2] == k]
        self.MELODIES = [(e[0], e[2], e[3]) for e in EV if e[1] == 'melody' and e[2] == k]      # t, k, until
        self.NOTE_EV = [(e[0], e[2]) for e in EV if e[1] == 'note' and e[8] == 0 and e[2] == k]
        self._notes = None
        self.CAP_BAND = SAFE[1] + 34                                                            # 2 caption lines

    # ---------------------------------------------------------------- the run (render.py 155-258)
    def notes_built(self):
        """render.py _notes_build: per note id of its own: frames, positions, rolling direction and turn (from the
        recorded path), radius, times (built on first use: it reads the camera)"""
        if self._notes is not None:
            return self._notes
        P, k = self.P, self.k
        T, RUN, EV, FPS = P.T, P.RUN, P.EV, P.FPS
        D = {}
        for i, rows in enumerate(RUN['NOTES']):
            for owner, x, z, r, t0, t1, tl, nid, kind, hit in rows:
                if int(owner) != k:
                    continue
                d = D.setdefault(int(nid), dict(owner=int(owner), r=float(r), t0=float(t0), t1=float(t1), tl=float(tl),
                                                kind=int(kind), f=[], p=[], hit=None))
                d['f'].append(i)
                d['p'].append((x, z))
                if hit and d['hit'] is None:
                    d['hit'] = T.T_FIGHT + i / FPS
        aims = {e[3]: e[6] for e in EV if e[1] == 'note'}                          # id -> launch aim
        right = P.camera(0.0).M[0]
        for nid, d in D.items():
            Pp = np.array(d['p'], float)
            a0 = aims.get(nid, 0.0)
            U = np.zeros_like(Pp)
            u = np.array([math.cos(a0), math.sin(a0)])
            phi = np.zeros(len(Pp))
            for j in range(len(Pp)):
                if j > 0:
                    step = Pp[j] - Pp[j - 1]
                    ds = float(np.hypot(*step))
                    if ds > 1e-4:
                        u = step / ds
                    sw = S.to_world(step[0], step[1], 0.0) - S.to_world(0.0, 0.0, 0.0)      # rolling right on screen
                    sgn = 1.0 if sw @ right >= 0 else -1.0                                   # turns it clockwise
                    phi[j] = phi[j - 1] + sgn * ds / (d['r'] * NOTE_MEAN)
                U[j] = u
            d['P'], d['U'], d['PHI'] = Pp, U, phi
            d['f'] = np.array(d['f'])
        self._notes = D
        return D

    def notes_at(self, t):
        """render.py notes_at: [(id, c, u, phi, r, lift, scale, glow, owner, kind)] of its whole notes on screen at t"""
        P = self.P
        T = P.T
        out = []
        if t < T.T_FIGHT or t >= P.T_HS + 0.6:
            return out
        tt = min(t, P.T_KO)
        fi = P.sim_i(tt)
        fade = 1.0 - ss(P.T_HS, P.T_HS + 0.6, t)
        for nid, d in self.notes_built().items():
            if not (d['f'][0] <= fi <= d['f'][-1]):
                continue
            j = int(fi - d['f'][0])
            c, u, phi = d['P'][j], d['U'][j], d['PHI'][j]
            lift, glow = 0.0, 0.0
            if d['tl'] > d['t0'] + 1e-6 and tt < d['tl']:                          # dropped from above (VIOLA JOKE)
                s_ = (tt - d['t0']) / (d['tl'] - d['t0'])
                lift = 4.6 * (1 - s_ * s_)
                phi = 0.4 * math.sin(6 * s_)
            sc = ease(min(1.0, (tt - d['t0']) / 0.10)) if d['tl'] <= d['t0'] + 1e-6 else 1.0
            sc *= 1.0 - ss(d['t1'] - 0.14, d['t1'], tt)                             # a whole note's length: then it's gone
            if d['hit'] is not None and 0 <= tt - d['hit'] < 0.25:
                glow = 1 - (tt - d['hit']) / 0.25
            if tt - d['t0'] < 0.15:
                glow = max(glow, 1 - (tt - d['t0']) / 0.15)
            sc *= fade
            if sc > 0.03:
                out.append((nid, c, u, phi, d['r'], lift, sc, glow, d['owner'], d['kind']))
        return out

    def notes(self, F):
        """notes_at(F.t), once a frame"""
        key = ('notes', self.k)
        if key not in F.__dict__:
            F.__dict__[key] = self.notes_at(F.t)
        return F.__dict__[key]

    def rest_state(self, t):
        """(resting 0/1, count, melody) from the record; nothing before the fight or after the K.O."""
        P = self.P
        if not (P.T.T_FIGHT <= t < P.T_KO):
            return 0, 0, 0
        r = P.RUN['REST'][P.sim_i(t)][self.k]
        return int(r[0]), int(r[1]), int(r[2])

    def stun_k(self, kk, t):
        tt = min(t, self.P.T_KO)
        for t0, v, d in self.STUNS:
            if v == kk and t0 <= tt < t0 + d + 0.15:
                return min(1.0, (tt - t0) / 0.06) * (1 - ss(t0 + d, t0 + d + 0.15, tt))
        return 0.0

    def frz(self, kk, t):
        """VIOLA JOKE's freeze on ball kk at t (render.py fighter(): the stun state, inside the fight only)"""
        P = self.P
        if kk != self.enemy or not (P.T.T_FIGHT <= t < P.T_HS):
            return 0.0
        return self.stun_k(kk, t)

    def laugh_k(self, t):
        """the crowd's laugh after VIOLA JOKE's rimshot (0..1)"""
        best = 0.0
        for t0, k, v, tl, x, z in self.JOKES:
            if t0 <= t < t0 + LAUGH + 0.4:
                best = max(best, min(1.0, (t - t0) / 0.12) * (1 - ss(t0 + LAUGH, t0 + LAUGH + 0.4, t)))
        return best

    def melody_k(self, t):
        tt = min(t, self.P.T_KO)
        for t0, kk, until in self.MELODIES:
            if t0 <= tt < until:
                return min(1.0, (tt - t0) / 0.2) * (1 - ss(until - 0.25, until, tt))
        return 0.0

    # ---------------------------------------------------------------- the ball
    def title_closeup(self, r):
        return render_padded(self.cfg['pp'], r, float(self.cfg['view'][0]), float(self.cfg['view'][2]))

    def sprite(self, r, yaw, roll):
        return render_padded(self.cfg['pp'], r, float(yaw), float(roll))

    def state(self, kk, st, t, phase, gl=0.0):
        if phase == 'fight' and kk == self.k:                  # MELODY AT LAST: a teal halo, pulsing on the beat
            st['glow'] = 0.45 * self.melody_k(t) * (0.6 + 0.4 * self.T.beat_pulse(t, 0.2))

    def ball(self, kk, st, t, glass, roll):
        frz = self.frz(kk, t)                                   # VIOLA JOKE: the frozen enemy's glass goes grey
        if frz > 0:
            glass = glass * (1 - 0.3 * frz) + np.array([168, 176, 196]) * 0.3 * frz
        return glass, roll

    def ball_tint(self, kk, st, t, col):
        frz = self.frz(kk, t)
        if frz > 0:
            col = col * (1 - frz) + np.array([150, 150, 168]) * frz
        return col

    def ball_lit(self, kk, st, t, lit):
        frz = self.frz(kk, t)                                   # ... and its instrument grey and still
        if frz > 0:
            g_ = lit.mean(-1, keepdims=True)
            lit = lit * (1 - 0.7 * frz) + (g_ * 0.85 + np.array([20, 24, 40])) * 0.7 * frz
        return lit

    # ---------------------------------------------------------------- events -> camera, crowd, sparks
    def shake(self, e):
        if e[1] == 'hit' and e[7] in ('blade', 'wave', 'note'):
            big = e[7] == 'note' and e[8] == 2                                    # VIOLA JOKE's note lands
            return 2.5 if big else {'blade': 0.4, 'wave': 2.0, 'note': 1.3}[e[7]]
        return None

    def flash_hit(self, e):
        return e[7] in ('wave', 'note') and e[8] == 2

    def crowd_laugh(self, t):
        return self.laugh_k(t)

    def spark(self, e):
        P = self.P
        how = e[7]
        n, sp, life = (16, 3.2, 0.3) if how in ('wave', 'note') else \
            ((8, 2.4, 0.22) if how == 'blade' else (5, 1.8, 0.18))
        return P.COL[e[2]] * 0.6 + 100, n, sp, life

    def hit_glow(self, e):
        if e[7] in ('wave', 'blade', 'note'):
            big = e[7] != 'blade'
            return (14, 0.35) if big else (8, 0.22)
        return None

    # ---------------------------------------------------------------- effects (render.py _fx_build)
    def fx_event(self, e):
        P = self.P
        t_, kind = e[0], e[1]
        up = P.up
        if kind == 'hit' and e[7] == 'note':                       # a whole note crushes (VIOLA JOKE's: the big one)
            att, x, z = e[2], e[5], e[6]
            pos = up(x, z, R)
            big = e[8] == 2
            P.BU.add(t_, pos, P.COL[att], 2.0 if big else 1.1)
            P.FL.add(t_, up(x, z, 0.4), P.COL[att], radius=2.6 if big else 1.6, peak=1.0 if big else 0.7,
                     decay=0.45 if big else 0.28)
            P.PA.emit(t_, up(x, z, 0.1), 'dust', n=26 if big else 10, spread=180, speed=(0.6, 1.8),
                      scale=1.3 if big else 1.0)
            P.PA.emit(t_, pos, 'chips', n=8 if big else 4, col=INKY)
            P.MK.add(t_, 'crack', x, z, size=1.4 if big else 0.55, n=8 if big else 4, life=6.0 if big else 3.0,
                     seed=int(t_ * 100))
            if big:
                P.MK.add(t_, 'scorch', x, z, size=1.2, col=(40, 44, 64), life=6.0, seed=int(t_ * 10) + 1, strength=0.5)
        elif kind == 'note':                                       # a note appears: a teal flash at its spot, ink motes
            x, z = e[4], e[5]
            if e[9] <= t_ + 1e-6:                                  # rolling at once (a dropped one shows on landing)
                P.FL.add(t_, up(x, z, 0.5), P.COL[e[2]], radius=1.3, peak=0.45, decay=0.25)
                P.PA.emit(t_, up(x, z, 0.3), 'motes', n=5, col=[tuple(P.COL[e[2]] * 0.5 + 120)])
        elif kind == 'joke':                                       # the target glows teal while the note falls
            k, tl, x, z = e[2], e[4], e[5], e[6]
            P.FL.add(t_ + 0.05, up(x, z, 2.0), P.COL[k], radius=2.2, peak=0.6, decay=max(0.3, tl - t_))

    def ko_fx(self, pos):
        P = self.P
        P.PA.emit(P.T_KO, pos, 'chips', n=24, spread=180, speed=(0.8, 2.2), col=[tuple(WOOD_COL), tuple(WOOD_COL * 1.3)])

    # ---------------------------------------------------------------- the canvas
    def shadows_free(self, F, spot):
        return [proj_point(ST.note_shadow(F.cam, c, r * sc, phi, lift), spot)
                for nid, c, u, phi, r, lift, sc, glow, owner, kind in self.notes(F)]

    def canvas(self, Ic, C):
        """the resting viola's pool (it pulses on each count)"""
        P, k, T, t = self.P, self.k, self.T, C.t
        st = C.fs[k]
        resting, count, mel = self.rest_state(t)
        mk_ = self.melody_k(t)
        if C.in_fight and (resting or mk_ > 0):
            sxp, szp = C.sxp, C.szp
            rd = np.hypot(sxp - st['p'][0], szp - st['p'][1])
            ph = ((t - T.T_FIGHT) % T.BEAT) / T.BEAT
            ring_r = R * (1.15 + 0.7 * ph)
            band = np.clip(1 - np.abs(rd - ring_r) / 0.06, 0, 1) * (1 - ph) * (0.6 if resting else 0.4)
            pool = np.clip(1 - rd / (R * 1.5), 0, 1) * 0.10 * (1 + mk_)
            Ic += (band * 0.8 + pool)[:, None] * (P.COL[k] / 255.0 + 0.1)[None]
        return Ic

    def canvas_after(self, Ic, C):
        """VIOLA JOKE: the target under the falling note, then its landing ring"""
        P, t = self.P, C.t
        sxp, szp = C.sxp, C.szp
        for t0, k, v_, tl, x, z in self.JOKES:
            if t0 <= t < tl and t < P.T_HS:
                u = (t - t0) / (tl - t0)
                rd = np.hypot(sxp - x, szp - z)
                rad = self.cfg['abilities']['low']['size'] * float(P.RUN['BL'][P.sim_i(t0), k])
                m = rd < rad * (0.4 + 0.6 * u)
                Ic[m] *= 1 - 0.5 * u
                ring = np.abs(rd - rad) < 0.05
                Ic[ring] += (P.COL[k] / 255.0) * (0.4 + 0.5 * (math.sin(t * 40) > 0))
            elif 0 <= t - tl < 0.6 and t < P.T_HS:
                u = (t - tl) / 0.6
                rd = np.hypot(sxp - x, szp - z)
                band = np.clip(1 - np.abs(rd - (1.2 + 1.8 * u)) / 0.2, 0, 1) * (1 - u)
                Ic += band[:, None] * np.array([0.6, 0.95, 0.9])[None]
        return Ic

    # ---------------------------------------------------------------- the painter pass
    def items_free(self, F):
        cam = F.cam
        return [(cam.depth(S.to_world(nt[1][0], nt[1][1], 0.5 * nt[4] * nt[6] + nt[5]))[0], nt) for nt in self.notes(F)]

    def draw_item(self, F, payload):
        nid, c, u, phi, r, lift, sc, glow, owner, nkind = payload
        ST.draw_note(F.fr, F.cam, F.L, shade, c, r, phi, self.P.COL[owner], lift=lift, scale=sc,
                     glow=max(glow, 0.5 * self.melody_k(F.t)))

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        if kind == 'string':                                      # a string curling free (#28)
            pts = [np.array([sx_, sz_]) + d * (j - 3) * 0.16 + nn * curl * 0.10 * math.sin(j) for j in range(7)]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
            ImageDraw.Draw(im).line([tuple(q) for q in sp], fill=(230, 226, 214), width=1)
            arr = np.asarray(im).astype(np.float32)
        elif kind == 'bridge':                                    # the maple bridge, pale, with its two feet
            q4 = [np.array([sx_, sz_]) + d * 0.12 * a1 + nn * 0.05 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            P.layer_blend(arr, [sp], np.array([228, 198, 148]), 1.0)
        elif kind == 'scroll':                                    # the scroll: a small varnished spiral
            pts = [np.array([sx_, sz_]) + (0.13 - 0.016 * j) * np.array([math.cos(rot + j * 0.9), math.sin(rot + j * 0.9)])
                   for j in range(8)]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
            im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
            ImageDraw.Draw(im).line([tuple(q) for q in sp], fill=tuple(int(v) for v in WOOD_COL * 1.2), width=2)
            arr = np.asarray(im).astype(np.float32)
        elif kind == 'wood':                                      # wood: tumbling flat chips in its varnish
            tri = [np.array([sx_, sz_]) + d * size, np.array([sx_, sz_]) - d * size * 0.7 + nn * size * 0.8,
                   np.array([sx_, sz_]) - d * size * 0.4 - nn * size * 0.7]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in tri]))
            P.layer_blend(arr, [sp], np.clip(WOOD_COL * (0.75 + 0.25 * math.cos(rot * 2)) * 1.2, 0, 255), 1.0)
        return arr

    # ---------------------------------------------------------------- lens, glows, post
    def shimmer_after(self, img, cam, t):
        P = self.P
        for t0, k, v, tl, x, z in self.JOKES:                     # the big note's landing
            if 0 <= t - tl < 0.6 and t < P.T_HS:
                u = (t - tl) / 0.6
                img = fx.shimmer_world(img, cam, S.to_world(x, z, 0.05), 1.2 + 1.8 * u, 0.35, 3.0, t, k=1 - u)
        return img

    def glows(self, F, drawn):
        cam, P = F.cam, self.P
        for nt in self.notes(F):                                  # a soft teal glow round each rolling note
            nid, c, u, phi, r, lift, sc, glow, owner, kind = nt
            s_ = cam.wp(S.to_world(c[0], c[1], 0.5 * r * sc + lift)[None])[0]
            F.fr.glow(s_[0], s_[1], 10 + 14 * r * sc, tuple(P.COL[owner] * 0.7 + 60), 0.10 + 0.25 * glow)

    def fringe(self, e):
        return e[7] == 'wave' or (e[7] == 'note' and e[8] == 2)

    def caption_up(self, t):
        P = self.P
        live = [(a, b) for a, b, _ in P.CAP_ABIL[0] + P.CAP_ABIL[1] + P.CAP_KO + P.CAP_WINS + P.CAP_COMMENT]
        return any(a - 0.05 <= t < b + 0.1 for a, b in live)

    def after_post(self, img, F, drawn):
        """the rests + counts, the rising eighths (crisp, over the scene: render.py draw_rests)"""
        if not (self.P.TT.door_offset(F.t) == 0.0):
            self.draw_rests(img, F.t, drawn)
        return img

    def draw_rests(self, img, t, drawn):
        P, k, T = self.P, self.k, self.T
        if k not in drawn:
            return
        resting, count, mel = self.rest_state(t)
        (sx, sy), rp = drawn[k]
        col_k, dark_k = P.COL[k], P.DARK[k]
        if resting and count > 0:
            # the giant quarter rest over the ball (it settles in on each counted beat, sways a little)
            t_beat = T.T_FIGHT + math.floor((t - T.T_FIGHT) / T.BEAT + 1e-6) * T.BEAT
            u = min(1.0, (t - t_beat) / 0.08)
            gy = sy - rp - REST_G.shape[0] / 2 - 3 - 3 * (1 - u)
            col = np.clip(col_k * 0.55 + 110, 0, 255)
            ST.stamp_glyph(img, REST_G, sx + 1.5 * math.sin(2 * math.pi * t / T.BAR), gy, col, alpha=0.4 + 0.6 * u,
                           outline=tuple(int(v) for v in dark_k * 0.45))
            # the small count above it: "1", "1 2", "1 2 3" (the newest bright)
            digits = [str(j + 1) for j in range(count)]
            txt_w = len(digits) * (ADV + 2) - 2
            y_ = int(round(gy - REST_G.shape[0] / 2 - 11))
            x_ = _safe_x(int(round(sx - txt_w / 2)), txt_w, y_, 7)
            if y_ < self.CAP_BAND and self.caption_up(t):             # under a caption: beside the rest glyph instead
                y_ = int(round(gy - 3))
                x_ = int(round(sx + REST_G.shape[1] / 2 + 4))
                if x_ + txt_w > (300 if y_ + 7 > 290 else SAFE[2]) - 1:
                    x_ = int(round(sx - REST_G.shape[1] / 2 - 4 - txt_w))
            if SAFE[1] <= y_ <= SAFE[3] - 8 and not (y_ < self.CAP_BAND and self.caption_up(t)):
                for j, dg in enumerate(digits):
                    c_ = (255, 250, 236) if j == count - 1 else tuple(int(v) for v in np.clip(col_k * 0.5 + 90, 0, 255))
                    label(img, dg, x_ + j * (ADV + 2), y_, c_, 1)
        mk = self.melody_k(t)
        rising = [te for te, kk in self.NOTE_EV if 0 <= min(t, P.T_KO) - te < 0.9 and mk > 0]
        if t >= P.T_HS and k == P.WIN:                                 # the champion's solo: eighths on each note
            rising = [s_ for s_ in P.SOLO_T if 0 <= t - s_ < 0.9]
        for j, te in enumerate(rising):
            age = (min(t, P.T_KO) if t < P.T_HS else t) - te
            x_ = sx + (8 + 3 * j) * (1 if (int(te * 10) % 2) else -1) + 4 * math.sin(age * 7)
            y_ = sy - rp * 0.4 - 26 * age
            ST.stamp_glyph(img, EIGHTH_G, x_, y_, tuple(int(v) for v in np.clip(col_k * 0.5 + 125, 0, 255)),
                           alpha=1 - ss(0.5, 0.9, age), shadow=False)


# ================================================================ #42 audio.py: the VIOLA's part, moved
# MELODY AT LAST: the music hands the viola the melody for its 2 bars (timeline.py 166-167: beats, midi, beats)
VIOLA_LEAD = [(0, 67, 1.0), (1, 70, 0.5), (1.5, 68, 0.5), (2, 67, 1.5), (3.5, 65, 0.5), (4, 63, 1.0), (5, 65, 0.75),
              (5.75, 67, 0.25), (6, 62, 1.0), (7, 60, 1.0)]


def fit(T, m, chord, lo=48, hi=79):
    """timeline.py fit: nudge a melody note onto the chord's scale (C harmonic minor; over Ab / Fm the B becomes Bb)"""
    pcs = [(T.KEY_ROOT + s) % 12 for s in T.SCALE]
    if chord in ('Ab', 'Fm', 'Cm'):
        pcs = [p if p != 11 else 10 for p in pcs]
    while m % 12 not in pcs:
        m -= 1
    return max(lo, min(hi, m))


def scratch(dur, seed=5):
    """bow scratch: rosin noise, band-limited, gritty (#28)"""
    from reelkit import audio as A
    n = int(dur * A.SR)
    x = np.random.default_rng(seed).normal(0, 1, n)
    x = A.bp(x, 1800, 7000) * (1 + 0.8 * np.sign(np.sin(2 * np.pi * 37 * A.tt(n))))
    return x * A.expenv(n, dur * 0.4, 0.004)


def pizz(m, vel=0.8, muted=False):
    """pizzicato: a plucked string with a little wooden body (#28); `muted` = palm-muted, short and dull (the rests)"""
    from reelkit import audio as A
    SR = A.SR
    n = int((0.25 if muted else 0.5) * SR)
    f = A.mtof(m)
    t = A.tt(n)
    dec = 0.05 if muted else 0.16
    x = (np.sin(2 * np.pi * f * t) + 0.5 * np.sin(4 * np.pi * f * t) * np.exp(-t / 0.05)
         + 0.25 * np.sin(6 * np.pi * f * t) * np.exp(-t / 0.03)) * np.exp(-t / dec)
    x += 0.15 * A.bp(np.random.default_rng(m).normal(0, 1, n), 800, 3000) * np.exp(-t / 0.01)
    return A.lp(x, 1600 if muted else 4500) * vel


def addp(*xs):
    """sum signals of different lengths (zero-padded)"""
    n = max(len(x) for x in xs)
    return sum(np.pad(np.asarray(x, float), (0, n - len(x))) for x in xs)


def tom(f0=200.0, dec=0.18, vel=1.0):
    from reelkit import audio as A
    SR = A.SR
    n = int(0.45 * SR)
    t = A.tt(n)
    f = f0 * (1 + 0.6 * np.exp(-t / 0.03))
    x = np.sin(2 * np.pi * np.cumsum(f) / SR) * np.exp(-t / dec)
    x += 0.3 * A.bp(np.random.default_rng(int(f0)).normal(0, 1, n), 300, 3000) * np.exp(-t / 0.02)
    return x * vel


def rimshot(S16):
    """ba-dum-tss: two toms (high, low + the kick) and a ride / crash 'tss'"""
    from reelkit import audio as A
    SR = A.SR
    n = int(1.6 * SR)
    out = np.zeros(n)

    def put(x, t, g):
        a = int(t * SR)
        b = min(n, a + len(x))
        out[a:b] += x[:b - a] * g
    put(addp(A.snare(0.08) * 0.6, tom(230, 0.14)), 0.0, 0.9)
    put(addp(tom(150, 0.22), A.kick(0.8, 0.25, 120, 50)[:int(0.4 * SR)] * 0.7), 1.5 * S16, 1.0)
    nt = int(1.2 * SR)
    tt_ = A.tt(nt)
    tss = A.hp(np.random.default_rng(77).normal(0, 1, nt), 5500) * np.exp(-tt_ / 0.35)
    tss += 0.12 * sum(np.sin(2 * np.pi * f * tt_) for f in (5100, 6650, 8230)) * np.exp(-tt_ / 0.5)
    put(tss, 3 * S16, 0.55)
    return out


def ha(f0, seed, dur=0.13, vowel=((800, 1.0), (1200, 0.6), (2600, 0.25))):
    """one 'ha': a breathy voiced burst (a pulse train through the vowel's formants) with an aspirated onset"""
    from reelkit import audio as A
    n = int(dur * A.SR)
    t = A.tt(n)
    rng = np.random.default_rng(seed)
    f = f0 * (1 + 0.06 * np.exp(-t / 0.04)) * (1 - 0.08 * t / dur)
    ph = np.cumsum(f) / A.SR
    pulse = (2 * (ph % 1.0) - 1) ** 3
    noise = rng.normal(0, 1, n)
    src = pulse * np.clip((t - 0.015) / 0.01, 0, 1) + 0.35 * noise
    x = sum(g * A.bp(src, fc * 0.82, fc * 1.18) for fc, g in vowel)
    env = np.clip(t / 0.012, 0, 1) * np.exp(-np.maximum(t - 0.03, 0) / (dur * 0.45))
    return x * env


def crowd_laugh(dur=2.1, n_voices=34, seed=55):
    """the whole crowd bursts out laughing: many voices, each a run of 'ha's (faster and higher at first, then
    trailing off), random onsets, pitches and pans; returns stereo (n, 2)"""
    from reelkit import audio as A
    SR = A.SR
    n = int(dur * SR)
    out = np.zeros((n, 2))
    rng = np.random.default_rng(seed)
    for v in range(n_voices):
        f0 = rng.uniform(140, 330) if v % 3 else rng.uniform(260, 420)      # men, women, a few shrieks
        t = rng.uniform(0.0, 0.28)
        rate = rng.uniform(0.15, 0.21)
        pan = rng.uniform(-0.8, 0.8)
        amp = rng.uniform(0.5, 1.0)
        j = 0
        while t < dur - 0.2:
            x = ha(f0 * (1 - 0.03 * j), int(seed * 1000 + v * 37 + j), rng.uniform(0.10, 0.15))
            g = amp * math.exp(-t / (dur * 0.7)) * (1.0 if j < 6 else 0.7)
            a = int(t * SR)
            b = min(n, a + len(x))
            out[a:b, 0] += x[:b - a] * g * (1 - pan) * 0.5
            out[a:b, 1] += x[:b - a] * g * (1 + pan) * 0.5
            t += rate * (1 + 0.04 * j) + rng.uniform(-0.02, 0.02)
            j += 1
    return out / math.sqrt(n_voices) * 2.2


def sub808(m0, m1, dur, slide=0.0, vel=1.0):
    """the 808: a sine sub with a punchy pitch drop on the attack, saturated, gliding to m1 over the last `slide` s
    (#42's bed; VIOLA JOKE's crush drops it)"""
    from reelkit import audio as A
    SR = A.SR
    n = int((dur + 0.05) * SR)
    t = A.tt(n)
    f0, f1 = A.mtof(m0), A.mtof(m1)
    u = np.clip((t - (dur - slide)) / max(slide, 1e-3), 0, 1) if slide > 0 else np.zeros(n)
    f = f0 * (f1 / f0) ** (u * u * (3 - 2 * u)) * (1 + 0.5 * np.exp(-t / 0.02))
    x = np.sin(2 * np.pi * np.cumsum(f) / SR)
    x = np.tanh(2.2 * x) / np.tanh(2.2)
    env = np.clip(t / 0.004, 0, 1) * np.clip((dur + 0.05 - t) / 0.05, 0, 1) * (0.75 + 0.25 * np.exp(-t / 0.15))
    return A.lp(x, 900) * env * vel


class Sound(base.Sound):
    bus_gain, send = 1.7, 0.4                                      # #42's viola bus (BUS_GAIN, its reverb send)
    band = (120, 1500)                                             # #42's band check: the inner voice C3-G4 + harmonics
    colour_args = dict(cut0=70, cut_span=300, drive=4.5)           # the viola keeps its low strings as it thins

    def setup_run(self):
        M, k = self.M, self.k
        if getattr(self, '_ready', False):
            return
        self._ready = True
        self.kinds = {}                                            # note id -> kind (the run's records)
        for rows in M.RUN['NOTES']:
            for row in rows:
                if int(row[0]) == k:
                    self.kinds.setdefault(int(row[7]), int(row[8]))
        mel = []
        for e in M.EV:
            if e[1] == 'melody' and e[2] == k:
                s = math.ceil((e[0] + 1e-6) / self.T.BEAT) * self.T.BEAT       # from the beat after it fires
                mel.append((s, min(e[3], M.T_KO)))
        self.mel_win = mel

    def in_melody(self, t):
        return any(a <= t < b for a, b in self.mel_win)

    def voice(self, m, dur, vel=0.85):
        M = self.M
        return M.note(M.CFG[self.k]['pp'], m, round(dur, 3), round(dur + 0.3, 2), round(vel, 1))

    def viola(self, m, dur, vel=0.85, accent=0.0):
        """a bowed viola note (the new voice) with an optional heavy bow attack (rosin crunch)"""
        x = self.voice(m, dur, vel)
        if accent > 0:
            s = scratch(0.07, int(m * 7))[:len(x)]
            x = x.copy()
            x[:len(s)] += accent * 0.25 * s
        return x

    def cello(self, m, dur, vel=0.85):
        return self.M.note('CELLO', m, round(dur, 3), round(dur + 0.2, 2), round(vel, 1))

    def inner(self, t):
        """timeline.py viola_inner: a chord tone a fifth (or so) under the enemy's line (#42: the violin's), in the
        viola's warm range (C3-G4)"""
        T = self.T
        c = T.chord_at(t)
        tgt = T.line_at(self.M.NAMES[self.enemy], t) - 7
        cands = T.chord_tones(48, 67, c)
        return min(cands, key=lambda m: (abs(m - tgt), m))

    # ---------------------------------------------------------------- the score
    def title_blast(self, mix, t):
        for m, g in ((48, 0.42), (55, 0.36), (63, 0.30)):     # a dark chord on the C string (C3 G3 Eb4)
            mix.place(self.bus, self.viola(m, 0.8, 1.0, accent=0.5), t, g, pan=-0.2)

    def count_in(self, mix):
        for j, (tc, txt) in enumerate(self.T.COUNT[:3]):     # the viola counts it in
            mix.place(self.bus, pizz((55, 59, 62)[j], 0.9), tc, 0.22, pan=-0.2)

    def play(self, mix):
        M, T, k = self.M, self.T, self.k
        A, EV, q16, S16 = M.A, M.EV, M.q16, T.S16
        self.setup_run()
        thin_a = M.Thinner(9)
        for e in EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'rest' and e[2] == k:                              # the counting: a soft muted pluck on 1-2-3
                tq = q16(t)
                m = self.inner(tq) - (12 if e[3] == 1 else 0)
                mix.place(self.bus, pizz(max(48, m), 0.7, muted=True), tq, 0.22 + 0.03 * e[3], pan=-0.2)
            elif kind == 'note' and e[2] == k:
                nkind, t_land, t1 = e[8], e[9], e[10]
                ts = q16(t_land)
                if nkind != 2 and self.in_melody(ts):
                    continue                                              # the lead line plays these (below)
                slot = round(ts / S16)
                if nkind == 0 and not thin_a.keep(slot, M.hp_at(k, ts) / 100.0):
                    continue
                dur = max(0.3, min(T.BAR, t1 - t_land) * 0.96)
                m = self.inner(ts)
                if nkind == 2:                                            # VIOLA JOKE: the open C + G, fortissimo
                    for mm, g in ((48, 0.50), (55, 0.42), (60, 0.26)):
                        mix.place(self.bus, self.viola(mm, dur, 1.0, accent=0.8), ts, g, pan=-0.15)
                    continue
                sig, g, lead = self.colour(self.viola(m, dur, 0.9, accent=0.45), ts, 0.8)
                gain = 0.52 if nkind == 0 else 0.38
                mix.place(self.bus, sig, ts, gain * g, pan=-0.2)
                if nkind == 1:                                            # DOUBLE STOP: a second string a sixth above
                    m2 = min(T.chord_tones(m + 7, m + 10, T.chord_at(ts)) or [m + 7])
                    mix.place(self.bus, self.colour(self.viola(m2, dur, 0.85, accent=0.3), ts)[0], ts, 0.26 * g,
                              pan=-0.25)
                elif lead > 0.04:                                         # the leader: doubled an octave down
                    mix.place(self.bus, self.colour(self.viola(max(48, m - 12), dur, 0.7), ts)[0], ts, 0.16 * g,
                              pan=-0.2)
            elif kind == 'crush' and e[2] == k:                           # a whole note crushes: a low crunch + thud
                tq = q16(t)
                big = self.kinds.get(int(e[3])) == 2
                mix.place('fx', A.thud(0.5 if big else 0.32, 110, 36, seed=int(t * 13)), tq, 0.42 if big else 0.26)
                mix.place('cellos', self.cello(36, 0.12, 1.0), tq, 0.20 if big else 0.12)
                mix.place(self.bus, scratch(0.12, int(t * 17)), tq, 0.10 if big else 0.06, pan=-0.2)
                if big:
                    mix.place('drums', A.crash(2.0), tq, 0.12)
                    mix.place('bass808', sub808(24, 24, 0.9), tq, 0.70)
            elif kind == 'ability' and e[2] == k:
                tq = q16(t)
                if e[4] == 'double_stop':                                 # two strings dug in at once
                    mix.place(self.bus, scratch(0.1, int(t * 29)), tq, 0.10, pan=-0.2)
                elif e[4] == 'joke':                                      # ba-dum-tss + the crowd laughing
                    mix.place('fx', rimshot(S16), tq, 0.42)
                    lg = crowd_laugh(2.1)
                    mix.place('laugh', lg[:, 0], tq + 3 * S16 + 0.05, 0.52, pan=-0.6)
                    mix.place('laugh', lg[:, 1], tq + 3 * S16 + 0.05, 0.52, pan=0.6)
                    mix.place('fx', A.whoosh(T.BEAT, 200, 3000, rise=False), tq + 0.1, 0.10)    # the note falling
        # MELODY AT LAST: the viola sings the lead for its 2 bars
        for a, b in self.mel_win:
            prev = None
            for bt, m, dl in VIOLA_LEAD:
                ts = a + bt * T.BEAT
                if ts >= b - 0.02:
                    break
                mm = fit(T, m, T.chord_at(ts + 0.01), 52, 76)
                x = self.viola(mm, dl * T.BEAT * 0.97, 0.95, accent=0.3 if prev is None or abs(mm - prev) > 2 else 0.1)
                sig, g, _ = self.colour(x, ts, 0.5)
                mix.place(self.bus, sig, ts, 0.50 * g, pan=-0.1)
                mix.place(self.bus, self.colour(self.viola(max(48, mm - 12), dl * T.BEAT * 0.97, 0.7), ts)[0], ts, 0.14,
                          pan=-0.2)
                prev = mm
            mix.place('fx', A.whoosh(0.5, 300, 4000), a - 0.25, 0.06)

    def fall_apart(self, mix):
        T_HS = self.M.P.T_HS                                              # a groaning slide down
        for j, m in enumerate((67, 63, 60, 55, 51, 48)):
            mix.place(self.bus, self.viola(m, 0.12, 0.8), T_HS + 0.1 * j, 0.24, pan=(j - 2.5) / 6)

    def stinger(self, mix, tw):
        for m, g in ((48, 0.44), (55, 0.36), (63, 0.30), (67, 0.26)):
            mix.place('solo', self.viola(m, 1.1, 1.0, accent=0.6), tw, g)

    def solo(self, mix, t0):
        T = self.T
        for bt, m, d in T.SOLO[self.name]:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            mix.place('solo', self.viola(m, d * T.BEAT * 0.95, 0.95, accent=0.3), ts, 0.52)

    def reassemble(self, mix):
        T, q16 = self.T, self.M.q16
        for j in range(6):
            tj = q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / 6)
            mix.place(self.bus, pizz((48, 55, 60, 63, 67, 72)[j], 0.7), tj, 0.20, pan=-0.2)

    def last_bar(self, mix):
        T = self.T
        t35 = (T.N_BARS - 1) * T.BAR                                      # climbing G into the C on frame 0
        for bt in range(4):
            mix.place(self.bus, self.viola((55, 59, 62, 67)[bt], 0.3, 0.9), t35 + bt * T.BEAT, 0.20, pan=-0.2)
