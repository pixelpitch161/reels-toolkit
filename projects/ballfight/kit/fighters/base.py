"""The two halves of a fighter module: Look (how it draws in the ring) and Sound (how it plays in the score). A fighter
module (kit/fighters/<name>.py) subclasses both; every hook is optional, the defaults draw / play nothing beyond the
shared ball (glass + Pixel Pitch sprite), sparks and shatter. The renderer and the audio call the hooks in a fixed
order (the order #41's renderer drew things in), so a fighter only ever adds its own layers.

Look(P, k): P is kit.picture (RUN, EV, T, COL, CFG, fighter(), sim_i(), FL / BU / PA / MK / MO effects, ...), k its
side. Sound(M, k): M is kit.music (the voices, colour(), the Thinner, place helpers, the run)."""
import math

import numpy as np


class Look:
    glass = (236, 232, 226)          # its ball's glass (the ring and the VS card close-up): #30's cream
    flare_scale = 1.0                # attack lights in its colour x this (a near-white fighter whitens the canvas: 0.85)
    shards = ()                      # its parts in the K.O. shatter (after 24 glass shards), drawn by draw_shard
    spark_big = ('slam', 'wave', 'shockwave', 'cannon')   # its hit kinds with the big spark spray
    spark_flash = ()                 # its hit kinds with a 3 px white impact point (#27's blade / slam)
    flash_hits = ('slam', 'wave', 'shockwave')            # its hit kinds that set off the crowd's flashbulbs
    fringe_hits = ('slam', 'wave', 'shockwave')           # its hit kinds that get chromatic fringing
    quiet_hits = ()                  # its hit kinds that don't flash the victim's HP bar (a sting)
    teleports = False                # its ball can jump: no speed lines across a teleport
    title_pattern = None             # fn(base) -> base: its VS-card panel pattern (else title.PATTERNS has it, or plain)
    rope_kinds = {'wave': 1.0, 'shockwave': 0.6}         # its ability kinds that shake the ropes (strength)
    sprite_k = 0.80                  # its instrument's size in the ball (x the ball's radius; #33's acoustic: 0.86)
    block_burst = (255, 226, 150)    # the ink burst where a hit lands on its shield (a 'block' it took)
    block_spark = ((160, 220, 255), 12, 2.6, 0.3)        # (colour, n, speed, life) of that block's spark spray

    def __init__(self, P, k):
        self.P, self.k, self.T = P, k, P.T
        self.name = P.NAMES[k]
        self.cfg = P.CFG[k]
        self.col = P.COL[k]
        self.enemy = 1 - k

    # ---------------------------------------------------------------- setup (once, from the run)
    def setup(self):
        """precompute its state from the run's events (baskets, beams, ...)"""

    def title_closeup(self, r):
        """its own close-up sprite for the VS card (RGBA, radius r), or None for the Pixel Pitch model"""
        return None

    # ---------------------------------------------------------------- the ball
    def sprite(self, r, yaw, roll):
        """its own in-ball sprite (RGBA), or None for the Pixel Pitch model"""
        return None

    def sprite_rolls(self, kk, roll):
        """extra sprite rolls ball kk needs (a debuff that tilts the enemy's instrument): [] = none"""
        return []

    def ball(self, kk, st, t, glass, roll):
        """modify ball kk's glass tint / sprite roll at t (its debuffs on the enemy): returns (glass, roll)"""
        return glass, roll

    def ball_sprite(self, kk, st, t, rp, yaw):
        """its own ball's instrument at t instead of the usual one (#38 IDENTITY CRISIS: the enemy's model, tinted):
        (RGBA sprite, fn(lit) -> lit or None), or None for the usual sprite"""
        return None

    def extra_sprites(self):
        """[(side, yaw centre, yaw swing, roll)] more Pixel Pitch views for the sprite cache (that side's model)"""
        return []

    def state(self, kk, st, t, phase, gl=0.0):
        """add to ball kk's picture state (its own weapon fields, or its status effects on the enemy); phase is
        'rev' (corners, before FIGHT!), 'fight' or 'tail' (after the K.O.; gl = the glide home, 0..1)"""

    def hides_hp(self, kk, t):
        """is ball kk hidden under something of its (no HP number drawn)?"""
        return False

    # ---------------------------------------------------------------- events -> camera, ropes, crowd, sparks
    def shake(self, e):
        """camera shake for one of its events (None = none)"""
        if e[1] == 'slam':
            return 1.6
        return None

    def rope(self, e):
        """rope wobble strength for one of its events (abilities first, then others: #41's order)"""
        if e[1] == 'ability':
            return self.rope_kinds.get(e[4])
        if e[1] == 'slam':
            return 0.9
        return None

    def spark(self, e):
        """(colour, n, speed, life) of the spark spray for one of its hits"""
        P = self.P
        how = e[7]
        n, sp, life = (16, 3.2, 0.3) if how in self.spark_big else (5, 1.8, 0.18)
        return P.COL[e[2]] * 0.6 + 100, n, sp, life

    def hit_glow(self, e):
        """(radius px, alpha) of the glow on one of its fresh hits, or None"""
        return None

    # ---------------------------------------------------------------- effects (reelkit.fx), built once in time order
    def fx_setup(self):
        """effects from its own state (before the event pass)"""

    def fx_event(self, e):
        """effects for one of its events (hits it dealt, its abilities, its slams), in event order"""

    def fx_after(self):
        """effects after the event pass"""

    def ko_fx(self, pos):
        """its K.O. spray (it lost): particles from where it shattered"""
        self.P.PA.emit(self.P.T_KO, pos, 'sparks', n=30, spread=180, speed=(0.8, 2.2))

    def fx_end(self):
        """effects after the K.O.'s (the champion's solo look, e.g. #35's solo pipes kicking up dust)"""

    def light(self, L, t):
        """its own point lights in the scene's Lights at t (e.g. #35's CATHEDRAL dome glowing from inside), after the
        attack flares"""

    # ---------------------------------------------------------------- the canvas (per pixel)
    def shadows(self, st, spot):
        """extra casters right after its ball (its weapon): [polys in the canvas grid frame]"""
        return []

    def shadows_free(self, F, spot):
        """extra casters after both balls (things it left on the canvas)"""
        return []

    def canvas(self, Ic, C):
        """canvas light decals (C: the canvas context, see picture.render); returns Ic"""
        return Ic

    def canvas_wave(self, Ic, C, w):
        """the canvas band of one of its expanding rings (RUN['WAVES'] row w); returns Ic"""
        return Ic

    def canvas_after(self, Ic, C):
        """decals after the rings (e.g. a slam's darkening); returns Ic"""
        return Ic

    # ---------------------------------------------------------------- the painter pass, effects over the ring
    def items_ball(self, F, st):
        """[(depth, payload)] drawn in the painter pass, queued right after its ball (its weapon)"""
        return []

    def items_free(self, F):
        """[(depth, payload)] queued after both balls (its things on the canvas)"""
        return []

    def draw_item(self, F, payload):
        """draw one of its painter items (F.fr: the frame)"""

    def over_ring(self, F, arr, depth):
        """its effects over the ring, after the front ropes (beams, a giant bell): returns (arr, depth)"""
        return arr, depth

    def draw_shard(self, arr, cam, kind, p, rot, size, curl):
        """one of its shatter parts (kinds from `shards`): returns arr (may be a new array)"""
        return arr

    # ---------------------------------------------------------------- lens, glows, post
    def shimmer(self, img, cam, t, w):
        """heat shimmer for one of its rings (RUN['WAVES'] row w): returns img"""
        return img

    def shimmer_after(self, img, cam, t):
        return img

    def glows(self, F, drawn):
        """its own glows (F.fr.glow)"""

    def fringe(self, e):
        return e[7] in self.fringe_hits

    def fringe_size(self, e):
        """(radius px, shift px) of the chromatic fringe on one of its fringed hits"""
        return 26, 2.5

    def after_grade(self, img, F, drawn):
        """crisp overlays after the grade (status outlines on the enemy): returns img"""
        return img

    # ---------------------------------------------------------------- checks
    def focus_points(self, t, st):
        """[(world point, label)] of its subject parts that must stay in focus at t"""
        return []


class Sound:
    bus_gain = 1.5                   # its bus in the master (its own episode's mix)
    send = 0.35                      # its reverb send
    band = (200, 4000)               # its band for the band check (it must sit above the bed there)
    colour_args = dict(cut0=90, cut_span=520, drive=4.5)   # how it thins + dirties as its HP drops (music.colour)

    def __init__(self, M, k):
        self.M, self.k, self.T = M, k, M.T
        self.name = M.NAMES[k]
        self.bus = self.name.lower()
        self.role = M.T.ROLES.get(self.name, 'melody')
        self.enemy = 1 - k

    def voice(self, m, dur, vel=0.85):
        """one note in its voice (a debuff on it can play a sour twin in it); default: its Pixel Pitch voice"""
        return self.M.note(self.M.CFG[self.k]['pp'], m, round(dur, 3), round(dur + 0.2, 2), round(vel, 1))

    def detune(self, t):
        """the OUT OF TUNE depth on it at t (0..1): the engine's recorded status"""
        M = self.M
        if 'DET' not in M.RUN or not (self.T.T_FIGHT <= t < M.T_KO):
            return 0.0
        left, dep = M.RUN['DET'][M.P.sim_i(t), self.k]
        return float(dep) if left > 0 else 0.0

    def colour(self, x, t, drive_k=1.0, **kw):
        a = dict(self.colour_args)
        a.update(kw)
        return self.M.colour(x, self.k, t, drive_k, **a)

    # the score, in the order kit.music calls it
    def title_blast(self, mix, t):
        """its hit as its name slams in on the VS card (config TITLE_HITS)"""

    def count_in(self, mix):
        """its notes on the count-in (bar 1) and the pickup into FIGHT!"""

    def play(self, mix):
        """its part in the fight, from the run's events"""

    def fall_apart(self, mix):
        """it lost: the sound of it shattering (from P.T_HS)"""

    def stinger(self, mix, tw):
        """it won: its stinger on "(WINNER) WINS" """

    def solo(self, mix, t0):
        """it won: its solo (config SOLO) from t0, stopping at the reassembly"""

    def reassemble(self, mix):
        """it lost: its pieces fly home as a rising run"""

    def last_bar(self, mix):
        """its part in the build into the title slam (a role, e.g. the bass climbing the dominant)"""


def wrap(a):
    return (a + math.pi) % (2 * math.pi) - math.pi


def unit(a):
    return np.array([math.cos(a), math.sin(a)])
