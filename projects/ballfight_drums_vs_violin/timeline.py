"""#28 BALLFIGHT #2: Drums vs Violin - ONE timeline for picture and sound.

Idea: brute force vs finesse on #27's engine, ring, HUD and count-in. DRUMS (red corner) blasts a drumhead ring out
of its ball on every beat (its reach grows per landed pulse), so its attacks ARE the groove: kick on 1 and 3, snare on
2 and 4, toms when a pulse lands. VIOLIN (blue corner) spins its bow around the ball like a rapier: long, thin, light,
and every landed hit makes it spin faster, so slow strokes become frantic spiccato; the bow's turns play the melody.
Levels from damage dealt unlock two abilities each; below 25 HP each gets one desperation move. Honest simulated fight
(projects/ballfight/fight.py, rules re-tuned once for this pair with tune.py, then frozen), the seed picked for pacing,
never edited.
Hook: the same giant 3 . 2 . 1 . FIGHT! in the ring = the band's count-in (drum sticks + a violin chop per count, the
drop lands on FIGHT); top caption "DRUMS / VS VIOLIN" with the BALLFIGHT #2 tag.
Music, mood B (dark Balkan fire): D harmonic minor, upright-bass oom-pah + an accordion-ish pad, fiery, not happy, at
loop_bpm(140) = 141.64 BPM (36 bars = 61 s; 1 bar = the countdown). The bed has NO drums of its own: the drum fighter
is the kit, the violin is the lead. Whoever is losing, their half of the band falls apart.

Beat sheet (61 s; events after 2 s are the run's own: the seed was chosen so they land near these targets):
    0-1.69   balls rev in corners; giant 3 . 2 . 1 in the ring          DRUMS / VS VIOLIN, then FIGHT!
    1.69-8   launch; pulses vs bow, HP ticking                           -
    8-30     pulse reach grows, bow speeds up; unlocks                   DRUM ROLL, PIZZICATO, CRASH CYMBAL, TREMOLO
    32-44    one dips below 25 HP                                        DOUBLE KICK or SCREECH
    44-52    final exchange, HP in single digits                         -
    ~49-53   K.O.: hit-stop, slow-mo shatter (drumheads / strings)       K.O. -> (WINNER) WINS
    then-58  winner's solo under the spot                                COMMENT THE / NEXT MATCH-UP
    58-61    loser reassembles; both glide to their corners; HP refills; "3" lands at 61.0 = frame 0
"""
import math

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(140, DUR)        # 141.64: 36 bars in 61 s, so the groove is in phase at the loop point
BEAT = 60.0 / BPM               # 0.4236 s: one drum pulse
BAR = 4 * BEAT                  # 1.6944 s = the countdown
N_BARS = 36
S16 = BEAT / 4                  # hits snap to the nearest 16th in the audio (<= 53 ms)

FIGHTERS = ('DRUMS', 'VIOLIN')  # red corner (screen left), blue corner (screen right)
SERIES = 'BALLFIGHT #2'
T_FIGHT = BAR                   # FIGHT! lands on the bar-1 downbeat: the drop, the launch and the first kick
COUNT = [(0.0, '3'), (BEAT, '2'), (2 * BEAT, '1'), (BAR, 'FIGHT!')]

# ---------------------------------------------------------------- frozen rules (tune.py; the same for every seed)
DMG_SCALE = 1.3                 # tuned once (tune.py) so the median K.O. sits near 50 s with each side winning about half

# ---------------------------------------------------------------- story targets (seed_search.py scores runs against these)
T_KO = (49.0, 53.0)
MIN_LEAD_CHANGES = 2
MAX_WINNER_HP = 30.0
CAP_GAP = 4.0                   # never two ability captions within 4 s
CAP_TARGETS = {'DRUM ROLL': 10, 'PIZZICATO': 15, 'CRASH CYMBAL': 22, 'TREMOLO': 28, 'DOUBLE KICK': 40, 'SCREECH': 40}

# ---------------------------------------------------------------- the scripted tail (after the honest K.O.)
SLOWMO = 0.7                    # s of picture time the last instant of the fight is stretched over (the K.O. hit)
SLOWMO_RATE = 0.25
T_REASSEMBLE = 34 * BAR + BEAT  # 58.03: the loser's pieces fly back together
T_GLIDE = 35 * BAR              # 59.31, bar 35: both glide back to their corners, HP refills, weapons reset
T_REV = T_GLIDE + 1.05          # in the corners, revving (same motion as the opening)


# ---------------------------------------------------------------- music (D harmonic minor, dark Balkan fire)
KEY_ROOT = 38                   # D
SCALE = (0, 2, 3, 5, 7, 8, 11)  # D harmonic minor: D E F G A Bb C#
# chords per bar (36 bars): the count-in on the dominant, then the 4-bar loop i - i - iv - V (Dm . Dm . Gm . A7),
# ending on two bars of A7 (bar 35 + the count-in bar 0) so the loop seam is the dominant resolving on FIGHT!
CHORDS = ['A'] + ['Dm', 'Dm', 'Gm', 'A'] * 8 + ['Dm', 'Gm', 'A']
CHORD_ROOT = {'Dm': 38, 'Gm': 43, 'A': 45}
CHORD_PCS = {'Dm': (2, 5, 9), 'Gm': (7, 10, 2), 'A': (9, 1, 4, 7)}
assert len(CHORDS) == N_BARS


def chord_at(t):
    b = int(max(0.0, t) // BAR) % N_BARS
    return CHORDS[b]


def scale_notes(lo, hi, chord=None):
    """D harmonic minor between midi lo..hi (over Gm the C# is lowered to C: G minor's own 7th)"""
    pcs = [(KEY_ROOT + s) % 12 for s in SCALE]
    if chord == 'Gm':
        pcs = [p if p != 1 else 0 for p in pcs]
    return [m for m in range(lo, hi + 1) if m % 12 in pcs]


# The violin's line: an original Balkan-fire melody, one pitch per 16th over the 4-bar loop (64 slots). The bow
# samples it: every quarter turn of the bow plays the line's note at that moment (snapped to the 16th), so a slow bow
# plays a few long strokes of the tune and a fast bow plays the whole thing as frantic spiccato.
_M = {  # (bar in loop, 16th) -> midi; slots in between hold the last note
    (0, 0): 74, (0, 2): 73, (0, 3): 74, (0, 4): 76, (0, 6): 77, (0, 8): 76, (0, 10): 74, (0, 12): 73, (0, 14): 70,
    (1, 0): 69, (1, 2): 70, (1, 3): 69, (1, 4): 67, (1, 6): 69, (1, 8): 70, (1, 10): 73, (1, 12): 74, (1, 14): 76,
    (2, 0): 77, (2, 2): 79, (2, 4): 77, (2, 6): 74, (2, 8): 82, (2, 10): 81, (2, 12): 79, (2, 14): 77,
    (3, 0): 76, (3, 2): 77, (3, 3): 76, (3, 4): 73, (3, 6): 76, (3, 8): 79, (3, 10): 81, (3, 12): 82, (3, 14): 85,
}
MELODY = []
_last = 74
for _b in range(4):
    for _s in range(16):
        _last = _M.get((_b, _s), _last)
        MELODY.append(_last)


def melody_at(t):
    """the line's pitch at time t (bars 1.. follow the 4-bar loop)"""
    k = int(round(t / S16))
    b = (k // 16 - 1) % 4
    return MELODY[b * 16 + k % 16]


# the winner's solo (beat offsets from the solo's first bar, midi or drum, beats)
SOLO = {
    # a violin cadenza: up the harmonic minor in a flurry, an augmented-2nd cry, down in sighs, a trill, the low D
    'VIOLIN': [(0, 74, .25), (.25, 76, .25), (.5, 77, .25), (.75, 79, .25), (1, 81, .5), (1.5, 82, .25), (1.75, 85, .25),
               (2, 86, .75), (2.75, 85, .25), (3, 82, .5), (3.5, 81, .5), (4, 79, .25), (4.25, 81, .25), (4.5, 79, .25),
               (4.75, 77, .25), (5, 76, .5), (5.5, 73, .5), (6, 74, .25), (6.25, 76, .25), (6.5, 74, .25), (6.75, 76, .25),
               (7, 77, .5), (7.5, 76, .25), (7.75, 73, .25), (8, 70, .5), (8.5, 69, .5), (9, 73, .25), (9.25, 76, .25),
               (9.5, 79, .25), (9.75, 82, .25), (10, 85, .5), (10.5, 81, .5), (11, 74, 1.0)],
    # a drum solo: (beat, piece, vel) with piece one of K S T1 T2 T3 C (kick, snare, toms high/mid/floor, crash)
    'DRUMS': [(0, 'C', 1.0), (0, 'K', 1.0), (.5, 'S', .8), (.75, 'S', .6), (1, 'K', .9), (1.5, 'T1', .8), (1.75, 'T1', .7),
              (2, 'T2', .9), (2.25, 'T2', .7), (2.5, 'T3', .9), (2.75, 'T3', .8), (3, 'K', 1.0), (3.25, 'S', .7),
              (3.5, 'S', .8), (3.75, 'S', .9), (4, 'C', .9), (4, 'K', 1.0), (4.5, 'K', .8), (4.75, 'S', .8), (5, 'K', .9),
              (5.25, 'T1', .7), (5.5, 'T2', .8), (5.75, 'T3', .9), (6, 'S', .9), (6.25, 'S', .5), (6.5, 'S', .7),
              (6.75, 'S', .9), (7, 'K', 1.0), (7.25, 'T3', .8), (7.5, 'T2', .8), (7.75, 'T1', .8), (8, 'C', 1.0),
              (8, 'K', 1.0), (8.5, 'S', .9), (9, 'K', .9), (9.25, 'K', .8), (9.5, 'S', .9), (10, 'T1', .8), (10.25, 'T2', .8),
              (10.5, 'T3', .9), (10.75, 'T3', .9), (11, 'C', 1.0), (11, 'K', 1.0)],
}


def bar_t(b):
    return b * BAR


def beat_env(t, decay=0.12):
    return math.exp(-((t % BEAT) / decay))


def beat_pulse(t, decay=0.12, pre=0.07):
    """a pulse on every beat that ramps up over the last `pre` s before it (no one-frame snap), so the beat that lands on
    frame 0 doesn't jump at the loop seam"""
    ph = t % BEAT
    up = min(1.0, max(0.0, (ph - (BEAT - pre)) / pre))
    return max(math.exp(-ph / decay), up * up * (3 - 2 * up))


def next_bar(t):
    return math.ceil(t / BAR - 1e-6) * BAR
