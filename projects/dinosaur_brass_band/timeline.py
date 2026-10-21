"""Dinosaur Brass Band - ONE timeline for picture and sound (IDEAS #8).

Hook: "A 75M-YEAR-OLD / HORN". Parasaurolophus's crest is a hollow air tube (CT-scanned skull from
1995, Sandia/NMMNH simulation 1997). Mechanism: a longer air column gives a lower note, so each
animal's crest length sets its pitch: f = AIR_K / air_length (same physics as a blown bottle or a
brass tube). Five crest sizes = five registers, stacking into an original brass chorale in G with
the biggest adult on the bass (G1 is a music choice, never shown as a claim; no Hz on screen).

Beat sheet (60 s)
    0-4    one adult lifts its head and honks                A 75M-YEAR-OLD / HORN
    4-8    push in, crest turns X-ray: hollow passages        IT'S HOLLOW
    8-12   CT slices stack up and rebuild the skull           350 CT SLICES (counter)
    12-16  glowing air flows through the passages             AIR IN... / ...NOTE OUT
    16-20  tube-length bars + bottle icon, two test tones     LONGER TUBE / = LOWER NOTE
    20-24  a juvenile with a short crest answers, high        BABIES: HIGHER / (PROBABLY)
    24-29  adult and juvenile call and answer                 CALL & / ANSWER
    29-34  the herd walks in, 5 sizes honk one by one         5 SIZES = / 5 NOTES
    34.5-  adults set the rhythm (bass on the beat)
    40.5-  mid-sized animals add harmony
    46.5-  climax: every crest up, sun flares                 THE FIRST / BRASS BAND?
    52.5-  bottle icon, herd walks off, lone adult honks      BLOW ACROSS / A BOTTLE, SAME PHYSICS
    58.6-  adult lowers its head = the opening frame (loop)
"""
import math

FPS = 30
DUR = 60.0
NF = int(FPS * DUR)


def mtof(m):
    return 440.0 * 2 ** ((m - 69) / 12)


def ss(a, b, x):
    x = min(1.0, max(0.0, (x - a) / (b - a)))
    return x * x * (3 - 2 * x)


def ease(x):
    x = min(1.0, max(0.0, x))
    return x * x * x * (x * (6 * x - 15) + 10)


# ------------------------------------------------------------------ the herd (big -> small)
# home = the note the crest's air column gives (MIDI); crest air length is 1/f of it.
VOICES = [
    dict(name='XL', scale=1.00, home=31, col=(255, 96, 112), head_f=1.30,
         pal=dict(body=(124, 140, 92), stripe=(96, 106, 74), belly=(222, 212, 150), crest=(220, 92, 60), beak=(228, 208, 162)),
         dl=(0.0, 0.0), heading=20.0),
    dict(name='L', scale=0.82, home=38, col=(255, 158, 64), head_f=1.32,
         pal=dict(body=(146, 132, 88), stripe=(112, 98, 70), belly=(226, 214, 152), crest=(232, 122, 58), beak=(232, 214, 168)),
         dl=(2.2, 2.2), heading=18.0),
    dict(name='M', scale=0.66, home=47, col=(248, 220, 84), head_f=1.38,
         pal=dict(body=(110, 142, 108), stripe=(84, 110, 86), belly=(220, 214, 156), crest=(236, 152, 66), beak=(230, 216, 170)),
         dl=(4.0, 3.9), heading=22.0),
    dict(name='S', scale=0.50, home=55, col=(120, 244, 196), head_f=1.48,
         pal=dict(body=(152, 148, 100), stripe=(120, 114, 80), belly=(230, 220, 160), crest=(238, 176, 86), beak=(236, 222, 176)),
         dl=(5.5, 5.2), heading=18.0),
    dict(name='XS', scale=0.40, home=62, col=(110, 212, 255), head_f=1.58,
         pal=dict(body=(164, 154, 108), stripe=(130, 120, 86), belly=(234, 224, 166), crest=(242, 198, 108), beak=(238, 226, 182)),
         dl=(6.8, 6.2), heading=24.0),
]
AIR_K = 1.0
for v in VOICES:
    dp, lt = v['dl']                                                  # layout frame: depth toward camera, lat to the right
    v['pos'] = (dp * 0.5 + lt * 0.866, dp * 0.866 - lt * 0.5)
    v['air'] = mtof(VOICES[0]['home']) / mtof(v['home'])          # relative air-column length (XL = 1)
    v['crest_f'] = v['air'] / v['scale']                            # crest length factor in the scaled model

# walk-in: curved path from behind-right (depth -9, lat +5 from the spot) that arrives along the heading,
# so nobody walks through the adult. walk-off: a curve that turns away into the back-right (EXIT_TO),
# front animal first, so nobody passes close to the camera.
ENTRY = {'S': (19.0, 20.9), 'L': (27.5, 30.2), 'M': (27.8, 30.4), 'XS': (28.1, 30.6)}
ENTRY_FROM = (-9.0, 5.0)
EXIT = {'XS': (52.6, 56.6), 'S': (52.75, 56.75), 'M': (52.9, 56.9), 'L': (53.05, 57.05)}
EXIT_TO = (-13.0, 10.0)

# ------------------------------------------------------------------ section times
T_XRAY0, T_XRAY1 = 4.2, 16.4      # crest shown as X-ray
T_CT0, T_CT1 = 8.1, 11.9          # CT slices build the skull
T_AIR0, T_AIR1 = 12.1, 16.2       # glowing air through the passages
T_TUBE0, T_TUBE1 = 16.0, 20.0     # tube-length panel
T_STAFF0 = 23.8                   # piano-roll panel slides in
T_STACK = 30.6
T_SONG = 34.5
BPM = 80
BEAT = 60.0 / BPM
T_CLIMAX = T_SONG + 16 * BEAT     # 46.5
T_END = T_SONG + 24 * BEAT        # 52.5
T_LAST = 56.6                     # lone adult's final honk
T_LOWER = 58.55                   # head lowers back to the opening pose

# ------------------------------------------------------------------ honks (t0, dur, voice index, midi, style)
HONKS = [
    (0.85, 2.35, 0, 31, 'call'),
    (12.4, 3.4, 0, 31, 'rumble'),
    (21.0, 1.25, 3, 55, 'call'),
    (24.15, 1.0, 0, 31, 'call'),
    (25.2, 0.55, 3, 55, 'call'),
    (25.85, 0.55, 3, 57, 'call'),
    (26.6, 0.95, 0, 38, 'call'),
    (27.55, 0.6, 3, 55, 'call'),
    (28.3, 1.2, 0, 31, 'call'),
    (28.3, 1.2, 3, 55, 'call'),
]
for k, t0 in enumerate([T_STACK + 0.72 * i for i in range(5)]):
    HONKS.append((t0, T_SONG - 0.25 - t0, k, VOICES[k]['home'], 'hold'))

# the chorale (original): beats from T_SONG, (beat, length, midi) per voice. 6 bars of 4/4 in G:
# | G | C D | Em C | Am D | G Em | C D G |
SONG = {
    0: [(0, .9, 31), (1, .9, 31), (2, 1.9, 31), (4, .9, 36), (5, .9, 36), (6, 1.9, 38), (8, .9, 28), (9, .9, 28), (10, 1.9, 36),
        (12, .9, 33), (13, .9, 33), (14, 1.9, 38), (16, 1.9, 31), (18, 1.9, 28), (20, .9, 36), (21, .9, 38), (22, 2.2, 31)],
    1: [(0, 3.8, 38), (4, 1.9, 40), (6, 1.9, 42), (8, 1.9, 43), (10, 1.9, 40), (12, 1.9, 40), (14, 1.9, 42), (16, 1.9, 38),
        (18, 1.9, 35), (20, .9, 40), (21, .9, 42), (22, 2.2, 38)],
    2: [(8, 1.9, 47), (10, 1.9, 48), (12, 1.9, 48), (14, 1.9, 45), (16, 1.9, 47), (18, 1.9, 47), (20, .9, 48), (21, .9, 45), (22, 2.2, 47)],
    3: [(12, 1.9, 52), (14, 1.9, 50), (16, 1.9, 50), (18, 1.9, 52), (20, .9, 52), (21, .9, 54), (22, 2.2, 55)],
    4: [(16, .9, 62), (17, .9, 67), (18, 1.9, 64), (20, .9, 64), (21, .9, 66), (22, 2.2, 67)],
}
for k, notes in SONG.items():
    for b, ln, m in notes:
        HONKS.append((T_SONG + b * BEAT, ln * BEAT, k, m, 'song'))
HONKS.append((T_LAST, 2.0, 0, 31, 'call'))
HONKS.sort(key=lambda h: h[0])
CHORDS = [(0, 'G'), (4, 'C'), (6, 'D'), (8, 'Em'), (10, 'C'), (12, 'Am'), (14, 'D'), (16, 'G'), (18, 'Em'), (20, 'C'), (21, 'D'), (22, 'G')]

TEST_TONES = [(16.9, 1.15, 'long', 55), (18.45, 1.15, 'short', 67)]   # half the tube = one octave up
BOTTLE_BLOWS = [(53.0, 1.25, 74, 0.62), (54.55, 1.4, 67, 0.32)]       # (t0, dur, midi, water level 0-1)
SCAN_TICKS = [8.35 + 0.1 * i for i in range(34)]


def ct_count(t):
    return int(round(350 * ease((t - (T_CT0 + 0.1)) / (T_CT1 - T_CT0 - 0.5))))


def ct_frac(t):
    return ct_count(t) / 350.0


# ------------------------------------------------------------------ captions
def captions(t):
    n = ct_count(t)
    return [
        (0.0, 4.0, ["A 75M-YEAR-OLD", "*HORN*"]),
        (4.1, 8.0, ["IT'S *HOLLOW*"]),
        (8.1, 12.0, [f"*{n:3d}* CT SLICES"]),
        (12.1, 16.0, ["*AIR* IN...", "...A *NOTE* OUT"]),
        (16.1, 20.0, ["LONGER TUBE", "= *LOWER* NOTE"]),
        (20.1, 24.0, ["BABIES: *HIGHER*", "(PROBABLY)"]),
        (24.1, 28.9, ["CALL &", "*ANSWER*"]),
        (29.0, 34.4, ["*5* SIZES =", "*5* NOTES"]),
        (46.5, 52.4, ["THE FIRST", "*BRASS BAND*?"]),
        (52.5, 56.2, ["BLOW ACROSS", "A *BOTTLE*"]),
        (56.3, 59.6, ["*SAME* PHYSICS"]),
    ]


# ------------------------------------------------------------------ poses (shared by picture; audio uses HONKS)
def honk_env(t, t0, dur, att=0.06, rel=0.25):
    """amplitude-like 0..1 envelope of one honk"""
    if t < t0 or t > t0 + dur + rel:
        return 0.0
    a = min(1.0, (t - t0) / att)
    r = 1.0 if t <= t0 + dur else 1 - (t - t0 - dur) / rel
    return max(0.0, a * r)


def honk_level(k, t):
    return max([honk_env(t, h[0], h[1]) for h in HONKS if h[2] == k] + [0.0])


def current_notes(t):
    """[(voice, midi, level)] sounding at t"""
    out = []
    for h in HONKS:
        e = honk_env(t, h[0], h[1])
        if e > 0:
            out.append((h[2], h[3], e))
    return out


def baseline(k, t):
    if k == 0:
        if t < 0.15:
            return 0.0
        if t < 4.5:
            return max(ease((t - 0.15) / 0.7) if t < 3.2 else 0.0, 0.55 + 0.45 * (1 - ss(3.0, 4.4, t)) if t >= 3.0 else 0.0)
        if t < 20.0:
            return 0.55
        if t < T_SONG:
            return 0.42
        if t < T_CLIMAX:
            return 0.38
        if t < T_END:
            return 0.85
        if t < T_LOWER:
            return 0.45
        return 0.45 * (1 - ease((t - T_LOWER) / (DUR - T_LOWER - 0.12)))
    if t < T_SONG:
        return 0.4
    if t < T_CLIMAX:
        return 0.36
    if t < T_END:
        return 0.85
    return 0.3


def raise_amt(k, t):
    """0 = head low (rest), 1 = crest up, snout to the sky (honk)"""
    r = baseline(k, t)
    for h in HONKS:
        if h[2] != k:
            continue
        t0, dur = h[0], h[1]
        if t0 - 0.5 <= t <= t0 + dur + 0.5:
            up = ss(t0 - 0.45, t0 - 0.05, t)
            down = 1 - ss(t0 + dur, t0 + dur + 0.45, t)
            r = max(r, min(up, down))
    return min(1.0, r)


def _dl2xz(dp, lt):
    return dp * 0.5 + lt * 0.866, dp * 0.866 - lt * 0.5


def walk(k, t):
    """(dx, dz, heading_deg, dist_walked, speed 0..1) for voice k at time t"""
    v = VOICES[k]
    name = v['name']
    hd = math.radians(v['heading'])
    f = (math.cos(hd), math.sin(hd))
    if name in ENTRY:
        t0, t1 = ENTRY[name]
        u = (t - t0) / (t1 - t0)
        if u < 1:
            u = max(u, 0.0)
            e = 1 - (1 - u) ** 2
            sx, sz = _dl2xz(*ENTRY_FROM)
            cx, cz = -3.5 * f[0], -3.5 * f[1]                    # control point: arrive along the heading
            px = 2 * (1 - e) * e * cx + (1 - e) ** 2 * sx
            pz = 2 * (1 - e) * e * cz + (1 - e) ** 2 * sz
            tx = 2 * (1 - e) * (cx - sx) + 2 * e * (0 - cx)
            tz = 2 * (1 - e) * (cz - sz) + 2 * e * (0 - cz)
            head = math.degrees(math.atan2(tz, tx))
            L = 11.5
            return px, pz, head, L * e, (2 * (1 - u) if t >= t0 else 0.0)
    if name in EXIT:
        t0, t1 = EXIT[name]
        u = (t - t0) / (t1 - t0)
        if u > 0:
            uu = min(u, 1.0)
            e = uu * uu * (1.6 - 0.6 * uu)                      # ease out of standing, keep walking
            cx, cz = 2.5 * f[0], 2.5 * f[1]                      # step forward first, then curve away
            ex, ez = _dl2xz(*EXIT_TO)
            px = 2 * (1 - e) * e * cx + e * e * ex
            pz = 2 * (1 - e) * e * cz + e * e * ez
            tx = 2 * (1 - e) * cx + 2 * e * (ex - cx)
            tz = 2 * (1 - e) * cz + 2 * e * (ez - cz)
            head = math.degrees(math.atan2(tz, tx)) if e > 0.01 else v['heading']
            return px, pz, head, 17.0 * e, (min(1.0, 2.5 * uu) if u < 1 else 0.0)
    return 0.0, 0.0, v['heading'], 0.0, 0.0


def present(k, t):
    """is voice k in the scene at time t"""
    name = VOICES[k]['name']
    if name in ENTRY and t < ENTRY[name][0]:
        return False
    if name in EXIT and t > EXIT[name][1]:
        return False
    return True
