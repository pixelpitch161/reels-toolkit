"""ONE timeline for picture and sound: The Mountain King Escape (IDEAS #19).

Every melody note of our 60 s cut of Grieg's theme is one bounce of the ball off the wall of the ring it is
trapped in; the phrase-ending notes are the escapes (the ball slips through that ring's gap, the ring
shatters). The tempo accelerates (152 -> 265 BPM), so the bounces speed up with the music.

Times
- video time t in [0, 60).
- music time m: m = t, except the loop tail (t >= T_WRAP) where m = t - 60. The restart of the theme that
  the video loops into starts at m = M0 = -3.0 (video 57.0), so frame 0 is already mid-phrase.
- beats b: beat 0 = the restart's first note (m = M0). Tempo is BPM0 for HOLD beats, then accelerates.

Phrase plan (A/B/B#: see score.py). ring breaks ('mid' = last note of bar 2, 'end' = last note):
    0 A    restart, low pizzicato + bass               ring 1
    1 A    + bassoon                                  ring 1 escapes at the end (~8.8 s)
    2 B    up a fifth, strings join, tempo climbing   ring 2 (end)
    3 B                                               ring 3 (mid)
    4 A    octave doubling, timpani on breaks         ring 4 (mid), ring 5 (end)
    5 A    brass enters                               ring 6 (end)
    6 A                                               ring 7 (mid)
    7 B    full orchestra                             ring 8 (mid)
    8 B#   (D#)                                       ring 9 (mid)
    9 A    prestissimo, snare rolls                   ring 10 (mid), ring 11 (end)
   10 coda two stabs, the run, the final chord        ring 12 (final chord, ~52 s)
"""
import math
import numpy as np
import score as SC

FPS = 30
DUR = 60.0
NF = int(FPS * DUR)

BPM0, BPM1, CURVE = 152.0, 265.0, 0.9
HOLD = 32                       # beats at BPM0 (phrases 0-1)
M0 = -3.0                       # m-time of the restart's first note
T_WRAP = 56.0                   # video times >= this belong to the restart (m = t - 60)

PLAN = [  # (phrase, section, breaks)
    ('A', 0, ()), ('A', 1, ('end',)), ('B', 2, ('end',)), ('B', 2, ('mid',)), ('A', 3, ('mid', 'end')),
    ('A', 4, ('end',)), ('A', 4, ('mid',)), ('B', 5, ('mid',)), ('B#', 5, ('mid',)), ('A', 6, ('mid', 'end')),
    ('coda', 7, ('end',)),
]
SECTION_NAMES = ['pizz', 'bassoon', 'strings', 'octaves', 'brass', 'tutti', 'prestissimo', 'coda']
NRINGS = 12

# ------------------------------------------------------------------ tempo map
TOTAL_BEATS = sum(SC.phrase_len(p) for p, _, _ in PLAN) / 2.0
_B = np.arange(-16.0, TOTAL_BEATS + 24.0 + 1e-9, 1.0 / 128)
_u = np.clip((_B - HOLD) / (TOTAL_BEATS - HOLD), 0, 1)
_BPM = BPM0 * (BPM1 / BPM0) ** (_u ** CURVE)
_BPM[_B > TOTAL_BEATS] = BPM1
_dt = 60.0 / _BPM / 128
_M = np.concatenate([[0.0], np.cumsum(_dt[:-1])])
_M = _M - np.interp(0.0, _B, _M) + M0


def m_of_beat(b):
    return np.interp(b, _B, _M)


def beat_of_m(m):
    return np.interp(m, _M, _B)


def bpm_at_m(m):
    return float(np.interp(m, _M, _BPM))


def mtime(t):
    """video time -> music time"""
    return t - DUR if t >= T_WRAP else t


# ------------------------------------------------------------------ notes, rings, breaks
NOTES = []          # dicts: m, b, beats, dur, pitch, phrase, pi (phrase idx), ni, ring, kind, sec, bar, stab
RING_BREAK = {}     # ring k -> m of its escape note
PHRASE_START = []   # (m, b, phrase, section)
SEC_START = {}

ring = 1
b0 = 0.0
for pi, (ph, sec, brk) in enumerate(PLAN):
    notes = SC.PHRASES[ph]
    PHRASE_START.append((float(m_of_beat(b0)), b0, ph, sec))
    SEC_START.setdefault(sec, float(m_of_beat(b0)))
    e = 0
    idx_real = [i for i, (p, d) in enumerate(notes) if p is not None]
    ends = {}
    # last note of bar 2 (ends at eighth 16) and last note
    acc = 0
    for i, (p, d) in enumerate(notes):
        acc += d
        if acc == 16 and p is not None:
            ends['mid'] = i
    ends['end'] = idx_real[-1]
    esc = {ends[k] for k in brk}
    for i, (p, d) in enumerate(notes):
        if p is not None:
            b = b0 + e / 2.0
            kind = 'escape' if i in esc else 'bounce'
            if ph == 'coda' and i == idx_real[-1]:
                kind = 'final'
            NOTES.append(dict(m=float(m_of_beat(b)), b=b, beats=d / 2.0, dur=float(m_of_beat(b + d / 2.0) - m_of_beat(b)),
                              pitch=p, phrase=ph, pi=pi, ni=i, ring=ring, kind=kind, sec=sec, bar=e // 8,
                              stab=(ph == 'coda' and i in SC.CODA_STABS)))
            if kind != 'bounce':
                RING_BREAK[ring] = NOTES[-1]['m']
                ring += 1
        e += d
    b0 += e / 2.0
assert ring == NRINGS + 1, ring

M_FINAL = RING_BREAK[NRINGS]
M_END = float(m_of_beat(TOTAL_BEATS))
N_NOTES = len(NOTES)
NOTE_M = np.array([n['m'] for n in NOTES])
NOTE_RING = np.array([n['ring'] for n in NOTES])

# near-misses: (ring, target m) -> the nearest plain bounce on that ring (never its first bounce or the one
# right before its escape); 'pair' = two in a row
_NM_REQ = [(1, 7.7, 1), (4, 21.6, 1), (6, 29.6, 1), (8, 37.3, 2), (10, 44.3, 1), (12, M_FINAL - 1.2, 1)]
NEAR_MISS = []
for rk, tm, cnt in _NM_REQ:
    cand = [i for i, n in enumerate(NOTES) if n['ring'] == rk and n['kind'] == 'bounce'
            and i > 0 and NOTES[i - 1]['ring'] == rk and NOTES[i + 1]['kind'] == 'bounce']
    j = min(cand, key=lambda i: abs(NOTES[i]['m'] - tm))
    for c in range(cnt):
        NEAR_MISS.append(j + 2 * c)          # a pair = skim, cross the ring, skim the other edge
NEAR_MISS = sorted(set(NEAR_MISS))
for i in NEAR_MISS:
    NOTES[i]['near'] = True


def rings_left(m):
    return NRINGS - sum(1 for k, mb in RING_BREAK.items() if m >= mb)


def ring_at(m):
    """ring the ball is in at music time m (13 = escaped)"""
    for k in range(1, NRINGS + 1):
        if m < RING_BREAK[k]:
            return k
    return NRINGS + 1


def section_at(m):
    s = 0
    for m0, b, ph, sec in PHRASE_START:
        if m >= m0 - 1e-9:
            s = sec
    return s


def note_env(m, decay=0.12, kinds=None):
    """0..1 pulse on every note onset before m"""
    i = np.searchsorted(NOTE_M, m, 'right') - 1
    if i < 0:
        return 0.0
    if kinds and NOTES[i]['kind'] not in kinds:
        return 0.0
    return math.exp(-(m - NOTE_M[i]) / decay)


def break_env(m, decay=0.35):
    best = 0.0
    for k, mb in RING_BREAK.items():
        if 0 <= m - mb < 4 * decay:
            best = max(best, math.exp(-(m - mb) / decay))
    return best


# ------------------------------------------------------------------ the loop tail (video time)
T_FINAL = M_FINAL                      # final chord: ring 12 shatters, ball rockets up the shaft
T_SUMMIT = T_FINAL + 1.25              # ball out of the summit crack
T_BONG = T_FINAL + 2.25                # ball hits the giant ring
T_REENTER = 55.95                      # ball back down the shaft, into the cavern (above the rings)
T_FIRST = DUR + M0                     # 57.0: first bounce on ring 1 = first note of the restart
T_REFORM0, T_REFORM1 = T_FINAL + 1.6, 55.6    # shards fly back into rings

# ------------------------------------------------------------------ captions (video time)
_r = RING_BREAK
CAPTIONS = [
    (-0.5, 3.0, ["CAN IT *ESCAPE*", "BEFORE IT *ENDS?*"]),
    (3.1, 7.6, ["*12* RINGS", "*60* SECONDS"]),
    (_r[1] + 0.1, 11.9, ["*11* LEFT"]),
    (12.0, 19.8, ["EVERY *BOUNCE*", "= 1 *NOTE*"]),
    (_r[5] + 0.1, 28.4, ["*7* LEFT"]),
    (28.5, 36.6, ["IT'S *SPEEDING*", "*UP...*"]),
    (_r[9] + 0.1, 45.6, ["*3* LEFT", f"*{int(60 - _r[9] - 0.1)}* SECONDS"]),
    (_r[11] + 0.1, T_FINAL - 0.05, ["*1* LEFT..."]),
    (T_FINAL + 0.05, T_BONG - 0.1, ["*ESCAPED!*"]),
    (T_BONG, 58.9, ["*TRAPPED* AGAIN?"]),
    (59.5, 60.5, ["CAN IT *ESCAPE*", "BEFORE IT *ENDS?*"]),
]


def countdown(t):
    """HUD timer (seconds left), frozen at the escape, back to 60.00 for the restart"""
    if t < T_FINAL:
        return 60.0 - t
    if t < T_BONG + 0.4:
        return 60.0 - T_FINAL
    if t < T_REENTER:
        u = (t - T_BONG - 0.4) / (T_REENTER - T_BONG - 0.4)
        u = u * u * (3 - 2 * u)
        return (60.0 - T_FINAL) * (1 - u) + 60.0 * u
    return 60.0


def hud_rings(t):
    """RINGS LEFT counter: counts down on breaks, back up 0 -> 12 as the rings re-form"""
    m = mtime(t)
    if t >= T_WRAP:
        return NRINGS
    if t >= T_REFORM0:
        u = min(1.0, (t - T_REFORM0) / (T_REFORM1 - T_REFORM0))
        return int(round(u * NRINGS))
    return rings_left(m)


if __name__ == '__main__':
    print(f'{N_NOTES} notes, {TOTAL_BEATS:.0f} beats, final chord m = {M_FINAL:.2f}, end {M_END:.2f}')
    for k in range(1, NRINGS + 1):
        n = sum(1 for x in NOTES if x['ring'] == k and x['kind'] == 'bounce')
        print(f'ring {k:2d}: escape at {RING_BREAK[k]:6.2f} s  ({bpm_at_m(RING_BREAK[k]):5.1f} BPM)  {n} bounces')
    for m0, b, ph, sec in PHRASE_START:
        print(f'phrase {ph:4s} sec {SECTION_NAMES[sec]:11s} m {m0:6.2f}  beat {b:5.1f}  {bpm_at_m(m0):5.1f} BPM')
    print('near-misses at', [round(NOTES[i]['m'], 2) for i in NEAR_MISS], 'rings', [NOTES[i]['ring'] for i in NEAR_MISS])
    gaps = np.diff(NOTE_M)
    print(f'max bounce rate {1 / gaps.min():.1f}/s  (last bar {1 / gaps[-6:].min():.1f}/s)')
    for c in CAPTIONS:
        print(f'{c[0]:6.2f}-{c[1]:6.2f}  {" / ".join(c[2])}')
