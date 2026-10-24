"""#27 BALLFIGHT #1: Guitar vs Piano - ONE timeline for picture and sound.

Idea: two glass balls, each holding a low-poly instrument, fight in a boxing ring (math.vsmath-style ability balls, ours).
Each fights with its instrument and every hit plays it, so the fight is the duet: GUITAR (red corner) swings its own
neck like a sword (+1 fret per landed hit), PIANO (blue corner) fires its keys on the beat (one more key per volley
as its volleys land). Levels from damage dealt unlock two abilities each; below 25 HP each gets one desperation move.
Honest simulated fight (fight.py, rules frozen after tune.py), the seed picked for pacing, never edited.
Hook: a giant 3 . 2 . 1 . FIGHT! in the ring = the band's count-in (guitar chord + piano octave + 808 per count, the
drop lands on FIGHT); top caption "GUITAR / VS PIANO".
Music, mood A (drift phonk): cowbell riff, sliding 808, menacing E minor at loop_bpm(130) = 129.84 BPM (33 bars = 61 s;
1 bar = the countdown). Every hit plays the attacker's instrument (snapped to the nearest 16th); the leader's part gets
fuller, the loser's thins and distorts as its HP drops; the winner plays an original victory solo.

Beat sheet (61 s; events after 2 s are the run's own: the seed was chosen so they land near these targets):
    0-1.85   balls rev in corners; giant 3 . 2 . 1 in the ring          GUITAR / VS PIANO, then FIGHT!
    1.85-8   launch, first clashes, HP ticking                           -
    8-20     neck lengthens, volleys grow, first unlocks                 POWER CHORD (~10), SUSTAIN (~15)
    20-32    lead swings; ring crowded with keys                         SHRED (~24), GLISSANDO (~30)
    32-44    both weapons huge, flashbulbs; one dips below 25 HP         FEEDBACK or GRAND SLAM (~40)
    44-52    final exchange, HP in single digits                         -
    ~52      K.O.: brief slow-mo; the loser shatters into keys/strings   K.O. -> (WINNER) WINS
    52-58    winner's victory solo; the crowd lights up                  COMMENT THE / NEXT MATCH-UP
    58-61    loser reassembles; both glide back to their corners; HP refills; "3" lands at 61.0 = frame 0
"""
FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(130, DUR)        # 129.84: 33 bars in 61 s, so the groove is in phase at the loop point
BEAT = 60.0 / BPM
BAR = 4 * BEAT                  # 1.8485 s = the countdown
N_BARS = 33
S16 = BEAT / 4                  # hits snap to the nearest 16th in the audio (<= 58 ms)

FIGHTERS = ('GUITAR', 'PIANO')  # red corner (screen left), blue corner (screen right)
T_FIGHT = BAR                   # FIGHT! lands on the bar-1 downbeat: the drop, and the launch
COUNT = [(0.0, '3'), (BEAT, '2'), (2 * BEAT, '1'), (BAR, 'FIGHT!')]

# ---------------------------------------------------------------- frozen rules (tune.py; the same for every seed)
DMG_SCALE = 0.96                # tuned once so the median K.O. sits near 50 s with each side winning ~half the runs

# ---------------------------------------------------------------- story targets (seed_search.py scores runs against these)
T_KO = (49.0, 53.0)
MIN_LEAD_CHANGES = 2
MAX_WINNER_HP = 30.0
CAP_GAP = 4.0                   # never two ability captions within 4 s
CAP_TARGETS = {'POWER CHORD': 10, 'SUSTAIN': 15, 'SHRED': 24, 'GLISSANDO': 30, 'FEEDBACK': 40, 'GRAND SLAM': 40}

# ---------------------------------------------------------------- the scripted tail (after the honest K.O.)
SLOWMO = 0.7                    # s of picture time the last instant of the fight is stretched over (the K.O. hit)
SLOWMO_RATE = 0.25
T_REASSEMBLE = 58.0             # the loser's pieces fly back together
T_GLIDE = 59.15                 # bar 32: both glide back to their corners, HP refills, weapons reset
T_REV = 60.2                    # in the corners, revving (same motion as the opening)


# ---------------------------------------------------------------- music (E minor, drift phonk)
KEY_ROOT = 40                   # E
SCALE = (0, 2, 3, 5, 7, 8, 10)  # E natural minor with the flat 2nd (F) used in the riff: Phrygian menace
# chords per bar (33 bars): the count-in, then a 4-bar loop i - i - bVI - V (E5 . E5 . C5 . B5)
CHORDS = ['E'] + ['E', 'E', 'C', 'B'] * 8
CHORD_ROOT = {'E': 40, 'C': 36, 'B': 35, 'D': 38, 'A': 33}
CHORD_TONES = {'E': (4, 7, 11), 'C': (0, 4, 7), 'B': (11, 3, 6), 'D': (2, 6, 9), 'A': (9, 0, 4)}
# the winner's victory solo, an original motif in its own idiom (beat offsets from the solo's first bar, midi, beats)
SOLO = {
    'PIANO': [(0, 64, .5), (.5, 67, .5), (1, 71, .5), (1.5, 76, .5), (2, 79, .75), (2.75, 78, .25), (3, 76, .5), (3.5, 74, .5),
              (4, 72, .5), (4.5, 76, .5), (5, 79, .5), (5.5, 84, 1.0), (6.5, 83, .25), (6.75, 81, .25), (7, 79, .5), (7.5, 76, .5),
              (8, 75, .5), (8.5, 78, .5), (9, 81, .5), (9.5, 83, .5), (10, 87, .25), (10.25, 83, .25), (10.5, 81, .25),
              (10.75, 78, .25), (11, 75, 1.0)],
    'GUITAR': [(0, 64, .5), (.5, 67, .5), (1, 69, .25), (1.25, 71, .75), (2, 74, .5), (2.5, 71, .5), (3, 69, .25), (3.25, 67, .25),
               (3.5, 64, .5), (4, 72, .75), (4.75, 71, .25), (5, 69, .5), (5.5, 67, .5), (6, 64, 1.0), (7, 62, .5), (7.5, 64, .5),
               (8, 71, .5), (8.5, 74, .5), (9, 76, 1.0), (10, 75, .5), (10.5, 71, .5), (11, 66, 1.0)],
}


def chord_at(t):
    b = int(max(0.0, t) // BAR) % N_BARS
    return CHORDS[b]


def bar_t(b):
    return b * BAR


def beat_env(t, decay=0.12):
    import math
    return math.exp(-((t % BEAT) / decay))


def beat_pulse(t, decay=0.12, pre=0.07):
    """a pulse on every beat that ramps up over the last `pre` s before it (no one-frame snap), so the beat that lands on
    frame 0 doesn't jump at the loop seam"""
    import math
    ph = t % BEAT
    up = min(1.0, max(0.0, (ph - (BEAT - pre)) / pre))
    return max(math.exp(-ph / decay), up * up * (3 - 2 * up))


def next_bar(t):
    import math
    return math.ceil(t / BAR - 1e-6) * BAR
