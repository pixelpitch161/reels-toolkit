"""BALLFIGHT engine: a small, honest 2D fight between two balls in a square ring (top-down), reusable by every episode.

    from fight import Rules, Fight
    run = Fight('GUITAR', 'PIANO', seed=7, rules=Rules(bpm=129.84, t_fight=1.8485)).run()
    run['winner'], run['t_ko'], run['events'] ...

Rules (the same for both fighters; nothing is scripted or edited):
  - balls move at one constant speed and bounce off the ropes (the ring is |x|, |y| <= A in the ring's own frame); the
    ropes "give" a little (a seeded +-ROPE_GIVE deg on every bounce, so no run settles into a fixed orbit); ball vs ball
    is an elastic clash, after which both go back to the shared speed;
  - weapons and abilities come from fighters.py (blade / volley / pulse / beam / orbit; shockwave, shield, haste, arc, wave,
    slam); a volley may `curve` (homing projectiles with a turn-rate limit and a lifetime) or `bounce` (ricochet off
    the ropes up to N times);
  - every landed hit grows the attacker's weapon (a blade gets longer and / or spins faster, a volley gets one more
    projectile, a pulse reaches further), so the fight escalates by itself; levels come from damage dealt (LV2 / LV3 unlock abilities), and the 'low' ability fires once
    when HP drops below LOW_HP;
  - volleys fire and abilities trigger on the beat grid, pulses on the 16th grid (the fight IS the music); a blade's hits are detected with a
    swept test, so nothing tunnels at any spin speed; a spinning blade destroys enemy projectiles it sweeps through;
  - the fight ends at the first K.O. (HP <= 0).
The run records every frame (30 fps) for the renderer and every event (hits, blocks, parries, ropes, volleys,
abilities, level-ups, the K.O.) for the soundtrack and the captions. Seeded: same seed, same fight."""
import math
import numpy as np
from fighters import FIGHTERS


class Rules:
    def __init__(self, bpm=129.84, t_fight=1.8485, fps=30, t_max=58.0, dmg_scale=1.0, **kw):
        self.A = 3.0                # half-size of the ring inside the ropes (ring frame units)
        self.R = 0.6                # ball radius (both the same size: fairness)
        self.V = 2.6                # the shared ball speed
        self.DT = 1.0 / 120.0       # 4 substeps per frame
        self.FPS = fps
        self.ROPE_GIVE = 5.0        # deg of seeded jitter on every rope bounce
        self.HP = 100.0
        self.LOW_HP = 25.0
        self.LV_DMG = (14.0, 36.0)  # damage dealt to reach LV2, LV3
        self.KNOCK_T = 0.38         # s a knockback speed boost takes to decay
        self.LAUNCH_SPREAD = 38.0   # deg either side of the corner -> centre line, seeded
        self.KEY_R = 0.11           # projectile hit radius
        self.bpm = bpm
        self.beat = 60.0 / bpm
        self.t_fight = t_fight
        self.t_max = t_max
        self.dmg_scale = dmg_scale
        for k, v in kw.items():
            setattr(self, k, v)


def _wrap(a):
    return (a + math.pi) % (2 * math.pi) - math.pi


def _clears(w):
    """does this weapon destroy enemy projectiles it passes? (beams: `clears`, default True)"""
    return w.get('clears', True)


def _swept(th0, dth, phi, half):
    """does a blade sweeping from th0 by dth (signed) cross the angular interval phi +- half?"""
    if dth < 0:
        th0, dth = th0 + dth, -dth
    d = (phi - half - th0) % (2 * math.pi)
    return d <= dth or d >= 2 * math.pi - 2 * half


class Ball:
    def __init__(self, name, side, rules):
        self.name = name
        self.cfg = FIGHTERS[name]
        self.w = self.cfg['weapon']
        self.side = side              # 0 = red corner (left), 1 = blue corner (right)
        self.hp = rules.HP
        self.dealt = 0.0
        self.level = 1
        self.boost = 0.0
        self.p = np.zeros(2)
        self.v = np.zeros(2)
        # blade
        self.th = 0.0
        self.spin_dir = 1.0 if side == 0 else -1.0
        self.spin = self.w.get('spin', 0.0)            # rad/s; grows by `spin_grow` per landed hit (blade option)
        self.blade_len = self.w.get('base', 0.0)     # blade length, or a pulse's reach beyond the ball's surface
        self.frets = self.w.get('frets0', 0)
        self.cool = 0.0
        self.pull = self.w.get('pull', 0.0)             # gravity toward this ball (weapon option `pull`), grows per landed pulse
        # volley
        self.n_vol = float(self.w.get('n0', 0))   # a float: `grow` may add a fraction of a key per landed volley
        self.haste_mult = 1.0
        # beam (opt-in: only fighters whose weapon kind is 'beam' use these)
        self.beam_w = self.w.get('width', 0.0)      # beam width; `blade_len` holds the beam's length beyond the surface
        self.charge = None                          # dict(aim, kb_fire, t0) while the bell charges
        # ability state
        self.shield = 0.0
        self.haste = 0.0
        self.ready = {}               # ability slot -> time it may fire next (None = locked)
        self.used = set()
        self.low_done = False
        # dash (#33): contact window of the current dash, whether it already hit, whether it's on the weapon's own grid
        self.dash_t = 0.0
        self.dash_hit = True
        self.dash_main = True


class Fight:
    def __init__(self, a='GUITAR', b='PIANO', seed=0, rules=None, record=True):
        self.R = rules or Rules()
        self.rng = np.random.default_rng(seed)
        self.seed = seed
        self.balls = [Ball(a, 0, self.R), Ball(b, 1, self.R)]
        self.record = record
        A, Rb = self.R.A, self.R.R
        # corners: red = (-A, +A) (screen left), blue = (+A, -A) (screen right) in the ring frame
        self.corner = [np.array([-A + 1.15, A - 1.15]), np.array([A - 1.15, -A + 1.15])]
        for k, bl in enumerate(self.balls):
            bl.p = self.corner[k].copy()
            to_c = -bl.p / np.linalg.norm(bl.p)
            ang = math.atan2(to_c[1], to_c[0]) + math.radians(self.rng.uniform(-1, 1) * self.R.LAUNCH_SPREAD)
            bl.v = self.R.V * np.array([math.cos(ang), math.sin(ang)])
            bl.th = self.rng.uniform(0, 2 * math.pi)
        # projectiles: arrays (grown as needed)
        self.kp = np.zeros((0, 2))
        self.kv = np.zeros((0, 2))
        self.kown = np.zeros(0, int)
        self.kid = np.zeros(0, int)
        self.kvol = np.zeros(0, int)      # volley id (growth: a volley that lands counts once)
        self.kidx = np.zeros(0, int)      # index in its volley (= its note up the scale)
        self.kdmg = np.zeros(0)
        self.next_id = 0
        self.next_vol = 0
        self.vol_landed = set()
        self.waves = []                   # dict(owner, c, r, speed, rmax, dmg, knock, kind, hit=False, name)
        self.slams = []                   # dict(owner, c, t_slam, radius, dmg, knock, done)
        self.arcs = []                    # dict(owner, t0, t1, aim, sweep, n, emitted, speed, dmg)
        self.beams = []                   # dict(owner, o, d, len, w, t1, hit, dmg, knock): active beam blasts
        self.kcurve = {}                  # projectile id -> (turn rate rad/s, time it fades): curving volleys only
        self.khr = {}                     # projectile id -> hit radius (when it isn't Rules.KEY_R)
        self.kbounce = {}                 # projectile id -> rope bounces so far: volleys with `bounce` only
        self.lines = []                   # 'line' hazards: rows of pipes erupting from the floor (dicts, see fire_line)
        self.strings = []                 # 'trap' hazards: tripwire strings across the ring (dicts, see lay_string)
        self.next_hz = 0                  # hazard id (lines + strings), for events and the renderer
        self.cones = []                   # (#33) dict(owner, c, aim, half, r, rmax, speed, dmg, knock, hit, grow)
        self.events = []
        self.t = 0.0
        self.winner = None
        self.t_ko = None

    # ------------------------------------------------------------------ helpers
    def ev(self, kind, who, *data):
        self.events.append((round(self.t, 5), kind, who) + tuple(data))

    def damage(self, att, vic, dmg, kind, x, y, sub=0):
        """apply a hit (unless shielded); returns True if it landed"""
        V = self.balls[vic]
        if getattr(V, 'rw_left', 0.0) > 0:                   # rewinding (ability kind 'rewind'): intangible
            return False
        if V.shield > 0:
            self.ev('block', vic, att, x, y, kind)
            return False
        d = dmg * self.R.dmg_scale
        V.hp -= d
        A_ = self.balls[att]
        A_.dealt += d
        self.ev('hit', att, vic, d, x, y, kind, sub)
        return True

    def knock(self, vic, away, k):
        V = self.balls[vic]
        n = np.linalg.norm(away)
        if n < 1e-9:
            return
        away = away / n
        cur = V.v / (np.linalg.norm(V.v) + 1e-12)
        d = 0.8 * away + 0.2 * cur
        V.v = d / (np.linalg.norm(d) + 1e-12) * self.R.V
        V.boost = max(V.boost, k)

    def spawn_key(self, owner, pos, ang, speed, dmg, vol, idx):
        self.kp = np.vstack([self.kp, pos])
        self.kv = np.vstack([self.kv, [speed * math.cos(ang), speed * math.sin(ang)]])
        self.kown = np.append(self.kown, owner)
        self.kid = np.append(self.kid, self.next_id)
        self.kvol = np.append(self.kvol, vol)
        self.kidx = np.append(self.kidx, idx)
        self.kdmg = np.append(self.kdmg, dmg)
        self.next_id += 1

    def kill_keys(self, mask, why, by=-1):
        if not mask.any():
            return
        for i in np.nonzero(mask)[0]:
            self.ev(why, int(self.kown[i]), by, float(self.kp[i, 0]), float(self.kp[i, 1]), int(self.kid[i]))
        keep = ~mask
        self.kp, self.kv, self.kown, self.kid = self.kp[keep], self.kv[keep], self.kown[keep], self.kid[keep]
        self.kvol, self.kidx, self.kdmg = self.kvol[keep], self.kidx[keep], self.kdmg[keep]

    # ------------------------------------------------------------------ weapons
    def fire_volley(self, k):
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        n = int(B.n_vol + 1e-9)
        d = O.p - B.p
        aim = math.atan2(d[1], d[0])
        spread = math.radians(w['spread']) if n < 2 else min(math.radians(w['spread']), math.radians(w['max_fan']) / (n - 1))
        vol = self.next_vol
        self.next_vol += 1
        for j in range(n):
            a = aim + (j - (n - 1) / 2) * spread
            pos = B.p + (self.R.R + 0.06) * np.array([math.cos(a), math.sin(a)])
            self.spawn_key(k, pos, a, w['speed'], w['dmg'], vol, j)
            if w.get('curve'):
                self.kcurve[self.next_id - 1] = (math.radians(w['curve']), self.t + w.get('life', 3.0))
            if w.get('hit_r'):
                self.khr[self.next_id - 1] = w['hit_r']
        self.ev('fire', k, n, vol)

    def fire_pulse(self, k, main):
        """a pulse: a ring blasts out from the ball to its reach (`blade_len` beyond the surface). Pulses on the weapon's
        own grid (`main`) grow the reach when they land; extra pulses from haste (e.g. a drum roll on the 16ths) hit for
        `sub_dmg` x and don't grow it"""
        B = self.balls[k]
        w = B.w
        dmg = w['dmg'] * (1.0 if main else w.get('sub_dmg', 0.5))
        self.waves.append(dict(owner=k, c=B.p.copy(), r=self.R.R, speed=w['speed'], rmax=self.R.R + B.blade_len, dmg=dmg,
                               knock=w['knock'] * (1.0 if main else 0.5), kind='pulse', hit=False, name='pulse',
                               t0=self.t, grow=main, clears=w.get('clears', False)))
        self.ev('pulse', k, int(main), float(B.blade_len))

    # ---- #33 weapon kinds: 'dash' (body-slam dash, contact damage) and 'cone' (a strummed cone of sound). Both fire on
    # their own 16th-grid counter in run(); `blade_len` holds the dash power / the cone's length (recorded as 'BL').
    def fire_dash(self, k, main):
        """the ball throws itself at the opponent (aimed a little ahead of it): a speed burst of `blade_len` x the shared
        speed that decays like a knockback, so power = speed + distance. Contact during the next `window` s hits once."""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        power = B.blade_len * (1.0 if main else w.get('sub_power', 1.0))
        d = O.p - B.p
        dist = float(np.hypot(*d))
        lead = 0.5 * dist / (self.R.V * (1 + power))
        tgt = O.p + O.v * lead
        dd = tgt - B.p
        ang = math.atan2(dd[1], dd[0])
        B.boost = max(B.boost, power)
        B.v = np.array([math.cos(ang), math.sin(ang)]) * self.R.V * (1 + B.boost)
        B.dash_t = w['window']
        B.dash_hit = False
        B.dash_main = bool(main)
        self.ev('dash', k, int(main), float(power), float(B.p[0]), float(B.p[1]), float(ang))

    def fire_cone(self, k, main):
        """a wedge of sound (half-angle `half` deg) aimed at the opponent, expanding from the ball to `blade_len` beyond
        its surface; it hits once, knocks back, and a landed cone on the weapon's own grid grows the length"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        d = O.p - B.p
        aim = math.atan2(d[1], d[0])
        self.cones.append(dict(owner=k, c=B.p.copy(), aim=aim, half=math.radians(w['half']), r=self.R.R,
                               rmax=self.R.R + B.blade_len, speed=w['speed'],
                               dmg=w['dmg'] * (1.0 if main else w.get('sub_dmg', 0.5)),
                               knock=w['knock'] * (1.0 if main else 0.6), hit=False, grow=bool(main)))
        self.ev('cone', k, int(main), float(B.blade_len), float(B.p[0]), float(B.p[1]), float(aim))

    def dash_cone_step(self, dt):
        R = self.R.R
        for k, B in enumerate(self.balls):
            if B.w['kind'] != 'dash' or B.dash_t <= 0:
                continue
            B.dash_t = max(0.0, B.dash_t - dt)
            if B.dash_hit:
                continue
            O = self.balls[1 - k]
            rel = O.p - B.p
            dist = float(np.hypot(*rel))
            if dist <= 2 * R + 1e-6:
                B.dash_hit = True
                hx, hy = (B.p + rel / max(dist, 1e-9) * R).tolist()
                w = B.w
                landed = self.damage(k, 1 - k, w['dmg'] * (1.0 if B.dash_main else w.get('sub_dmg', 0.5)), 'dash',
                                     hx, hy, int(B.dash_main))
                self.knock(1 - k, rel, w['knock'])
                if landed and B.dash_main:
                    B.blade_len = min(w['max_power'], B.blade_len + w['grow'])
        for C in self.cones:
            r0 = C['r']
            C['r'] = min(C['rmax'], r0 + C['speed'] * dt)
            if C['hit']:
                continue
            o = 1 - C['owner']
            O = self.balls[o]
            rel = O.p - C['c']
            dist = float(np.hypot(*rel))
            off = abs(_wrap(math.atan2(rel[1], rel[0]) - C['aim']))
            if off <= C['half'] + math.asin(min(1.0, R / max(dist, 1e-6))) and r0 < dist - R * 0.5 <= C['r'] + 1e-9:
                C['hit'] = True
                hx, hy = (C['c'] + rel / max(dist, 1e-6) * (dist - R)).tolist()
                landed = self.damage(C['owner'], o, C['dmg'], 'cone', hx, hy)
                self.knock(o, rel, C['knock'])
                if landed and C['grow']:
                    Bc = self.balls[C['owner']]
                    Bc.blade_len = min(Bc.w['max_len'], Bc.blade_len + Bc.w['grow'])
        self.cones = [C for C in self.cones if C['r'] < C['rmax'] - 1e-9]

    def blade_step(self, k, dt):
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        mult = self.ab_mult(k)
        dth = B.spin_dir * B.spin * mult * dt
        th0 = B.th
        B.th = (B.th + dth) % (2 * math.pi)
        B.cool = max(0.0, B.cool - dt)
        R = self.R.R
        reach = R + B.blade_len
        # parry: enemy projectiles the blade sweeps through
        if w.get('parry') and len(self.kp):
            enemy = self.kown != k
            if enemy.any():
                rel = self.kp - B.p[None]
                dist = np.hypot(rel[:, 0], rel[:, 1])
                inr = enemy & (dist > R * 0.8) & (dist < reach + self.R.KEY_R)
                if inr.any():
                    hit = np.zeros(len(self.kp), bool)
                    for i in np.nonzero(inr)[0]:
                        phi = math.atan2(rel[i, 1], rel[i, 0])
                        half = math.asin(min(1.0, self.R.KEY_R / max(dist[i], 1e-6))) + 0.04
                        if _swept(th0, dth, phi, half):
                            hit[i] = True
                    self.kill_keys(hit, 'parry', k)
        # hit the opponent
        if B.cool <= 0:
            rel = O.p - B.p
            dist = float(np.hypot(*rel))
            if dist - R < reach:
                phi = math.atan2(rel[1], rel[0])
                half = math.asin(min(1.0, R / max(dist, 1e-6)))
                if _swept(th0, dth, phi, half):
                    hx, hy = (B.p + rel / dist * (dist - R)).tolist()
                    if O.shield > 0:
                        self.ev('block', 1 - k, k, hx, hy, 'blade')
                        B.spin_dir *= -1.0                       # the blade bounces off the shield and reverses
                        B.cool = w['cooldown']
                    else:
                        landed = self.damage(k, 1 - k, w['dmg'], 'blade', hx, hy, B.frets)
                        B.cool = w['cooldown']
                        tang = np.array([-math.sin(B.th), math.cos(B.th)]) * B.spin_dir
                        self.knock(1 - k, rel / dist + 0.6 * tang, w.get('knock', 0.8))
                        if landed:
                            if B.blade_len + w['grow'] <= w['max_len'] + 1e-9:
                                B.blade_len += w['grow']
                                B.frets += 1
                            if w.get('spin_grow'):
                                B.spin = min(w['spin_max'], B.spin + w['spin_grow'])

    def beam_beat(self, k, kb):
        """beam weapons, on the beat grid: fire a charged beam when its beat comes, then start the next charge every
        `every` beats (haste divides the interval). The aim locks when the charge starts, at the opponent's predicted
        spot `lead` x the charge time ahead (a straight line: walls, clashes and knockbacks make it miss)"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        if B.charge is not None and kb >= B.charge['kb_fire']:
            self.beam_fire(k)
        ev = max(1, int(round(w['every'] / self.ab_mult(k))))
        if B.charge is None and kb % ev == 0:
            nb = max(1, min(int(w.get('charge_beats', 1)), ev))
            tc = nb * self.R.beat
            tgt = O.p + O.v * tc * w.get('lead', 1.0)
            src = B.p + B.v * tc
            d = tgt - src
            aim = math.atan2(d[1], d[0])
            B.charge = dict(aim=aim, kb_fire=kb + nb, t0=self.t)
            self.ev('charge', k, aim, tc, float(B.blade_len), float(B.beam_w))

    def beam_fire(self, k):
        B = self.balls[k]
        w = B.w
        a = B.charge['aim']
        d = np.array([math.cos(a), math.sin(a)])
        o = B.p + d * (self.R.R + 0.04)
        self.beams.append(dict(owner=k, o=o, d=d, len=float(B.blade_len), w=float(B.beam_w), t1=self.t + w.get('hold', 0.12),
                               hit=False, dmg=w['dmg'], knock=w.get('knock', 0.9)))
        self.ev('beam', k, float(o[0]), float(o[1]), a, float(B.blade_len), float(B.beam_w))
        B.charge = None

    def beams_step(self, dt):
        """beam fighters' bells track the opponent (locked while charging); active beams hit once each (a capsule test,
        so a growing width matters) and pierce enemy projectiles in their path; a landed beam grows length + width"""
        R = self.R.R
        for k, B in enumerate(self.balls):
            if B.w['kind'] != 'beam':
                continue
            if B.charge is not None:
                B.th = B.charge['aim'] % (2 * math.pi)
            else:
                rel = self.balls[1 - k].p - B.p
                want = math.atan2(rel[1], rel[0])
                dth = _wrap(want - B.th)
                step = B.w.get('track', 5.0) * dt
                B.th = (B.th + max(-step, min(step, dth))) % (2 * math.pi)
        for Bm in self.beams:
            k = Bm['owner']
            o = 1 - k
            O = self.balls[o]
            rel = O.p - Bm['o']
            s_ = min(Bm['len'], max(0.0, float(rel @ Bm['d'])))
            off = rel - Bm['d'] * s_
            dist = float(np.hypot(*off))
            if not Bm['hit'] and dist < R + Bm['w'] / 2:
                Bm['hit'] = True
                n = off / dist if dist > 1e-6 else -Bm['d']
                hx, hy = (O.p - n * R).tolist()
                landed = self.damage(k, o, Bm['dmg'], 'beam', hx, hy)
                self.knock(o, Bm['d'] + 0.3 * n, Bm['knock'])
                if landed:
                    A_ = self.balls[k]
                    A_.blade_len = min(A_.w['max_len'], A_.blade_len + A_.w['grow'])
                    A_.beam_w = min(A_.w['max_w'], A_.beam_w + A_.w.get('grow_w', 0.0))
            if len(self.kp) and _clears(self.balls[k].w):
                rk = self.key_r()
                relk = self.kp - Bm['o'][None]
                sk = np.clip(relk @ Bm['d'], 0.0, Bm['len'])
                dk = np.hypot(*(relk - sk[:, None] * Bm['d'][None]).T)
                self.kill_keys((self.kown == o) & (dk < Bm['w'] / 2 + rk), 'cleared', k)
        self.beams = [Bm for Bm in self.beams if self.t < Bm['t1'] - 1e-9]

    def key_r(self):
        """every projectile's hit radius (Rules.KEY_R unless its weapon set `hit_r`)"""
        if not self.khr:
            return self.R.KEY_R
        return np.array([self.khr.get(int(i), self.R.KEY_R) for i in self.kid])

    def steer(self, dt):
        """curving volleys: each homing projectile turns toward its target at a limited rate; it fades after its life"""
        if not len(self.kp):
            return
        live = set(int(i) for i in self.kid)
        for i in [i for i in self.kcurve if i not in live]:
            del self.kcurve[i]
            self.khr.pop(i, None)
        fade = np.zeros(len(self.kp), bool)
        for j in range(len(self.kp)):
            c = self.kcurve.get(int(self.kid[j]))
            if c is None:
                continue
            rate, t_die = c
            if self.t >= t_die:
                fade[j] = True
                continue
            tgt = self.balls[1 - int(self.kown[j])].p
            rel = tgt - self.kp[j]
            want = math.atan2(rel[1], rel[0])
            cur = math.atan2(self.kv[j, 1], self.kv[j, 0])
            dth = _wrap(want - cur)
            a = cur + max(-rate * dt, min(rate * dt, dth))
            sp = float(np.hypot(*self.kv[j]))
            self.kv[j] = [sp * math.cos(a), sp * math.sin(a)]
        self.kill_keys(fade, 'fade')

    def ricochet(self):
        """volley option `bounce` (#31's vinyl discs): a projectile that crosses a rope is mirrored back into the ring
        instead of dying, up to its weapon's `bounce` times (counted per projectile id); then it leaves like any other"""
        A = self.R.A
        past = np.abs(self.kp) > A
        for i in np.nonzero(past.any(1))[0]:
            nb = self.balls[int(self.kown[i])].w.get('bounce', 0)
            kid = int(self.kid[i])
            c = self.kbounce.get(kid, 0)
            if c >= nb:
                continue
            for ax in (0, 1):
                if past[i, ax]:
                    self.kp[i, ax] = math.copysign(A, self.kp[i, ax]) * 2 - self.kp[i, ax]
                    self.kv[i, ax] = -self.kv[i, ax]
            self.kbounce[kid] = c + 1
            self.ev('ricochet', int(self.kown[i]), float(self.kp[i, 0]), float(self.kp[i, 1]), kid, c + 1)

    # ------------------------------------------------------------------ hazards: 'line' (erupting pipes), 'trap' (tripwire strings)
    def hazard_beat(self, k, kb):
        """'line' / 'trap' weapons act on the beat grid, every `every` beats (haste divides the interval)"""
        B = self.balls[k]
        if kb % max(1, int(round(B.w['every'] / self.ab_mult(k)))) == 0:
            if B.w['kind'] == 'line':
                self.fire_line(k)
            else:
                self.lay_string(k)

    def _to_ropes(self, p, u):
        """largest s >= 0 with p + s u still inside the ropes (|x|, |y| <= A); 0 if p is outside"""
        A = self.R.A
        s = math.inf
        for ax in (0, 1):
            if u[ax] > 1e-9:
                s = min(s, (A - p[ax]) / u[ax])
            elif u[ax] < -1e-9:
                s = min(s, (-A - p[ax]) / u[ax])
        return max(0.0, s)

    def fire_line(self, k):
        """'line': a row of pipes erupts from the floor, from the ball's surface toward the opponent (aimed where it is
        now; it moves, so long lines can miss). The eruption front runs out at `speed`; each pipe stays up `linger` s
        and then sinks; the row stops at the ropes. One hit per row (capsule test, half-width `width` / 2); a landed row
        grows the row's length (`blade_len`) by `grow` up to `max_len`"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        d = O.p - B.p
        n = float(np.hypot(*d))
        u = d / n if n > 1e-9 else B.v / (np.linalg.norm(B.v) + 1e-12)
        p0 = B.p + u * (self.R.R + w.get('gap', 0.05))
        L = min(float(B.blade_len), self._to_ropes(p0, u))
        hid = self.next_hz
        self.next_hz += 1
        self.lines.append(dict(owner=k, id=hid, p0=p0, u=u, L=L, t0=self.t, speed=w['speed'], linger=w['linger'],
                               width=w['width'], dmg=w['dmg'], knock=w.get('knock', 0.8), hit=False))
        self.ev('line', k, hid, float(p0[0]), float(p0[1]), float(u[0]), float(u[1]), L, w['speed'], w['linger'],
                w['width'])

    def lay_string(self, k):
        """'trap': a string is stretched rope to rope across the ring, through the point `at` of the way from this ball
        to the opponent and square to the line between them. Whoever crosses an enemy string takes `dmg` (once per
        `cool` s per string; `knock` = a stumble forward, no turn); its owner passes freely. A fighter keeps
        int(n_vol) strings (`n0`, +`grow` per landed crossing up to `n_max`): laying one more drops its oldest"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        A = self.R.A
        d = O.p - B.p
        n = float(np.hypot(*d))
        nrm = d / n if n > 1e-9 else B.v / (np.linalg.norm(B.v) + 1e-12)
        tdir = np.array([-nrm[1], nrm[0]])
        c = B.p + d * w.get('at', 0.5)
        lo, hi = -math.inf, math.inf
        for ax in (0, 1):
            if abs(tdir[ax]) > 1e-9:
                s1, s2 = (-A - c[ax]) / tdir[ax], (A - c[ax]) / tdir[ax]
                lo, hi = max(lo, min(s1, s2)), min(hi, max(s1, s2))
        a, b = c + tdir * lo, c + tdir * hi
        hid = self.next_hz
        self.next_hz += 1
        self.strings.append(dict(owner=k, id=hid, a=a, nrm=nrm, tdir=tdir, len=hi - lo, t0=self.t, cool=0.0,
                                 last=float((O.p - a) @ nrm), dmg=w['dmg'], knock=w.get('knock', 0.0)))
        self.ev('string', k, hid, float(a[0]), float(a[1]), float(b[0]), float(b[1]))
        mine = [S for S in self.strings if S['owner'] == k]
        cap = max(1, int(B.n_vol + 1e-9))
        for S in mine[:max(0, len(mine) - cap)]:
            self.strings.remove(S)
            self.ev('string_off', k, S['id'])

    def hazards_step(self, dt):
        R = self.R.R
        for Ln in self.lines:
            k = Ln['owner']
            O = self.balls[1 - k]
            el = self.t - Ln['t0']
            hi = min(Ln['L'], el * Ln['speed'])
            lo = max(0.0, (el - Ln['linger']) * Ln['speed'])
            if Ln['hit'] or hi <= lo:
                continue
            rel = O.p - Ln['p0']
            s_ = min(hi, max(lo, float(rel @ Ln['u'])))
            off = rel - Ln['u'] * s_
            dist = float(np.hypot(*off))
            if dist < R + Ln['width'] / 2:
                Ln['hit'] = True
                nv = off / dist if dist > 1e-6 else np.array([-Ln['u'][1], Ln['u'][0]])
                hx, hy = (O.p - nv * R).tolist()
                landed = self.damage(k, 1 - k, Ln['dmg'], 'pipe', hx, hy, Ln['id'])
                self.knock(1 - k, nv + 0.4 * Ln['u'], Ln['knock'])
                if landed:
                    B = self.balls[k]
                    B.blade_len = min(B.w['max_len'], B.blade_len + B.w['grow'])
        self.lines = [Ln for Ln in self.lines if (self.t - Ln['t0'] - Ln['linger']) * Ln['speed'] < Ln['L'] - 1e-9]
        for S in self.strings:
            k = S['owner']
            O = self.balls[1 - k]
            S['cool'] = max(0.0, S['cool'] - dt)
            rel = O.p - S['a']
            sd = float(rel @ S['nrm'])
            if sd * S['last'] < 0 and S['cool'] <= 0 and 0.0 <= float(rel @ S['tdir']) <= S['len']:
                hx, hy = (O.p - S['nrm'] * sd).tolist()
                landed = self.damage(k, 1 - k, S['dmg'], 'string', hx, hy, S['id'])
                S['cool'] = self.balls[k].w.get('cool', 0.4)
                if S['knock'] > 0:
                    self.knock(1 - k, O.v, S['knock'])
                if landed:
                    B = self.balls[k]
                    B.n_vol = min(B.w['n_max'], B.n_vol + B.w['grow'])
            if sd != 0.0:
                S['last'] = sd

    ORBIT_SEATS = (0, 4, 2, 6, 1, 5, 3, 7)   # orbit satellites take these seats (of 8, 45 deg apart): spread evenly,
                                             # and a new satellite never moves the others

    def orbit_sats(self, k):
        """an orbit weapon's satellites: [(j, ring-frame position)], j = 0.. in the order they were added (its sub index)"""
        B = self.balls[k]
        r = self.R.R + B.blade_len
        out = []
        for j in range(int(B.n_vol + 1e-9)):
            a = B.th + self.ORBIT_SEATS[j] * math.pi / 4
            out.append((j, B.p + r * np.array([math.cos(a), math.sin(a)])))
        return out

    def orbit_step(self, k, dt):
        """weapon kind 'orbit' (#31's arpeggiator): `n0` satellites circle the ball at `base` beyond its surface, `spin`
        rad/s (haste multiplies it; with `wobble` the orbit radius also swings by that fraction, `wobble_beats` per cycle,
        while haste lasts); a satellite that touches the opponent hits it (`dmg`, `knock`) and rests `cooldown` s; every
        landed hit adds `grow` satellites up to `n_max` (8 seats); with `parry` a satellite destroys enemy projectiles it
        touches and rests `parry_cool` s. `th` = the orbit's angle, `n_vol` = satellites, `blade_len` = the live radius"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        if not hasattr(B, 'sat_cool'):
            B.sat_cool = np.zeros(len(self.ORBIT_SEATS))
            B.orb_base = B.blade_len
        B.th = (B.th + B.spin_dir * B.spin * self.ab_mult(k) * dt) % (2 * math.pi)
        B.sat_cool = np.maximum(0.0, B.sat_cool - dt)
        wob = w.get('wobble', 0.0) * min(1.0, B.haste / 0.25) if B.haste > 0 else 0.0
        B.blade_len = B.orb_base * (1 + wob * math.sin(2 * math.pi * self.t / (w.get('wobble_beats', 1.0) * self.R.beat)))
        R, rs = self.R.R, w['size']
        sats = self.orbit_sats(k)
        if getattr(O, 'rw_left', 0.0) <= 0:                      # (a rewinding opponent is intangible)
            for j, q in sats:
                if B.sat_cool[j] > 0:
                    continue
                rel = O.p - q
                dist = float(np.hypot(*rel))
                if dist < R + rs:
                    hx, hy = (O.p - rel / max(dist, 1e-6) * R).tolist()
                    B.sat_cool[j] = w['cooldown']
                    landed = self.damage(k, 1 - k, w['dmg'], 'orbit', hx, hy, j)
                    self.knock(1 - k, rel, w.get('knock', 0.3))
                    if landed:
                        B.n_vol = min(w['n_max'], B.n_vol + w['grow'])
        if w.get('parry') and len(self.kp):
            for j, q in sats:
                if B.sat_cool[j] > 0 or not len(self.kp):
                    continue
                m = (self.kown != k) & (np.hypot(*(self.kp - q[None]).T) < rs + self.R.KEY_R)
                if m.any():
                    self.kill_keys(m, 'parry', k)
                    B.sat_cool[j] = w.get('parry_cool', w['cooldown'])

    # ------------------------------------------------------------------ ability kind 'rewind' (#31 REWIND)
    def _rewind_ab(self, B):
        return next(a for a in B.cfg['abilities'].values() if a['kind'] == 'rewind')

    def _rewind_then(self, B, ab):
        """the ball's recorded state `window` s ago: (t, p, v, hp), or None if the record doesn't reach back that far"""
        H = getattr(B, 'rw_hist', None)
        if not H or H[0][0] > self.t - ab['window'] + 1e-6:
            return None
        for h in H:
            if h[0] >= self.t - ab['window'] - 1e-9:
                return h
        return None

    def rewind_ready(self, k, ab):
        """a rewind fires only when it would undo at least `min_undo` HP (damage taken in the last `window` s)"""
        B = self.balls[k]
        then = self._rewind_then(B, ab)
        return then is not None and then[3] - B.hp >= ab.get('min_undo', 0.0)

    def start_rewind(self, k, ab):
        """REWIND: undo the last `window` s of damage taken (HP back to its recorded value then, as a 'heal' event) and
        zip back along the ball's own recorded path to where it was then, in `zip_beats` beats; intangible while it zips,
        then it carries on with the velocity it had then. The opponent keeps what it gained (levels, weapon growth)"""
        B = self.balls[k]
        then = self._rewind_then(B, ab)
        if then is None:
            return
        pts = [h[1] for h in B.rw_hist if h[0] >= then[0] - 1e-9]
        B.rw_path = np.array(pts[::-1] + [then[1]])          # now -> then
        B.rw_dur = ab['zip_beats'] * self.R.beat
        B.rw_left = B.rw_dur
        heal = max(0.0, min(self.R.HP, then[3]) - B.hp)
        B.rw_hp = (B.hp, B.hp + heal)
        B.rw_v = then[2].copy()
        self.ev('heal', k, heal, then[0])

    def rewind_step(self, dt, ks):
        """keep each rewinder's record (one entry per substep, `window` s deep) and drive a rewind in progress"""
        for k in ks:
            B = self.balls[k]
            if not hasattr(B, 'rw_hist'):
                from collections import deque
                B.rw_hist = deque(maxlen=int(round(self._rewind_ab(B)['window'] / self.R.DT)) + 3)
            if getattr(B, 'rw_left', 0.0) > 0:
                B.rw_left = max(0.0, B.rw_left - dt)
                u = 1.0 - B.rw_left / B.rw_dur
                f = u * (len(B.rw_path) - 1)
                i0 = min(int(f), len(B.rw_path) - 1)
                i1 = min(i0 + 1, len(B.rw_path) - 1)
                B.p = B.rw_path[i0] + (B.rw_path[i1] - B.rw_path[i0]) * (f - i0)
                B.hp = B.rw_hp[0] + (B.rw_hp[1] - B.rw_hp[0]) * u
                if B.rw_left <= 0:
                    B.v = B.rw_v / (np.linalg.norm(B.rw_v) + 1e-12) * self.R.V
                    B.boost = 0.0
                    B.rw_hist.clear()                             # the undone stretch can't be rewound into again
                    self.ev('rewound', k, float(B.p[0]), float(B.p[1]))
                continue
            B.rw_hist.append((self.t, B.p.copy(), B.v.copy(), B.hp))

    # ------------------------------------------------------------------ abilities
    def ab_dur(self, ab):
        """an ability's duration in s: `dur` (s) or `dur_beats` (beats at this episode's tempo)"""
        return ab['dur'] if 'dur' in ab else ab['dur_beats'] * self.R.beat

    def ab_mult(self, k):
        return self.balls[k].haste_mult if self.balls[k].haste > 0 else 1.0

    def trigger(self, k, slot):
        B, O = self.balls[k], self.balls[1 - k]
        ab = B.cfg['abilities'][slot]
        kind = ab['kind']
        first = slot not in B.used
        B.used.add(slot)
        self.ev('ability', k, ab['name'], kind, int(first))
        if kind in ('shockwave', 'wave'):
            self.waves.append(dict(owner=k, c=B.p.copy(), r=self.R.R, speed=ab['speed'], rmax=ab['radius'], dmg=ab['dmg'],
                                   knock=ab['knock'], kind=kind, hit=False, name=ab['name'], t0=self.t))
        elif kind == 'shield':
            B.shield = self.ab_dur(ab)
        elif kind == 'haste':
            B.haste = self.ab_dur(ab)
            B.haste_mult = ab['mult']
        elif kind == 'arc':
            d = O.p - B.p
            aim = math.atan2(d[1], d[0])
            self.arcs.append(dict(owner=k, t0=self.t, t1=self.t + ab['beats'] * self.R.beat, aim=aim,
                                  sweep=math.radians(ab['sweep']), n=ab['n'], emitted=0, speed=ab['speed'], dmg=ab['dmg'],
                                  vol=self.next_vol))
            self.next_vol += 1
        elif kind == 'slam':
            lead = ab['tele'] * self.R.beat
            tgt = O.p + O.v * lead * 0.6
            A = self.R.A - self.R.R
            tgt = np.clip(tgt, -A, A)
            self.slams.append(dict(owner=k, c=tgt, t0=self.t, t_slam=self.t + lead, radius=ab['radius'], dmg=ab['dmg'],
                                   knock=ab['knock'], done=False, name=ab['name']))
        elif kind == 'rewind':
            self.start_rewind(k, ab)

    def abilities_step(self, on_beat):
        for k, B in enumerate(self.balls):
            # unlocks by damage dealt
            lv = 1 + sum(B.dealt >= x for x in self.R.LV_DMG)
            while B.level < lv:
                B.level += 1
                slot = 'unlock1' if B.level == 2 else 'unlock2'
                B.ready[slot] = self.t
                self.ev('level', k, B.level)
            if not on_beat:
                continue
            if not B.low_done and B.hp < self.R.LOW_HP:
                B.low_done = True
                self.trigger(k, 'low')
                continue
            for slot in ('unlock1', 'unlock2'):
                r = B.ready.get(slot)
                if r is not None and B.cfg['abilities'][slot]['kind'] == 'rewind' and not self.rewind_ready(k, B.cfg['abilities'][slot]):
                    continue                                        # a rewind waits for damage worth undoing
                if r is not None and self.t >= r - 1e-6:
                    self.trigger(k, slot)
                    B.ready[slot] = self.t + B.cfg['abilities'][slot]['every_bars'] * 4 * self.R.beat
                    break                                           # one ability per fighter per beat

    def effects_step(self, dt):
        R = self.R.R
        for W in self.waves:
            r0 = W['r']
            W['r'] = min(W['rmax'], r0 + W['speed'] * dt)
            o = 1 - W['owner']
            O = self.balls[o]
            rel = O.p - W['c']
            dist = float(np.hypot(*rel))
            if not W['hit'] and r0 < dist - R * 0.5 <= W['r'] + 1e-9:
                W['hit'] = True
                hx, hy = (W['c'] + rel / max(dist, 1e-6) * (dist - R)).tolist()
                landed = self.damage(W['owner'], o, W['dmg'], W['kind'], hx, hy)
                self.knock(o, rel, W['knock'])
                if landed and W.get('grow'):
                    Bw = self.balls[W['owner']]
                    Bw.blade_len = min(Bw.w['max_reach'], Bw.blade_len + Bw.w['grow'])
                    if Bw.w.get('pull_grow'):
                        Bw.pull = min(Bw.w['pull_max'], Bw.pull + Bw.w['pull_grow'])
            if len(self.kp) and W.get('clears', True):
                kr = np.hypot(*(self.kp - W['c'][None]).T)
                m = (self.kown == o) & (kr > r0 - 0.15) & (kr <= W['r'] + 0.05)
                self.kill_keys(m, 'cleared', W['owner'])
        self.waves = [W for W in self.waves if W['r'] < W['rmax'] - 1e-9]
        for S in self.slams:
            if not S['done'] and self.t >= S['t_slam']:
                S['done'] = True
                o = 1 - S['owner']
                O = self.balls[o]
                rel = O.p - S['c']
                dist = float(np.hypot(*rel))
                self.ev('slam', S['owner'], float(S['c'][0]), float(S['c'][1]), int(dist < S['radius'] + R * 0.6))
                if dist < S['radius'] + R * 0.6:
                    self.damage(S['owner'], o, S['dmg'], 'slam', float(O.p[0]), float(O.p[1]))
                    self.knock(o, rel if dist > 1e-3 else O.v, S['knock'])
        for Ar in self.arcs:
            n = Ar['n']
            while Ar['emitted'] < n:
                j = Ar['emitted']
                te = Ar['t0'] + (Ar['t1'] - Ar['t0']) * j / max(1, n - 1)
                if self.t + 1e-9 < te:
                    break
                B = self.balls[Ar['owner']]
                a = Ar['aim'] - Ar['sweep'] / 2 + Ar['sweep'] * j / max(1, n - 1)
                pos = B.p + (R + 0.06) * np.array([math.cos(a), math.sin(a)])
                self.spawn_key(Ar['owner'], pos, a, Ar['speed'], Ar['dmg'], Ar['vol'], j)
                Ar['emitted'] += 1
        self.arcs = [Ar for Ar in self.arcs if Ar['emitted'] < Ar['n']]

    def pull_step(self, k, dt):
        """gravity (weapon option `pull`): the opponent's heading bends toward ball k with a constant pull (units/s^2);
        its speed is kept (constant-speed rule), so the pull only steers"""
        B, O = self.balls[k], self.balls[1 - k]
        d = B.p - O.p
        dist = float(np.hypot(*d))
        if dist < 1e-6 or B.pull <= 0:
            return
        sp = float(np.linalg.norm(O.v))
        v = O.v + d / dist * B.pull * dt
        O.v = v / (np.linalg.norm(v) + 1e-12) * sp

    # ------------------------------------------------------------------ motion
    def move(self, dt):
        A, R = self.R.A, self.R.R
        for k, B in enumerate(self.balls):
            if getattr(B, 'rw_left', 0.0) > 0:                  # rewinding: rewind_step drives it along its path
                continue
            B.boost *= math.exp(-dt / self.R.KNOCK_T)
            s = self.R.V * (1 + B.boost)
            v = B.v / (np.linalg.norm(B.v) + 1e-12) * s
            B.p = B.p + v * dt
            B.v = v
            for ax in (0, 1):
                lim = A - R
                if abs(B.p[ax]) > lim:
                    B.p[ax] = math.copysign(lim, B.p[ax]) * 2 - B.p[ax]
                    B.v[ax] = -B.v[ax]
                    g = math.radians(self.rng.normal(0, self.R.ROPE_GIVE * 0.6))
                    c, s_ = math.cos(g), math.sin(g)
                    nv = np.array([c * B.v[0] - s_ * B.v[1], s_ * B.v[0] + c * B.v[1]])
                    if np.sign(nv[ax]) == np.sign(B.v[ax]):     # never bend back into the rope
                        B.v = nv
                    self.ev('rope', k, ax, int(math.copysign(1, B.p[ax])), float(B.p[0]), float(B.p[1]), float(s))
        # ball vs ball
        P, Q = self.balls
        d = Q.p - P.p
        dist = float(np.hypot(*d))
        if dist < 2 * R and getattr(P, 'rw_left', 0.0) <= 0 and getattr(Q, 'rw_left', 0.0) <= 0:
            n = d / max(dist, 1e-9)
            over = 2 * R - dist
            P.p -= n * over / 2
            Q.p += n * over / 2
            vp, vq = float(P.v @ n), float(Q.v @ n)
            if vp - vq > 0:
                P.v += (vq - vp) * n
                Q.v += (vp - vq) * n
            for B in (P, Q):
                B.v = B.v / (np.linalg.norm(B.v) + 1e-12) * self.R.V * (1 + B.boost)
            mx, my = ((P.p + Q.p) / 2).tolist()
            self.ev('clash', -1, mx, my)
        # projectiles
        if self.kcurve:
            self.steer(dt)
        if len(self.kp):
            self.kp = self.kp + self.kv * dt
            if self.balls[0].w.get('bounce') or self.balls[1].w.get('bounce'):
                self.ricochet()
            out = (np.abs(self.kp) > A).any(1)
            self.kill_keys(out, 'wall')
        if len(self.kp):
            for k, B in enumerate(self.balls):
                if getattr(B, 'rw_left', 0.0) > 0:              # projectiles pass through a rewinding ball
                    continue
                enemy = self.kown != k
                if not enemy.any():
                    continue
                dd = np.hypot(*(self.kp - B.p[None]).T)
                m = enemy & (dd < R + (self.key_r() if self.khr else self.R.KEY_R))
                if m.any():
                    for i in np.nonzero(m)[0]:
                        att = int(self.kown[i])
                        landed = self.damage(att, k, float(self.kdmg[i]), 'key', float(self.kp[i, 0]), float(self.kp[i, 1]),
                                             int(self.kidx[i]))
                        if landed:
                            vol = int(self.kvol[i])
                            if vol not in self.vol_landed:
                                self.vol_landed.add(vol)
                                Att = self.balls[att]
                                if Att.w['kind'] == 'volley':
                                    Att.n_vol = min(Att.w['n_max'], Att.n_vol + Att.w['grow'])
                            # a light nudge along the key's flight
                            kv = self.kv[i] / (np.linalg.norm(self.kv[i]) + 1e-9)
                            cur = B.v / (np.linalg.norm(B.v) + 1e-9)
                            nv = 0.88 * cur + 0.12 * kv
                            B.v = nv / (np.linalg.norm(nv) + 1e-9) * np.linalg.norm(B.v)
                    keep = ~m
                    self.kp, self.kv, self.kown, self.kid = self.kp[keep], self.kv[keep], self.kown[keep], self.kid[keep]
                    self.kvol, self.kidx, self.kdmg = self.kvol[keep], self.kidx[keep], self.kdmg[keep]
                    if not len(self.kp):
                        break

    # ------------------------------------------------------------------ record
    KMAX = 72

    def snap(self, rec):
        B0, B1 = self.balls
        rec['P'].append([B0.p.copy(), B1.p.copy()])
        rec['V'].append([B0.v.copy(), B1.v.copy()])
        rec['TH'].append([B0.th, B1.th])
        rec['BL'].append([B0.blade_len, B1.blade_len])
        rec['FR'].append([B0.frets, B1.frets])
        rec['NV'].append([int(B0.n_vol + 1e-9), int(B1.n_vol + 1e-9)])
        rec['HP'].append([max(0.0, B0.hp), max(0.0, B1.hp)])
        rec['LV'].append([B0.level, B1.level])
        rec['SH'].append([B0.shield, B1.shield])
        rec['HA'].append([B0.haste, B1.haste])
        rec['SPIN'].append([B0.spin_dir, B1.spin_dir])
        rec['OM'].append([B0.spin, B1.spin])
        K = np.full((self.KMAX, 7), np.nan)
        n = min(self.KMAX, len(self.kp))
        if n:
            ang = np.arctan2(self.kv[:n, 1], self.kv[:n, 0])
            K[:n] = np.stack([self.kp[:n, 0], self.kp[:n, 1], ang, self.kown[:n], self.kid[:n], self.kidx[:n],
                              self.kvol[:n]], 1)
        rec['K'].append(K)
        rec['WAVES'].append([(W['owner'], float(W['c'][0]), float(W['c'][1]), W['r'], W['rmax'], W['kind'], W['t0'])
                             for W in self.waves])
        rec['SLAMS'].append([(S['owner'], float(S['c'][0]), float(S['c'][1]), S['t0'], S['t_slam'], S['radius'])
                             for S in self.slams if self.t < S['t_slam'] + 0.6])

    # ------------------------------------------------------------------ run
    def run(self):
        R = self.R
        rec = {k: [] for k in ('P', 'V', 'TH', 'BL', 'FR', 'NV', 'HP', 'LV', 'SH', 'HA', 'SPIN', 'OM', 'K', 'WAVES', 'SLAMS')} \
            if self.record else None
        dt = R.DT
        steps_per_frame = int(round(1.0 / (R.FPS * dt)))
        k0 = int(math.ceil(R.t_fight / R.beat - 1e-6))           # first beat of the fight
        next_beat = k0
        s16 = R.beat / 4.0
        k16_0 = int(math.ceil(R.t_fight / s16 - 1e-6))       # first 16th of the fight (pulse weapons)
        next16 = k16_0
        pulse = [k for k, B in enumerate(self.balls) if B.w['kind'] == 'pulse']
        pullers = [k for k, B in enumerate(self.balls) if B.w.get('pull')]
        rewinders = [k for k, B in enumerate(self.balls) if any(a['kind'] == 'rewind' for a in B.cfg['abilities'].values())]
        v16 = [k for k, B in enumerate(self.balls) if B.w['kind'] == 'volley' and B.w.get('on16')]   # volleys on the 16th
        # grid (option `on16`), so haste can fire an every-beat volley on the 8ths / 16ths
        dc = [k for k, B in enumerate(self.balls) if B.w['kind'] in ('dash', 'cone')]       # (#33)
        dc_k0 = int(math.ceil(R.t_fight / s16 - 1e-6))
        dc_next = dc_k0
        self.t = R.t_fight
        f0 = int(round(R.t_fight * R.FPS))
        step = 0
        if rec is not None:
            rec['f0'] = f0
            self.snap(rec)                                       # frame f0
        while self.t < R.t_max:
            self.t = R.t_fight + (step + 1) * dt
            on_beat = False
            if self.t + 1e-9 >= next_beat * R.beat:
                on_beat = True
                kb = next_beat - k0
                next_beat += 1
                for k, B in enumerate(self.balls):
                    if B.w['kind'] == 'volley' and not B.w.get('on16'):
                        ev = max(1, int(round(B.w['every'] / (self.ab_mult(k)))))
                        if kb % ev == 0:
                            self.fire_volley(k)
                    elif B.w['kind'] == 'beam':
                        self.beam_beat(k, kb)
                    elif B.w['kind'] in ('line', 'trap'):
                        self.hazard_beat(k, kb)
            if (pulse or v16) and self.t + 1e-9 >= next16 * s16:
                k16 = next16 - k16_0
                next16 += 1
                for k in pulse:
                    B = self.balls[k]
                    main = 4 * B.w['every']                          # the weapon's own grid, in 16ths
                    every16 = max(1, int(round(main / self.ab_mult(k))))
                    if k16 % every16 == 0:
                        self.fire_pulse(k, k16 % main == 0)
                for k in v16:
                    B = self.balls[k]
                    every16 = max(1, int(round(4 * B.w['every'] / self.ab_mult(k))))
                    if k16 % every16 == 0:
                        self.fire_volley(k)
            if dc and self.t + 1e-9 >= dc_next * s16:          # (#33) dash / cone weapons: their own 16th counter
                dc_k = dc_next - dc_k0
                dc_next += 1
                for k in dc:
                    B = self.balls[k]
                    dc_main = int(round(4 * B.w['every']))
                    dc_every = max(1, int(round(dc_main / self.ab_mult(k))))
                    if dc_k % dc_every == 0:
                        (self.fire_dash if B.w['kind'] == 'dash' else self.fire_cone)(k, dc_k % dc_main == 0)
            for k, B in enumerate(self.balls):
                B.shield = max(0.0, B.shield - dt)
                B.haste = max(0.0, B.haste - dt)
            for k in pullers:
                self.pull_step(k, dt)
            self.move(dt)
            if rewinders:
                self.rewind_step(dt, rewinders)
            for k, B in enumerate(self.balls):
                if B.w['kind'] == 'blade':
                    self.blade_step(k, dt)
                elif B.w['kind'] == 'orbit':
                    self.orbit_step(k, dt)
            if self.beams or any(B.w['kind'] == 'beam' for B in self.balls):
                self.beams_step(dt)
            if self.lines or self.strings:
                self.hazards_step(dt)
            self.effects_step(dt)
            if dc:
                self.dash_cone_step(dt)                          # (#33)
            self.abilities_step(on_beat)
            step += 1
            dead = [k for k, B in enumerate(self.balls) if B.hp <= 0]
            if dead:
                lo = min(dead, key=lambda k: self.balls[k].hp)
                self.winner = 1 - lo
                self.t_ko = self.t
                self.ev('ko', lo, self.winner)
                if rec is not None:
                    self.snap(rec)
                break
            if rec is not None and step % steps_per_frame == 0:
                self.snap(rec)
        out = dict(seed=self.seed, winner=self.winner, t_ko=self.t_ko, events=self.events,
                   names=[B.name for B in self.balls], corner=[c.tolist() for c in self.corner])
        if rec is not None:
            out.update(rec)
        return out


def lead_changes(events, hp0=100.0, hyst=5.0, t_from=4.0):
    """number of times the HP lead flips (with hysteresis), from the event list"""
    hp = [hp0, hp0]
    lead, n, times = 0, 0, []
    for e in events:
        if e[1] == 'heal':                      # ability kind 'rewind' gives back damage taken
            hp[e[2]] += e[3]
            continue
        if e[1] == 'hit':
            hp[e[3]] -= e[4]
            d = hp[0] - hp[1]
            new = 1 if d > hyst else (-1 if d < -hyst else lead)
            if new != lead and new != 0:
                if lead != 0 and e[0] >= t_from:
                    n += 1
                    times.append(e[0])
                lead = new
    return n, times
