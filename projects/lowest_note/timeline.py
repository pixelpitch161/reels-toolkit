"""IDEAS #2  The Lowest Note in the Universe.  ONE timeline for picture AND sound.

Science (Chandra, 2003): jets from the black hole at the heart of the Perseus cluster inflate bubbles
in the cluster's hot gas. Each bubble pushes out like a piston and launches a pressure wave. The
bubbles come at regular intervals, so the waves make a steady tone: a B-flat about 57 octaves below
the B-flat above middle C (B-flat4 = 466.16 Hz). One crest every ~9.8 million years.

The music: an OCTAVE counter climbs from -57. Each step doubles the frequency (like speeding the
tape up 2x). At -9 a crest arrives about once a second, so every wave is an audible kick (55 BPM);
it doubles to 109 and 219 BPM, blurs into a flutter, and at -4 (29 Hz) it is heard as a pitch
(B-flat0). Each further octave adds a layer under an original B-flat-major melody.

Wave crests on screen are launched exactly when the soundtrack's crests sound (from 29 s on).
Before that the picture runs as a time-lapse with the same accelerating cadence."""
import math
import numpy as np

FPS = 30
DUR = 60.0
NF = int(FPS * DUR)

BB4 = 466.1638                 # B-flat4 in Hz (the B-flat above middle C)
YEAR = 365.25 * 86400.0


def freq(k):
    """frequency of the note k octaves from B-flat4 (k <= 0)"""
    return BB4 * 2.0 ** k


def period(k):
    return 1.0 / freq(k)


# ---------------------------------------------------------------- the beat sections (crest-aligned)
T9 = 29.0                       # first audible crest: octave -9 = 0.91 Hz = 55 BPM
P = {k: period(k) for k in range(-9, 1)}
T8 = T9 + 3 * P[-9]             # 3 kicks at 55 BPM
T7 = T8 + 3 * P[-8]             # 3 kicks at 109 BPM
T6 = T7 + 4 * P[-7]             # 4 kicks at 219 BPM
T5 = T6 + 11 * P[-6]            # flutter 7.3 Hz
T4 = T5 + 21 * P[-5]            # flutter 14.6 Hz, then 29.1 Hz = a pitch (B-flat0)
T3, T2, T1, T0 = 39.5, 41.0, 42.5, 44.0     # octaves stack: B-flat1, 2, 3, 4

T_CLIMAX = T0                   # all octaves + the melody
BEAT = P[-8]                    # climax beat = the hole's note 8 octaves down = 109.3 BPM
BPM = 60.0 / BEAT
T_LAND = T_CLIMAX + 16 * BEAT   # melody lands on B-flat4 (4 bars of 4)
T_REWIND0, T_REWIND1 = 53.6, 57.2
T_PANEL_IN, T_PANEL_OUT = 14.0, 57.75

T_REVEAL = 4.0
T_JETS = 4.6                    # jets fire
T_HAZE0, T_HAZE1 = 8.0, 10.6    # hot-gas haze lights up
T_POP = 12.5                    # the bubble pops: first visible piston wave


# ---------------------------------------------------------------- octave counter
def _ease(x):
    x = min(1.0, max(0.0, x))
    return x * x * x * (x * (6 * x - 15) + 10)


_EX = np.linspace(0, 1, 2001)
_EY = np.array([_ease(x) for x in _EX])


def _climb(t0, t1, k0, k1):
    """integer steps k0 -> k1 spread over [t0, t1]: slow, fast, slow (last step lands on t1)"""
    n = abs(k1 - k0)
    s = 1 if k1 > k0 else -1
    return [(t0 + (t1 - t0) * float(np.interp(j / n, _EY, _EX)), k0 + s * j) for j in range(1, n + 1)]


STEPS = (_climb(16.9, 18.7, -57, -34) + _climb(20.0, 21.0, -34, -25) + _climb(23.0, 23.6, -25, -21)
         + _climb(26.0, 26.8, -21, -15) + _climb(28.15, T9, -15, -9)
         + [(T8, -8), (T7, -7), (T6, -6), (T5, -5), (T4, -4), (T3, -3), (T2, -2), (T1, -1), (T0, 0)]
         + _climb(T_REWIND0, T_REWIND1, 0, -57))
STEP_T = np.array([s[0] for s in STEPS])
STEP_K = np.array([s[1] for s in STEPS])


def octave(t):
    i = int(np.searchsorted(STEP_T, t, side='right')) - 1
    return -57 if i < 0 else int(STEP_K[i])


def last_step(t):
    """time of the most recent counter step (for blinks)"""
    i = int(np.searchsorted(STEP_T, t, side='right')) - 1
    return -1e9 if i < 0 else float(STEP_T[i])


def readout(k):
    """the period / rate line on the HUD for octave k"""
    if k >= -4:
        return f"{freq(k):.1f} HZ"
    if k >= -6:
        return f"{freq(k):.1f} HZ"
    if k >= -9:
        return f"{60 * freq(k):.0f} BPM"
    p = period(k)
    if p >= YEAR * 1e6:
        return f"{p / YEAR / 1e6:.1f}M YRS"
    if p >= YEAR * 1e3:
        return f"{p / YEAR / 1e3:.0f}K YRS"
    if p >= YEAR * 1.05:
        return f"{p / YEAR:.1f} YRS"
    if p >= 86400 * 2:
        return f"{p / 86400:.0f} DAYS"
    if p >= 3600 * 2:
        return f"{p / 3600:.0f} HRS"
    if p >= 120:
        return f"{p / 60:.0f} MIN"
    return f"{p:.0f} SEC"


NOTE_NAME = {-4: "B♭0", -3: "B♭1", -2: "B♭2", -1: "B♭3", 0: "B♭4"}

# ---------------------------------------------------------------- wave crests (ring launches)
# (time, amplitude, group).  Groups: A hook/time-lapse, B audible beats, F flutter, C climax, E outro
LAUNCH = [(-4.6, 0.95, 'A'), (T_POP, 1.0, 'A')]
for t in (16.1, 19.0, 21.4, 23.4, 25.1, 26.55, 27.82):          # accelerating time-lapse
    LAUNCH.append((t, 1.0, 'A'))
for k, t0, t1 in ((-9, T9, T8), (-8, T8, T7), (-7, T7, T6), (-6, T6, T5), (-5, T5, T4)):
    n = int(round((t1 - t0) / P[k]))
    for j in range(n):
        LAUNCH.append((t0 + j * P[k], 1.0 if k >= -7 else 0.55, 'B' if k >= -7 else 'F'))
for j in range(17):                                               # climax beats + the landing
    LAUNCH.append((T_CLIMAX + j * BEAT, 1.15 if j == 16 else 1.0, 'C'))
T_LOOP_RING = 55.4                                               # = the hook's ring (-4.6) one loop later
LAUNCH.append((T_LOOP_RING, 0.95, 'E'))
LAUNCH.sort()
LT = np.array([l[0] for l in LAUNCH])
NEXT_AFTER_LAST = T_POP + DUR                                    # bubble cycle wraps to the opening

KICKS = [t for t, a, g in LAUNCH if g in ('B', 'F', 'C') and t < DUR]   # audible crests
ACCEL_THUMPS = [t for t, a, g in LAUNCH if g == 'A' and 0 < t < T9]
R0 = 0.5                       # rings start at the bubble's edge


# ---------------------------------------------------------------- ring speed -> travel s(t)
def _vkeys():
    return [(-10.0, 0.075), (12.5, 0.075), (16.0, 0.30), (T9, 0.75), (T8 - 0.1, 0.75), (T8 + 0.15, 1.2),
            (T7 - 0.1, 1.2), (T7 + 0.15, 1.8), (T6 - 0.1, 1.8), (T6 + 0.15, 2.4), (T5 - 0.1, 2.4), (T5 + 0.15, 3.0),
            (T4 + 0.5, 3.0), (T_CLIMAX - 1.5, 1.2), (T_LAND + 0.25, 1.2), (T_LOOP_RING - 0.6, 0.075), (80.0, 0.075)]


def _v(t):
    ks = _vkeys()
    for (a, va), (b, vb) in zip(ks, ks[1:]):
        if t <= b:
            u = (t - a) / (b - a)
            u = u * u * (3 - 2 * u)
            return va + (vb - va) * u
    return ks[-1][1]


_TG = np.arange(-10.0, 80.0, 1.0 / 600)
_VG = np.array([_v(x) for x in _TG])
_SG = np.concatenate([[0.0], np.cumsum((_VG[1:] + _VG[:-1]) * 0.5 / 600)])
L_S = np.interp(LT, _TG, _SG)


def travel(t):
    return float(np.interp(t, _TG, _SG))


def speed(t):
    return float(np.interp(t, _TG, _VG))


def group_env(g, t):
    """time envelope per group (rings fade as the song moves on)"""
    if g == 'C':
        return 1.0 - _ease((t - (T_LAND + 0.4)) / 3.6)
    if g in ('B', 'F'):
        return 1.0 - _ease((t - (T4 - 0.3)) / 1.2)
    if g == 'A':
        return 1.0 - _ease((t - (T6 - 0.3)) / 1.5)
    return 1.0


def rings(t):
    """active rings: list of (radius, amp, age, group, solid). solid = spaced enough to be 3D ridges"""
    s = travel(t)
    out = []
    for i, (tl, a, g) in enumerate(LAUNCH):
        if tl > t:
            break
        r = R0 + s - L_S[i]
        if r > 3.5:
            continue
        ae = a * group_env(g, t)
        if ae < 0.02:
            continue
        gap = (LT[i + 1] - tl) if i + 1 < len(LT) else 99.0
        spacing = speed(tl) * gap
        out.append((r, ae, t - tl, g, spacing > 0.28 and g != 'F'))
    if t < 0.0:
        return out
    return out


def bubble_phase(t):
    """0..1 through the current inflate cycle (launch to launch); cycle length in s"""
    i = int(np.searchsorted(LT, t, side='right')) - 1
    if i < 0:
        return 0.0, 17.1
    t0 = LT[i]
    t1 = LT[i + 1] if i + 1 < len(LT) else NEXT_AFTER_LAST
    if t0 == LAUNCH[-1][0] and t0 >= T_LOOP_RING:
        t1 = NEXT_AFTER_LAST
    return (t - t0) / (t1 - t0), t1 - t0


def recent_launches(t, n=6):
    i = int(np.searchsorted(LT, t, side='right'))
    return [LAUNCH[j] for j in range(max(0, i - n), i)]


# ---------------------------------------------------------------- envelopes for the picture
def ss(a, b, x):
    x = min(1.0, max(0.0, (x - a) / (b - a)))
    return x * x * (3 - 2 * x)


def jets_k(t):
    """0.3 idle -> 1 fired; back to idle for the loop"""
    return 0.3 + 0.7 * ss(T_JETS, T_JETS + 0.7, t) * (1 - ss(55.6, 58.8, t))


def haze_k(t):
    return 0.74 + 0.26 * ss(T_HAZE0, T_HAZE1, t) * (1 - ss(54.5, 59.0, t))


def band_k(oct_k, t):
    """colour band per octave (B-flat0..4) lights when its octave joins; fades in the outro"""
    t_on = {-4: T4, -3: T3, -2: T2, -1: T1, 0: T0}[oct_k]
    return ss(t_on - 0.1, t_on + 0.5, t) * (1 - ss(T_LAND + 0.6, 57.0, t))


def shimmer_k(t):
    """rings blur to shimmer (flutter) -> handed over to the colour bands"""
    return ss(T6 - 0.2, T5, t) * (1 - ss(T4 + 0.2, T4 + 1.6, t))


def beat_env(t, decay=0.11):
    """pulse on every climax beat (1 at the hit)"""
    if t < T_CLIMAX - 0.01 or t > T_LAND + 0.6:
        return 0.0
    ph = (t - T_CLIMAX) % BEAT
    return math.exp(-ph / decay)


def kick_env(t, decay=0.12):
    """pulse on every audible crest (29 s on)"""
    i = int(np.searchsorted(KICK_T, t, side='right')) - 1
    if i < 0:
        return 0.0
    return math.exp(-(t - KICK_T[i]) / decay)


KICK_T = np.array(KICKS)

# ---------------------------------------------------------------- captions
CAPTIONS = [
    (0.0, 3.95, ["*57 OCTAVES*", "DOWN"]),
    (4.05, 7.95, ["A *BLACK HOLE*", "IS HUMMING"]),
    (8.05, 10.95, ["SPACE IS SILENT...", "EXCEPT *HERE*"]),
    (11.05, 13.95, ["EACH *BUBBLE*", "= ONE *WAVE*"]),
    (14.05, 16.85, ["ONE WAVE EVERY", "*10 MILLION* YRS"]),
    (17.6, 19.95, ["-34 OCTAVES", "≈1 WAVE A *YEAR*"]),
    (20.4, 22.95, ["-25 OCTAVES", "≈1 WAVE A *DAY*"]),
    (23.25, 25.95, ["-21 OCTAVES", "≈1 WAVE AN *HOUR*"]),
    (26.35, 28.95, ["-15 OCTAVES", "≈1 A *MINUTE*"]),
    (T9 + 0.02, T8 - 0.05, ["-9 OCTAVES", "1 WAVE A *SECOND*"]),
    (T8 + 0.02, T7 - 0.05, ["-8 OCTAVES", "*109* BPM"]),
    (T7 + 0.02, T6 - 0.05, ["-7 OCTAVES", "*219* BPM"]),
    (T6 + 0.02, T4 - 0.05, ["BEATS → *?*"]),
    (T4 + 0.02, T_CLIMAX - 0.05, ["-4 OCTAVES:", "IT'S A *NOTE* NOW"]),
    (T_CLIMAX + 0.02, T_LAND - 0.05, ["A *B♭*", "FROM A BLACK HOLE"]),
    (T_LAND + 0.02, 56.2, ["HUM A *B♭*..."]),
    (56.3, 59.3, ["YOU'RE *57*", "OCTAVES *UP*"]),
]

# ---------------------------------------------------------------- music (original, B-flat major)
# chords per climax bar (MIDI roots), melody as (beat offset from T_CLIMAX, MIDI note, beats)
CLIMAX_CHORDS = [(46, [58, 62, 65]), (43, [55, 58, 62]), (39, [55, 58, 63]), (41, [57, 60, 63])]   # Bb Gm Eb F7
MELODY = [
    (0.0, 74, 1.0), (1.0, 72, 0.5), (1.5, 70, 0.5), (2.0, 65, 1.0), (3.0, 70, 1.0),          # Bb
    (4.0, 67, 1.0), (5.0, 70, 0.5), (5.5, 74, 0.5), (6.0, 72, 2.0),                          # Gm
    (8.0, 75, 1.0), (9.0, 74, 0.5), (9.5, 72, 0.5), (10.0, 70, 1.0), (11.0, 67, 1.0),        # Eb
    (12.0, 72, 1.0), (13.0, 74, 0.5), (13.5, 75, 0.5), (14.0, 72, 1.0), (15.0, 69, 1.0),     # F7
    (16.0, 70, 6.0),                                                                          # lands on Bb4
]
SWELL_CHORDS = [(46, [58, 62, 65, 70]), (43, [55, 58, 62, 67]), (39, [55, 58, 63, 67]), (41, [57, 60, 65, 69])]


if __name__ == '__main__':
    print(f"-57: f = {freq(-57):.3e} Hz, period = {period(-57) / YEAR / 1e6:.2f} M yr")
    for k in (-34, -25, -21, -15, -9, -8, -7, -6, -5, -4, 0):
        print(f"{k:4d}  {freq(k):10.4g} Hz   {readout(k)}")
    print(f"T8 {T8:.3f}  T7 {T7:.3f}  T6 {T6:.3f}  T5 {T5:.3f}  T4 {T4:.3f}  BPM {BPM:.2f}  land {T_LAND:.3f}")
    print('steps', len(STEPS), 'launches', len(LAUNCH), 'kicks', len(KICKS))
    for t in (0, 4, 12.4, 12.6, 20, 28.9, 30, 33, 36, 40, 45, 52, 55, 58, 59.99):
        rs = rings(t)
        print(f"t={t:5.2f} oct={octave(t):4d} v={speed(t):.2f} rings={len(rs)} "
              + ' '.join(f"{r:.2f}" for r, *_ in rs[-6:]), 'phase %.2f' % bubble_phase(t)[0])
    print('loop check: r(0) =', [round(r, 3) for r, *_ in rings(0.0)], ' r(60) =', [round(r, 3) for r, *_ in rings(60.0)])
    print('bubble phase 0 / 60:', bubble_phase(0.0), bubble_phase(59.999))
