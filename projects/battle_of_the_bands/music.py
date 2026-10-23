"""The band: an original 4-part arrangement (A minor, 120 BPM, | Am | F | C | G |) and the gating that turns
each team's moves into its notes. Imported by audio.py (what to play) and render.py (cone pumps + the
note sparkles on the wall), so picture and sound use one note list.

Every move = a note: a part's note on the 16th grid only plays if its team claimed at least one tile since
the part's previous note slot (busy team = full part, stalled team = gaps). The note's sparkle is drawn on
the last tile that team claimed. Opening burst (frame 0 = the empty stage, the four seeds drop): the parts are not yet on the grid,
every few claims ring a random chord tone at once (chaos, in key); they lock to the grid at 5.4 s.

Parts:  SAND = drums (kick / snare / hats + tom fills), BALL = lead melody (8 bars), RIPPLE = bass (8ths),
        VINE = keys (16th arpeggio). After the win, the winner plays its part ungated plus the melody."""
import os
import numpy as np

import timeline as T

HERE = os.path.dirname(os.path.abspath(__file__))
CHORDS = [(57, 'Am', [57, 60, 64]), (53, 'F', [53, 57, 60]), (48, 'C', [48, 52, 55]), (55, 'G', [55, 59, 62])]
LOCK0, LOCK1 = T.GRID0 + T.BAR, T.GRID0 + 2 * T.BAR   # free -> grid crossfade: 3.4 -> 5.4 s (bar lines)
NBARS = int(T.DUR / T.BAR)        # 30 bars; bar k starts at GRID0 + k * BAR (bar -1 is the pickup before 1.4 s)

# melody: per bar of the 8-bar tune, (16th step, midi, length in 16ths)
MELODY = [
    [(0, 76, 3), (3, 74, 1), (4, 72, 2), (6, 71, 2), (8, 69, 4), (12, 72, 2), (14, 76, 2)],
    [(0, 77, 4), (4, 76, 2), (6, 72, 2), (8, 69, 6), (14, 67, 2)],
    [(0, 67, 2), (2, 72, 2), (4, 76, 4), (8, 79, 3), (11, 76, 1), (12, 74, 4)],
    [(0, 74, 2), (2, 71, 2), (4, 67, 4), (10, 71, 2), (12, 74, 2), (14, 76, 2)],
    [(0, 81, 4), (4, 79, 2), (6, 76, 2), (8, 72, 2), (10, 76, 2), (12, 81, 4)],
    [(0, 84, 3), (3, 81, 1), (4, 77, 4), (8, 81, 2), (10, 79, 2), (12, 77, 2), (14, 76, 2)],
    [(0, 76, 2), (2, 79, 2), (4, 84, 4), (8, 83, 2), (10, 79, 2), (12, 76, 4)],
    [(0, 74, 4), (4, 79, 2), (6, 71, 2), (8, 74, 6), (14, 71, 2)],
]
# drums: per 16th step, list of hits; 'k' kick, 's' snare, 'h' hat, 'o' open hat, 'g' ghost snare
DRUM_BAR = {0: 'kh', 1: 'h', 2: 'h', 3: 'gh', 4: 'sh', 5: 'h', 6: 'kh', 7: 'h', 8: 'kh', 9: 'h', 10: 'ko', 11: 'h',
            12: 'sh', 13: 'h', 14: 'gh', 15: 'h'}
FILL_BAR = {0: 'kh', 2: 'h', 4: 'sh', 6: 'kh', 8: 't0', 9: 't0', 10: 't1', 11: 't1', 12: 't2', 13: 't2', 14: 't3', 15: 's'}
TOMS = [50, 47, 43, 40]
GATE_MIN = [0.0, 0.0, T.BEAT, 0.0]     # shortest look-back per part (the bass listens over a whole ring period)
ARP = [0, 1, 2, 3, 4, 3, 2, 1, 0, 2, 4, 5, 4, 2, 1, 2]       # index into the 6-note voicing


def chord(bar):
    return CHORDS[bar % 4]


def voicing(bar):
    root, name, tri = chord(bar)
    v = sorted({(n - 57) % 12 + 57 for n in tri})            # A3..G#4 range
    return [n for n in v] + [n + 12 for n in v]


def part_notes(q, bar):
    """grid notes of team q's part in one bar: (16th step, midi, length 16ths, kind)"""
    root, name, tri = chord(bar)
    out = []
    if q == 0:                                              # drums
        pat = FILL_BAR if bar % 4 == 3 else DRUM_BAR
        for s, hits in pat.items():
            for h in (hits if not hits.startswith('t') else [hits]):
                if h.startswith('t'):
                    out.append((s, TOMS[int(h[1])] + 12, 2, 'tom'))
                else:
                    out.append((s, 0, 1, {'k': 'kick', 's': 'snare', 'h': 'hat', 'o': 'ohat', 'g': 'ghost'}[h]))
    elif q == 1:                                            # lead
        for s, m, L in MELODY[bar % 8]:
            out.append((s, m, L, 'lead'))
    elif q == 2:                                            # bass: 8ths root / octave, approach on 14
        r = root - 24
        nxt = chord(bar + 1)[0] - 24
        for s in range(0, 16, 2):
            m = r if s % 4 == 0 else r + 12
            if s == 14:
                m = nxt + (1 if nxt < r else -1) + 12 if abs(nxt - r) > 2 else r + 7
            out.append((s, m, 2, 'bass'))
    elif q == 3:                                            # keys: 16th arpeggio over a 6-note voicing
        v = voicing(bar)
        for s in range(16):
            out.append((s, v[ARP[s]], 1, 'keys'))
    return out


def _claims(prefix='ev'):
    """per team: sorted video times of its claims (prefix 'ev') or attempted moves ('at') + their tiles"""
    D = T.D
    ts = np.array([T.video_time(t) for t in D[prefix + '_t']])
    k = D[prefix + '_k'].astype(int)
    out = []
    for q in range(4):
        m = k == q + 1
        o = np.argsort(ts[m], kind='stable')
        out.append((ts[m][o], D[prefix + '_i'][m][o].astype(int), D[prefix + '_j'][m][o].astype(int)))
    return out


def build():
    """the full note list: structured array (t, team, midi, dur, vel, kind, i, j, layer)"""
    C = _claims('ev')
    AT = _claims('at')
    rng = np.random.default_rng(120)
    rows = []
    win = T.WINNER
    for q in range(4):
        ts, ii, jj = C[q]
        ats, aii, ajj = AT[q]
        last_slot = None
        for bar in range(-1, NBARS):
            for s, m, L, kind in part_notes(q, bar):
                t = T.GRID0 + bar * T.BAR + s * T.S16
                dur = L * T.S16
                solo = T.T_WIN is not None and t >= T.T_WIN and t < T.T_RESTART and q == win
                if not T.alive(q, t) and not solo:
                    continue
                if t < LOCK0 or t >= T.T_RESTART:
                    continue
                w0 = (last_slot if last_slot is not None else t - T.BEAT)
                w0 = max(min(w0, t - GATE_MIN[q]), t - T.BEAT)
                last_slot = t
                a, b = np.searchsorted(ts, [w0 + 1e-6, t + 1e-6])
                n = b - a
                gi, gj = (int(ii[b - 1]), int(jj[b - 1])) if n else (-1, -1)
                vel = min(1.0, 0.72 + 0.07 * n) if not solo else 1.0
                if n == 0 and not solo:
                    a2, b2 = np.searchsorted(ats, [w0 + 1e-6, t + 1e-6])
                    if b2 == a2:
                        continue                               # no move at all since the last slot: the part stutters
                    gi, gj = int(aii[b2 - 1]), int(ajj[b2 - 1])
                    vel = 0.55 if q == 2 else 0.42             # a move that didn't land: a ghost note
                g = min(1.0, (t - LOCK0) / (LOCK1 - LOCK0)) if t < LOCK1 else 1.0
                rows.append((t, q, m, dur, vel * g, kind, gi, gj, 'grid'))
        # the melody in the winner's own voice (the solo)
        if T.T_WIN is not None and q == win and q != 1:
            t0 = T.T_WIN
            for bar in range(-1, NBARS):
                for s, m, L in MELODY[bar % 8]:
                    t = T.GRID0 + bar * T.BAR + s * T.S16
                    if t0 <= t < T.T_RESTART:
                        rows.append((t, q, m, L * T.S16, 0.95, 'solo', -1, -1, 'solo'))
        # free (unquantised) layer: the opening burst and the next run's burst
        nfree = 0
        for t, i, j in zip(ts, ii, jj):
            if t < LOCK1 or t >= T.T_RESTART:
                nfree += 1
                if nfree % 6:
                    continue
                if t < LOCK1:
                    g = 1.0 if t < LOCK0 else 1 - (t - LOCK0) / (LOCK1 - LOCK0)
                else:
                    g = min(1.0, (t - T.T_RESTART) / 0.5)
                bar = int((t - T.GRID0) // T.BAR)
                root, name, tri = chord(bar)
                if q == 0:
                    kind, m = ['kick', 'snare', 'hat', 'tom'][rng.integers(4)], TOMS[rng.integers(4)] + 12
                elif q == 2:
                    kind, m = 'bass', tri[rng.integers(3)] - 24 + 12 * rng.integers(2)
                elif q == 3:
                    kind, m = 'keys', voicing(bar)[rng.integers(6)]
                else:
                    kind, m = 'lead', tri[rng.integers(3)] + 12 + 12 * rng.integers(2)
                rows.append((float(t), q, int(m), 0.18, 0.8 * g, kind, int(i), int(j), 'free'))
    dt = np.dtype([('t', 'f8'), ('team', 'i4'), ('midi', 'i4'), ('dur', 'f8'), ('vel', 'f8'), ('kind', 'U6'),
                   ('i', 'i4'), ('j', 'i4'), ('layer', 'U4')])
    arr = np.array(rows, dt)
    return arr[np.argsort(arr['t'], kind='stable')]


_NOTES = []


def notes():
    if not _NOTES:
        p = os.path.join(HERE, 'notes.npy')
        sim_m = os.path.getmtime(os.path.join(HERE, 'sim.npz'))
        if os.path.exists(p) and os.path.getmtime(p) > max(sim_m, os.path.getmtime(__file__)):
            _NOTES.append(np.load(p))
        else:
            a = build()
            np.save(p, a)
            _NOTES.append(a)
    return _NOTES[0]


def gains(t):
    """per-team part gain at video time t: volume follows stage share (smoothed), the others swell when a band
    drops out so the mix never thins"""
    sh = np.array(_smooth_share(t))
    al = np.array([T.alive(q, t) for q in range(4)], float)
    sh = sh / max(sh.sum(), 1e-6)                      # share of the claimed stage (FILL counts like FIGHT)
    g = (0.35 + 2.6 * sh) * al
    tot = g.sum()
    if tot > 0:
        g *= (1.0 / tot) ** 0.55
    return g


_SH = {}


def _smooth_share(t):
    """team shares smoothed over ~0.35 s (reset at the restart, where a new run begins)"""
    f = int(t * T.FPS) % T.NF
    if not _SH:
        raw = np.array([T.share(k / T.FPS) for k in range(T.NF)])
        a = 1 - np.exp(-1 / (0.35 * T.FPS))
        sm = raw.copy()
        r0 = int(T.T_RESTART * T.FPS)
        for k in range(1, T.NF):
            sm[k] = raw[k] if k == r0 else sm[k - 1] + a * (raw[k] - sm[k - 1])
        _SH['s'] = sm
    return _SH['s'][f]


_LC = []


def lead_changes():
    """video times when the stage leader changes during the fight (smoothed shares, 1 % hysteresis)"""
    if not _LC:
        out, lead = [], None
        for f in range(int(T.T_BELL * T.FPS), int((T.T_WIN or 58) * T.FPS), 3):
            sh = _smooth_share(f / T.FPS)
            m = int(np.argmax(sh))
            if lead is None:
                lead = m
            elif m != lead and sh[m] > sh[lead] + 0.01:
                lead = m
                out.append((f / T.FPS, m))
        _LC.append(out)
    return _LC[0]


if __name__ == '__main__':
    a = notes()
    for q in range(4):
        m = (a['team'] == q) & (a['layer'] == 'grid')
        print(T.NAMES[q], 'grid notes', m.sum(), 'free', ((a['team'] == q) & (a['layer'] == 'free')).sum(),
              'solo', ((a['team'] == q) & (a['layer'] == 'solo')).sum())
    print('lead changes:', [(round(t, 1), T.NAMES[q]) for t, q in lead_changes()])
