"""PIANO v2 (blue), new in #45 (user, Oct 27: "PIANO ROLL"): a player-piano roll of punched paper lies along the
ring's back side through the piano's corner; on the 8th grid every row of holes passes the brass read bar and every
hole fires a key-shaped note-shot from its pitch lane across the ring (the rows are the piano's line in the episode's
music: what you see is what you hear); a landed row punches more holes per row. SUSTAIN (the pedal: the next rows'
notes hang in the air for a beat like mines, then fly at the enemy), GLISSANDO (a sweeping arc of keys from the ball),
88 KEYS (the roll runs off its spool: every hole fires at once, a full-ring barrage; then a new roll is threaded).

Look: the cream paper roll on the apron (a canvas decal, so the ropes, shadows and attack lights sit on it) scrolling
toward the brass read bar, its upcoming rows of holes visible before they fire, holes flashing blue as they fire, the
supply spool turning; the note-shots as small glowing white / black keys (sharps are black) with short trails,
SUSTAIN's hanging keys pulsing over blue rings, 88 KEYS' full row and the paper running off in shreds, then the new roll
sliding in from the spool; key shards on hits; keys, hammers and paper in the K.O. shatter; the VS-card panel (keys +
a punched roll).
Sound: the piano voice from the Pixel Pitch bridge. Every fired row IS the piano's line (its pitches, as the holes):
the melody note plus the harmony a denser row adds; SUSTAIN rows ring on with the pedal down until they fly; a landed
key adds a bright accent; GLISSANDO runs up the keyboard with the arc; 88 KEYS is a whole-keyboard smash and a
glissando down; the solo is the config's SOLO with left-hand chords."""
import math

import numpy as np
from PIL import Image, ImageDraw
from reelkit.config import W, H, FPS, BAYER_FULL, ss, ease
from reelkit.shadows import proj_point
import scene as S

from . import base

R = S.R_BALL
A = S.A
PAPER = np.array([238, 228, 200], float)          # cream punched paper
PAPER_DK = np.array([196, 184, 152], float)
HOLE = np.array([40, 34, 52], float)
BRASS = np.array([232, 188, 92], float)
WOOD = np.array([110, 64, 40], float)
KEY_W = np.array([246, 242, 230], float)          # ivory
KEY_B = np.array([34, 30, 40], float)             # ebony
BLACK_PC = (1, 3, 6, 8, 10)
BAR = 0.05                                         # the read bar: this far outside the fight wall (the apron)
DEPTH = 0.40                                       # paper visible from the bar out to the spool
SPOOL = BAR + DEPTH + 0.05
ROWGAP = 0.115                                     # paper between two rows (one 8th of scroll)
LAT = A - 0.18                                     # the paper's half-width along the side
SHOT_Y = 0.34                                      # note-shots fly at this height
HOLE_L, HOLE_W = 0.036, 0.048                      # half-sizes of a punched hole (along the scroll, across)


def is_black(m):
    return int(m) % 12 in BLACK_PC


class Look(base.Look):
    shards = ['key'] * 8 + ['blackkey'] * 4 + ['hammer'] * 5 + ['paper'] * 4
    spark_flash = ()
    flash_hits = ('slam', 'wave', 'shockwave')
    fringe_hits = ()
    rope_kinds = {'runoff': 0.8}

    # ---------------------------------------------------------------- setup
    def setup(self):
        P, k = self.P, self.k
        T = P.T
        self.T8 = T.BEAT / 2
        w = self.cfg['weapon']
        self.reg = w.get('reg', self.cfg['register'])
        self.margin = w.get('margin', 0.45)
        self.ax = 0 if P.CORNER0[k][0] < 0 else 1              # the axis whose -A side holds the roll (the engine's)
        ab1, low = self.cfg['abilities']['unlock1'], self.cfg['abilities']['low']
        self.out_t = ab1.get('out16', 2) * T.BEAT / 4
        self.hang_t = ab1.get('hang_beats', 1) * T.BEAT
        self.rows = {}                                         # abs8 -> (t, pitches, sustained)
        self.vols = {}                                         # vol -> (t, kind, sustained)
        self.runoffs = []                                      # (t, pitches, t_reload_end)
        for e in P.EV:
            if e[0] > P.T_KO + 1e-6:
                break
            if len(e) < 3 or e[2] != k:
                continue
            if e[1] == 'roll':
                self.rows[int(e[4])] = (e[0], tuple(e[6]), bool(e[5]))
                self.vols[int(e[3])] = (e[0], 'row', bool(e[5]))
            elif e[1] == 'runoff':
                self.vols[int(e[3])] = (e[0], 'runoff', False)
                self.runoffs.append((e[0], tuple(e[5]), e[0] + low.get('reload_beats', 0) * T.BEAT))
        self.row_t = np.array(sorted(v[0] for v in self.rows.values())) if self.rows else np.zeros(0)
        TT = sys_title()
        TT.PATTERNS[self.name] = title_pattern                  # v2's panel (#27 keeps its own in its own process)

    # ---------------------------------------------------------------- geometry
    def lane(self, m, margin=None):
        lo, hi = self.reg
        mg = self.margin if margin is None else margin
        u = (float(m) - lo) / max(1.0, float(hi - lo))
        return (A - mg) - 2 * (A - mg) * min(1.0, max(0.0, u))

    def ring_pt(self, out, lat):
        """(out: distance outside the fight wall, lat: along the side) -> ring frame (sx, sz)"""
        if self.ax == 0:
            return -A - out, lat
        return lat, -A - out

    def local(self, sxp, szp):
        """ring-frame pixels -> (out, lat) on the roll's side"""
        if self.ax == 0:
            return -(sxp + A), szp
        return -(szp + A), sxp

    # ---------------------------------------------------------------- the paper's state at t
    def paper(self, t):
        """(edge, gone): the paper's leading edge (out from the bar; 0 = threaded right up to the bar) and whether the
        old roll has run off (88 KEYS until the new one is threaded)"""
        for t0, pitches, t1 in self.runoffs:
            tc = self.P.fx_clock(t)
            if t0 <= tc < t1:
                u = (tc - t0) / max(1e-6, t1 - t0)
                if u < 0.35:
                    return DEPTH + 0.2, True                   # bare spool
                return DEPTH * (1 - ease((u - 0.35) / 0.65)), False   # the new roll slides in from the spool
        return 0.0, False

    def visible_rows(self, t):
        """[(out, pitches, abs8, fired)] of the rows on the visible paper at t (rows past the bar are under it)"""
        T8 = self.T8
        tc = self.P.fx_clock(t)
        j0 = int(math.floor(tc / T8)) - 1
        out = []
        for j in range(j0, j0 + int(DEPTH / ROWGAP) + 3):
            d = BAR + (j * T8 - tc) / T8 * ROWGAP
            if d < BAR - ROWGAP * 0.6 or d > BAR + DEPTH:
                continue
            r = self.rows.get(j)
            if r is not None:
                out.append((d, r[1], j, r[0] <= tc + 1e-6))
        return out

    # ---------------------------------------------------------------- the shots at t (from the run)
    def shots(self, t, i=None):
        """[(x, z, ang, pitch, sid, kind, age, hanging)] of the piano's live note-shots"""
        P, k = self.P, self.k
        if not (P.T.T_FIGHT <= t < P.T_HS):
            return []
        i = P.sim_i(t) if i is None else i
        K = P.RUN['K'][i]
        held = {int(h[0]): h[1] for h in P.RUN['HOLD'][i]} if 'HOLD' in P.RUN else {}
        out = []
        tc = min(t, P.T_KO)
        for row in K:
            if np.isnan(row[0]) or int(row[3]) != k:
                continue
            x, z, ang, own, sid, idx, vol = row
            v = self.vols.get(int(vol))
            if v is None:                                       # GLISSANDO's arc (index = its step up the scale)
                out.append((x, z, ang, None, int(sid), 'arc', 1.0, None))
                continue
            t0, kind, sus = v
            ts = held.get(int(sid))
            hang = (tc - ts) if (ts is not None and tc >= ts - 1e-6) else None    # s it has hung (a SUSTAIN mine)
            out.append((x, z, ang, int(idx), int(sid), kind, tc - t0, hang))
        return out

    def trail_pos(self, t, sid, back):
        P = self.P
        j = P.sim_i(t) - back
        if j < 0:
            return None
        K = P.RUN['K'][j]
        m = K[:, 4] == sid
        if not m.any():
            return None
        r = K[np.argmax(m)]
        return float(r[0]), float(r[1])

    # ---------------------------------------------------------------- events
    def shake(self, e):
        if e[1] == 'runoff':
            return 1.6
        if e[1] == 'slam':
            return 1.6
        return None

    def rope(self, e):
        if e[1] == 'runoff':
            return 0.8
        return super().rope(e)

    def spark(self, e):
        if e[7] == 'key':
            return self.P.COL[e[2]] * 0.5 + 120, 6, 2.2, 0.2
        return super().spark(e)

    def hit_glow(self, e):
        if e[7] == 'key':
            return 6, 0.22
        return None

    def fx_event(self, e):
        P, k = self.P, self.k
        t_ = e[0]
        if e[1] == 'roll':                                     # the row fires: a blue light at the read bar
            ps = e[6]
            lat = float(np.mean([self.lane(m) for m in ps]))
            pos = P.up(*self.ring_pt(BAR, lat), 0.06)
            P.FL.add(t_, pos, P.FL_COL[k], radius=1.0, peak=0.30 + 0.06 * len(ps), decay=0.14)
        elif e[1] == 'hit' and e[7] == 'key':
            pos = P.up(e[5], e[6], SHOT_Y + 0.1)
            P.BU.add(t_, pos, P.COL[k], 0.45, bright=False)
            P.PA.emit(t_, pos, 'keyshards', n=3, scale=0.8)
            P.FL.add(t_, pos, P.FL_COL[k], radius=1.0, peak=0.4, decay=0.16)
        elif e[1] == 'runoff':                                 # 88 KEYS: the whole roll lights, the paper runs off
            for lat in (-LAT * 0.7, 0.0, LAT * 0.7):
                P.FL.add(t_, P.up(*self.ring_pt(BAR, lat), 0.1), P.FL_COL[k], radius=2.4, peak=1.0, decay=0.45)
            for j in range(9):
                lat = -LAT + 2 * LAT * (j + 0.5) / 9
                P.PA.emit(t_ + 0.02 * j, P.up(*self.ring_pt(BAR + DEPTH * 0.5, lat), 0.05), 'shreds', n=5,
                          col=[tuple(PAPER), tuple(PAPER_DK), (250, 244, 226)], dir=(0.0, 1.0, 0.0), scale=1.2)
            P.BU.add(t_, P.up(*self.ring_pt(BAR, 0.0), 0.2), P.COL[k], 1.6)
        elif e[1] == 'sustain':                                # the pedal goes down: the roll glows
            P.FL.add(t_, P.up(*self.ring_pt(BAR + 0.2, 0.0), 0.2), P.FL_COL[k], radius=2.0, peak=0.5, decay=0.4)
        elif e[1] == 'ability' and e[4] == 'arc':
            st = P.fighter(k, t_)
            P.FL.add(t_, P.up(st['p'][0], st['p'][1], R), P.FL_COL[k], radius=1.6, peak=0.6, decay=0.3)

    def ko_fx(self, pos):
        self.P.PA.emit(self.P.T_KO, pos, 'keyshards', n=26, spread=180, speed=(0.8, 2.4))

    # ---------------------------------------------------------------- canvas: the roll, the mines' rings, shot lights
    def canvas_after(self, Ic, C):
        P, k = self.P, self.k
        t = C.t
        out, lat = self.local(C.sxp, C.szp)
        near = (out > -0.02) & (out < SPOOL + 0.08) & (np.abs(lat) < LAT + 0.05)
        if near.any():
            idx = np.nonzero(near)[0]
            o, la = out[idx], lat[idx]
            bay = BAYER_FULL[C.ys[idx], C.xs[idx]]
            rim = np.array(S.CANVAS_RIM, float)
            edge, gone = self.paper(t)
            paper = (o >= BAR + edge) & (o <= BAR + DEPTH) & (np.abs(la) <= LAT) & (not gone)
            mul = np.ones((len(idx), 3))
            add = np.zeros((len(idx), 3))
            # the paper (a cream decal: albedo x this = cream), faint octave guide lines, a scroll texture
            tc = P.fx_clock(t)
            ph = (o - BAR) / ROWGAP + t / self.T8                      # scroll phase (whole 8ths: loop-safe)
            tex = 0.96 + 0.04 * np.floor(((ph * 0.5) % 1.0) * 2 + bay * 0.5)
            cream = PAPER[None] / rim[None] * tex[:, None] * 1.12
            mul[paper] = cream[paper]
            lo, hi = self.reg
            for c in range(lo - lo % 12 + 12, hi + 1, 12):              # a faint line under every C
                guide = paper & (np.abs(la - self.lane(c)) < 0.012)
                mul[guide] = (PAPER_DK / rim)[None] * 0.95
            # the holes: rows scrolling toward the bar (their notes are what will play)
            for d, ps, j, fired in self.visible_rows(t):
                if d < BAR + edge:
                    continue
                band = paper & (np.abs(o - d) < HOLE_L)
                if not band.any():
                    continue
                for m in ps:
                    h = band & (np.abs(la - self.lane(m)) < HOLE_W)
                    mul[h] = HOLE / rim * 0.9
                    if d < BAR + 0.06:                                  # passing the bar: blue
                        add[h] += P.COL[k] / 255.0 * 1.4
            # the brass read bar along the side, lit where holes fire
            bar = (np.abs(o - (BAR - 0.014)) < 0.022) & (np.abs(la) <= LAT + 0.04)
            mul[bar] = BRASS / rim * 1.15
            gl = np.zeros(len(idx))
            if len(self.row_t):
                j = np.searchsorted(self.row_t, tc, 'right') - 1
                for jj in range(max(0, j - 1), j + 1):
                    te = self.row_t[jj]
                    if 0 <= tc - te < 0.2:
                        r = [v for v in self.rows.values() if v[0] == te][0]
                        a = math.exp(-(tc - te) / 0.07)
                        for m in r[1]:
                            gl += a * np.clip(1 - np.abs(la - self.lane(m)) / 0.12, 0, 1) * np.clip(1 - np.abs(o - BAR) / 0.14, 0, 1)
            for t0, ps, t1 in self.runoffs:                            # 88 KEYS: every lane at once
                if 0 <= tc - t0 < 0.5:
                    a = math.exp(-(tc - t0) / 0.18)
                    gl += a * 1.6 * (np.abs(la) <= LAT) * np.clip(1 - np.abs(o - BAR) / 0.3, 0, 1)
            add += (np.floor(np.clip(gl, 0, 1.6) * 3 + bay * 0.9) / 3)[:, None] * (P.COL[k] / 255.0 * 1.3 + 0.2)[None]
            # the supply spool: a wooden roller with brass end caps, turning (stripes step with the scroll)
            spool = (np.abs(o - SPOOL) < 0.035) & (np.abs(la) <= LAT + 0.02)
            caps = (np.abs(o - SPOOL) < 0.04) & (np.abs(np.abs(la) - (LAT + 0.05)) < 0.04)
            stripe = ((ph * 2.0) % 1.0) < 0.5
            mul[spool] = (np.where(stripe[:, None], WOOD * 1.15, WOOD * 0.85)[spool] / rim[None]) if not gone else \
                (WOOD * 0.7 / rim)[None]
            if not gone:
                rolled = spool & (np.abs(o - SPOOL) < 0.02)
                mul[rolled] = PAPER_DK / rim * 0.95
            mul[caps] = BRASS / rim
            # SUSTAIN: the pedal down: the bar and the paper glow softly
            st = C.fs[k]
            sus = float(P.RUN['ROLL'][C.i, k, 1]) if (C.in_fight and 'ROLL' in P.RUN) else 0.0
            if sus > 0:
                sk = min(1.0, sus / 0.25)
                add[paper] += sk * 0.18 * (P.COL[k] / 255.0)[None]
            Ic[idx] = Ic[idx] * mul + add
        # the mines' rings and the shots' light on the canvas
        if C.in_fight:
            for x, z, ang, m, sid, kind, age, hang in self.shots(C.t, C.i):
                rd = np.hypot(C.sxp - x, C.szp - z)
                if hang is not None:
                    pulse = 0.6 + 0.4 * math.sin(2 * math.pi * 4.0 * hang)
                    ring = np.clip(1 - np.abs(rd - 0.16) / 0.035, 0, 1) * pulse
                    Ic += (ring * 0.9)[:, None] * (P.COL[k] / 255.0 + 0.2)[None]
                else:
                    spot = np.clip(1 - rd / 0.3, 0, 1) ** 2
                    Ic += (spot * 0.35)[:, None] * (P.COL[k] / 255.0)[None]
        return Ic

    # ---------------------------------------------------------------- shadows of the shots
    def shadows_free(self, F, spot):
        out = []
        for x, z, ang, m, sid, kind, age, hang in self.shots(F.t, F.i if F.in_fight else None):
            Q = self.key_quad(x, z, ang, m, self.shot_y(kind, age, hang, F.t))
            out.append(proj_point(Q, spot))
        return out

    # ---------------------------------------------------------------- the note-shots (painter items)
    def shot_y(self, kind, age, hang, t):
        if kind == 'arc':
            return SHOT_Y
        y = 0.05 + (SHOT_Y - 0.05) * min(1.0, max(0.0, age / 0.08))
        if hang is not None:
            y += 0.03 * math.sin(2 * math.pi * 2.0 * hang)
        return y

    def key_quad(self, x, z, ang, m, y):
        black = m is not None and is_black(m)
        Lk, Wk = (0.24, 0.09) if black else (0.30, 0.12)
        d = np.array([math.cos(ang), math.sin(ang)])
        n = np.array([-d[1], d[0]])
        c = np.array([x, z])
        pts = [c + d * Lk / 2 + n * Wk / 2, c + d * Lk / 2 - n * Wk / 2, c - d * Lk / 2 - n * Wk / 2,
               c - d * Lk / 2 + n * Wk / 2]
        return np.array([S.to_world(p[0], p[1], y) for p in pts])

    def items_free(self, F):
        if not F.in_fight:
            return []
        out = []
        for s in self.shots(F.t, F.i):
            x, z, ang, m, sid, kind, age, hang = s
            out.append((F.cam.depth(S.to_world(x, z, SHOT_Y))[0], ('shot', s)))
        return out

    def draw_item(self, F, payload):
        P, k = self.P, self.k
        fr, cam = F.fr, F.cam
        x, z, ang, m, sid, kind, age, hang = payload[1]
        if hang is not None:
            ang = hang * 3.0 + sid                                 # a hanging key turns slowly
        y = self.shot_y(kind, age, hang, F.t)
        # a short trail back along its flight (where it was 1 and 2 frames ago)
        if hang is None:
            pts = [(x, z)]
            for b in (1, 2):
                q = self.trail_pos(F.t, sid, b)
                if q is not None and math.hypot(q[0] - pts[-1][0], q[1] - pts[-1][1]) < 0.6:
                    pts.append(q)
            if len(pts) > 1:
                sp = cam.wp(np.array([S.to_world(q[0], q[1], y) for q in pts]))
                c = tuple(int(v) for v in np.clip(P.COL[k] * 0.7 + 70, 0, 255))
                fr.draw.line([tuple(q) for q in sp], fill=c, width=2)
        Q = self.key_quad(x, z, ang, m, y)
        black = m is not None and is_black(m)
        base_ = KEY_B if black else KEY_W
        c = P.shade(F.L, np.array([[0, 1.0, 0]]), Q.mean(0)[None], base_, rimk=0.5)[0]
        glow = 0.35 if hang is None else 0.5 + 0.3 * math.sin(2 * math.pi * 4.0 * hang)
        c = c * (1 - 0.3 * glow) + P.COL[k] * 0.3 * glow + (30 if not black else 10) * glow
        fr.poly(cam, Q, np.clip(c, 0, 255))
        s = cam.wp(Q)
        rim_ = tuple(int(v) for v in np.clip(P.COL[k] * 0.55 + (120 if black else 90), 0, 255))
        fr.draw.line([tuple(q) for q in s] + [tuple(s[0])], fill=rim_, width=1)     # blue-lit edges (a black key reads)

    def glows(self, F, drawn):
        P, k = self.P, self.k
        if not F.in_fight:
            return
        for x, z, ang, m, sid, kind, age, hang in self.shots(F.t, F.i):
            y = self.shot_y(kind, age, hang, F.t)
            s_ = F.cam.wp(S.to_world(x, z, y)[None])[0]
            a = 0.30 if hang is None else 0.42 + 0.2 * math.sin(2 * math.pi * 4.0 * hang)
            F.fr.glow(s_[0], s_[1], 8 if hang is None else 10, tuple(P.COL[k] * 0.8 + 50), a)

    # ---------------------------------------------------------------- shards
    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        P = self.P
        sx_, sz_ = p[0], p[2]
        d = np.array([math.cos(rot), math.sin(rot)])
        nn = np.array([-d[1], d[0]])
        c0 = np.array([sx_, sz_])
        if kind in ('key', 'blackkey'):
            Lk, Wk = (0.26, 0.10) if kind == 'key' else (0.18, 0.07)
            q4 = [c0 + d * Lk / 2 * a1 + nn * Wk / 2 * a2 for a1, a2 in ((-1, -1), (1, -1), (1, 1), (-1, 1))]
            sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in q4]))
            P.layer_blend(arr, [sp], KEY_W if kind == 'key' else KEY_B, 1.0)
            return arr
        if kind == 'hammer':                                   # a felt hammer on its shank
            sp = P.screen(cam, np.array([S.to_world(*(c0 - d * 0.14), p[1]), S.to_world(*(c0 + d * 0.06), p[1])]))
            im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
            dr = ImageDraw.Draw(im)
            dr.line([tuple(q) for q in sp], fill=(200, 170, 120), width=1)
            x, y = sp[1]
            dr.ellipse([x - 2, y - 2, x + 2, y + 2], fill=(150, 40, 46))
            return np.asarray(im).astype(np.float32)
        # a curl of punched paper
        pts = [c0 + 0.12 * np.array([math.cos(rot + j * 0.6 * (1 + curl)), math.sin(rot + j * 0.6 * (1 + curl))])
               for j in range(6)]
        sp = P.screen(cam, np.array([S.to_world(q[0], q[1], p[1]) for q in pts]))
        im = Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))
        ImageDraw.Draw(im).line([tuple(q) for q in sp], fill=tuple(int(v) for v in PAPER), width=2)
        return np.asarray(im).astype(np.float32)

    # ---------------------------------------------------------------- checks
    def focus_points(self, t, st):
        out = []
        for lat in (-LAT, 0.0, LAT):
            out.append((S.to_world(*self.ring_pt(BAR, lat), 0.02), 'read bar'))
        out.append((S.to_world(*self.ring_pt(SPOOL, 0.0), 0.05), 'spool'))
        return out


def sys_title():
    import title as TT
    return TT


def title_pattern(base):
    """PIANO v2's VS-card panel: a keyboard along the top of the frame, below it a punched roll (cream-tinted paper with
    rows of holes spelling a little melody, the holes lit blue)"""
    TT = sys_title()
    XX, YY = TT.XX, TT.YY
    kw = 20.0
    kx = XX % kw
    col = np.floor(XX / kw)
    keys = YY < 96
    white = keys & (kx > 1.5) & (YY < 94)
    has_right = np.isin(col % 7, (0, 1, 3, 4, 5))            # C D F G A have a sharp to their right
    has_left = np.isin((col - 1) % 7, (0, 1, 3, 4, 5))
    black = (YY < 60) & (((kx > kw - 6.5) & has_right) | ((kx < 6.5) & has_left))
    out = np.where(white[..., None], base * 0.45 + np.array([236, 232, 222]) * 0.55, base)
    out = np.where(black[..., None], base * 0.2 + 12, out)
    out = np.where((keys & (YY >= 94))[..., None], base * 0.5, out)          # the key bed's shadow line
    paper = YY >= 100
    out = np.where(paper[..., None], out * 0.80 + np.array([238, 228, 200]) * 0.12, out)
    row = np.floor((YY - 100) / 20.0)
    lane = np.floor(XX / 22.0)
    melody = np.round(5 + 4 * np.sin(row * 0.9) + 2 * np.cos(row * 2.3)).astype(int)
    pick = (lane == melody) | (lane == melody + 3) | ((row % 2 == 0) & (lane == melody - 4)) | \
        ((row % 3 == 1) & (lane == melody + 7))
    hole = paper & (((YY - 100) % 20.0) < 8) & ((XX % 22.0) < 11) & pick
    rim = paper & (((YY - 98) % 20.0) < 12) & (((XX + 2) % 22.0) < 15) & pick & ~hole
    out = np.where(rim[..., None], out * 0.5 + np.array([84, 156, 255]) * 0.5, out)
    return np.where(hole[..., None], np.array([150, 205, 255]) * 0.85 + out * 0.15, out)


# ===================================================================================================== sound
class Sound(base.Sound):
    bus_gain, send = 1.7, 0.3
    band = (260, 4200)                                          # the piano's line (C4-C6) and its attack

    def __init__(self, M, k):
        super().__init__(M, k)
        self.bus = 'piano'

    def keys(self, ms, dur, vel=0.85, cap=None):
        """a chord (or one note) in the Pixel Pitch piano voice"""
        M = self.M
        ms = list(ms)
        cap = cap or round(dur + 0.35, 2)
        x = sum(M.note('PIANO', int(m), round(dur, 3), cap, round(vel * (1.0 if j == 0 else 0.8), 1))
                for j, m in enumerate(ms))
        return x / max(1.0, len(ms) ** 0.6)

    def title_blast(self, mix, t):
        """a hammered low chord: the key's root in octaves + the fifth, struck hard"""
        T = self.T
        r = T.KEY_ROOT
        hits = T.TITLE_HITS.get(self.name, (r, r + 7, r + 12, r + 19, r + 24))
        mix.place(self.bus, self.keys(hits, 0.9, 1.0, cap=1.3), t, 0.55)
        mix.place(self.bus, self.keys([hits[0] - 12], 0.9, 1.0, cap=1.3), t, 0.35)

    def count_in(self, mix):
        T = self.T
        ch = T.CHORDS[1]
        ct = T.chord_tones(60, 76, ch)[:3]
        for j, (tc, txt) in enumerate(T.COUNT[:3]):
            mix.place(self.bus, self.keys(ct, 0.18, 0.9), tc, 0.36)
        run = T.scale_notes(64, 84, ch)[-8:]                    # a pickup run up into FIGHT!
        for j, m in enumerate(run):
            mix.place(self.bus, self.keys([m], 0.08, 0.75), T.BAR + 3 * T.BEAT + j * T.BEAT / 8, 0.24)

    def play(self, mix):
        M, T, k = self.M, self.T, self.k
        A, EV, q16 = M.A, M.EV, M.q16
        thin = M.Thinner(45)
        ab = M.CFG[k]['abilities']
        hang = ab['unlock1'].get('out16', 2) * T.BEAT / 4 + ab['unlock1'].get('hang_beats', 1) * T.BEAT
        last_acc = None
        for e in EV:
            t, kind = e[0], e[1]
            if t > M.T_KO + 0.01:
                break
            if kind == 'roll' and e[2] == k:                    # the row = the line (+ its harmony): what you see
                tq = q16(t)
                slot = round(tq / T.S16)
                ps = list(e[6])
                hp = M.hp_at(k, t) / 100.0
                if len(ps) > 1 and not thin.keep(slot, hp):     # a hurt piano drops its harmony first
                    ps = ps[:1]
                sus = bool(e[5])
                dur = hang if sus else 0.24
                sig, g, lead = self.colour(self.keys(ps, dur, 0.9, cap=round(dur + (0.6 if sus else 0.3), 2)), t)
                mix.place(self.bus, sig, tq, 0.40 * g, pan=-0.2)
                if lead > 0.04:                                 # the leader: the melody doubled an octave up
                    mix.place(self.bus, self.colour(self.keys([ps[0] + 12], 0.16, 0.6), t)[0], tq, 0.10 * g, pan=-0.3)
            elif kind == 'hit' and e[2] == k and e[7] == 'key':  # a landed key: a bright accent (one per 16th)
                tq = q16(t)
                slot = round(tq / T.S16)
                if slot == last_acc:
                    continue
                last_acc = slot
                m = int(e[8]) if int(e[8]) >= 30 else T.scale_notes(72, 96, T.chord_at(tq))[int(e[8]) % 7]
                mix.place(self.bus, self.colour(self.keys([m + 12], 0.08, 0.7), t)[0], tq, 0.12, pan=0.1)
                mix.place('fx', A.thud(0.12, 220, 120, seed=int(t * 13)), tq, 0.035)
            elif kind == 'ability' and e[2] == k:
                tq = q16(t)
                name = e[3]
                if name == ab['unlock1']['name']:               # SUSTAIN: the pedal goes down, the strings ring
                    mix.place('fx', A.thud(0.3, 90, 50), tq, 0.10)
                    ch = T.chord_at(tq)
                    mix.place(self.bus, self.keys(T.chord_tones(48, 64, ch)[:4], 2 * T.BEAT, 0.5,
                                                  cap=round(2 * T.BEAT + 0.8, 2)), tq, 0.14)
                elif name == ab['unlock2']['name']:             # GLISSANDO: up the white keys with the arc
                    n = ab['unlock2']['n']
                    run = T.scale_notes(60, 100, T.chord_at(tq))[:n]
                    span = ab['unlock2']['beats'] * T.BEAT
                    for j, m in enumerate(run):
                        mix.place(self.bus, self.colour(self.keys([m], 0.09, 0.8), t)[0], tq + span * j / max(1, n - 1),
                                  0.22, pan=-0.4 + 0.8 * j / max(1, n - 1))
                elif name == ab['low']['name']:                 # 88 KEYS: the whole keyboard at once, then down
                    lo, hi = M.CFG[k]['register']
                    cluster = list(range(lo - 12, hi + 1, 3))
                    mix.place(self.bus, self.keys(cluster, 1.2, 1.0, cap=1.8) * 1.6, tq, 0.42)
                    mix.place(self.bus, self.keys([lo - 24, lo - 12], 1.4, 1.0, cap=2.0), tq, 0.40)
                    for j, m in enumerate(range(hi + 12, lo - 13, -2)):
                        mix.place(self.bus, self.keys([m], 0.06, 0.7), tq + 0.12 + j * 0.022, 0.16,
                                  pan=0.5 - 1.0 * j / 40)
                    mix.place('fx', A.thud(0.8, 100, 34), tq, 0.4)

    def fall_apart(self, mix):
        """the piano falls apart: a cluster slumping down, strings twanging loose, keys clattering"""
        M = self.M
        t0 = M.P.T_HS
        x = self.keys([40, 46, 51, 57], 1.0, 0.9, cap=1.4)
        mix.place(self.bus, M.bend(x, -7, 0.05, 1.0), t0, 0.30)
        for j, m in enumerate((84, 90, 79, 95, 72)):
            mix.place('fx', M.clank(m, 60 + j, 0.25, ((1.0, 0.05), (2.3, 0.03))), t0 + 0.07 * j, 0.06, pan=(j - 2) / 4)

    def stinger(self, mix, tw):
        T = self.T
        ch = T.CHORDS[0]
        mix.place('solo', self.keys([T.root_above(ch, 36)] + T.chord_tones(55, 79, ch)[:5], 1.2, 1.0, cap=1.8), tw, 0.45)

    def solo(self, mix, t0):
        """the config's solo in the right hand, the chord of each bar in the left hand on the bar line"""
        M, T = self.M, self.T
        notes = T.SOLO[self.name]
        for bt, m, d in notes:
            ts = t0 + bt * T.BEAT
            if ts > T.T_REASSEMBLE - 0.05:
                break
            mix.place('solo', self.keys([m], max(0.1, d * T.BEAT * 0.9), 0.95), ts, 0.46)
        b0 = int(round(t0 / T.BAR))
        for b in range(b0, int(T.T_REASSEMBLE / T.BAR + 0.5)):
            ch = T.CHORDS[b]
            lh = [T.root_above(ch, 36), T.root_above(ch, 36) + 12] + T.chord_tones(52, 64, ch)[:2]
            mix.place('solo', self.keys(lh, T.BAR * 0.8, 0.8, cap=round(T.BAR + 0.3, 2)), b * T.BAR, 0.26)

    def reassemble(self, mix):
        T, M = self.T, self.M
        run = T.scale_notes(52, 88, T.CHORDS[-2])[::2][:8]
        for j, m in enumerate(run):
            tj = M.q16(T.T_REASSEMBLE + (T.T_GLIDE - T.T_REASSEMBLE) * j / len(run))
            mix.place(self.bus, self.keys([m], 0.12, 0.75), tj, 0.22)

    def last_bar(self, mix):
        """the melody's run up the last chord into the title slam (16ths over the last beat)"""
        T = self.T
        t_last = (T.N_BARS - 1) * T.BAR
        ch = T.CHORDS[-1]
        tones = T.chord_tones(60, 88, ch)
        for j in range(8):
            m = tones[min(len(tones) - 1, j)]
            mix.place(self.bus, self.keys([m], 0.1, 0.8), t_last + 2 * T.BEAT + j * T.BEAT / 4, 0.24)
