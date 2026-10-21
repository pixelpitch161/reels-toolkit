"""Shared script for the 60 s video: modes, sweep, song notes, events."""
import math

FPS = 30
DUR = 60.0
NF = int(DUR * FPS)
HZ_PER_K = 20.5          # resonance frequency = HZ_PER_K * (n^2 + m^2)

# mode registry: name -> (n, m, s)
MODES = {
    'H1': (5, 7, 1), 'H2': (7, 11, 1),
    'S': (0, 3, -1),
    'R1': (0, 2, 1), 'R2': (1, 3, 1), 'R3': (2, 4, 1), 'R4': (1, 5, -1),
    'R5': (3, 5, 1), 'R6': (4, 6, 1), 'R7': (4, 7, 1), 'R8': (5, 8, 1),
    'R9': (5, 9, 1), 'R10': (7, 9, 1), 'R11': (8, 10, 1), 'R12': (10, 12, 1),
    'E5': (3, 5, 1), 'F#5': (2, 6, -1), 'G#5': (1, 5, -1), 'A5': (3, 6, 1),
    'B5': (4, 6, 1), 'C#6': (3, 7, 1), 'FIN': (5, 7, 1),
}


def kval(name):
    n, m, _ = MODES[name]
    return n * n + m * m


def res_hz(name):
    return HZ_PER_K * kval(name)


# ---------------------------------------------------------------- hook
HOOK = [('H1', 0.30, 2.10), ('H2', 2.15, 4.00)]
T_REVEAL = 4.2
# ---------------------------------------------------------------- slow motion
T_SLOW0, T_SLOW_FLING, T_SLOW1 = 7.3, 9.8, 14.3
# ---------------------------------------------------------------- sweep
SWEEP_NAMES = [f'R{i}' for i in range(1, 13)]
HOLDS = [1.6, 1.5, 1.4, 1.3, 1.2, 1.1, 1.0, 0.95, 0.9, 0.85, 0.8, 1.5]
GLIDE = 0.3
T_SWEEP0 = 14.8          # glide in from 60 Hz
SWEEP = []               # (name, t_arrive, t_leave)
_t = T_SWEEP0 + 0.4
for _nm, _h in zip(SWEEP_NAMES, HOLDS):
    SWEEP.append((_nm, _t, _t + _h))
    _t += _h + GLIDE
T_SWEEP1 = SWEEP[-1][2]  # 32.6
# ---------------------------------------------------------------- song
T_SCRAMBLE1 = 33.0
T_SONG = 35.4
EIGHTH = 1.0 / 3.0
BAR = 6 * EIGHTH
# Grieg, Morning Mood (Peer Gynt, 1875), opening theme, bars 1-4 (E major, 6/8)
# (position in eighths within the bar, pitch, length in eighths)
THEME = [
    [(0, 'B5', 1), (1, 'G#5', 1), (2, 'F#5', 1), (3, 'E5', 1), (4, 'F#5', 1), (5, 'G#5', 1)],
    [(0, 'B5', 0.85), (0.85, 'A5', 0.15), (1, 'G#5', 1), (2, 'F#5', 1), (3, 'E5', 1),
     (4, 'F#5', 0.5), (4.5, 'G#5', 0.5), (5, 'F#5', 0.5), (5.5, 'G#5', 0.5)],
    [(0, 'B5', 1), (1, 'G#5', 1), (2, 'B5', 1), (3, 'C#6', 1), (4, 'G#5', 1), (5, 'C#6', 1)],
    [(0, 'B5', 1), (1, 'G#5', 1), (2, 'F#5', 1), (3, 'E5', 3)],
]
NOTE_MIDI = {'E5': 76, 'F#5': 78, 'G#5': 80, 'A5': 81, 'B5': 83, 'C#6': 85, 'E6': 88}
SONG = []                # (t_on, t_off, pitch)
for _rep in range(2):
    for _b, _bar in enumerate(THEME):
        for _pos, _p, _ln in _bar:
            _t0 = T_SONG + (_rep * 4 + _b) * BAR + _pos * EIGHTH
            SONG.append((_t0, _t0 + _ln * EIGHTH, _p))
T_SUNRISE = T_SONG + 4 * BAR            # 43.4: second pass, full band
T_FINAL = T_SONG + 8 * BAR              # 51.4: final chord
T_FINAL_OFF = 56.4
T_SCRAMBLE2 = 58.9


def midi_hz(m):
    return 440.0 * 2 ** ((m - 69) / 12)


def ease(x):
    x = min(1.0, max(0.0, x))
    return x * x * x * (x * (6 * x - 15) + 10)


def sweep_freq(t):
    """Frequency of the tone generator during the sweep (log glides)."""
    if t < T_SWEEP0:
        return None
    prev_f, prev_t = 60.0, T_SWEEP0
    for nm, ta, tl in SWEEP:
        f = res_hz(nm)
        if t < ta:
            u = ease((t - prev_t) / (ta - prev_t))
            return math.exp(math.log(prev_f) + (math.log(f) - math.log(prev_f)) * u)
        if t < tl:
            return f
        prev_f, prev_t = f, tl + 0.0
        # glide starts when the hold ends
    return res_hz(SWEEP[-1][0])
