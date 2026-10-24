"""#29 BALLFIGHT #3: Trumpet vs Sax - ONE timeline for picture and sound.

Idea: jazz's oldest argument, who's the lead horn, settled in the ring on #27's engine, ring, HUD and count-in.
TRUMPET (left corner, yellow) fights at range with a BEAM: its bell charges for a beat (glowing, the aim locked on where
the sax is heading), then fires a straight piercing blast; every landed blast makes the beam longer and wider. SAX (right
corner, teal) blows SMOKE RINGS: wobbly rings that curve toward the trumpet; one more ring per volley as volleys land.
Levels from damage dealt unlock two abilities each; below 25 HP each gets one desperation move. Honest simulated fight
(projects/ballfight/fight.py, rules tuned once for this pair with tune.py, then frozen), the seed picked for pacing,
never edited.
Hook: the giant 3 . 2 . 1 . FIGHT! in the ring = the band's count-in (brushes + a horn stab per count, the drop lands on
FIGHT); top caption "TRUMPET / VS SAX" with the BALLFIGHT #3 tag.
Music, noir jazz: fast and smoky, C minor, walking bass, brushed drums, muted piano comping, loop_bpm(132) = 133.77 BPM
(34 bars = 61 s; 1 bar = the count-in). The two horns are the leads: every charge swells a held trumpet note and every
beam is a bright stab that climbs with each landed hit; every smoke ring is a breathy sax growl with a bluesy bend. The
leader's horn gets fuller and louder, the loser's thins and dirties as its HP drops.

Beat sheet (61 s; events after 2 s are the run's own: the seed was chosen so they land near these targets):
    0-1.79   balls rev in corners; giant 3 . 2 . 1 in the ring          TRUMPET / VS SAX, then FIGHT!
    1.79-8   launch; beams vs smoke rings, HP ticking                    -
    8-30     beam grows, more rings; unlocks                             FANFARE, SMOKE SCREEN, DOUBLE TONGUE, BEBOP RUN
    32-44    one dips below 25 HP                                        HIGH C or HONK
    44-52    final exchange, HP in single digits                         -
    ~49-53   K.O.: hit-stop, slow-mo shatter (brass, valves, keys)       K.O. -> (WINNER) WINS
    then-58  winner's solo under the spot                                COMMENT THE / NEXT MATCH-UP
    58-61    loser reassembles; both glide to their corners; HP refills; "3" lands at 61.0 = frame 0
"""
import math

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(132, DUR)        # 133.77: 34 bars in 61 s, so the groove is in phase at the loop point
BEAT = 60.0 / BPM               # 0.4485 s
BAR = 4 * BEAT                  # 1.7941 s = the count-in
N_BARS = 34
S16 = BEAT / 4                  # hits snap to the nearest 16th in the audio (<= 56 ms)

FIGHTERS = ('TRUMPET', 'SAX')   # left corner (screen left), right corner (screen right)
SERIES = 'BALLFIGHT #3'         # the on-screen episode tag (renumber here if posted in another order)
T_FIGHT = BAR                   # FIGHT! lands on the bar-1 downbeat: the drop and the launch
COUNT = [(0.0, '3'), (BEAT, '2'), (2 * BEAT, '1'), (BAR, 'FIGHT!')]

# ---------------------------------------------------------------- frozen rules (tune.py; the same for every seed)
DMG_SCALE = 0.95                # tuned once (tune.py): median K.O. ~50.7 s, TRUMPET 45 % / SAX 55 % over 100 seeds

# ---------------------------------------------------------------- story targets (seed_search.py scores runs against these)
T_KO = (49.0, 53.0)
MIN_LEAD_CHANGES = 2
MAX_WINNER_HP = 30.0
CAP_GAP = 4.0                   # never two ability captions within 4 s
CAP_TARGETS = {'FANFARE': 10, 'SMOKE SCREEN': 15, 'DOUBLE TONGUE': 22, 'BEBOP RUN': 28, 'HIGH C': 40, 'HONK': 40}

# ---------------------------------------------------------------- the scripted tail (after the honest K.O.)
SLOWMO = 0.7                    # s of picture time the last instant of the fight is stretched over (the K.O. hit)
SLOWMO_RATE = 0.25
T_REASSEMBLE = 32 * BAR + BEAT  # 57.86: the loser's pieces fly back together
T_GLIDE = 33 * BAR              # 59.21, bar 33: both glide back to their corners, HP refills, weapons reset
T_REV = T_GLIDE + 1.05          # in the corners, revving (same motion as the opening)


# ---------------------------------------------------------------- music (C minor, noir jazz)
KEY_ROOT = 36                   # C
# chords per bar (34 bars): the count-in on G7, then the 4-bar noir loop i - bVI7 - iv - V7 (Cm . Ab7 . Fm . G7 alt),
# the last bar Ab7 -> the count-in's G7 -> Cm on FIGHT!, so the loop seam is the turnaround resolving on the drop
CHORDS = ['G7'] + ['Cm', 'Ab7', 'Fm', 'G7'] * 8 + ['Ab7']
CHORD_ROOT = {'Cm': 36, 'Ab7': 32, 'Fm': 41, 'G7': 43}
CHORD_PCS = {'Cm': (0, 3, 7, 10, 2), 'Ab7': (8, 0, 3, 6, 10), 'Fm': (5, 8, 0, 3, 7), 'G7': (7, 11, 2, 5, 8)}
assert len(CHORDS) == N_BARS
BLUES = (0, 3, 5, 6, 7, 10)     # C blues scale (pitch classes from C)


def chord_at(t):
    b = int(max(0.0, t) // BAR) % N_BARS
    return CHORDS[b]


def scale_notes(lo, hi, chord=None):
    """chord tones + the C minor scale's colour notes between midi lo..hi (the horns' pool for a chord)"""
    pcs = set(CHORD_PCS[chord]) if chord else {0, 2, 3, 5, 7, 8, 10}
    return [m for m in range(lo, hi + 1) if m % 12 in pcs]


def bar_t(b):
    return b * BAR


def beat_env(t, decay=0.12):
    return math.exp(-((t % BEAT) / decay))


def beat_pulse(t, decay=0.12, pre=0.07):
    """a pulse on every beat that ramps up over the last `pre` s before it (no one-frame snap), so the beat that lands on
    frame 0 doesn't jump at the loop seam"""
    ph = t % BEAT
    if t > DUR - SEAM_PRE - 1e-9:                    # the downbeat on frame 0 ramps in over 4 frames: a gentler seam
        pre = SEAM_PRE
    up = min(1.0, max(0.0, (ph - (BEAT - pre)) / pre))
    return max(math.exp(-ph / decay), up * up * (3 - 2 * up))


SEAM_PRE = 0.14


def next_bar(t):
    return math.ceil(t / BAR - 1e-6) * BAR


# the winner's solo (beat offsets from the solo's first bar: (beat, midi, beats)); original motifs in each horn's idiom
SOLO = {
    # trumpet: a bop line up the C minor triad, a cry on the Ab7's flat 7, a falling Fm run, the leading tone held high
    'TRUMPET': [(0, 72, .5), (.5, 75, .5), (1, 79, .75), (1.75, 77, .25), (2, 75, .5), (2.5, 74, .25), (2.75, 72, .25),
                (3, 79, 1.0), (4, 80, .5), (4.5, 78, .25), (4.75, 75, .25), (5, 72, .5), (5.5, 75, .5), (6, 78, .75),
                (6.75, 80, .25), (7, 84, 1.0), (8, 82, .25), (8.25, 80, .25), (8.5, 77, .5), (9, 80, .5), (9.5, 77, .25),
                (9.75, 75, .25), (10, 72, .5), (10.5, 75, .5), (11, 79, .5), (11.5, 81, .5), (12, 83, 1.25)],
    # sax: a smoky blues line with the flat 5, a growled low note, a bebop run up the Fm, a long bent G
    'SAX': [(0, 60, .5), (.5, 63, .25), (.75, 65, .25), (1, 66, .5), (1.5, 67, .5), (2, 70, .75), (2.75, 67, .25),
            (3, 63, 1.0), (4, 56, .75), (4.75, 60, .25), (5, 63, .5), (5.5, 66, .5), (6, 68, .5), (6.5, 66, .25),
            (6.75, 63, .25), (7, 60, 1.0), (8, 65, .25), (8.25, 67, .25), (8.5, 68, .25), (8.75, 70, .25), (9, 72, .5),
            (9.5, 75, .5), (10, 72, .25), (10.25, 70, .25), (10.5, 68, .5), (11, 67, .5), (11.5, 66, .5),
            (12, 67, 1.25)],
}
