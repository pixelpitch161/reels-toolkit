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
Opt-in options (#34; nothing changes for fighters that don't use them): blade `mode='thrust'` (the blade points at the
enemy and shoots straight out to its reach on the beat, then pulls back: a lance instead of a spin); on a `curve`
volley, `curve_grow` / `curve_max` (deg/s: the turn rate grows per landed volley), and an 'arc' ability with
`curve=True` fires curving projectiles at the owner's current rate; `slow` on a shockwave / wave (dict(mult, dur): the
enemy it hits moves at `mult` x speed for `dur` s).
#29 v3 (opt-in): weapon kinds `combo` (three piston punches) and `flail` (strap flail that wraps the enemy), and an
ability kind `beam` (a charged giant blast through the beam machinery, e.g. HIGH C).
#36 (opt-in, crossovers): `Rules(dmg_side=(a, b))` scales the damage each fighter deals (left, right) on top of the
shared `dmg_scale`: the only per-episode lever when two roster fighters meet (fighters always fight as built).
#30 v2 (opt-in): weapon kinds `inflate` (the ball swells: per-ball radius / mass / speed, then a zig-zag balloon flight;
contact damage) and `bite` (jaws open on the grid and snap a 16th-grid later), and an ability kind `clamp` (a ring-wide
bite after a telegraph).
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
        self.rake_dir = -1.0          # (#33 v2) rake strokes alternate down (+1) / up (-1); the first one is a down
        # opt-in (#34): thrust (blade mode), curve growth (volley option), slow (shockwave option)
        self.ext = self.w.get('rest', 0.0)            # thrust: the lance's current length beyond the surface
        self.thr = None                               # thrust: time the current thrust started (None = idle)
        self.thr_hit = False
        self.thr_k = 1.0                              # thrust: time scale of the current thrust (haste)
        self.curve_now = self.w.get('curve', 0.0)     # curving volley with `curve_grow`: current turn rate (deg/s)
        self.slow = 0.0                               # s left of a slow on this ball
        self.slow_mult = 1.0


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
        for k, bl in enumerate(self.balls):               # thrust: the closed lance starts aimed at the enemy
            if bl.w.get('mode') == 'thrust':
                d = self.balls[1 - k].p - bl.p
                bl.th = math.atan2(d[1], d[0]) % (2 * math.pi)
        self.uses_ext = any(bl.w.get('mode') == 'thrust' or bl.w.get('curve_grow') or
                            any(ab.get('slow') for ab in bl.cfg['abilities'].values()) for bl in self.balls)
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
        self.surfs, self.rakes = [], []   # (#33 v2) SURF'S UP waves, SIX-STRING RAKE strokes
        self.whips, self.sweeps, self.spouts = [], [], []    # (#33 v2) ability kinds 'whip', 'sweep', 'spout'
        self.x33_log = []                 # (#33 v2) waterspout path (t, owner, x, z) for the renderer
        self.sucks = []                   # #30 bellows suction rings (collapsing): dict(owner, c, r, rmax, speed, dmg, knock)
        self.clamps = []                  # #30 v2 ability kind 'clamp' (FULL BELLOWS): dict(owner, t0, t_snap, axis, ...)
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
        hc = getattr(self.R, 'dmg_side', None)     # opt-in (#36, crossovers): per-fighter damage handicap
        if hc is not None:
            d *= hc[att]
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
            if w.get('curve_grow'):                        # #34: the turn rate grows per landed volley
                self.kcurve[self.next_id - 1] = (math.radians(B.curve_now), self.t + w.get('life', 3.0))
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

    # ---- #33 v2 (BALLFIGHT v2 redesign): weapon kinds 'surf' (SURF'S UP) and 'rake' (SIX-STRING RAKE), ability kinds
    # 'whip' (FINGERPICK), 'sweep' (BIG STRUM), 'spout' (UKE-NADO). Weapons fire on the dash / cone 16th counter;
    # `blade_len` = the wave height / the rake length (recorded as 'BL'). The renderer rebuilds waves, strokes, whips and
    # the sweep from their events (closed-form motion); the waterspout's path is logged (out['X33_SPOUT']).
    @staticmethod
    def surf_dist(tau, V, power, kt):
        """how far a surf wave's front has travelled tau s after the strum (the ride's speed burst decays like a knock)"""
        return V * (tau + power * kt * (1.0 - math.exp(-tau / kt)))

    def fire_surf(self, k, main):
        """SURF'S UP: the strum raises a wave the ball rides at the opponent: a speed burst (power grows with the wave
        height) and a wave front `hw` wide that crashes into the enemy once, pushing it along the wave into the ropes"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        h = B.blade_len
        power = (w['p0'] + w['p_h'] * h) * (1.0 if main else w.get('sub_power', 1.0))
        d = O.p - B.p
        dist = float(np.hypot(*d))
        lead = 0.5 * dist / (self.R.V * (1 + power))
        dd = O.p + O.v * lead - B.p
        ang = self.aim_blend(B, dd, w.get('aim_k', 1.0))
        u = np.array([math.cos(ang), math.sin(ang)])
        B.boost = max(B.boost, power)
        B.v = u * self.R.V * (1 + B.boost)
        hw = w['w0'] + w['w_h'] * h
        o = B.p + u * self.R.R * 0.3
        self.surfs.append(dict(owner=k, o=o, u=u, n=np.array([-u[1], u[0]]), t0=self.t, power=power, hw=hw,
                               life=w['life'], dmg=w['dmg'] * (1.0 if main else w.get('sub_dmg', 0.5)),
                               knock=w['knock'] * (0.8 + 0.5 * h), hit=False, grow=bool(main)))
        self.ev('surf', k, int(main), float(h), float(o[0]), float(o[1]), float(ang), float(power), float(hw))

    def fire_rake(self, k, main):
        """SIX-STRING RAKE: six parallel string lines (pointing at the opponent, `blade_len` long) sweep sideways across
        the ring in front of the ball in `stroke` s, alternating down / up strokes; one hit per stroke, which flicks the
        enemy along the stroke"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        aim = self.aim_blend(B, O.p - B.p, w.get('aim_k', 1.0))
        B.rake_dir = -B.rake_dir
        S = w['sweep']
        self.rakes.append(dict(owner=k, c=B.p.copy(), aim=aim, u=np.array([math.cos(aim), math.sin(aim)]),
                               n=np.array([-math.sin(aim), math.cos(aim)]), dir=B.rake_dir, L=B.blade_len, S=S,
                               hb=2.5 * w['gap'], t0=self.t, dur=w['stroke'],
                               dmg=w['dmg'] * (1.0 if main else w.get('sub_dmg', 0.5)),
                               knock=w['knock'] * (1.0 if main else 0.6), hit=False, grow=bool(main), s_prev=-B.rake_dir * S))
        self.ev('rake', k, int(main), float(B.blade_len), float(B.p[0]), float(B.p[1]), float(aim), float(B.rake_dir),
                float(S), float(w['stroke']))

    @staticmethod
    def aim_blend(B, d, k_aim):
        """the attack's direction: toward d (the enemy) blended with where the ball is already heading (`aim_k` 1 = dead
        on target, less = it goes 'in front of it', so it can miss)"""
        if k_aim >= 1.0:
            return math.atan2(d[1], d[0])
        dh = d / (np.hypot(*d) + 1e-9)
        vh = B.v / (np.hypot(*B.v) + 1e-9)
        m = k_aim * dh + (1 - k_aim) * vh
        return math.atan2(m[1], m[0])

    @staticmethod
    def rake_lateral(tau, dur, S, direction):
        x = min(1.0, max(0.0, tau / dur))
        return -direction * S + 2 * direction * S * x * x * (3 - 2 * x)

    @staticmethod
    def sweep_front(tau, dur, reach, R):
        x = min(1.0, max(0.0, tau / dur))
        return R + reach * x ** 1.5

    def ab_x33(self, k, ab, kind):
        B, O = self.balls[k], self.balls[1 - k]
        d = O.p - B.p
        aim = math.atan2(d[1], d[0])
        if kind == 'whip':                     # FINGERPICK: single strings flick out like whips, one per note
            self.whips.append(dict(owner=k, t0=self.t, n=ab['n'], span=ab['beats'] * self.R.beat, emitted=0,
                                   len=ab['len'], dmg=ab['dmg'], knock=ab['knock'], t_ext=ab.get('t_ext', 0.07), live=[]))
        elif kind == 'sweep':                  # BIG STRUM: all six strings sweep the whole ring
            u = np.array([math.cos(aim), math.sin(aim)])
            self.sweeps.append(dict(owner=k, c=B.p.copy(), u=u, t0=self.t, dur=ab['beats'] * self.R.beat,
                                    reach=ab['reach'], dmg=ab['dmg'], knock=ab['knock'], hit=False))
            self.ev('sweep', k, float(B.p[0]), float(B.p[1]), float(aim), float(ab['reach']), float(ab['beats'] * self.R.beat))
        elif kind == 'spout':                  # UKE-NADO: a waterspout that sweeps the ring after the enemy
            p = B.p + np.array([math.cos(aim), math.sin(aim)]) * self.R.R * 1.2
            self.spouts.append(dict(owner=k, p=p, ang=aim, t0=self.t, life=ab['life'], speed=ab['speed'], turn=ab['turn'],
                                    radius=ab['radius'], tick=ab['tick'], dmg=ab['dmg'], max_ticks=ab['max_ticks'],
                                    knock=ab['knock'], ticks=0, last=-9.0, n=0))
            self.ev('spout', k, float(p[0]), float(p[1]), float(aim))

    def surf_rake_step(self, dt):
        R, V = self.R.R, self.R.V
        A = self.R.A
        for Sf in self.surfs:
            tau = self.t - Sf['t0']
            c = Sf['o'] + Sf['u'] * self.surf_dist(tau, V, Sf['power'], self.R.KNOCK_T)
            if abs(c[0]) >= A - 0.05 or abs(c[1]) >= A - 0.05 or tau >= Sf['life']:
                Sf['done'] = True
                self.ev('surf_end', Sf['owner'], float(c[0]), float(c[1]), int(tau < Sf['life']))
                continue
            if Sf['hit']:
                continue
            o = 1 - Sf['owner']
            O = self.balls[o]
            rel = O.p - c
            a, b = float(rel @ Sf['u']), float(rel @ Sf['n'])
            if -R * 0.3 <= a <= R and abs(b) <= Sf['hw'] + R * 0.6:
                Sf['hit'] = True
                hx, hy = (O.p - Sf['u'] * R).tolist()
                landed = self.damage(Sf['owner'], o, Sf['dmg'], 'surf', hx, hy, int(Sf['grow']))
                self.knock(o, Sf['u'], Sf['knock'])
                if landed and Sf['grow']:
                    Bw = self.balls[Sf['owner']]
                    Bw.blade_len = min(Bw.w['max_h'], Bw.blade_len + Bw.w['grow'])
        self.surfs = [Sf for Sf in self.surfs if not Sf.get('done')]
        for Rk in self.rakes:
            tau = self.t - Rk['t0']
            s_now = self.rake_lateral(tau, Rk['dur'], Rk['S'], Rk['dir'])
            s_prev, Rk['s_prev'] = Rk['s_prev'], s_now
            if Rk['hit']:
                continue
            o = 1 - Rk['owner']
            O = self.balls[o]
            rel = O.p - Rk['c']
            a, b = float(rel @ Rk['u']), float(rel @ Rk['n'])
            lo, hi = min(s_prev, s_now) - Rk['hb'], max(s_prev, s_now) + Rk['hb']
            if R * 0.3 <= a <= R + Rk['L'] + R * 0.5 and hi >= b - R and lo <= b + R:
                Rk['hit'] = True
                hx, hy = (Rk['c'] + Rk['u'] * a + Rk['n'] * s_now).tolist()
                landed = self.damage(Rk['owner'], o, Rk['dmg'], 'rake', hx, hy, int(Rk['dir'] > 0))
                self.knock(o, Rk['n'] * Rk['dir'] + 0.25 * Rk['u'], Rk['knock'])
                if landed and Rk['grow']:
                    Bw = self.balls[Rk['owner']]
                    Bw.blade_len = min(Bw.w['max_len'], Bw.blade_len + Bw.w['grow'])
        self.rakes = [Rk for Rk in self.rakes if self.t - Rk['t0'] < Rk['dur'] + 1e-9]
        for Wp in self.whips:
            B = self.balls[Wp['owner']]
            O = self.balls[1 - Wp['owner']]
            while Wp['emitted'] < Wp['n']:
                j = Wp['emitted']
                te = Wp['t0'] + Wp['span'] * j / max(1, Wp['n'] - 1)
                if self.t + 1e-9 < te:
                    break
                d = O.p - B.p
                ang = math.atan2(d[1], d[0])
                Wp['live'].append(dict(j=j, t0=self.t, ang=ang, hit=False))
                self.ev('whip', Wp['owner'], j, float(ang), float(Wp['len']))
                Wp['emitted'] += 1
            for lw in Wp['live']:
                tau = self.t - lw['t0']
                if lw['hit'] or tau > Wp['t_ext']:
                    continue
                u = np.array([math.cos(lw['ang']), math.sin(lw['ang'])])
                p0 = B.p + u * R
                L = Wp['len'] * min(1.0, tau / Wp['t_ext'])
                rel = O.p - p0
                a = float(np.clip(rel @ u, 0.0, L))
                if float(np.hypot(*(rel - u * a))) < R + 0.05:
                    lw['hit'] = True
                    hx, hy = (p0 + u * a).tolist()
                    self.damage(Wp['owner'], 1 - Wp['owner'], Wp['dmg'], 'whip', hx, hy, lw['j'])
                    self.knock(1 - Wp['owner'], u, Wp['knock'])
            Wp['live'] = [lw for lw in Wp['live'] if self.t - lw['t0'] < 2 * Wp['t_ext']]
        self.whips = [Wp for Wp in self.whips if Wp['emitted'] < Wp['n'] or Wp['live']]
        for Sw in self.sweeps:
            tau = self.t - Sw['t0']
            if not Sw['hit']:
                o = 1 - Sw['owner']
                O = self.balls[o]
                a = float((O.p - Sw['c']) @ Sw['u']) - self.sweep_front(tau, Sw['dur'], Sw['reach'], R)
                if -R * 1.2 <= a <= R:
                    Sw['hit'] = True
                    hx, hy = (O.p - Sw['u'] * R).tolist()
                    self.damage(Sw['owner'], o, Sw['dmg'], 'sweep', hx, hy)
                    self.knock(o, Sw['u'], Sw['knock'])
        self.sweeps = [Sw for Sw in self.sweeps if self.t - Sw['t0'] < Sw['dur'] + 1e-9]
        for Sp in self.spouts:
            O = self.balls[1 - Sp['owner']]
            d = O.p - Sp['p']
            want = math.atan2(d[1], d[0])
            Sp['ang'] += float(np.clip(_wrap(want - Sp['ang']), -Sp['turn'] * dt, Sp['turn'] * dt))
            Sp['p'] = Sp['p'] + Sp['speed'] * dt * np.array([math.cos(Sp['ang']), math.sin(Sp['ang'])])
            lim = A - Sp['radius']
            for ax in (0, 1):
                if abs(Sp['p'][ax]) > lim:
                    Sp['p'][ax] = math.copysign(lim, Sp['p'][ax]) * 2 - Sp['p'][ax]
                    v = np.array([math.cos(Sp['ang']), math.sin(Sp['ang'])])
                    v[ax] = -v[ax]
                    Sp['ang'] = math.atan2(v[1], v[0])
            Sp['n'] += 1
            if Sp['n'] % 4 == 0:
                self.x33_log.append((round(self.t, 5), Sp['owner'], float(Sp['p'][0]), float(Sp['p'][1])))
            dist = float(np.hypot(*d))
            if dist < Sp['radius'] + R and self.t - Sp['last'] >= Sp['tick'] - 1e-9 and Sp['ticks'] < Sp['max_ticks']:
                Sp['last'] = self.t
                Sp['ticks'] += 1
                rel = d / max(dist, 1e-9)
                tang = np.array([-rel[1], rel[0]])
                hx, hy = (O.p - rel * R).tolist()
                self.damage(Sp['owner'], 1 - Sp['owner'], Sp['dmg'], 'spout', hx, hy, Sp['ticks'])
                self.knock(1 - Sp['owner'], tang * 0.9 + rel * 0.4, Sp['knock'])
            if self.t - Sp['t0'] >= Sp['life'] or Sp['ticks'] >= Sp['max_ticks']:
                Sp['done'] = True
                self.ev('spout_end', Sp['owner'], float(Sp['p'][0]), float(Sp['p'][1]))
        self.spouts = [Sp for Sp in self.spouts if not Sp.get('done')]

    def start_thrust(self, k):
        """thrust (#34): lock the lance on where the enemy is now and shoot it out (haste speeds the whole thrust up)"""
        B, O = self.balls[k], self.balls[1 - k]
        rel = O.p - B.p
        B.th = math.atan2(rel[1], rel[0]) % (2 * math.pi)
        B.thr = self.t
        B.thr_hit = False
        B.thr_k = 1.0 / self.ab_mult(k)
        self.ev('thrust', k, float(B.blade_len))

    def thrust_step(self, k, dt):
        """thrust (#34): idle, the closed lance tracks the enemy (turn-rate limited); a thrust shoots it out to the reach
        (`t_out`), holds (`t_hold`) and pulls it back (`t_back`); it hits once per thrust while going out or held (the
        enemy ball against the lance segment, swept every substep: the tip moves less than a ball per substep)"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        R = self.R.R
        rest = w.get('rest', 0.25)
        if B.thr is None:
            rel = O.p - B.p
            dth = _wrap(math.atan2(rel[1], rel[0]) - B.th)
            lim = w.get('aim_rate', 8.0) * dt
            B.th = (B.th + max(-lim, min(lim, dth))) % (2 * math.pi)
            B.ext = rest
            return
        u = (self.t - B.thr) / B.thr_k
        t_out, t_hold, t_back = w['t_out'], w['t_hold'], w['t_back']
        if u < t_out:
            e = u / t_out
            B.ext = rest + (B.blade_len - rest) * (1 - (1 - e) ** 2)
            live = True
        elif u < t_out + t_hold:
            B.ext = B.blade_len
            live = True
        elif u < t_out + t_hold + t_back:
            e = (u - t_out - t_hold) / t_back
            B.ext = B.blade_len + (rest - B.blade_len) * e * e * (3 - 2 * e)
            live = False
        else:
            B.thr = None
            B.ext = rest
            return
        d = np.array([math.cos(B.th), math.sin(B.th)])
        a = B.p + d * R * 0.9
        L_ = R * 0.1 + B.ext
        if live and w.get('parry') and len(self.kp):        # the lance swats enemy projectiles it passes through
            enemy = self.kown != k
            if enemy.any():
                rel = self.kp - a[None]
                s_ = np.clip(rel @ d, 0, L_)
                dd = np.hypot(*(rel - s_[:, None] * d[None]).T)
                self.kill_keys(enemy & (dd < self.R.KEY_R + 0.04), 'parry', k)
        if live and not B.thr_hit:
            rel = O.p - a
            s_ = min(max(float(rel @ d), 0.0), L_)
            c = a + d * s_
            dist = float(np.hypot(*(O.p - c)))
            if dist < R:
                B.thr_hit = True
                hx, hy = (O.p + (c - O.p) / max(dist, 1e-6) * R).tolist()
                if O.shield > 0:
                    self.ev('block', 1 - k, k, hx, hy, 'blade')
                    B.thr = self.t - (t_out + t_hold) * B.thr_k          # blocked: the lance pulls straight back
                else:
                    landed = self.damage(k, 1 - k, w['dmg'], 'blade', hx, hy, B.frets)
                    self.knock(1 - k, d, w.get('knock', 0.8))
                    if landed and B.blade_len + w['grow'] <= w['max_len'] + 1e-9:
                        B.blade_len += w['grow']
                        B.frets += 1

    def blade_step(self, k, dt):
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        if w.get('mode') == 'thrust':
            return self.thrust_step(k, dt)
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
                if landed and Bm.get('grow', True):          # an ability's beam (#29 v3) grows no weapon
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

    # ------------------------------------------------------------------ #29 v3: valve combo (opt-in weapon kind 'combo')
    def combo_setup(self, k):
        B, O = self.balls[k], self.balls[1 - k]
        B.punch_q, B.punch_i, B.punches, B.combo_n = [], 0, [], 0
        d = O.p - B.p
        B.th = math.atan2(d[1], d[0]) % (2 * math.pi)            # the valves start facing the enemy

    def combo_beat(self, k, kb):
        """combo: every `every` beats (haste divides it) `punches` pistons punch out of the ball one after another, `gap`
        beats apart (1-2-3 on the 8ths; haste halves the gap too); each aims at the enemy as it starts"""
        B = self.balls[k]
        w = B.w
        if kb % max(1, int(round(w['every'] / self.ab_mult(k)))) == 0:
            g = w.get('gap', 0.5) * self.R.beat / self.ab_mult(k)
            n = int(w.get('punches', 3))
            B.punch_q = [self.t + j * g for j in range(n)]
            B.punch_i = 0
            B.combo_n += 1
            self.ev('combo', k, B.combo_n, n, g)

    def combo_step(self, k, dt):
        """combo: between punches the valves turn toward the enemy (`aim_rate`); a piston shoots out to the reach
        (`t_out`), holds (`t_hold`), pulls back (`t_back`); it hits once (the enemy against the piston, head radius
        `head_r`), the last punch of a combo harder (`dmg3`, `knock3`); a landed punch lengthens the reach (`grow`)"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        R = self.R.R
        rel = O.p - B.p
        want = math.atan2(rel[1], rel[0])
        lim = w.get('aim_rate', 8.0) * dt
        B.th = (B.th + max(-lim, min(lim, _wrap(want - B.th)))) % (2 * math.pi)
        n = int(w.get('punches', 3))
        while B.punch_q and self.t + 1e-9 >= B.punch_q[0]:
            B.punch_q.pop(0)
            i = B.punch_i
            B.punch_i += 1
            B.punches.append(dict(i=i, t0=self.t, th=want, off=(i - (n - 1) / 2) * w.get('spread', 0.15), hit=False,
                                  k=1.0 / self.ab_mult(k)))
            self.ev('punch', k, i, want, float(B.blade_len), B.combo_n)
        t_out, t_hold, t_back = w['t_out'], w['t_hold'], w['t_back']
        live = []
        for P in B.punches:
            u = (self.t - P['t0']) / P['k']
            if u >= t_out + t_hold + t_back:
                continue
            live.append(P)
            if P['hit'] or u >= t_out + t_hold:
                continue
            e = min(1.0, u / t_out)
            ext = B.blade_len * (1 - (1 - e) ** 2)
            d = np.array([math.cos(P['th']), math.sin(P['th'])])
            a0 = B.p + d * R * 0.8 + np.array([-d[1], d[0]]) * P['off']
            relp = O.p - a0
            c = a0 + d * min(max(float(relp @ d), 0.0), R * 0.2 + ext)
            dist = float(np.hypot(*(O.p - c)))
            if dist < R + w['head_r']:
                P['hit'] = True
                hx, hy = (O.p + (c - O.p) / max(dist, 1e-6) * R).tolist()
                if O.shield > 0:
                    self.ev('block', 1 - k, k, hx, hy, 'piston')
                    P['t0'] = self.t - (t_out + t_hold) * P['k']          # blocked: it pulls straight back
                else:
                    last = P['i'] == n - 1
                    landed = self.damage(k, 1 - k, w['dmg3'] if last else w['dmg'], 'piston', hx, hy, P['i'])
                    self.knock(1 - k, d, w['knock3'] if last else w['knock'])
                    if landed:
                        B.blade_len = min(w['max_len'], B.blade_len + w['grow'])
        B.punches = live

    # ------------------------------------------------------------------ #29 v3: strap flail (opt-in weapon kind 'flail')
    def flail_setup(self, k):
        B = self.balls[k]
        B.wrap = None
        B.spin = B.w.get('spin0', 3.0)

    def flail_step(self, k, dt):
        """flail: a heavy head on a strap (centre distance R + `blade_len`) swings round the ball, spinning up by
        `spin_acc` toward `spin_max` (haste: x mult; `haste_snap` jumps straight to it). The head hits once on contact
        (damage grows with momentum, from `dmg_min` x up), then the strap WRAPS the enemy: until a beat `wrap_beats` on,
        the enemy is leashed and reeled in to `reel` (centre distance), then yanked (`yank`) and the swing restarts.
        A landed hit lengthens the strap (`grow`). The balls themselves keep the shared constant speed."""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        R = self.R.R
        if B.wrap is not None:
            Wr = B.wrap
            rel = O.p - B.p
            dist = float(np.hypot(*rel))
            B.th = math.atan2(rel[1], rel[0]) % (2 * math.pi)
            if self.t + 1e-9 >= Wr['t_end']:
                B.wrap = None
                self.ev('unwrap', k, float(O.p[0]), float(O.p[1]))
                self.knock(1 - k, -rel, w.get('yank', 1.0))
                B.spin = w.get('spin0', 3.0)
                B.cool = w.get('cooldown', 0.3)
                return
            u = min(1.0, (self.t - Wr['t0']) / max(1e-6, Wr['t_end'] - Wr['t0']))
            L = Wr['L0'] + (Wr['L1'] - Wr['L0']) * u
            if dist > L and dist > 1e-6:
                n = rel / dist
                O.p = B.p + n * L
                vr = float(O.v @ n)
                if vr > 0:
                    sp = float(np.linalg.norm(O.v))
                    v = O.v - n * vr
                    if float(np.hypot(*v)) < 1e-6:
                        v = np.array([-n[1], n[0]])
                    O.v = v / float(np.hypot(*v)) * sp
            return
        mult = self.ab_mult(k)
        smax = w['spin_max'] * mult
        if B.haste > 0 and w.get('haste_snap'):
            B.spin = smax
        elif B.spin < smax:
            B.spin = min(smax, B.spin + w['spin_acc'] * mult * dt)
        else:
            B.spin = max(smax, B.spin - w['spin_acc'] * dt)
        B.th = (B.th + B.spin_dir * B.spin * dt) % (2 * math.pi)
        B.cool = max(0.0, B.cool - dt)
        if B.cool > 0:
            return
        hd = np.array([math.cos(B.th), math.sin(B.th)])
        head = B.p + (R + B.blade_len) * hd
        rel = O.p - head
        dist = float(np.hypot(*rel))
        if dist < R + w['head_r']:
            hx, hy = (O.p - rel / max(dist, 1e-6) * R).tolist()
            B.cool = w.get('cooldown', 0.3)
            if O.shield > 0:
                self.ev('block', 1 - k, k, hx, hy, 'flail')
                B.spin_dir *= -1.0                                  # it bounces off the shield and swings back
                B.spin *= 0.5
                return
            mom = min(1.0, B.spin / w['spin_max'])
            m0 = w.get('dmg_min', 0.4)
            landed = self.damage(k, 1 - k, w['dmg'] * (m0 + (1 - m0) * mom), 'flail', hx, hy, int(round(10 * mom)))
            tang = np.array([-hd[1], hd[0]]) * B.spin_dir
            self.knock(1 - k, tang + 0.5 * hd, w.get('knock', 0.8) * (0.5 + 0.5 * mom))
            if landed:
                B.blade_len = min(w['max_len'], B.blade_len + w['grow'])
                nb = math.ceil(self.t / self.R.beat - 1e-6) + w.get('wrap_beats', 2) - 1
                t_end = max(self.t + 0.5 * self.R.beat, nb * self.R.beat)
                d0 = float(np.hypot(*(O.p - B.p)))
                B.wrap = dict(t0=self.t, t_end=t_end, L0=d0, L1=max(2 * R + 0.05, min(d0, w.get('reel', 1.6))))
                B.spin = 0.0
                self.ev('wrap', k, t_end, d0, float(B.wrap['L1']))

    # ------------------------------------------------------------------ #32 v2: oom-pah swallow (opt-in weapon kind 'swallow')
    def swallow_setup(self, k):
        B, O = self.balls[k], self.balls[1 - k]
        B.sw = None                                   # the current oom-pah: dict(t_in, t_pah, caught, ev_i) or None
        B.cannon = None                               # after a spit: dict(t_end, ev_i) until the enemy hits a rope
        d = O.p - B.p
        B.th = math.atan2(d[1], d[0]) % (2 * math.pi)            # the bell starts facing the enemy

    def swallow_beat(self, k, kb):
        """swallow: every `every` beats (haste divides it) the bell goes "oom": it inhales for `inhale` beats; "pah"
        comes `pah` beats after the oom (both scale with haste)"""
        B = self.balls[k]
        w = B.w
        m = self.ab_mult(k)
        if kb % max(1, int(round(w['every'] / m))) == 0 and (B.sw is None or B.sw['done']):
            B.sw = dict(t0=self.t, t_in=self.t + w['inhale'] * self.R.beat / m, t_pah=self.t + w['pah'] * self.R.beat / m,
                        caught=False, done=False)
            self.ev('oom', k, B.sw['t_in'], B.sw['t_pah'], float(B.blade_len))

    def swallow_step(self, k, dt):
        """swallow: the bell turns toward the enemy (`aim_rate`). While it inhales, an enemy whose surface is within the
        suction reach (`blade_len`) is steered hard toward the bell (`suck`, units/s^2; its speed is kept); touching the
        bell (`catch` margin) it is SWALLOWED (`dmg`): it rides inside the tuba, held, until "pah" spits it out like a
        cannonball (`dmg_spit`, speed boost `spit_boost`) toward the open ring (seeded `jitter` deg); if it hits a rope
        within `spit_t` s it takes `dmg_rope`. A swallow that lands lengthens the reach (`grow`, up to `max_reach`)."""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        R = self.R.R
        rel = O.p - B.p
        lim = w.get('aim_rate', 6.0) * dt
        B.th = (B.th + max(-lim, min(lim, _wrap(math.atan2(rel[1], rel[0]) - B.th)))) % (2 * math.pi)
        if B.cannon is not None:                         # the spat enemy: a rope hit while it flies is the cannonball
            for e in self.events[B.cannon['ev_i']:]:
                if e[1] == 'rope' and e[2] == 1 - k:
                    self.damage(k, 1 - k, w['dmg_rope'], 'cannon', e[5], e[6])
                    B.cannon = None
                    break
            else:
                B.cannon['ev_i'] = len(self.events)
                if self.t > B.cannon['t_end']:
                    B.cannon = None
        S = B.sw
        if S is None or S['done']:
            return
        if S['caught']:
            O.p = B.p.copy()                             # inside the tuba
        elif self.t < S['t_in']:
            dist = float(np.hypot(*rel))
            if dist - 2 * R < B.blade_len and dist > 1e-6:
                sp = float(np.linalg.norm(O.v))
                v = O.v - rel / dist * w['suck'] * dt
                O.v = v / (np.linalg.norm(v) + 1e-12) * sp
            if dist < 2 * R + w.get('catch', 0.12):
                hx, hy = (B.p + rel / max(dist, 1e-6) * R).tolist()
                if O.shield > 0:
                    self.ev('block', 1 - k, k, hx, hy, 'swallow')
                    S['t_in'] = self.t
                else:
                    landed = self.damage(k, 1 - k, w['dmg'], 'gulp', hx, hy)
                    if landed:
                        S['caught'] = True
                        O.held = True
                        O.p = B.p.copy()
                        B.blade_len = min(w['max_reach'], B.blade_len + w['grow'])
                        self.ev('gulp', k, hx, hy, float(B.blade_len))
        if self.t + 1e-9 >= S['t_pah']:
            S['done'] = True
            if not S['caught']:
                self.ev('pah', k, 0, float(B.th))
                return
            to_open = -B.p if float(np.hypot(*B.p)) > 0.3 else np.array([math.cos(B.th), math.sin(B.th)])
            a = math.atan2(to_open[1], to_open[0]) + math.radians(self.rng.uniform(-1, 1) * w.get('jitter', 25.0))
            d = np.array([math.cos(a), math.sin(a)])
            lim_ = self.R.A - R - 1e-3
            O.p = np.clip(B.p + d * (2 * R + 0.02), -lim_, lim_)
            O.v = d * self.R.V
            O.boost = max(O.boost, w['spit_boost'])
            O.held = False
            self.ev('pah', k, 1, a)
            self.damage(k, 1 - k, w['dmg_spit'], 'spit', float(O.p[0]), float(O.p[1]))
            B.cannon = dict(t_end=self.t + w.get('spit_t', 0.9), ev_i=len(self.events))

    # ------------------------------------------------------------------ #32 v2: snake charmer (opt-in weapon kind 'striker')
    def striker_setup(self, k):
        B = self.balls[k]
        B.baskets = []                                # dict(id, home (2,), cp (2,), t_drop, strike or None)
        B.next_bk = 0
        B.dance_until = -1.0

    def striker_beat(self, k, kb):
        """striker: every `drop_every` beats the fighter drops a basket where it stands (up to `max_baskets`; the
        oldest is taken back for a new one); its cobra rises over `rise` beats"""
        B = self.balls[k]
        w = B.w
        if kb % int(w['drop_every']) == 0:
            gone = -1
            if len(B.baskets) >= int(w['max_baskets']):
                gone = B.baskets.pop(0)['id']
            bk = dict(id=B.next_bk, home=B.p.copy(), cp=B.p.copy(), t_drop=self.t, strike=None)
            B.next_bk += 1
            B.baskets.append(bk)
            self.ev('basket', k, bk['id'], float(B.p[0]), float(B.p[1]), gone)

    def striker_fire(self, k):
        """on the strike grid (every `every` beats, haste divides it): every risen cobra with the enemy in range lunges
        at it (reach `blade_len` from the cobra to the enemy's surface)"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        R = self.R.R
        for bk in B.baskets:
            if self.t < bk['t_drop'] + w['rise'] * self.R.beat - 1e-9 or bk['strike'] is not None:
                continue
            rel = O.p - bk['cp']
            dist = float(np.hypot(*rel))
            if dist - R > B.blade_len + w.get('slack', 0.25):
                continue
            a = math.atan2(rel[1], rel[0])
            ln = min(B.blade_len + R * 0.4, max(0.2, dist - R * 0.6))
            tk = B.t_strike / self.ab_mult(k)
            bk['strike'] = dict(t0=self.t, t_hit=self.t + tk, t_end=self.t + tk * 2.6, aim=a, len=ln, done=False)
            self.ev('strike', k, bk['id'], a, ln, tk)

    def striker_step(self, k, dt):
        """striker: a lunge reaches its length after `t_strike` s (shorter by `speed_grow` per landed bite, down to
        `t_min`) and bites if the enemy touches the fang line (capsule, `head_r`): `dmg`, `knock`; a landed bite grows
        the reach (`grow`, up to `max_reach`). During SNAKE DANCE (ability kind 'dance') the cobras leave their baskets
        and slither after the enemy (`slither` units/s), then slither home."""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        R = self.R.R
        dancing = self.t < B.dance_until
        for bk in B.baskets:
            tgt = O.p if dancing else bk['home']
            d = tgt - bk['cp']
            dist = float(np.hypot(*d))
            stop = (2 * R + 0.1) if dancing else 0.0
            if dist > stop + 1e-6 and bk['strike'] is None:
                sp = w['slither'] * (1.0 if dancing else 1.6) * dt
                bk['cp'] = bk['cp'] + d / dist * min(sp, dist - stop)
            S = bk['strike']
            if S is None:
                continue
            if not S['done'] and self.t + 1e-9 >= S['t_hit']:
                S['done'] = True
                u = np.array([math.cos(S['aim']), math.sin(S['aim'])])
                a0 = bk['cp']
                q = O.p - a0
                c = a0 + u * min(max(float(q @ u), 0.0), S['len'])
                dd = float(np.hypot(*(O.p - c)))
                if dd < R + w['head_r']:
                    hx, hy = (O.p + (c - O.p) / max(dd, 1e-6) * R).tolist()
                    if O.shield > 0:
                        self.ev('block', 1 - k, k, hx, hy, 'fang')
                    else:
                        landed = self.damage(k, 1 - k, w['dmg'], 'fang', hx, hy, bk['id'])
                        self.knock(1 - k, u, w['knock'])
                        if landed:
                            B.blade_len = min(w['max_reach'], B.blade_len + w['grow'])
                            B.t_strike = max(w['t_min'], B.t_strike - w.get('speed_grow', 0.0))
            if self.t >= S['t_end']:
                bk['strike'] = None

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
    # ------------------------------------------------------------------ #30 weapon kinds on the 16th grid: aura, bellows
    def grid16_fire(self, k, k16):
        """called on every 16th of the fight for 'aura' / 'bellows' fighters: fires on the weapon's own grid (`every`
        beats), `haste` divides the step (DOUBLE DRONE ticks twice as often, POLKA strokes every beat)"""
        B = self.balls[k]
        w = B.w
        if w['kind'] == 'bite' and getattr(B, 'bite', None) is not None and self.t + self.R.DT >= B.bite['t_snap']:
            self.bite_snap(k)                                  # #30 v2: the jaws close on a 16th
        main = max(1, int(round(4 * w['every'])))
        step16 = max(1, int(round(main / self.ab_mult(k))))
        if k16 % step16:
            return
        if w['kind'] == 'aura':
            self.aura_tick(k)
        elif w['kind'] == 'inflate':                          # #30 v2
            self.inflate_start(k)
        elif w['kind'] == 'bite':                             # #30 v2
            if getattr(B, 'bite', None) is None:
                self.bite_fire(k, k16 % main == 0)
        else:
            self.fire_bellows(k, k16 % main == 0)

    def aura_tick(self, k):
        """the drone aura (#30 BAGPIPES): a ring of radius R + reach around the ball hurts the opponent on every tick
        while it is inside (its centre within the ring + half a ball). The reach grows by `grow` on the first landed
        tick of each contact (entering the aura), up to `max_reach`. No knock: the drone doesn't push. The hit's `sub`
        field is 1 on a contact's first tick."""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        R = self.R.R
        rel = O.p - B.p
        dist = float(np.hypot(*rel))
        inside = dist - 0.5 * R < R + B.blade_len
        was = getattr(B, 'aura_in', False)
        B.aura_in = inside
        if not inside:
            return
        hx, hy = (B.p + rel / max(dist, 1e-6) * max(R, dist - R)).tolist()
        landed = self.damage(k, 1 - k, w['dmg'], 'aura', hx, hy, int(not was))
        if landed and not was:
            B.blade_len = min(w['max_reach'], B.blade_len + w['grow'])

    def fire_bellows(self, k, main):
        """the bellows (#30 ACCORDION): strokes alternate push / pull, starting with a push. A push is a blast ring out
        from the ball to its reach (hits once: damage + knock away, through the shared wave code); a pull is a suction
        ring collapsing from `pull_k` x the reach back onto the ball (hits once: damage + a yank toward the accordion).
        Main strokes (the weapon's own grid) grow the reach when they land; the knock grows with the reach
        (`force_grow`); haste strokes hit for `sub_dmg` x, knock at 0.6 x and don't grow it."""
        B = self.balls[k]
        w = B.w
        n = getattr(B, 'bel_n', 0)
        B.bel_n = n + 1
        push = n % 2 == 0
        f = 1.0 + w.get('force_grow', 0.0) * (B.blade_len - w['base'])
        dmg = w['dmg'] * (1.0 if main else w.get('sub_dmg', 0.5))
        knock = w['knock'] * f * (1.0 if main else 0.6)
        if push:
            self.waves.append(dict(owner=k, c=B.p.copy(), r=self.R.R, speed=w['speed'], rmax=self.R.R + B.blade_len,
                                   dmg=dmg, knock=knock, kind='bellows', hit=False, name='push', t0=self.t, grow=main,
                                   clears=False))
        else:
            r0 = self.R.R + B.blade_len * w.get('pull_k', 1.0)
            self.sucks.append(dict(owner=k, c=B.p.copy(), r=r0, rmax=r0, speed=w['speed'] * w.get('pull_speed', 1.0),
                                   dmg=dmg, knock=knock, hit=False, t0=self.t, grow=main))
        self.ev('bellows', k, int(main), int(push), float(B.blade_len), float(B.p[0]), float(B.p[1]))

    def suction_step(self, dt):
        """#30: collapsing suction rings; the hit yanks the opponent toward the accordion's current position"""
        R = self.R.R
        for W in self.sucks:
            r0 = W['r']
            W['r'] = max(R, r0 - W['speed'] * dt)
            o = 1 - W['owner']
            O, B = self.balls[o], self.balls[W['owner']]
            rel = O.p - W['c']
            dist = float(np.hypot(*rel))
            if not W['hit'] and W['r'] <= dist - R * 0.5 < r0:
                W['hit'] = True
                hx, hy = (W['c'] + rel / max(dist, 1e-6) * (dist - R)).tolist()
                landed = self.damage(W['owner'], o, W['dmg'], 'suction', hx, hy)
                self.knock(o, B.p - O.p, W['knock'])
                if landed and W['grow']:
                    B.blade_len = min(B.w['max_reach'], B.blade_len + B.w['grow'])
        self.sucks = [W for W in self.sucks if W['r'] > R + 1e-9]

    # ------------------------------------------------------------------ weapon kind 'tether' (#31 v2: PATCH CABLES)
    def _tether(self, B):
        if not hasattr(B, 'tc'):
            B.tc = dict(plugs=[], cables=[], volts=[], next=0, slots=float(B.w['n0']), pulse=0)
            B.n_vol = B.tc['slots']
        return B.tc

    def tether_beat(self, k, kb):
        """every `every` beats, while fewer cables are plugged in (or flying) than its slots, the synth fires a plug at the
        opponent (straight, `plug_speed`)"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        tc = self._tether(B)
        if kb % w['every'] or len(tc['cables']) + len(tc['plugs']) >= int(tc['slots'] + 1e-9):
            return
        d = O.p - B.p
        a = math.atan2(d[1], d[0])
        u = np.array([math.cos(a), math.sin(a)])
        pos = B.p + (self.R.R + 0.06) * u
        tc['plugs'].append(dict(id=tc['next'], p=pos, v=w['plug_speed'] * u))
        self.ev('plug_fire', k, tc['next'], float(pos[0]), float(pos[1]), a, float(w['plug_speed']))
        tc['next'] += 1

    def tether_16(self, k, k16):
        """on the 16th grid, every `volt16` 16ths (haste divides it): a voltage pulse leaves the synth down every plugged
        cable (one arpeggio note); it arrives after cable length / `volt_speed`"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        tc = self._tether(B)
        if not tc['cables'] or k16 % max(1, int(round(w['volt16'] / self.ab_mult(k)))):
            return
        dist = float(np.hypot(*(O.p - B.p)))
        for C in tc['cables']:
            tc['volts'].append(dict(cid=C['id'], t_arr=self.t + dist / w['volt_speed'], n=tc['pulse']))
            self.ev('volt', k, C['id'], tc['pulse'])
        tc['pulse'] += 1

    def tether_step(self, k, dt):
        """plugs fly; a plug that reaches the opponent jabs it (`plug_dmg`) and plugs a cable in for `life_beats` beats
        (+`grow` slots per landed plug, up to `n_max`); pulses that arrive while their cable is still in hit for `volt_dmg`.
        The leash: while any cable is in, the opponent can't get farther than `length` from the synth (it bounces off the
        end of its leash), unless it is flung harder than `snap` x the cables plugged in: then they all snap.
        `n_vol` holds the slots (recorded)"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        tc = self._tether(B)
        A = self.R.A
        rO = getattr(O, 'rad', self.R.R)
        ghost = getattr(O, 'rw_left', 0.0) > 0                  # a rewinding opponent is intangible
        for P in list(tc['plugs']):
            P['p'] = P['p'] + P['v'] * dt
            if np.abs(P['p']).max() > A:
                tc['plugs'].remove(P)
                self.ev('plug_miss', k, P['id'], float(P['p'][0]), float(P['p'][1]))
            elif not ghost and float(np.hypot(*(O.p - P['p']))) < rO + w.get('plug_r', 0.14):
                tc['plugs'].remove(P)
                x, y = P['p'].tolist()
                if self.damage(k, 1 - k, w['plug_dmg'], 'plug', x, y, P['id']):
                    tc['cables'].append(dict(id=P['id'], t1=self.t + w['life_beats'] * self.R.beat))
                    tc['slots'] = min(w['n_max'], tc['slots'] + w['grow'])
                    B.n_vol = tc['slots']
                    self.ev('plug', k, P['id'], x, y)
        for C in list(tc['cables']):
            if self.t >= C['t1']:
                tc['cables'].remove(C)
                self.ev('unplug', k, C['id'], 0)
        live = {C['id'] for C in tc['cables']}
        for V in list(tc['volts']):
            if self.t + 1e-9 >= V['t_arr']:
                tc['volts'].remove(V)
                if V['cid'] in live and not ghost:
                    self.damage(k, 1 - k, w['volt_dmg'], 'volt', float(O.p[0]), float(O.p[1]), V['n'])
        if tc['cables'] and not ghost:
            rel = O.p - B.p
            dist = float(np.hypot(*rel))
            if dist > w['length']:
                n = rel / dist
                if O.boost > w['snap'] * len(tc['cables']):
                    self.ev('snap', k, len(tc['cables']), float(O.p[0]), float(O.p[1]))
                    for C in tc['cables']:
                        self.ev('unplug', k, C['id'], 1)
                    tc['cables'], tc['volts'] = [], []
                else:
                    O.p = B.p + n * w['length']
                    vn = float(O.v @ n)
                    if vn > 0:
                        O.v = O.v - 2 * vn * n
                        self.ev('leash', k, float(O.p[0]), float(O.p[1]))

    # ------------------------------------------------------------------ ability kind 'lash' (#31 v2 SAWTOOTH)
    def start_lash(self, k, ab):
        """SAWTOOTH: the cables go saw-wave and lash side to side, cutting. For `beats` beats, n arms (the tether's slots,
        at least `n_min`) of length `len` reach out of the ball around the aim at the opponent (`spread` deg apart) and
        sweep across `sweep` deg in a saw-tooth (a linear sweep every `period_beats`, then a snap back; arm j lags by
        `lag` of a period); a forward sweep that crosses the opponent cuts (`dmg`, `knock`), each arm resting `cool` s"""
        B, O = self.balls[k], self.balls[1 - k]
        tc = getattr(B, 'tc', None)
        n = max(ab.get('n_min', 1), int(tc['slots'] + 1e-9) if tc else 0)
        d = O.p - B.p
        aim = math.atan2(d[1], d[0])
        L = dict(owner=k, t0=self.t, t1=self.t + ab['beats'] * self.R.beat, aim=aim, n=n, len=ab['len'],
                 spread=math.radians(ab['spread']), sweep=math.radians(ab['sweep']),
                 period=ab['period_beats'] * self.R.beat, lag=ab.get('lag', 0.15), dmg=ab['dmg'], knock=ab['knock'],
                 cool=ab['cool'], rest=[-9.0] * n, prev=[None] * n)
        if not hasattr(self, 'lashes'):
            self.lashes = []
        self.lashes.append(L)
        self.ev('lash', k, aim, n, float(ab['len']), float(L['t1'] - L['t0']), float(L['period']),
                float(L['sweep']), float(L['spread']), float(L['lag']))

    @staticmethod
    def lash_angle(L, j, t):
        """arm j's angle at time t (the renderer uses the same function)"""
        ph = ((t - L['t0']) / L['period'] + j * L['lag']) % 1.0
        return L['aim'] + (j - (L['n'] - 1) / 2) * L['spread'] + L['sweep'] * (ph - 0.5), ph

    def lash_step(self, dt):
        for L in self.lashes:
            k = L['owner']
            B, O = self.balls[k], self.balls[1 - k]
            rO = getattr(O, 'rad', self.R.R)
            rel = O.p - B.p
            dist = float(np.hypot(*rel))
            phi = math.atan2(rel[1], rel[0])
            for j in range(L['n']):
                a, ph = self.lash_angle(L, j, self.t)
                pa = L['prev'][j]
                L['prev'][j] = (a, ph)
                if pa is None or ph < pa[1] or self.t < L['rest'][j] + L['cool']:
                    continue                                        # the snap back doesn't cut
                if getattr(O, 'rw_left', 0.0) > 0 or dist - rO > L['len'] + self.R.R:
                    continue
                half = math.asin(min(1.0, rO / max(dist, 1e-6)))
                if _swept(pa[0], a - pa[0], phi, half):
                    L['rest'][j] = self.t
                    hx, hy = (B.p + rel / max(dist, 1e-6) * (dist - rO)).tolist()
                    self.damage(k, 1 - k, L['dmg'], 'lash', hx, hy, j)
                    side = np.array([-math.sin(a), math.cos(a)])
                    self.knock(1 - k, side + 0.3 * rel / max(dist, 1e-6), L['knock'])
        self.lashes = [L for L in self.lashes if self.t < L['t1']]

    # ------------------------------------------------------------------ weapon kind 'scratch_floor' (#31 v2: THE RING IS THE RECORD)
    def scratch_beat(self, k, kb):
        """every `every` beats (haste divides it) the DJ scratches the record the ring sits on: over `dur_beats` the floor
        jerks forward, then back (directions alternate scratch to scratch). The DJ's own ball is anchored by its needle;
        the opponent is flung by each jerk (scratch_step). `blade_len` holds the scratch strength (so it is recorded)"""
        B = self.balls[k]
        w = B.w
        if not hasattr(B, 'sf'):
            B.sf = dict(n=0, pending=[], live=-1.0, hit=True, seen=len(self.events))
            B.blade_len = w['base']
        ev = max(1, int(round(w['every'] / self.ab_mult(k))))
        if kb % ev:
            return
        d = w['dur_beats'] * self.R.beat
        sgn = 1.0 if B.sf['n'] % 2 == 0 else -1.0
        B.sf['n'] += 1
        B.sf['pending'] = [(self.t, sgn), (self.t + d / 2, -sgn)]
        self.ev('scratch', k, d, float(B.blade_len), sgn)

    def scratch_step(self, k, dt):
        """each jerk flings the opponent around the ring centre (tangent in the jerk's direction + `out` x outward), with
        a knock of the current strength; the first rope it hits within `window` s is a rope slam: `dmg` x impact speed /
        the shared speed (capped at `cap`); a landed slam adds `grow` strength up to `max`"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        sf = getattr(B, 'sf', None)
        if sf is None:
            return
        while sf['pending'] and self.t + 1e-9 >= sf['pending'][0][0]:
            t0, sgn = sf['pending'].pop(0)
            if getattr(O, 'rw_left', 0.0) > 0:                 # a rewinding opponent is intangible
                continue
            rn = float(np.hypot(*O.p))
            outw = O.p / rn if rn > 1e-6 else O.v / (np.linalg.norm(O.v) + 1e-12)
            tang = np.array([-outw[1], outw[0]]) * sgn
            self.knock(1 - k, tang + w.get('out', 0.4) * outw, B.blade_len)
            sf['live'] = self.t + w['window']
            sf['hit'] = False
            self.ev('jerk', k, sgn, float(O.p[0]), float(O.p[1]), float(B.blade_len))
        if not sf['hit'] and self.t <= sf['live']:
            for e in self.events[sf['seen']:]:
                if e[1] == 'rope' and e[2] == 1 - k:
                    sf['hit'] = True
                    dmg = w['dmg'] * min(w.get('cap', 1.6), e[7] / self.R.V)
                    if self.damage(k, 1 - k, dmg, 'scratch', e[5], e[6]):
                        B.blade_len = min(w['max'], B.blade_len + w['grow'])
                    break
        sf['seen'] = len(self.events)

    # ------------------------------------------------------------------ ability kind 'rewind' (#31 REWIND)
    # ------------------------------------------------------------------ #30 v2: weapon kind 'inflate' (BAG INFLATE)
    def inflate_setup(self, k):
        """per-ball size / mass / speed for an 'inflate' fighter (read by move() through getattr defaults)"""
        B = self.balls[k]
        w = B.w
        B.rad, B.mass, B.vmul = self.R.R, 1.0, 1.0
        B.inf = None                                   # the running cycle: dict(phase, t0, t1, dur_i, dur_f, s, ...)
        B.inf_scale = w['scale0']                      # inflated size (x R), +`grow` per landed hit
        B.inf_fly = w['fly_beats']                     # flight length (beats), +`fly_grow` per landed hit
        B.inf_cool = 0.0
        B.blade_len = B.inf_scale                      # recorded in BL: the signature's current size
        if not hasattr(self, 'clash_hooks'):
            self.clash_hooks = []
        self.clash_hooks.append(lambda k=k: self.inflate_clash(k))

    def inflate_start(self, k):
        """a cycle starts on the weapon's grid: the bag swells for `inflate_beats` (heavy, slow, body-slams), then lets
        go and rockets around like a released balloon for the flight length (haste divides both); a cycle still
        running is never cut short (the next start is skipped)"""
        B = self.balls[k]
        w = B.w
        if B.inf is not None:
            return
        mult = self.ab_mult(k)
        dur_i = w['inflate_beats'] * self.R.beat / mult
        fly = min(B.inf_fly, w['every'] - w['inflate_beats'] - w.get('rest_beats', 0.25))
        dur_f = max(0.25, fly) * self.R.beat / mult
        B.inf = dict(phase='inflate', t0=self.t, dur_i=dur_i, dur_f=dur_f, s=B.inf_scale)
        self.ev('inflate', k, float(B.inf_scale), dur_i, dur_f)

    def inflate_step(self, k, dt):
        """advance the cycle: swell (radius, mass up, speed down), release (aimed at the enemy, seeded wobble), the
        zig-zag flight (sharp seeded turns, alternating sides, a little homing), deflating back to R"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        R = self.R.R
        B.inf_cool = max(0.0, B.inf_cool - dt)
        C = B.inf
        if C is None:
            return
        s = C['s']
        if C['phase'] == 'inflate':
            u = min(1.0, (self.t - C['t0']) / C['dur_i'])
            e = u * u * (3 - 2 * u)
            B.rad = R * (1 + (s - 1) * e)
            B.mass = 1 + (s - 1) * e * w.get('mass_k', 1.0)
            B.vmul = 1 - (1 - w['slow']) * e
            if u >= 1.0:
                d = O.p - B.p
                a = math.atan2(d[1], d[0]) + math.radians(self.rng.uniform(-1, 1) * w.get('aim_wobble', 20.0))
                B.v = np.array([math.cos(a), math.sin(a)]) * self.R.V
                B.vmul = w['fly_speed']
                C.update(phase='fly', t1=self.t, zig_t=self.t + self.rng.uniform(*w['zig']), zig_s=1.0 if self.rng.random() < 0.5 else -1.0)
                self.ev('release', k, float(s), C['dur_f'], float(B.p[0]), float(B.p[1]))
            return
        u = min(1.0, (self.t - C['t1']) / C['dur_f'])
        f = (1 - u) ** 1.5
        B.rad = R * (1 + (s - 1) * f)
        B.mass = 1 + (s - 1) * f * w.get('mass_k', 1.0)
        B.vmul = w['fly_speed'] * (1 - w.get('fly_fade', 0.35) * u)
        if self.t >= C['zig_t'] and u < 1.0:
            C['zig_s'] = -C['zig_s']
            turn = math.radians(self.rng.uniform(*w['zig_turn'])) * C['zig_s']
            c_, s_ = math.cos(turn), math.sin(turn)
            v = B.v / (np.linalg.norm(B.v) + 1e-12)
            v = np.array([c_ * v[0] - s_ * v[1], s_ * v[0] + c_ * v[1]])
            d = O.p - B.p
            d = d / (np.linalg.norm(d) + 1e-12)
            hm = w.get('home', 0.3)
            v = (1 - hm) * v + hm * d
            B.v = v / (np.linalg.norm(v) + 1e-12) * self.R.V
            C['zig_t'] = self.t + self.rng.uniform(*w['zig'])
        if u >= 1.0:
            B.rad, B.mass, B.vmul = R, 1.0, 1.0
            B.inf = None
            self.ev('land', k, float(B.p[0]), float(B.p[1]))

    def inflate_clash(self, k):
        """contact damage after a clash: a body slam while swollen (scaled by how big it is), a flight hit while it
        rockets; landed hits grow the inflated size and the flight length"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        C = B.inf
        if C is None or B.inf_cool > 0:
            return
        R = self.R.R
        s = C['s']
        if C['phase'] == 'inflate':
            big = (B.rad / R - 1) / max(1e-6, s - 1)
            if big < w.get('slam_min', 0.3):
                return
            dmg, how, kn = w['slam_dmg'] * big, 'bodyslam', w['knock'] * 0.8
        else:
            dmg, how, kn = w['fly_dmg'], 'flight', w['knock']
        rel = O.p - B.p
        dist = float(np.hypot(*rel))
        hx, hy = (B.p + rel / max(dist, 1e-6) * B.rad).tolist()
        landed = self.damage(k, 1 - k, dmg, how, hx, hy)
        self.knock(1 - k, rel, kn)
        B.inf_cool = w['cool']
        if landed:
            B.inf_scale = min(w['scale_max'], B.inf_scale + w['grow'])
            B.inf_fly = min(w['fly_max'], B.inf_fly + w['fly_grow'])
            B.blade_len = B.inf_scale

    # ------------------------------------------------------------------ #30 v2: weapon kind 'bite' (BELLOWS BITE)
    def bite_fire(self, k, main):
        """the two halves fly apart with the bellows stretched between them like jaws, aimed (and locked) at where the
        enemy will be when they snap shut `open16` 16ths later"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        t_open = w.get('open16', 2) * self.R.beat / 4.0
        tgt = O.p + O.v * t_open * w.get('lead', 0.6)
        d = tgt - B.p
        aim = math.atan2(d[1], d[0])
        B.bite = dict(t0=self.t, t_snap=self.t + t_open, aim=aim, reach=float(B.blade_len), main=main)
        self.ev('bite', k, int(main), aim, float(B.blade_len), t_open)

    def bite_snap(self, k):
        """the jaws snap shut: a hit if the enemy is inside the mouth (within the reach and the jaws' half-angle of the
        locked aim, measured from where the accordion is now); main bites that land grow the reach"""
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        Bt = B.bite
        B.bite = None
        R = self.R.R
        ro = getattr(O, 'rad', R)
        rel = O.p - B.p
        dist = float(np.hypot(*rel))
        phi = math.atan2(rel[1], rel[0])
        half = math.radians(w['half']) + math.asin(min(1.0, ro / max(dist, 1e-6)))
        inside = dist - ro <= R + Bt['reach'] and abs(_wrap(phi - Bt['aim'])) <= half
        landed = False
        if inside:
            hx, hy = (B.p + rel / max(dist, 1e-6) * (dist - ro)).tolist()
            landed = self.damage(k, 1 - k, w['dmg'] * (1.0 if Bt['main'] else w.get('sub_dmg', 0.5)), 'bite', hx, hy,
                                 int(Bt['main']))
            self.knock(1 - k, rel, w['knock'] * (1.0 if Bt['main'] else 0.6))
            if landed and Bt['main']:
                B.blade_len = min(w['max_reach'], B.blade_len + w['grow'])
        self.ev('snap', k, int(inside), int(landed), Bt['aim'], Bt['reach'])

    # ------------------------------------------------------------------ #30 v2: ability kind 'clamp' (FULL BELLOWS)
    def clamp_start(self, k, ab):
        """the bellows stretch across the whole ring (the halves fly to the ropes on either side of the accordion, along
        the line to the enemy) for `tele` beats, then clamp: a ring-wide bite that lands unless the enemy is shielded"""
        B, O = self.balls[k], self.balls[1 - k]
        d = O.p - B.p
        axis = math.atan2(d[1], d[0])
        t_snap = self.t + ab['tele'] * self.R.beat
        self.clamps.append(dict(owner=k, t0=self.t, t_snap=t_snap, axis=axis, dmg=ab['dmg'], knock=ab['knock'], done=False))
        self.ev('clamp', k, axis, t_snap - self.t, float(B.p[0]), float(B.p[1]))

    def clamps_step(self):
        for C in self.clamps:
            if C['done'] or self.t + 1e-9 < C['t_snap']:
                continue
            C['done'] = True
            k = C['owner']
            B, O = self.balls[k], self.balls[1 - k]
            landed = self.damage(k, 1 - k, C['dmg'], 'clamp', float(O.p[0]), float(O.p[1]))
            self.knock(1 - k, B.p - O.p, C['knock'])            # crushed and dragged toward the accordion
            self.ev('clamp_snap', k, int(landed), float(O.p[0]), float(O.p[1]))
        self.clamps = [C for C in self.clamps if not C['done']]

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
    # ------------------------------------------------------------------ #34 v2: weapon kind 'pack' (THE HUNT)
    # A horn call every `every` beats brands the enemy (a mark lasting `mark_beats`) and releases one hound from the bell
    # while the pack is under its size (`n0`, + `grow` per landed bite, up to `n_max`). Hounds (radius `r`) chase the
    # marked enemy (speed `speed`, turn rate `turn`; haste speeds them up), keep apart, and bite on the 8th grid when in
    # reach (`dmg`, `knock`), then recoil and wait `cool_beats`. Unmarked, they heel behind the horn; while the horn is
    # shielded they circle it at `guard_r` (and still bite whatever comes close). The enemy's blade (spinning, or a live
    # thrust lance), its clearing waves and its slams knock hounds out; the next call releases a new one. Ability kind
    # 'charge' (TALLY-HO): the pack, filled to `n_max` for the charge, lines up at the bell and charges across the ring as
    # one front (`speed`, `width`; one hit: `dmg`, `knock`); the extra hounds go back into the bell afterwards.
    HOUND_MODES = {'hunt': 0, 'heel': 1, 'guard': 2, 'charge': 3}
    HMAX = 8

    def pack_setup(self, k):
        B = self.balls[k]
        B.hounds, B.hound_id, B.calls = [], 0, 0
        B.n_pack = float(B.w.get('n0', 1))
        for b in self.balls:
            b.mark = 0.0
        self.pack_charges = []

    def _spawn_hound(self, k, mode='hunt'):
        """a hound pours out of the bell, out to one side of the line to the enemy (alternating sides), so the pack fans out"""
        B, O = self.balls[k], self.balls[1 - k]
        d = O.p - B.p
        a = math.atan2(d[1], d[0]) + (1 if B.hound_id % 2 == 0 else -1) * math.radians(B.w.get('fan', 70.0))
        p = B.p + (self.R.R + B.w['r']) * np.array([math.cos(a), math.sin(a)])
        h = dict(id=B.hound_id, p=p, a=a, mode=mode, cool=0.0, recoil=0.0)
        B.hounds.append(h)
        self.ev('hound', k, B.hound_id, float(p[0]), float(p[1]))
        B.hound_id += 1
        return h

    def pack_call(self, k):
        """the horn call: brand the enemy, release a hound if the pack is under its size"""
        B, O = self.balls[k], self.balls[1 - k]
        if any(C['owner'] == k for C in self.pack_charges):
            return
        O.mark = B.w['mark_beats'] * self.R.beat
        B.calls += 1
        rel = 0
        if len(B.hounds) < int(B.n_pack + 1e-9):
            self._spawn_hound(k)
            rel = 1
        self.ev('call', k, B.calls, rel, len(B.hounds))

    def _hound_down(self, k, h, why):
        self.balls[k].hounds.remove(h)
        self.ev('hound_down', k, h['id'], float(h['p'][0]), float(h['p'][1]), why)

    def pack_hazards(self, k, dt):
        """the enemy's weapons knock hounds out: its blade (spinning, or a thrust lance while it is going out / held), its
        clearing waves passing over them, its slams landing on them"""
        B, O = self.balls[k], self.balls[1 - k]
        o, R, rh = 1 - k, self.R.R, B.w['r']
        seg = None
        if O.w['kind'] == 'blade':
            d = np.array([math.cos(O.th), math.sin(O.th)])
            if O.w.get('mode') == 'thrust':
                if O.thr is not None and (self.t - O.thr) / O.thr_k < O.w['t_out'] + O.w['t_hold']:
                    seg = (O.p + d * R * 0.9, d, R * 0.1 + O.ext)
            else:
                seg = (O.p + d * R * 0.9, d, R * 0.1 + O.blade_len)
        for h in list(B.hounds):
            if h['mode'] == 'charge':
                continue
            if seg is not None:
                a, d, L_ = seg
                rel = h['p'] - a
                s_ = min(max(float(rel @ d), 0.0), L_)
                if float(np.hypot(*(rel - d * s_))) < rh + 0.05:
                    self._hound_down(k, h, 'blade')
                    continue
            gone = False
            for W in self.waves:
                if W['owner'] == o and W.get('clears', True) and abs(float(np.hypot(*(h['p'] - W['c']))) - W['r']) < rh + 0.1:
                    self._hound_down(k, h, W['kind'])
                    gone = True
                    break
            if gone:
                continue
            for S in self.slams:
                if S['owner'] == o and S['done'] and self.t - S['t_slam'] < dt * 1.01 and \
                        float(np.hypot(*(h['p'] - S['c']))) < S['radius'] + rh:
                    self._hound_down(k, h, 'slam')
                    break

    def pack_step(self, k, dt, bite):
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        R, rh, A = self.R.R, w['r'], self.R.A
        O.mark = max(0.0, O.mark - dt)
        mult = self.ab_mult(k)
        guard = B.shield > 0
        live = [h for h in B.hounds if h['mode'] != 'charge']
        for i, h in enumerate(live):
            h['cool'] = max(0.0, h['cool'] - dt)
            h['recoil'] = max(0.0, h['recoil'] - dt)
            if guard:
                ang = self.t * w.get('guard_w', 3.2) + 2 * math.pi * i / len(live)
                tgt, mode = B.p + w['guard_r'] * np.array([math.cos(ang), math.sin(ang)]), 'guard'
            elif O.mark > 0:                                         # hunt: circle to a flank, then go in for the bite
                dist = float(np.hypot(*(O.p - h['p'])))
                pred = O.p + O.v * min(0.5, dist / (w['speed'] * mult)) * 0.6
                ph0 = math.atan2(B.p[1] - O.p[1], B.p[0] - O.p[0])
                ph = ph0 + (1 if h['id'] % 2 == 0 else -1) * (w.get('flank', 1.9) + 0.3 * (i // 2))
                fl = pred + (R + 0.8) * np.array([math.cos(ph), math.sin(ph)])
                tgt, mode = (pred if dist < R + 0.9 or float(np.hypot(*(fl - h['p']))) < 0.6 else fl), 'hunt'
            else:
                back = -B.v / (np.linalg.norm(B.v) + 1e-9)
                side = np.array([-back[1], back[0]]) * (0.32 * (i - (len(live) - 1) / 2))
                tgt, mode = B.p + back * (R + 0.45 + 0.2 * i) + side, 'heel'
            h['mode'] = mode
            rel = tgt - h['p']
            dist = float(np.hypot(*rel))
            if h['recoil'] <= 0:
                da = _wrap(math.atan2(rel[1], rel[0]) - h['a'])
                lim = w['turn'] * (1.8 if mode == 'guard' else 1.0) * min(2.0, mult) * dt
                h['a'] += max(-lim, min(lim, da))
            sp = w['speed'] * mult * (1.3 if h['recoil'] > 0 else 1.0)
            if mode != 'hunt':
                sp *= min(1.0, 0.25 + dist / 0.8)                  # ease in to the heel / guard spot
            v = sp * np.array([math.cos(h['a']), math.sin(h['a'])])
            for g in live:                                          # keep apart (a pack, not a pile)
                if g is not h:
                    d_ = h['p'] - g['p']
                    dd = float(np.hypot(*d_))
                    if 1e-9 < dd < 2.3 * rh:
                        v = v + d_ / dd * (2.3 * rh - dd) * 7.0
            h['p'] = h['p'] + v * dt
            for ax in (0, 1):                                       # the ropes
                lim_ = A - rh
                if abs(h['p'][ax]) > lim_:
                    h['p'][ax] = math.copysign(lim_, h['p'][ax])
                    c = np.array([math.cos(h['a']), math.sin(h['a'])])
                    c[ax] = -c[ax]
                    h['a'] = math.atan2(c[1], c[0])
            for ball in (O, B):                                     # never inside a ball
                d_ = h['p'] - ball.p
                dd = float(np.hypot(*d_))
                if dd < R + rh * 0.8:
                    h['p'] = ball.p + d_ / max(dd, 1e-9) * (R + rh * 0.8)
        if bite:                                                    # bites on the 8th grid
            for h in live:
                if h['cool'] > 0:
                    continue
                rel = O.p - h['p']
                dist = float(np.hypot(*rel))
                if dist < R + rh + 0.08:
                    hx, hy = (h['p'] + rel / max(dist, 1e-9) * rh).tolist()
                    if O.shield > 0:
                        self.ev('block', 1 - k, k, hx, hy, 'bite')
                    else:
                        landed = self.damage(k, 1 - k, w['dmg'], 'bite', hx, hy, h['id'])
                        self.knock(1 - k, rel, w['knock'])
                        if landed:
                            B.n_pack = min(w['n_max'], B.n_pack + w['grow'])
                    h['cool'] = w['cool_beats'] * self.R.beat
                    h['recoil'] = 0.22
                    h['a'] = math.atan2(-rel[1], -rel[0])
        self.pack_charge_step(k, dt)
        self.pack_hazards(k, dt)

    def start_charge(self, k, ab):
        """ability kind 'charge' (TALLY-HO): fill the pack to `n_max`, line it up at the bell, charge as one front"""
        B, O = self.balls[k], self.balls[1 - k]
        d = O.p - B.p
        u = d / (np.linalg.norm(d) + 1e-9)
        nrm = np.array([-u[1], u[0]])
        keep = [h['id'] for h in B.hounds]
        while len(B.hounds) < B.w['n_max']:
            self._spawn_hound(k, 'charge')
        n = len(B.hounds)
        for i, h in enumerate(B.hounds):
            h['mode'] = 'charge'
            h['slot'] = (i - (n - 1) / 2) * ab['width'] / max(1, n - 1)
        o = B.p.copy()
        self.pack_charges.append(dict(owner=k, o=o, u=u, nrm=nrm, s=self.R.R, speed=ab['speed'], width=ab['width'],
                                      hit=False, dmg=ab['dmg'], knock=ab['knock'], keep=keep, t0=self.t))
        self.ev('pack_charge', k, float(o[0]), float(o[1]), float(u[0]), float(u[1]), float(ab['width']))

    def pack_charge_step(self, k, dt):
        B, O = self.balls[k], self.balls[1 - k]
        R, A = self.R.R, self.R.A
        for C in [C for C in self.pack_charges if C['owner'] == k]:
            s0 = C['s']
            C['s'] = s0 + C['speed'] * dt
            for h in B.hounds:
                if h['mode'] == 'charge':
                    h['p'] = C['o'] + C['u'] * C['s'] + C['nrm'] * h['slot']
                    h['a'] = math.atan2(C['u'][1], C['u'][0])
            rel = O.p - C['o']
            along, lat = float(rel @ C['u']), abs(float(rel @ C['nrm']))
            if not C['hit'] and s0 < along - R * 0.5 <= C['s'] and lat < C['width'] / 2 + R:
                C['hit'] = True
                hx, hy = (O.p - C['u'] * R).tolist()
                self.damage(k, 1 - k, C['dmg'], 'charge', hx, hy)
                self.knock(1 - k, C['u'], C['knock'])
            front = C['o'] + C['u'] * C['s']
            if np.abs(front).max() > A + 0.3 or C['s'] > 4 * A:            # past the far ropes: the pack breaks up
                for h in list(B.hounds):
                    if h['mode'] != 'charge':
                        continue
                    h['p'] = np.clip(h['p'], -(A - B.w['r']), A - B.w['r'])
                    h['mode'], h['recoil'], h['cool'] = 'hunt', 0.0, 0.0
                    if h['id'] not in C['keep'] and len([g for g in B.hounds if g['id'] in C['keep'] or g is h]) > int(B.n_pack + 1e-9):
                        self._hound_down(k, h, 'home')
                self.pack_charges.remove(C)
                self.ev('pack_charge_end', k)

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
                                   knock=ab['knock'], kind=kind, hit=False, name=ab['name'], t0=self.t,
                                   slow=ab.get('slow')))
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
                                  vol=self.next_vol, curve=bool(ab.get('curve'))))
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
        elif kind == 'lash':                                      # #31 v2 SAWTOOTH
            self.start_lash(k, ab)
        elif kind == 'beam':                                    # #29 v3 (HIGH C): a charged giant blast
            self.ab_beam_charge(k, ab)
        elif kind in ('whip', 'sweep', 'spout'):                # #33 v2: FINGERPICK / BIG STRUM / UKE-NADO
            self.ab_x33(k, ab, kind)
        elif kind == 'charge':                                  # #34 v2 (TALLY-HO): the pack charges as one front
            self.start_charge(k, ab)
        elif kind == 'clamp':                                   # #30 v2 (FULL BELLOWS): a ring-wide bite
            self.clamp_start(k, ab)
        elif kind == 'dance':                                   # #32 v2 (SNAKE DANCE): cobras chase the enemy
            B.dance_until = self.t + self.ab_dur(ab)

    def ab_beam_charge(self, k, ab):
        """ability kind 'beam' (#29 v3): charge for `charge_beats` with the aim locked on the enemy's spot predicted
        `lead` x the charge ahead, then one giant blast (`len`, `width`, `dmg`, `knock`, live `hold` s) through the beam
        machinery; it pierces projectiles and grows no weapon"""
        B, O = self.balls[k], self.balls[1 - k]
        tc = ab.get('charge_beats', 1) * self.R.beat
        d = (O.p + O.v * tc * ab.get('lead', 0.7)) - (B.p + B.v * tc)
        aim = math.atan2(d[1], d[0])
        B.ab_charge = dict(aim=aim, t_fire=self.t + tc, ab=ab)
        self.ev('charge', k, aim, tc, float(ab['len']), float(ab['width']))

    def ab_beam_fire(self, k):
        B = self.balls[k]
        ch = B.ab_charge
        ab, a = ch['ab'], ch['aim']
        d = np.array([math.cos(a), math.sin(a)])
        o = B.p + d * (self.R.R + 0.04)
        self.beams.append(dict(owner=k, o=o, d=d, len=float(ab['len']), w=float(ab['width']), t1=self.t + ab.get('hold', 0.3),
                               hit=False, dmg=ab['dmg'], knock=ab.get('knock', 1.5), grow=False))
        self.ev('beam', k, float(o[0]), float(o[1]), a, float(ab['len']), float(ab['width']))
        B.ab_charge = None

    def abilities_step(self, on_beat):
        for k, B in enumerate(self.balls):                      # #29 v3: a charged ability beam fires when its beat comes
            ch = getattr(B, 'ab_charge', None)
            if ch is not None and self.t + 1e-9 >= ch['t_fire']:
                self.ab_beam_fire(k)
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
                if landed and W.get('slow'):                 # #34: the enemy it hits is slowed for a moment
                    O.slow, O.slow_mult = W['slow']['dur'], W['slow']['mult']
                    self.ev('slow', o, W['owner'], W['slow']['dur'])
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
                if Ar.get('curve'):                              # #34: an arc of curving projectiles (the owner's rate)
                    self.kcurve[self.next_id - 1] = (math.radians(B.curve_now), self.t + B.w.get('life', 3.0))
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
            if getattr(B, 'held', False):                       # #32 v2: swallowed, it rides inside the swallower
                continue
            B.boost *= math.exp(-dt / self.R.KNOCK_T)
            s = self.R.V * (1 + B.boost)
            if B.slow > 0:                                  # #34: slowed (e.g. SAD TROMBONE)
                s *= B.slow_mult
            if getattr(B, 'vmul', None) is not None:        # #30 v2: per-ball speed factor (inflate: heavy / rocketing)
                s *= B.vmul
            v = B.v / (np.linalg.norm(B.v) + 1e-12) * s
            B.p = B.p + v * dt
            B.v = v
            for ax in (0, 1):
                lim = A - getattr(B, 'rad', R)              # #30 v2: per-ball radius (default: the shared R)
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
        rP, rQ = getattr(P, 'rad', R), getattr(Q, 'rad', R)    # #30 v2: per-ball radius (R + R == 2 * R exactly)
        if dist < rP + rQ and getattr(P, 'rw_left', 0.0) <= 0 and getattr(Q, 'rw_left', 0.0) <= 0 \
                and not getattr(P, 'held', False) and not getattr(Q, 'held', False):   # #32 v2: no clash while swallowed
            n = d / max(dist, 1e-9)
            over = rP + rQ - dist
            if hasattr(P, 'mass') or hasattr(Q, 'mass'):        # #30 v2: unequal masses (inflate): the light ball gives
                mP, mQ = getattr(P, 'mass', 1.0), getattr(Q, 'mass', 1.0)
                P.p -= n * over * mQ / (mP + mQ)
                Q.p += n * over * mP / (mP + mQ)
                vp, vq = float(P.v @ n), float(Q.v @ n)
                if vp - vq > 0:
                    P.v += 2 * mQ / (mP + mQ) * (vq - vp) * n
                    Q.v += 2 * mP / (mP + mQ) * (vp - vq) * n
            else:
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
            for hk in getattr(self, 'clash_hooks', ()):         # #30 v2: contact damage (inflate body slam / flight)
                hk()
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
                m = enemy & (dd < getattr(B, 'rad', R) + (self.key_r() if self.khr else self.R.KEY_R))
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
                                    if Att.w.get('curve_grow'):        # #34: its arrows curve harder too
                                        Att.curve_now = min(Att.w['curve_max'], Att.curve_now + Att.w['curve_grow'])
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
        if 'RAD' in rec:                                    # #30 v2, only recorded when a fighter changes size
            rec['RAD'].append([getattr(B0, 'rad', self.R.R), getattr(B1, 'rad', self.R.R)])
        if 'COB' in rec:                                    # #32 v2: baskets + cobras (only when a fighter uses 'striker')
            rec['COB'].append([[(bk['id'], float(bk['home'][0]), float(bk['home'][1]), float(bk['cp'][0]),
                                 float(bk['cp'][1]), bk['t_drop']) for bk in getattr(B_, 'baskets', [])] for B_ in self.balls])
        if 'EXT' in rec:                                    # #34, only recorded when a fighter uses the options
            rec['EXT'].append([B0.ext, B1.ext])
            rec['SLW'].append([B0.slow, B1.slow])
            rec['CRV'].append([B0.curve_now, B1.curve_now])
        if 'HOUNDS' in rec:                                 # #34 v2, only recorded when a fighter uses the 'pack' kind
            Hs = np.full((2, self.HMAX, 6), np.nan)
            for j, Bj in enumerate((B0, B1)):
                for i, h in enumerate(getattr(Bj, 'hounds', [])[:self.HMAX]):
                    Hs[j, i] = (h['p'][0], h['p'][1], h['a'], self.HOUND_MODES[h['mode']], h['id'], h['cool'])
            rec['HOUNDS'].append(Hs)
            rec['MARK'].append([getattr(B0, 'mark', 0.0), getattr(B1, 'mark', 0.0)])
            rec['PACKN'].append([getattr(B0, 'n_pack', 0.0), getattr(B1, 'n_pack', 0.0)])
            rec['CHG'].append([(C['owner'], float(C['o'][0]), float(C['o'][1]), float(C['u'][0]), float(C['u'][1]), C['s'],
                                C['width'], C['t0']) for C in self.pack_charges])

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
        dc = [k for k, B in enumerate(self.balls) if B.w['kind'] in ('dash', 'cone', 'surf', 'rake')]   # (#33, v2)
        dc_k0 = int(math.ceil(R.t_fight / s16 - 1e-6))
        dc_next = dc_k0
        grid16 = [k for k, B in enumerate(self.balls) if B.w['kind'] in ('aura', 'bellows', 'inflate', 'bite')]   # #30
        n16b = k16_0
        for k, B in enumerate(self.balls):                       # #29 v3 (opt-in weapon kinds)
            if B.w['kind'] == 'combo':
                self.combo_setup(k)
            elif B.w['kind'] == 'flail':
                self.flail_setup(k)
            elif B.w['kind'] == 'inflate':                       # #30 v2
                self.inflate_setup(k)
        if rec is not None and any(B.w['kind'] == 'inflate' for B in self.balls):
            rec['RAD'] = []
        for k, B in enumerate(self.balls):                       # #32 v2 (opt-in weapon kinds)
            if B.w['kind'] == 'swallow':
                self.swallow_setup(k)
            elif B.w['kind'] == 'striker':                     # #32 v2
                self.striker_setup(k)
                B.t_strike = B.w['t_strike']
        strikers = [k for k, B in enumerate(self.balls) if B.w['kind'] == 'striker']      # #32 v2: own 16th counter
        next16k = k16_0
        if rec is not None and strikers:
            rec['COB'] = []
        thrusters = [k for k, B in enumerate(self.balls) if B.w.get('mode') == 'thrust']      # #34
        next16t = k16_0
        tethers = [k for k, B in enumerate(self.balls) if B.w['kind'] == 'tether']              # #31 v2
        next16c = k16_0
        if rec is not None and self.uses_ext:
            rec['EXT'], rec['SLW'], rec['CRV'] = [], [], []
        packers = [k for k, B in enumerate(self.balls) if B.w['kind'] == 'pack']      # #34 v2 (opt-in)
        for k in packers:
            self.pack_setup(k)
        n16p = k16_0
        if rec is not None and packers:
            rec['HOUNDS'], rec['MARK'], rec['PACKN'], rec['CHG'] = [], [], [], []
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
                    elif B.w['kind'] == 'combo':                 # #29 v3
                        self.combo_beat(k, kb)
                    elif B.w['kind'] == 'swallow':               # #32 v2
                        self.swallow_beat(k, kb)
                    elif B.w['kind'] == 'striker':               # #32 v2 (basket drops)
                        self.striker_beat(k, kb)
                    elif B.w['kind'] in ('line', 'trap'):
                        self.hazard_beat(k, kb)
                    elif B.w['kind'] == 'scratch_floor':         # #31 v2
                        self.scratch_beat(k, kb)
                    elif B.w['kind'] == 'tether':                # #31 v2
                        self.tether_beat(k, kb)
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
                        getattr(self, 'fire_' + B.w['kind'])(k, dc_k % dc_main == 0)
            if grid16 and self.t + 1e-9 >= n16b * s16:            # #30: aura / bellows on their own 16th counter
                k16b = n16b - k16_0
                n16b += 1
                for k in grid16:
                    self.grid16_fire(k, k16b)
            if tethers and self.t + 1e-9 >= next16c * s16:       # #31 v2: cable pulses on their own 16th counter
                k16c = next16c - k16_0
                next16c += 1
                for k in tethers:
                    self.tether_16(k, k16c)
            if strikers and self.t + 1e-9 >= next16k * s16:      # #32 v2: cobra strikes on the 16th grid (haste divides it)
                k16k = next16k - k16_0
                next16k += 1
                for k in strikers:
                    main16 = int(round(4 * self.balls[k].w['every']))
                    if k16k % max(1, int(round(main16 / self.ab_mult(k)))) == 0:
                        self.striker_fire(k)
            if thrusters and self.t + 1e-9 >= next16t * s16:     # #34: thrusts on the 16th grid (haste divides it)
                k16t = next16t - k16_0
                next16t += 1
                for k in thrusters:
                    main16 = int(round(4 * self.balls[k].w['every']))
                    if k16t % max(1, int(round(main16 / self.ab_mult(k)))) == 0:
                        self.start_thrust(k)
            pack_bite = False
            if packers and self.t + 1e-9 >= n16p * s16:           # #34 v2: calls on the weapon's grid, bites on the 8ths
                k16p = n16p - k16_0
                n16p += 1
                pack_bite = k16p % 2 == 0
                for k in packers:
                    if k16p % max(1, int(round(4 * self.balls[k].w['every']))) == 0:
                        self.pack_call(k)
            for k, B in enumerate(self.balls):
                B.shield = max(0.0, B.shield - dt)
                B.haste = max(0.0, B.haste - dt)
                B.slow = max(0.0, B.slow - dt)
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
                elif B.w['kind'] == 'combo':                     # #29 v3
                    self.combo_step(k, dt)
                elif B.w['kind'] == 'flail':                     # #29 v3
                    self.flail_step(k, dt)
                elif B.w['kind'] == 'scratch_floor':             # #31 v2
                    self.scratch_step(k, dt)
                elif B.w['kind'] == 'tether':                    # #31 v2
                    self.tether_step(k, dt)
                elif B.w['kind'] == 'inflate':                   # #30 v2
                    self.inflate_step(k, dt)
                elif B.w['kind'] == 'swallow':                   # #32 v2
                    self.swallow_step(k, dt)
                elif B.w['kind'] == 'striker':                   # #32 v2
                    self.striker_step(k, dt)
            if getattr(self, 'lashes', None):                     # #31 v2 SAWTOOTH
                self.lash_step(dt)
            if self.beams or any(B.w['kind'] == 'beam' for B in self.balls):
                self.beams_step(dt)
            if self.lines or self.strings:
                self.hazards_step(dt)
            self.effects_step(dt)
            for k in packers:                                    # #34 v2: the hounds
                self.pack_step(k, dt, pack_bite)
            if dc:
                self.dash_cone_step(dt)                          # (#33)
            if dc or self.whips or self.sweeps or self.spouts:
                self.surf_rake_step(dt)                          # (#33 v2)
            if self.sucks:                                          # #30 bellows suction
                self.suction_step(dt)
            if self.clamps:                                         # #30 v2 FULL BELLOWS
                self.clamps_step()
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
        if self.x33_log:
            out['X33_SPOUT'] = self.x33_log                      # (#33 v2) the waterspout's path
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
