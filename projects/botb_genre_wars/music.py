"""The four genre arrangements of ONE original song (A minor, 118.03 BPM, | Am | F | Dm | E |, an 8-bar melody)
and the gating that turns each genre's moves into its notes. Imported by audio.py (what to play) and render.py
(cone pumps + note sparkles on the wall), so picture and sound use one note list.

Every genre plays the same progression, tempo and melody, each in its own style, so the stems blend and the mix
can morph from one genre to another:

    genre       drums                          bass                       harmony                     lead (melody)
    ORCHESTRA   timpani, suspended cymbal      galloping cellos           violin pad + brass stabs    french horns
    ROCK        kick/snare/8th hats, crash     8th pick bass              3-3-2 power chords          distorted guitar
    EDM         four-on-the-floor, clap, hats  off-beat sub bass          supersaw stabs              saw lead
    JAZZ        swung ride, brush, pedal hat   walking upright bass       rootless piano comping      tenor sax

Every move = a note: each layer's note on the grid only plays if its genre claimed a tile since that layer's
previous note (busy genre = full part, stalled genre = gaps); a move that didn't land plays a ghost note. The
sparkle is drawn on the tile the move claimed. Opening (frame 0 = the empty stage, the seeds drop): every few
claims ring a chord tone off the grid in the genre's own timbre (chaos, in key), locking to the grid by bar 2.
After the win, the winner plays its whole arrangement ungated (the solo)."""
import os
import numpy as np

import timeline as T

HERE = os.path.dirname(os.path.abspath(__file__))
# (name, root midi (3rd octave), triad, rootless jazz voicing, chord tones for arps)
CHORDS = [('Am', 57, [57, 60, 64], [60, 64, 67, 71]),          # Am9 (C E G B)
          ('F', 53, [53, 57, 60], [57, 60, 64, 67]),           # Fmaj9 (A C E G)
          ('Dm', 50, [50, 53, 57], [53, 57, 60, 64]),          # Dm9 (F A C E)
          ('E', 52, [52, 56, 59], [56, 59, 62, 65])]           # E7b9 (G# B D F)
LOCK0, LOCK1 = T.GRID0 + T.BAR, T.GRID0 + 2 * T.BAR   # free -> grid crossfade: 2.03 -> 4.07 s (bar lines)
NBARS = int(round(T.DUR / T.BAR))                      # 30
LAYERS = ['drums', 'bass', 'harm', 'lead']

# the melody: per bar of the 8-bar tune, (16th step, midi, length in 16ths); A harmonic minor, dramatic not sweet
MELODY = [
    [(0, 69, 2), (2, 72, 2), (4, 76, 4), (8, 81, 4), (12, 80, 2), (14, 76, 2)],
    [(0, 77, 6), (6, 76, 2), (8, 72, 4), (12, 69, 4)],
    [(0, 74, 2), (2, 77, 2), (4, 81, 4), (8, 77, 4), (12, 76, 2), (14, 74, 2)],
    [(0, 71, 4), (4, 68, 4), (8, 76, 8)],
    [(0, 81, 3), (3, 83, 1), (4, 84, 4), (8, 83, 2), (10, 81, 2), (12, 80, 4)],
    [(0, 81, 4), (4, 77, 4), (8, 72, 2), (10, 77, 2), (12, 76, 4)],
    [(0, 74, 4), (4, 77, 2), (6, 74, 2), (8, 69, 4), (12, 71, 2), (14, 72, 2)],
    [(0, 71, 6), (6, 68, 2), (8, 64, 4), (12, 68, 2), (14, 71, 2)],
]
LEAD_OCT = [-12, 0, 0, -12]           # horns and sax an octave down, guitar and saw lead as written
SW = 2 / 3                            # jazz swing: the off-beat 8th lands 2/3 through the beat
WALK = {0: [45, 48, 52, 42], 1: [41, 45, 48, 39], 2: [38, 41, 45, 41], 3: [40, 44, 47, 46]}   # root 3rd 5th approach
GATE_MIN = [0.0, 0.0, T.BEAT, 0.0]    # EDM rings claim in one burst per beat: listen over a whole beat


def chord(bar):
    return CHORDS[bar % 4]


def low(m, floor=28):
    while m < floor:
        m += 12
    return m


def part_notes(q, layer, bar):
    """grid notes of genre q's layer in one bar: [(16th step (float), midi, length 16ths, kind)]"""
    name, root, tri, rootless = chord(bar)
    cyc = bar % 4
    out = []
    if layer == 'lead':
        for s, m, L in MELODY[bar % 8]:
            st = s + (SW * 2 - 1) * 2 if (q == 3 and s % 4 == 2) else s     # jazz swings its off-beat 8ths
            out.append((st, m + LEAD_OCT[q], L, ['horn', 'glead', 'elead', 'sax'][q]))
        return out
    if q == 0:                                               # ---------------- ORCHESTRA
        if layer == 'drums':
            out.append((0, root - 12, 4, 'timp'))
            out.append((8, root - 12 - 5, 4, 'timp'))
            if cyc == 0:
                out.append((0, 0, 8, 'ocym'))
            if cyc == 3:
                for s in range(10, 16):
                    out.append((s, root - 12 - 5, 1, 'troll'))
        elif layer == 'bass':                                # galloping low strings: 16-16-8
            for b in range(4):
                for s, L in ((0, 1), (1, 1), (2, 2)):
                    out.append((4 * b + s, low(root - 12, 36), L, 'cello'))
        elif layer == 'harm':
            out.append((0, root, 16, 'vlns'))
            out.append((0, root, 3, 'brass'))
            out.append((6, root, 2, 'brass'))
            if cyc in (1, 3):
                out.append((10, root, 2, 'brass'))
    elif q == 1:                                             # ---------------- ROCK
        if layer == 'drums':
            for s in range(0, 16, 2):
                out.append((s, 0, 1, 'rhat'))
            for s in (0, 8, 10):
                out.append((s, 0, 1, 'rkick'))
            for s in (4, 12):
                out.append((s, 0, 1, 'rsnare'))
            if cyc == 0:
                out.append((0, 0, 8, 'crash'))
            if cyc == 3:
                out += [(13, 47, 1, 'rtom'), (14, 43, 1, 'rtom'), (15, 40, 1, 'rtom')]
        elif layer == 'bass':
            for s in range(0, 16, 2):
                out.append((s, low(root - 24), 2, 'rbass'))
        elif layer == 'harm':                                # 3-3-2 accents ring open, the rest palm-muted
            for s in range(0, 16, 2):
                if s in (0, 6, 12):
                    out.append((s, root - 12, 4 if s < 12 else 3, 'pchord'))
                else:
                    out.append((s, root - 12, 1, 'pmute'))
    elif q == 2:                                             # ---------------- EDM
        if layer == 'drums':
            for s in (0, 4, 8, 12):
                out.append((s, 0, 1, 'ekick'))
            for s in (4, 12):
                out.append((s, 0, 1, 'clap'))
            for s in (2, 6, 10, 14):
                out.append((s, 0, 1, 'eohat'))
            for s in range(1, 16, 2):
                out.append((s, 0, 1, 'ehat'))
            if cyc == 3:
                for s in (8, 10, 13, 14, 15):
                    out.append((s, 0, 1, 'clap'))
        elif layer == 'bass':
            for s in (2, 6, 10, 14):
                out.append((s, low(root - 24), 2, 'ebass'))
        elif layer == 'harm':
            for s in (0, 3, 6, 10, 12):
                out.append((s, root, 2, 'saw'))
    elif q == 3:                                             # ---------------- JAZZ
        if layer == 'drums':
            for s in (0, 4, 8, 12):
                out.append((s, 0, 1, 'ride'))
            for s in (4, 12):
                out.append((s + SW * 4 - 2 + 2, 0, 1, 'ride'))            # the swung skip note
                out.append((s, 0, 1, 'phat'))
            for s in (0, 8):
                out.append((s, 0, 1, 'jkick'))
            out.append((8 + SW * 4, 0, 1, 'brush'))
            if bar % 2:
                out.append((12 + SW * 4, 0, 1, 'brush'))
        elif layer == 'bass':
            for b, m in enumerate(WALK[cyc]):
                out.append((4 * b, m, 4, 'ubass'))
        elif layer == 'harm':                                # Charleston comping, varied every other bar
            if bar % 2 == 0:
                out += [(0, root, 3, 'piano'), (4 + SW * 4, root, 3, 'piano')]
            else:
                out += [(SW * 4, root, 3, 'piano'), (8 + SW * 4, root, 4, 'piano')]
    return out


def _claims(prefix='ev'):
    """per genre: sorted video times of its claims ('ev') or attempted moves ('at') + their tiles"""
    D = T.D
    ts = np.array([T.video_time(t) for t in D[prefix + '_t']])
    k = D[prefix + '_k'].astype(int)
    out = []
    for q in range(4):
        m = k == q + 1
        o = np.argsort(ts[m], kind='stable')
        out.append((ts[m][o], D[prefix + '_i'][m][o].astype(int), D[prefix + '_j'][m][o].astype(int)))
    return out


DT = np.dtype([('t', 'f8'), ('team', 'i4'), ('layer', 'U5'), ('kind', 'U6'), ('midi', 'i4'), ('ch', 'i4'),
               ('dur', 'f8'), ('vel', 'f8'), ('i', 'i4'), ('j', 'i4'), ('src', 'U4')])


def build():
    C = _claims('ev')
    AT = _claims('at')
    rng = np.random.default_rng(118)
    rows = []
    win = T.WINNER
    for q in range(4):
        ts, ii, jj = C[q]
        ats, aii, ajj = AT[q]
        for layer in LAYERS:
            last_slot = None
            slots = []
            for bar in range(NBARS):
                for s, m, L, kind in part_notes(q, layer, bar):
                    slots.append((T.GRID0 + bar * T.BAR + s * T.S16, m, L, kind, bar))
            slots.sort(key=lambda a: a[0])
            for t, m, L, kind, bar in slots:
                dur = L * T.S16
                solo = T.T_WIN is not None and T.T_WIN <= t < T.T_CRACK and q == win
                if not T.alive(q, t) and not solo:
                    continue
                if t < LOCK0 or t >= T.T_CRACK:
                    continue
                look = T.BAR if layer == 'harm' and kind == 'vlns' else T.BEAT
                w0 = last_slot if last_slot is not None else t - T.BEAT
                w0 = max(min(w0, t - GATE_MIN[q]), t - look)
                last_slot = t
                a, b = np.searchsorted(ts, [w0 + 1e-6, t + 1e-6])
                n = b - a
                gi, gj = (int(ii[b - 1]), int(jj[b - 1])) if n else (-1, -1)
                vel = min(1.0, 0.72 + 0.07 * n) if not solo else 1.0
                if n == 0 and not solo:
                    a2, b2 = np.searchsorted(ats, [w0 + 1e-6, t + 1e-6])
                    if b2 == a2:
                        continue                              # no move at all since the last note: the part stutters
                    gi, gj = int(aii[b2 - 1]), int(ajj[b2 - 1])
                    vel = 0.42                                # a move that didn't land: a ghost note
                g = min(1.0, (t - LOCK0) / (LOCK1 - LOCK0)) if t < LOCK1 else 1.0
                rows.append((t, q, layer, kind, m, bar % 4, dur, vel * g, gi, gj, 'grid'))
        # free (unquantised) layer: the opening burst, in each genre's own timbre
        nfree = 0
        for t, i, j in zip(ts, ii, jj):
            if t >= LOCK1:
                break
            nfree += 1
            if nfree % 6:
                continue
            g = 1.0 if t < LOCK0 else 1 - (t - LOCK0) / (LOCK1 - LOCK0)
            bar = int((t - T.GRID0) // T.BAR)
            name, root, tri, rl = chord(bar)
            m = int(tri[rng.integers(3)] + 12 * rng.integers(0, 2))
            kind = ['pizz', 'gpick', 'epluck', 'keys'][q]
            rows.append((float(t), q, 'free', kind, m, bar % 4, 0.2, 0.8 * g, int(i), int(j), 'free'))
    arr = np.array(rows, DT)
    return arr[np.argsort(arr['t'], kind='stable')]


_NOTES = []


def notes():
    if not _NOTES:
        p = os.path.join(HERE, 'notes.npy')
        src = max(os.path.getmtime(os.path.join(HERE, f)) for f in ('sim.npz', 'music.py', 'timeline.py'))
        if os.path.exists(p) and os.path.getmtime(p) > src:
            _NOTES.append(np.load(p))
        else:
            a = build()
            np.save(p, a)
            _NOTES.append(a)
    return _NOTES[0]


def presence(t):
    """per-genre share of the claimed stage among the genres still in, smoothed (sums to 1)"""
    sh = np.array(T.smooth_share(t)) * np.array([T.alive(q, t) for q in range(4)], float)
    s = sh.sum()
    return sh / s if s > 0 else np.full(4, 0.25)


def gains(t):
    """per-genre, per-layer gain at video time t (shape 4 x 4: genre x [drums, bass, harm, lead]).
    Bass + harmony follow stage share gently (everyone audible: the mix is the scoreboard); the drums follow it
    steeply and the melody steeper still, so the groove and the tune belong to whoever leads: the song morphs."""
    p = presence(t)
    al = np.array([T.alive(q, t) for q in range(4)], float)
    if T.T_WIN is not None and T.T_WIN <= t < T.T_CRACK:
        g = np.zeros((4, 4))
        g[T.WINNER] = 1.0
        return g
    g = np.zeros((4, 4))
    # equal-power normalisation: the sum of squared gains stays ~1, so the mix is as loud as one genre alone
    # (= the winner's solo) whatever the split
    soft = (0.5 + 2.2 * p) * al
    g[:, 1] = g[:, 2] = soft / max(np.sqrt((soft ** 2).sum()), 1e-9)
    for li, k, floor in ((0, 2.2, 0.2), (3, 4.0, 0.1)):
        w = (p ** k) * al
        w = w / max(np.sqrt((w ** 2).sum()), 1e-9)
        g[:, li] = floor * al + (1 - floor) * w
    return g


if __name__ == '__main__':
    a = notes()
    for q in range(4):
        print(f'{T.NAMES[q]:10s}', ' '.join(f"{L} {((a['team'] == q) & (a['layer'] == L)).sum():4d}" for L in LAYERS + ['free']))
    print('total', len(a))
