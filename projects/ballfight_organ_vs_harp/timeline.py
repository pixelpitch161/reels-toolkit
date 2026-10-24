"""#35 BALLFIGHT #9: Organ vs Harp - ONE timeline for picture and sound.

Idea: cathedral vs angels, heaven's instruments fight, on #27's engine, ring, HUD and count-in. ORGAN (left corner,
wine) fights with erupting pipes: on every second beat a row of organ pipes bursts up out of the canvas, from the
organ toward the harp (new weapon kind `line`), each row a big organ stab; every row that lands makes the next one
longer. HARP (right corner, pearl) is a trapper: on every second beat it stretches a glowing string rope to rope across
the ring between the two of them (new kind `trap`); whoever crosses an enemy string is cut by it and the string sings
(the harp passes freely); every landed crossing adds a string, up to 7, and the oldest fades. Levels from damage dealt
unlock two abilities each; below 25 HP each gets one desperation move. Honest simulated fight (projects/ballfight/
fight.py, rules re-tuned once for this pair with tune.py, then frozen), the seed picked for pacing, never edited.
Hook: the same giant 3 . 2 . 1 . FIGHT! in the ring = the band's count-in (a tolling bell + an organ stab + a harp
pluck per count, the drop lands on FIGHT); top caption "ORGAN / VS HARP" with the BALLFIGHT #9 tag.
Music, gothic horror: D minor, choir "aah" pad, a tolling bell, a low pedal drone, slow-burn half time that builds into
a pounding back half (toms, timpani, staccato choir), at loop_bpm(126) = 125.90 BPM (32 bars = 61 s; 1 bar = the
count-in). The organ fighter plays the chords (short stabs, never held, so the bed doesn't turn to mush), the harp
plays the arpeggios. Whoever is losing, their half of the band falls apart.

Beat sheet (61 s; events after 2 s are the run's own: the seed was chosen so they land near these targets):
    0-1.91   balls rev in corners; giant 3 . 2 . 1 in the ring            ORGAN / VS HARP, then FIGHT!
    1.91-8   launch; first pipe rows and the first strings; HP ticking     -
    8-30     rows grow, the string web fills; unlocks                      CATHEDRAL, PLUCK, PULL ALL STOPS, ANGEL WINGS
    32-44    one dips below 25 HP                                          TOCCATA or 47 STRINGS
    44-52    final exchange, HP in single digits                           -
    ~49-53   K.O.: hit-stop, slow-mo shatter (pipes / strings)             K.O. -> (WINNER) WINS
    then-58  winner's solo under the spot                                  COMMENT THE / NEXT MATCH-UP
    58-61    loser reassembles; both glide to their corners; HP refills; "3" lands at 61.0 = frame 0
"""
import math

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(126, DUR)        # 125.90: 32 bars in 61 s, so the groove is in phase at the loop point
BEAT = 60.0 / BPM               # 0.4766 s
BAR = 4 * BEAT                  # 1.9063 s = the countdown
N_BARS = 32
S16 = BEAT / 4                  # hits snap to the nearest 16th in the audio (<= 60 ms)

FIGHTERS = ('ORGAN', 'HARP')    # left corner (screen left), right corner (screen right)
SERIES = 'BALLFIGHT #9'
T_FIGHT = BAR                   # FIGHT! lands on the bar-1 downbeat: the drop, the launch and the first pipe row
COUNT = [(0.0, '3'), (BEAT, '2'), (2 * BEAT, '1'), (BAR, 'FIGHT!')]

# ---------------------------------------------------------------- frozen rules (tune.py; the same for every seed)
DMG_SCALE = 0.6                 # tuned once (tune.py) so the median K.O. sits near 50 s with each side winning about half

# ---------------------------------------------------------------- story targets (seed_search.py scores runs against these)
T_KO = (49.0, 53.0)
MIN_LEAD_CHANGES = 2
MAX_WINNER_HP = 30.0
CAP_GAP = 4.0                   # never two ability captions within 4 s
CAP_TARGETS = {'CATHEDRAL': 10, 'PLUCK': 15, 'PULL ALL STOPS': 22, 'ANGEL WINGS': 28, 'TOCCATA': 40, '47 STRINGS': 40}

# ---------------------------------------------------------------- the scripted tail (after the honest K.O.)
SLOWMO = 0.7                    # s of picture time the last instant of the fight is stretched over (the K.O. hit)
SLOWMO_RATE = 0.25
T_GLIDE = (N_BARS - 1) * BAR    # 59.09, bar 31: both glide back to their corners, HP refills, weapons reset
T_REASSEMBLE = T_GLIDE - 2 * BEAT   # 58.14: the loser's pieces fly back together
T_REV = T_GLIDE + 1.05          # in the corners, revving (same motion as the opening)


# ---------------------------------------------------------------- music (D minor, gothic horror)
KEY_ROOT = 38                   # D
SCALE = (0, 2, 3, 5, 7, 8, 10)  # D natural minor; the dominant (A) borrows C#
# chords per bar (32 bars): the count-in on the dominant; the slow burn (bars 1-12, i i VI V), the climb (13-20,
# i iv VI V), the pounding back half (21-28, i VI iv V), then i iv V into the seam, so the loop seam is the dominant
# (bar 31 + the count-in bar 0) resolving on FIGHT!
CHORDS = ['A'] + ['Dm', 'Dm', 'Bb', 'A'] * 3 + ['Dm', 'Gm', 'Bb', 'A'] * 2 + ['Dm', 'Bb', 'Gm', 'A'] * 2 + ['Dm', 'Gm', 'A']
CHORD_ROOT = {'Dm': 38, 'Gm': 43, 'Bb': 46, 'A': 45}
CHORD_PCS = {'Dm': (2, 5, 9), 'Gm': (7, 10, 2), 'Bb': (10, 2, 5), 'A': (9, 1, 4)}
assert len(CHORDS) == N_BARS
SECTION = [(0, 'count'), (1, 'burn'), (13, 'climb'), (21, 'pound')]      # (first bar, name); the tail is by the run


def bar_of(t):
    return int(max(0.0, t) // BAR) % N_BARS


def chord_at(t):
    return CHORDS[bar_of(t)]


def section_at(t):
    b = bar_of(t)
    name = 'count'
    for b0, nm in SECTION:
        if b >= b0:
            name = nm
    return name


def chord_notes(ch, lo, hi):
    """the chord's tones between midi lo..hi"""
    pcs = CHORD_PCS[ch]
    return [m for m in range(lo, hi + 1) if m % 12 in pcs]


def scale_notes(lo, hi, chord=None):
    """D minor between midi lo..hi (over A the C is raised to C#: the dominant's leading tone)"""
    pcs = [(KEY_ROOT + s) % 12 for s in SCALE]
    if chord == 'A':
        pcs = [p if p != 0 else 1 for p in pcs]
    return [m for m in range(lo, hi + 1) if m % 12 in pcs]


# the winner's solo: (beat offset from the solo's first bar, midi or chord, beats). Played until the reassembly.
SOLO = {
    # an organ toccata flourish: a mordent on A, a falling run, the diminished-seventh shudder, the low D chord
    'ORGAN': [(0, 69, .25), (.25, 67, .25), (.5, 69, 1.0), (1.5, 67, .25), (1.75, 65, .25), (2, 64, .25), (2.25, 62, .25),
              (2.5, 61, .75), (3.25, 62, .75), (4, ('dim', 61), 1.0), (5, ('dim', 64), 1.0), (6, ('dim', 67), .5),
              (6.5, ('dim', 70), .5), (7, ('Dm', 62), 1.0), (8, ('Dm', 50), 2.5)],
    # a harp cadenza: rising D-minor sweeps, a glissando down, a harmonic bell tone at the top
    'HARP': [(0, 50, .25), (.25, 57, .25), (.5, 62, .25), (.75, 65, .25), (1, 69, .25), (1.25, 74, .25), (1.5, 77, .5),
             (2, 81, .5), (2.5, 79, .25), (2.75, 77, .25), (3, 76, .25), (3.25, 74, .25), (3.5, 73, .5),
             (4, 46, .25), (4.25, 53, .25), (4.5, 58, .25), (4.75, 62, .25), (5, 65, .25), (5.25, 70, .25), (5.5, 74, .5),
             (6, ('gliss', 86), 1.0), (7, 74, .5), (7.5, 81, .5), (8, 86, 2.5)],
}


def bar_t(b):
    return b * BAR


def beat_env(t, decay=0.12):
    return math.exp(-((t % BEAT) / decay))


def beat_pulse(t, decay=0.12, pre=0.16):
    """a pulse on every beat that ramps up over the last `pre` s before it (no one-frame snap), so the beat that lands on
    frame 0 doesn't jump at the loop seam (0.16 s here: the pearl corner light is bright, so its pulse moves a lot of the
    frame; #27 / #28 used 0.07)"""
    ph = t % BEAT
    up = min(1.0, max(0.0, (ph - (BEAT - pre)) / pre))
    return max(math.exp(-ph / decay), up * up * (3 - 2 * up))


def next_bar(t):
    return math.ceil(t / BAR - 1e-6) * BAR
