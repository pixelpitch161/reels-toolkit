"""Shared script: temperature, chirps (= beats), bars, genres, captions.
Dolbear's law (NOAA form): chirps in 15 s + 40 = degrees F
-> chirps per minute = 4 (T - 40): 55F = 60/min ... 90F = 200/min."""
import math
import numpy as np

FPS = 30
DUR = 60.0
NF = int(DUR * FPS)

T_FF0, T_FF1 = 3.9, 7.65        # 15-second count shown at 4x speed
T_MUSIC = 8.05                  # first beat of the song
T_RISE0, T_RISE1 = 12.5, 47.5   # warming from 55 to 90 F
T_COOL_LEN = 3.0                # cold front: 90 -> 55


def chirp_rate(T):
    return (T - 40.0) / 15.0          # chirps per second


def bpm(T):
    return 4.0 * (T - 40.0)


# integrate beats from T_MUSIC to find a bar-aligned cool-down
_dt = 1 / 2400.0
_ts = np.arange(T_MUSIC, 56.0, _dt)


def _T_pre(t):
    if t < T_RISE0:
        return 55.0
    if t < T_RISE1:
        return 55.0 + 35.0 * (t - T_RISE0) / (T_RISE1 - T_RISE0)
    return 90.0


_phi = np.cumsum([chirp_rate(_T_pre(x)) * _dt for x in _ts])
BEATS_TO_COOL = int(math.ceil(np.interp(50.0, _ts, _phi) / 4.0) * 4)   # first bar line after 50 s
T_COOL0 = float(np.interp(BEATS_TO_COOL, _phi, _ts))
T_COOL1 = T_COOL0 + T_COOL_LEN


def temp(t):
    if t < T_COOL0:
        return _T_pre(t)
    if t < T_COOL1:
        u = (t - T_COOL0) / T_COOL_LEN
        u = 1 - (1 - u) ** 2
        return 90.0 - 35.0 * u
    return 55.0


def heat(t):
    return (temp(t) - 55.0) / 35.0


# --------------------------------------------------------------- chirps
def _chirps():
    out = []
    # hook: real time at 55 F (1 per second); phase chosen so the video loops
    t = HOOK_FIRST
    while t < T_FF0 - 0.2:
        out.append((t, 'real'))
        t += 1.0
    # the 15-second count at 4x: 15 chirps in 3.75 s
    for k in range(15):
        out.append((T_FF0 + 0.125 + k * 0.25, 'ff'))
    # song: beats follow the temperature until the cold front
    ts = np.arange(T_MUSIC, T_COOL0 + 1e-6, _dt)
    ph = np.concatenate([[0], np.cumsum([chirp_rate(temp(x)) * _dt for x in ts[:-1]])])
    for b in range(0, BEATS_TO_COOL):
        out.append((float(np.interp(b, ph, ts)), 'beat'))
    # cold front + outro: chirps slow down with the temperature
    t, phase = T_COOL0, 0.0
    tn = T_COOL0
    while tn < DUR + 1:
        phase += chirp_rate(temp(tn)) * _dt
        tn += _dt
        if phase >= 1.0:
            phase -= 1.0
            out.append((tn, 'cool'))
    return out


HOOK_FIRST = 0.68
CHIRPS = _chirps()
_last = [c[0] for c in CHIRPS if c[1] == 'cool' and c[0] < DUR][-1]
HOOK_FIRST = (_last + 1.0) - DUR
CHIRPS = _chirps()
BEAT_T = np.array([c[0] for c in CHIRPS if c[1] == 'beat'])
BAR_T = BEAT_T[::4]
NBARS = len(BAR_T)

GENRES = [  # (min temp, name)
    (55, 'LO-FI'), (60, 'HIP-HOP'), (65, 'FUNK'), (70, 'HOUSE'),
    (75, 'TRANCE'), (80, 'DRUM & BASS'), (85, 'CHIPTUNE'),
]


def genre_of_bar(b):
    T = temp(BAR_T[b])
    g = 0
    for k, (tmin, _) in enumerate(GENRES):
        if T >= tmin - 0.6:
            g = k
    return g


BAR_GENRE = [genre_of_bar(b) for b in range(NBARS)]


def bar_at(t):
    if t < BAR_T[0]:
        return -1
    return int(np.searchsorted(BAR_T, t, side='right') - 1)


def chirps_in_15s(T):
    return T - 40.0


if __name__ == '__main__':
    print('cool front', round(T_COOL0, 3), '->', round(T_COOL1, 3), 'beats', BEATS_TO_COOL, 'bars', NBARS)
    for b in range(NBARS):
        print(b, round(BAR_T[b], 2), round(temp(BAR_T[b]), 1), round(bpm(temp(BAR_T[b]))), GENRES[BAR_GENRE[b]][1])
    print('chirps total', len(CHIRPS), 'outro chirps', [round(c[0], 2) for c in CHIRPS if c[1] == 'cool'])
