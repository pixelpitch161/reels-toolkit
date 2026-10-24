"""BALLFIGHT engine: a small, honest 2D fight between two balls in a square ring (top-down), reusable by every episode.

    from fight import Rules, Fight
    run = Fight('GUITAR', 'PIANO', seed=7, rules=Rules(bpm=129.84, t_fight=1.8485)).run()
    run['winner'], run['t_ko'], run['events'] ...

Rules (the same for both fighters; nothing is scripted or edited):
  - balls move at one constant speed and bounce off the ropes (the ring is |x|, |y| <= A in the ring's own frame); the
    ropes "give" a little (a seeded +-ROPE_GIVE deg on every bounce, so no run settles into a fixed orbit); ball vs ball
    is an elastic clash, after which both go back to the shared speed;
  - weapons and abilities come from fighters.py (blade / volley; shockwave, shield, haste, arc, wave, slam);
  - every landed hit grows the attacker's weapon (a blade gets longer, a volley gets one more projectile), so the fight
    escalates by itself; levels come from damage dealt (LV2 / LV3 unlock abilities), and the 'low' ability fires once
    when HP drops below LOW_HP;
  - volleys fire and abilities trigger on the beat grid (the fight IS the music); a blade's hits are detected with a
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
        self.blade_len = self.w.get('base', 0.0)
        self.frets = self.w.get('frets0', 0)
        self.cool = 0.0
        # volley
        self.n_vol = float(self.w.get('n0', 0))   # a float: `grow` may add a fraction of a key per landed volley
        self.haste_mult = 1.0
        # ability state
        self.shield = 0.0
        self.haste = 0.0
        self.ready = {}               # ability slot -> time it may fire next (None = locked)
        self.used = set()
        self.low_done = False


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
        self.ev('fire', k, n, vol)

    def blade_step(self, k, dt):
        B, O = self.balls[k], self.balls[1 - k]
        w = B.w
        mult = self.ab_mult(k)
        dth = B.spin_dir * w['spin'] * mult * dt
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

    # ------------------------------------------------------------------ abilities
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
            B.shield = ab['dur']
        elif kind == 'haste':
            B.haste = ab['dur']
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
                self.damage(W['owner'], o, W['dmg'], W['kind'], hx, hy)
                self.knock(o, rel, W['knock'])
            if len(self.kp):
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

    # ------------------------------------------------------------------ motion
    def move(self, dt):
        A, R = self.R.A, self.R.R
        for k, B in enumerate(self.balls):
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
        if dist < 2 * R:
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
        if len(self.kp):
            self.kp = self.kp + self.kv * dt
            out = (np.abs(self.kp) > A).any(1)
            self.kill_keys(out, 'wall')
        if len(self.kp):
            for k, B in enumerate(self.balls):
                enemy = self.kown != k
                if not enemy.any():
                    continue
                dd = np.hypot(*(self.kp - B.p[None]).T)
                m = enemy & (dd < R + self.R.KEY_R)
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
        rec = {k: [] for k in ('P', 'V', 'TH', 'BL', 'FR', 'NV', 'HP', 'LV', 'SH', 'HA', 'SPIN', 'K', 'WAVES', 'SLAMS')} \
            if self.record else None
        dt = R.DT
        steps_per_frame = int(round(1.0 / (R.FPS * dt)))
        k0 = int(math.ceil(R.t_fight / R.beat - 1e-6))           # first beat of the fight
        next_beat = k0
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
                    if B.w['kind'] == 'volley':
                        ev = max(1, int(round(B.w['every'] / (self.ab_mult(k)))))
                        if kb % ev == 0:
                            self.fire_volley(k)
            for k, B in enumerate(self.balls):
                B.shield = max(0.0, B.shield - dt)
                B.haste = max(0.0, B.haste - dt)
            self.move(dt)
            for k, B in enumerate(self.balls):
                if B.w['kind'] == 'blade':
                    self.blade_step(k, dt)
            self.effects_step(dt)
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
