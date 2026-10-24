"""#32 BALLFIGHT #6: Tuba vs Flute - ONE timeline for picture and sound.

Idea: biggest vs smallest, lowest vs highest, on #27's engine, ring, HUD and count-in. TUBA (left corner, indigo) blasts
a slow, huge brass shockwave out of its ball on every bar (#28's `pulse`, every 4 beats) and pulls the flute toward it
all the time (new weapon option `pull`: gravity that only steers, the speed stays shared); every landed pulse grows the
wave's reach and the pull. FLUTE (right corner, silver) fires rapid tiny darts on every beat (a fast, light volley on
the 16th grid, new option `on16`, so FLUTTER TONGUE can double it), one more dart every second landed volley. Same
ball size and speed for both (fairness): "biggest vs smallest" lives in the weapons and the sound. Levels from damage
dealt unlock two abilities each; below 25 HP each gets one desperation move. Honest simulated fight
(projects/ballfight/fight.py, rules re-tuned once for this pair with tune.py, then frozen), the seed picked for pacing,
never edited.
Hook: the giant 3 . 2 . 1 . FIGHT! in the ring = the band's count-in (a tuba "oom" + a flute chirp + the snare per
count, the drop on FIGHT); top caption "TUBA / VS FLUTE" with the BALLFIGHT #6 tag.
Music, villain march: C minor, a heavy march at loop_bpm(126) = 125.90 BPM (32 bars = 61 s; 1 bar = the countdown).
Snare ostinato, low brass "pah" chords, a sinister music-box counter-line; the TUBA fighter supplies the bass (every
pulse is its oom), the FLUTE fighter the high flurries (every volley is a run of darts = a run of notes).

Beat sheet (61 s; events after 2 s are the run's own: the seed was chosen so they land near these targets):
    0-1.91   balls rev in corners; giant 3 . 2 . 1 in the ring          TUBA / VS FLUTE, then FIGHT!
    1.91-8   launch; brass waves vs darts, the pull, HP ticking          -
    8-30     waves + pull grow, more darts; unlocks                      OOMPAH, FLUTTER TONGUE, BRASS WALL, TRILL
    32-44    one dips below 25 HP                                        SUBSONIC or PICCOLO
    44-52    final exchange, HP in single digits                         -
    ~49-53   K.O.: hit-stop, slow-mo shatter (brass / silver tubes)      K.O. -> (WINNER) WINS
    then-58  winner's solo under the spot                                COMMENT THE / NEXT MATCH-UP
    58-61    loser reassembles; both glide to their corners; HP refills; "3" lands at 61.0 = frame 0
"""
import math

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(126, DUR)        # 125.90: 32 bars in 61 s, so the march is in phase at the loop point
BEAT = 60.0 / BPM               # 0.4766 s
BAR = 4 * BEAT                  # 1.9063 s = the countdown = one tuba wave
N_BARS = 32
S16 = BEAT / 4                  # hits snap to the nearest 16th in the audio (<= 60 ms)

FIGHTERS = ('TUBA', 'FLUTE')    # left corner (screen left), right corner (screen right)
SERIES = 'BALLFIGHT #6'
T_FIGHT = BAR                   # FIGHT! lands on the bar-1 downbeat: the drop, the launch and the first wave
COUNT = [(0.0, '3'), (BEAT, '2'), (2 * BEAT, '1'), (BAR, 'FIGHT!')]

# ---------------------------------------------------------------- frozen rules (tune.py; the same for every seed)
DMG_SCALE = 1.06                # tuned once (tune.py) so the median K.O. sits near 50 s with each side winning about half

# ---------------------------------------------------------------- story targets (seed_search.py scores runs against these)
T_KO = (49.0, 53.0)
MIN_LEAD_CHANGES = 2
MAX_WINNER_HP = 30.0
CAP_GAP = 4.0                   # never two ability captions within 4 s
MAX_GAP = 4.0                   # v2: no stretch over 4 s without a hit (a hard window: the fight must keep moving)
CAP_TARGETS = {'OOMPAH': 10, 'FLUTTER TONGUE': 15, 'BRASS WALL': 22, 'SNAKE DANCE': 28, 'SUBSONIC': 40, 'PICCOLO': 40}

# ---------------------------------------------------------------- the scripted tail (after the honest K.O.)
SLOWMO = 0.7                    # s of picture time the last instant of the fight is stretched over (the K.O. hit)
SLOWMO_RATE = 0.25
T_REASSEMBLE = 30 * BAR + BEAT  # 57.66: the loser's pieces fly back together
T_GLIDE = 31 * BAR              # 59.09, bar 31: both glide back to their corners, HP refills, weapons reset
T_REV = T_GLIDE + 1.05          # in the corners, revving (same motion as the opening)


# ---------------------------------------------------------------- music (C minor, villain march)
KEY_ROOT = 36                   # C2: the tuba's home
SCALE = (0, 2, 3, 5, 7, 8, 11)  # C harmonic minor: C D Eb F G Ab B
# chords per bar (32 bars): the count-in on the dominant, then the 4-bar march loop i - VI - N6 - V (Cm . Ab . Db . G),
# the Neapolitan Db with the B natural on top is the villain's augmented-sixth snarl into G; the last bars Cm . Ab . G
# lead into the count-in's G on frame 0 (two bars of G across the loop seam)
CHORDS = ['G'] + ['Cm', 'Ab', 'Db', 'G'] * 7 + ['Cm', 'Ab', 'G']
CHORD_ROOT = {'Cm': 36, 'Ab': 44, 'Db': 37, 'G': 43}
CHORD_FIFTH = {'Cm': 43, 'Ab': 39, 'Db': 44, 'G': 38}
CHORD_PCS = {'Cm': (0, 3, 7), 'Ab': (8, 0, 3), 'Db': (1, 5, 8, 11), 'G': (7, 11, 2, 5)}
assert len(CHORDS) == N_BARS


def chord_at(t):
    b = int(max(0.0, t) // BAR) % N_BARS
    return CHORDS[b]


def scale_notes(lo, hi, chord=None):
    """C harmonic minor between midi lo..hi (over the Neapolitan Db the D is lowered to Db)"""
    pcs = [(KEY_ROOT + s) % 12 for s in SCALE]
    if chord == 'Db':
        pcs = [p if p != 2 else 1 for p in pcs]
    return [m for m in range(lo, hi + 1) if m % 12 in pcs]


def chord_notes(lo, hi, chord):
    pcs = CHORD_PCS[chord]
    return [m for m in range(lo, hi + 1) if m % 12 in pcs]


# The music box's counter-line: an original sinister 4-bar line (one pitch per 8th, None = rest), high and thin, the
# villain's lullaby over the march. Bars follow the chord loop (Cm . Ab . Db . G).
MUSIC_BOX = [
    84, None, 87, None, 86, 84, 83, None,          # Cm: C Eb D C B
    84, None, 80, None, 79, None, 80, 84,          # Ab: C Ab G Ab C
    85, None, 89, None, 88, 85, 83, None,          # Db: Db F E(Fb) Db B   (the snarl)
    79, 83, 86, 89, 91, None, 86, 83,              # G : G B D F G D B
]


# v2: the flute's snake-charmer line (SNAKE CHARMER): an original sinuous 4-bar line in C harmonic minor with the
# augmented second (Ab-B), grace notes and a phrygian-dominant turn over G; NOT the cliche charmer tune. One pitch per
# 16th slot over the chord loop (Cm . Ab . Db . G), rests in between; the cobras sway to it while they're up.
CHARMER = {
    0: 79, 2: 80, 3: 79, 4: 83, 6: 84, 8: 83, 10: 80, 11: 79, 12: 77, 14: 75,
    16: 72, 18: 75, 20: 77, 21: 79, 22: 80, 24: 84, 26: 83, 28: 80, 30: 79,
    32: 77, 34: 80, 36: 85, 38: 84, 40: 85, 41: 84, 42: 83, 44: 80, 46: 77,
    48: 79, 49: 80, 50: 79, 52: 83, 54: 86, 56: 89, 58: 86, 60: 83, 62: 80,
}
CHARMER_SLOTS = sorted(CHARMER)


def charmer_note(slot):
    """(midi, length in 16ths) of the line at absolute 16th `slot` (bars from 1 follow the 4-bar loop), or None"""
    j = (slot - 16) % 64
    if j not in CHARMER:
        return None
    nxt = next((q for q in CHARMER_SLOTS if q > j), CHARMER_SLOTS[0] + 64)
    return CHARMER[j], nxt - j


# the winner's solo (beat offsets from the solo's first bar, midi, beats)
SOLO = {
    # the tuba's victory march: oom-pah with a menacing chromatic climb, a low pedal and a final growl on the low C
    'TUBA': [(0, 36, .75), (1, 43, .5), (2, 36, .75), (3, 43, .5), (3.5, 42, .5), (4, 41, .75), (5, 39, .5), (6, 38, .5),
             (6.5, 37, .5), (7, 36, 1.0), (8, 44, .75), (9, 43, .5), (9.5, 44, .25), (9.75, 46, .25), (10, 47, .5),
             (10.5, 48, .5), (11, 36, 1.0)],
    # the flute's cadenza: a flurry up the harmonic minor, a shrill trill, a sinister augmented-2nd fall, the low C
    'FLUTE': [(0, 72, .25), (.25, 74, .25), (.5, 75, .25), (.75, 77, .25), (1, 79, .25), (1.25, 80, .25), (1.5, 83, .25),
              (1.75, 84, .25), (2, 87, .75), (2.75, 86, .125), (2.875, 87, .125), (3, 86, .125), (3.125, 87, .125),
              (3.25, 86, .125), (3.375, 87, .125), (3.5, 89, .5), (4, 91, .5), (4.5, 89, .25), (4.75, 87, .25),
              (5, 86, .25), (5.25, 84, .25), (5.5, 83, .5), (6, 80, .5), (6.5, 79, .25), (6.75, 80, .25), (7, 83, .5),
              (7.5, 84, .5), (8, 91, .25), (8.25, 89, .25), (8.5, 87, .25), (8.75, 86, .25), (9, 84, .25),
              (9.25, 83, .25), (9.5, 80, .25), (9.75, 79, .25), (10, 75, .5), (10.5, 74, .5), (11, 72, 1.0)],
}


def bar_t(b):
    return b * BAR


def beat_env(t, decay=0.12):
    return math.exp(-((t % BEAT) / decay))


def beat_pulse(t, decay=0.12, pre=0.12):
    """a pulse on every beat that ramps up over the last `pre` s before it (no one-frame snap), so the beat that lands on
    frame 0 doesn't jump at the loop seam"""
    ph = t % BEAT
    up = min(1.0, max(0.0, (ph - (BEAT - pre)) / pre))
    return max(math.exp(-ph / decay), up * up * (3 - 2 * up))


def next_bar(t):
    return math.ceil(t / BAR - 1e-6) * BAR
