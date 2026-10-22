"""IDEAS #2  The Lowest Note in the Universe, v2 re-cut.  ONE timeline for picture AND sound.

Science (Chandra, 2003): jets from the black hole at the heart of the Perseus cluster inflate bubbles
in the cluster's hot gas. Each bubble pushes out like a piston and launches a pressure wave. The
bubbles come at regular intervals, so the waves make a steady tone: a B-flat about 57 octaves below
the B-flat above middle C (B-flat4 = 466.16 Hz). One crest every ~9.8 million years.

v2 (approved rework, docs/IDEAS.md "2 - v2 rework"): open on the loudest moment (the climax), tape-stop
to real speed, then speed the black hole back up in big beat-synced jumps, so there is rhythm from
frame 0 and the explanation runs over the beats, not before them.

THE GRID: one beat = the hole's note 8 octaves down (0.5492 s, 109.3 BPM). Every octave step from -9
lands on it exactly: -9 = a crest every 2 beats, -8 every beat, -7 every 8th, -6 every 16th (7.3 Hz),
-5 every 32nd (14.6 Hz), -4 every 64th (29.1 Hz = B-flat0, heard as a pitch). The drop is placed so
that 60.0 s is a bar line 22 bars later, and the cold open (t = 0) is the bar after it: the video loops."""
import math
import numpy as np

FPS = 30
DUR = 60.0
NF = int(FPS * DUR)

BB4 = 466.1638                 # B-flat4 in Hz (the B-flat above middle C)
YEAR = 365.25 * 86400.0


def freq(k):
    """frequency of the note k octaves from B-flat4"""
    return BB4 * 2.0 ** k


def period(k):
    return 1.0 / freq(k)


def speedup(k):
    """how much the real thing is sped up at octave k (2^(k+57))"""
    return 2.0 ** (k + 57)


# ---------------------------------------------------------------- the grid
BEAT = period(-8)               # 0.549163 s = 109.26 BPM
BPM = 60.0 / BEAT
BAR = 4 * BEAT
T_DROP = DUR - 88 * BEAT        # 11.674 s: 88 beats (22 bars) before the loop point


def G(n):
    """main grid: beat n after the drop (n may be negative or fractional)"""
    return T_DROP + n * BEAT


def C(k):
    """cold-open grid: beat k after t = 0 (= beat 88 + k of the main grid, one loop later)"""
    return k * BEAT


# ---------------------------------------------------------------- sections
N_COLD = 6                      # cold-open beats 0..5 (climax), then the tape-stop
T_STOP = C(5) + 0.05            # 2.796: tape-stop starts (the beat-5 hit dives with it)
T_THUD = G(-15)                 # 3.437: lands at real speed (sub thud, muffled bed starts)
T_YEAR, T_DAY, T_HOUR, T_MIN = G(-12), G(-9), G(-6), G(-3)     # spin-up slams, 3 beats apart
T9 = G(0)                       # the drop: octave -9, 1 crest per second (55 BPM)
T8 = G(12)                      # -8: 109 BPM
T7 = G(24)                      # -7: 219 BPM
T6 = G(32)                      # -6: flutter 7.3 Hz
T5 = G(36)                      # -5: flutter 14.6 Hz
T4 = G(40)                      # -4: 29.1 Hz = B-flat0, a note
T3, T2, T1, T0 = G(44), G(48), G(52), G(56)        # octaves stack, one per bar: B-flat1, 2, 3, 4
T_CLIMAX = G(60)                # the original melody over the full stack (the cold-open image re-forms)
T_LAND = G(76)                  # melody lands on B-flat4; hum prompt
T_BUILD = G(84)                 # last bar builds and slams into the cold open
T_HATS = G(8)                   # hats enter
SPIN_HITS = [(T_YEAR, -34), (T_DAY, -25), (T_HOUR, -21), (T_MIN, -15)]


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


FLICKER = [G(n) for n in range(77, 84)]            # hum section: counter flashes back to -57 on each beat
STEPS = (_climb(T_STOP, T_THUD - 0.05, 0, -57)     # slams down with the tape-stop
         + _climb(T_YEAR - 0.6, T_YEAR, -57, -34) + _climb(T_DAY - 0.5, T_DAY, -34, -25)
         + _climb(T_HOUR - 0.4, T_HOUR, -25, -21) + _climb(T_MIN - 0.45, T_MIN, -21, -15)
         + _climb(T9 - 1.1, T9, -15, -9)
         + [(T8, -8), (T7, -7), (T6, -6), (T5, -5), (T4, -4), (T3, -3), (T2, -2), (T1, -1), (T0, 0)])
for tf in FLICKER:
    STEPS += [(tf, -57), (tf + BEAT * 0.3, 0)]
STEPS.sort()
STEP_T = np.array([s[0] for s in STEPS])
STEP_K = np.array([s[1] for s in STEPS])


def octave(t):
    i = int(np.searchsorted(STEP_T, t, side='right')) - 1
    return 0 if i < 0 else int(STEP_K[i])


def octave_smooth(t):
    """continuous octave for the ruler marker: races between steps, holds on plateaus"""
    if t < STEP_T[0]:
        return 0.0
    i = int(np.searchsorted(STEP_T, t, side='right')) - 1
    k = float(STEP_K[i])
    if i + 1 < len(STEP_T):
        dt = STEP_T[i + 1] - STEP_T[i]
        if dt < 0.12 and abs(STEP_K[i + 1] - STEP_K[i]) == 1:       # inside a fast climb: glide
            k += (STEP_K[i + 1] - STEP_K[i]) * (t - STEP_T[i]) / dt
    return k


def last_step(t):
    """time of the most recent counter step (for blinks)"""
    i = int(np.searchsorted(STEP_T, t, side='right')) - 1
    return -1e9 if i < 0 else float(STEP_T[i])


def readout(k):
    """the period / rate line on the HUD for octave k"""
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


_NAMES = [(1e15, 'QUADRILLION'), (1e12, 'TRILLION'), (1e9, 'BILLION'), (1e6, 'MILLION'), (1e3, 'THOUSAND')]


def speed_line(k):
    if k <= -57:
        return "REAL SPEED"
    x = speedup(k)
    for v, nm in _NAMES:
        if x >= v:
            q = x / v
            return f"SPED UP {q:.0f} {nm}×" if q >= 10 else f"SPED UP {q:.1f} {nm}×"
    return f"SPED UP {x:.0f}×"


NOTE_NAME = {-4: "B♭0", -3: "B♭1", -2: "B♭2", -1: "B♭3", 0: "B♭4"}

# ---------------------------------------------------------------- wave crests (ring launches = kicks)
# (time, amplitude, group).  O cold open / pre-roll, A spin-up, B audible beats, F flutter, C climax + outro
LAUNCH = []
for k in range(N_COLD):
    LAUNCH.append((C(k), 1.0, 'O'))
for n in range(80, 88):                                       # the end's last beats, one loop earlier
    LAUNCH.append((G(n) - DUR, 1.0, 'O'))
for th, _ in SPIN_HITS:
    LAUNCH.append((th, 1.0, 'A'))
for b in (-2.0, -1.5, -1.0, -0.75, -0.5, -0.25):              # time-lapse build into the drop
    LAUNCH.append((G(b), 0.6, 'A'))
for k, n0, n1, step in ((-9, 0, 12, 2.0), (-8, 12, 24, 1.0), (-7, 24, 32, 0.5), (-6, 32, 36, 0.25), (-5, 36, 40, 0.125)):
    n = n0
    while n < n1 - 1e-9:
        LAUNCH.append((G(n), 1.0 if k >= -7 else 0.55, 'B' if k >= -7 else 'F'))
        n += step
for n in range(40, 88):                                       # from B-flat0 on: a crest-kick every beat
    LAUNCH.append((G(n), 1.15 if n in (40, 60, 76) else 1.0, 'C'))
LAUNCH.sort()
LT = np.array([l[0] for l in LAUNCH])
NEXT_AFTER_LAST = DUR                                         # the next crest after the last one = the loop point

KICKS = [t for t, a, g in LAUNCH if 0.0 <= t < DUR]           # every crest is heard
KICK_T = np.array([t for t, a, g in LAUNCH if g != 'F'])
R0 = 0.5                       # rings start at the bubble's edge


# ---------------------------------------------------------------- tape clock (scene time)
# The cold open runs at climax speed, the tape-stop freezes everything, the spin-up whips it round.
# Over the 60 s the scene clock advances exactly 120 s (one extra lap), so orbits loop seamlessly.
def _ss(a, b, x):
    x = np.clip((x - a) / (b - a), 0.0, 1.0)
    return x * x * (3 - 2 * x)


_TG = np.arange(-10.0, 80.0, 1.0 / 600)


def _rate_base(t):
    stop = (1 - _ss(T_STOP, T_THUD, t)) ** 1.5              # tape-stop curve
    wake = _ss(T_YEAR - 0.2, T9, t)                           # back to normal by the drop
    return np.maximum(stop, wake) * (t < 30) + (t >= 30) + 0.02 * ((t > T_STOP) & (t < T9))


def _whip(t):
    return np.exp(-((t - 8.6) / 1.9) ** 2) * (t > T_YEAR - 0.6) * (t < T9 + 0.6)


_RB = _rate_base(_TG)
_W = _whip(_TG)
_m = (_TG >= 0) & (_TG < DUR)
_need = 2 * DUR - _RB[_m].sum() / 600
WHIP = _need / (_W[_m].sum() / 600)
_RATE = _RB + WHIP * _W
_RATE[_TG < 0] = 1.0
_RATE[_TG >= DUR] = 1.0
_CLK = np.concatenate([[0.0], np.cumsum((_RATE[1:] + _RATE[:-1]) * 0.5 / 600)])
_CLK -= np.interp(0.0, _TG, _CLK)


def clock(t):
    """scene time for orbits / spins / drifts. clock(60) = 120 exactly (one extra lap)"""
    if t >= DUR:
        return clock(t - DUR) + 2 * DUR
    if t < 0:
        return clock(t + DUR) - 2 * DUR
    return float(np.interp(t, _TG, _CLK))


def clock_rate(t):
    return float(np.interp(t % DUR, _TG, _RATE))


# ---------------------------------------------------------------- ring speed -> travel s(t)
def _vkeys():
    pre = T_STOP
    return [(-12.0, 1.2), (pre, 1.2), (pre + 0.25, 0.5), (T_THUD, 0.02), (T_YEAR - 0.1, 0.02),
            (T_YEAR + 0.15, 0.22), (T_DAY + 0.15, 0.36), (T_HOUR + 0.15, 0.5), (T_MIN + 0.15, 0.62),
            (T9 - 0.4, 1.0), (T9 + 0.4, 0.75),
            (T8 - 0.1, 0.75), (T8 + 0.15, 1.2), (T7 - 0.1, 1.2), (T7 + 0.15, 1.8), (T6 - 0.1, 1.8), (T6 + 0.15, 2.4),
            (T5 - 0.1, 2.4), (T5 + 0.15, 3.0), (T4 + 0.5, 3.0), (T4 + 2.5, 1.2), (DUR + pre, 1.2), (80.0, 1.2)]


def _v(t):
    ks = _vkeys()
    for (a, va), (b, vb) in zip(ks, ks[1:]):
        if t <= b:
            u = (t - a) / (b - a)
            u = u * u * (3 - 2 * u)
            return va + (vb - va) * u
    return ks[-1][1]


_VG = np.array([_v(x) for x in _TG])
_SG = np.concatenate([[0.0], np.cumsum((_VG[1:] + _VG[:-1]) * 0.5 / 600)])
L_S = np.interp(LT, _TG, _SG)


def travel(t):
    return float(np.interp(t, _TG, _SG))


def speed(t):
    return float(np.interp(t, _TG, _VG))


def group_env(g, t):
    """time envelope per group"""
    if g == 'O':                                   # cold-open rings freeze, then fade at real speed
        return 1.0 - _ease((t - 3.6) / 1.3) if t < 30 else 1.0
    if g in ('B', 'F'):
        return 1.0 - _ease((t - (T4 - 0.3)) / 1.2)
    if g == 'A':
        return 1.0 - _ease((t - (T8 - 0.5)) / 1.5)
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
    return out


def bubble_phase(t):
    """0..1 through the current inflate cycle (launch to launch); cycle length in s"""
    i = int(np.searchsorted(LT, t, side='right')) - 1
    if i < 0:
        return 0.0, BEAT
    t0 = LT[i]
    t1 = LT[i + 1] if i + 1 < len(LT) else NEXT_AFTER_LAST
    return min(1.0, (t - t0) / (t1 - t0)), t1 - t0


def recent_launches(t, n=6):
    i = int(np.searchsorted(LT, t, side='right'))
    return [LAUNCH[j] for j in range(max(0, i - n), i)]


# ---------------------------------------------------------------- envelopes for the picture
def ss(a, b, x):
    x = min(1.0, max(0.0, (x - a) / (b - a)))
    return x * x * (3 - 2 * x)


def jets_k(t):
    """full in the cold open, idle at real speed, firing again with the spin-up"""
    return max(1 - 0.7 * ss(T_STOP, T_THUD, t), 0.3 + 0.3 * ss(T_YEAR - 0.2, T_YEAR + 0.3, t) + 0.4 * ss(T9 - 1.0, T9, t))


def haze_k(t):
    return 1.0 - 0.2 * ss(T_STOP, T_THUD, t) * (1 - ss(T_YEAR, T9, t))


def band_k(oct_k, t):
    """colour band per octave (B-flat0..4): on in the cold open, gone at real speed, back as each octave joins"""
    t_on = {-4: T4, -3: T3, -2: T2, -1: T1, 0: T0}[oct_k]
    cold = 1 - ss(T_STOP, T_THUD, t)
    return max(cold, ss(t_on - 0.1, t_on + 0.5, t))


def shimmer_k(t):
    """rings blur to shimmer (flutter) -> handed over to the colour bands"""
    return ss(T6 - 0.2, T5, t) * (1 - ss(T4 + 0.2, T4 + 1.6, t))


def freeze_k(t):
    """the 'real speed' look: 1 while frozen"""
    return ss(T_STOP, T_THUD, t) * (1 - ss(T_YEAR - 0.3, T_YEAR + 0.4, t))


def beat_env(t, decay=0.11):
    """pulse on every beat of the climax-style sections (cold open, B-flat0 to the end)"""
    if t < T_STOP:
        ph = t % BEAT
    elif t >= T4 - 0.01:
        ph = (t - T_DROP) % BEAT
    else:
        return 0.0
    return math.exp(-ph / decay)


def kick_env(t, decay=0.12):
    """pulse on every crest that is a kick"""
    i = int(np.searchsorted(KICK_T, t, side='right')) - 1
    if i < 0:
        return 0.0
    if KICK_T[i] < T_STOP and t > T_STOP:          # the tape-stop freezes the last cold-open pulse
        return math.exp(-(T_STOP - KICK_T[i]) / decay) * (1 - ss(T_STOP, T_THUD, t))
    return math.exp(-(t - KICK_T[i]) / decay)


def shake_amp(t):
    """camera shake strength (deg) per kick: big in the climax/cold open, frozen at real speed"""
    if t < T_STOP:
        return 0.75
    if t < T_YEAR - 0.05:                          # frozen mid-shake by the tape-stop, then settles
        return 0.75 * (1 - ss(T_THUD, T_THUD + 0.5, t))
    if t < T9:
        return 0.35
    if t < T8:
        return 0.4
    if t < T4:
        return 0.55
    if t < T_CLIMAX:
        return 0.55
    if t < T_LAND:
        return 0.75
    if t < T_BUILD:
        return 0.5
    return 0.5 + 0.25 * ss(T_BUILD, DUR, t)


def ripple_amp(g, a):
    return {'O': 1.0, 'A': 0.8, 'B': 0.85, 'F': 0.0, 'C': 1.0}[g] * min(1.0, a)


# ---------------------------------------------------------------- captions
CAPTIONS = [
    (0.0, 2.72, ["THIS NOTE IS", "*57 OCTAVES* DOWN"]),
    (2.80, T_YEAR - 0.05, ["REAL SPEED:", "1 WAVE / *10M* YRS"]),
    (T_YEAR + 0.02, T_DAY - 0.05, ["≈1 WAVE A *YEAR*"]),
    (T_DAY + 0.02, T_HOUR - 0.05, ["≈1 A *DAY*"]),
    (T_HOUR + 0.02, T_MIN - 0.05, ["≈1 AN *HOUR*"]),
    (T_MIN + 0.02, T9 - 0.05, ["≈1 A *MINUTE*"]),
    (T9 + 0.02, G(4) - 0.05, ["SPED UP", "*281 TRILLION* ×"]),
    (G(4) + 0.02, G(8) - 0.05, ["JETS BLOW", "*BUBBLES*"]),
    (G(8) + 0.02, T8 - 0.05, ["EACH POP =", "*1 WAVE*"]),
    (T8 + 0.02, G(15) - 0.05, ["SPED UP *2×*:", "*109* BPM"]),
    (G(15) + 0.02, G(20) - 0.05, ["SPACE IS SILENT", "EXCEPT IN *HOT GAS*"]),
    (G(20) + 0.02, T7 - 0.05, ["THE GAS CARRIES", "THE *WAVES*"]),
    (T7 + 0.02, T6 - 0.05, ["SPED UP *2×*:", "*219* BPM"]),
    (T6 + 0.02, T5 - 0.05, ["TOO FAST", "TO *COUNT*..."]),
    (T5 + 0.02, T4 - 0.05, ["BEATS → *?*"]),
    (T4 + 0.02, T3 - 0.05, ["IT'S A NOTE:", "*B♭*"]),
    (T3 + 0.02, T1 - 0.05, ["EACH DOUBLING =", "*1 OCTAVE* UP"]),
    (T1 + 0.02, T_CLIMAX - 0.05, ["SAME BLACK HOLE", "*4 OCTAVES* UP"]),
    (T_CLIMAX + 0.02, G(68) - 0.05, ["SPED UP *144*", "*QUADRILLION* ×"]),
    (G(68) + 0.02, T_LAND - 0.05, ["A *B♭* FROM", "A *BLACK HOLE*"]),
    (T_LAND + 0.02, G(82) - 0.05, ["HUM A *B♭*..."]),
    (G(82) + 0.02, DUR - 0.08, ["YOU'RE *57*", "OCTAVES *UP*"]),
]

# ---------------------------------------------------------------- music (original, B-flat major)
# chords per bar (MIDI roots + voicing), melody as (beat offset, MIDI note, beats)
CHORDS = [(46, [58, 62, 65]), (43, [55, 58, 62]), (39, [55, 58, 63]), (41, [57, 60, 63])]   # Bb Gm Eb F7
MELODY = [
    (0.0, 74, 1.0), (1.0, 72, 0.5), (1.5, 70, 0.5), (2.0, 65, 1.0), (3.0, 70, 1.0),          # Bb
    (4.0, 67, 1.0), (5.0, 70, 0.5), (5.5, 74, 0.5), (6.0, 72, 2.0),                          # Gm
    (8.0, 75, 1.0), (9.0, 74, 0.5), (9.5, 72, 0.5), (10.0, 70, 1.0), (11.0, 67, 1.0),        # Eb
    (12.0, 72, 1.0), (13.0, 74, 0.5), (13.5, 75, 0.5), (14.0, 72, 1.0), (15.0, 69, 1.0),     # F7
    (16.0, 70, 6.0),                                                                          # lands on Bb4
]


def chord_at(t):
    """chord index for main-grid time t (one chord per bar from the drop; cold open = bar 0)"""
    if t < T_STOP:
        return int(t // BAR) % 4
    return int(math.floor((t - T_DROP) / BAR)) % 4


if __name__ == '__main__':
    print(f"-57: f = {freq(-57):.3e} Hz, period = {period(-57) / YEAR / 1e6:.2f} M yr")
    print(f"BEAT {BEAT:.6f}  BPM {BPM:.3f}  T_DROP {T_DROP:.3f}  WHIP {WHIP:.2f}")
    for nm in ('T_STOP', 'T_THUD', 'T_YEAR', 'T_DAY', 'T_HOUR', 'T_MIN', 'T9', 'T8', 'T7', 'T6', 'T5', 'T4', 'T3', 'T2',
               'T1', 'T0', 'T_CLIMAX', 'T_LAND', 'T_BUILD'):
        print(f"  {nm:9s} {globals()[nm]:7.3f}")
    for k in (-57, -34, -25, -21, -15, -9, -8, -7, -6, -5, -4, 0):
        print(f"{k:4d}  {freq(k):10.4g} Hz   {readout(k):10s} {speed_line(k)}")
    print('steps', len(STEPS), 'launches', len(LAUNCH), 'kicks', len(KICKS))
    print('clock(0)', clock(0.0), 'clock(60-)', clock(59.9999), 'rate at 0/60', clock_rate(0.0), clock_rate(59.99))
    print('loop check: r(0)  =', [round(r, 3) for r, *_ in rings(0.0)])
    print('            r(60) =', [round(r, 3) for r, *_ in rings(60.0)])
    print('bubble phase 0 / 60-:', bubble_phase(0.0), bubble_phase(59.999))
    for t in (0, 2.0, 3.0, 4.0, 6.0, 9.0, 11.0, 12.5, 20, 27, 31, 35, 45, 55, 59.9):
        rs = rings(t)
        print(f"t={t:5.2f} oct={octave(t):4d} ({octave_smooth(t):6.2f}) v={speed(t):.2f} clk={clock(t):6.2f} rings={len(rs)} "
              + ' '.join(f"{r:.2f}" for r, *_ in rs[-6:]))
