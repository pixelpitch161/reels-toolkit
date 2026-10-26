"""#36 BALLFIGHT #10: Trombone vs Tuba - ONE timeline for picture and sound.

Idea: who owns the low end? The first crossover: two roster fighters, each fighting EXACTLY as built in its own
episode (projects/ballfight/fighters.py, unchanged): TROMBONE (left corner, mint, from #34) aims its slide and shoots it
out like a lance on every beat; TUBA (right corner, indigo, from #32 v2) inhales on "oom" and swallows the enemy, then
spits it into the ropes on "pah". The only per-episode lever is a damage handicap per fighter (DMG_SIDE, tuned once
with tune.py, then frozen; crossover rule). Honest simulated fight, seed picked for pacing, never edited.
Hook: bar 0 is the VS title card (projects/ballfight/title.py): mint / indigo panels split diagonally, both balls big
and close; TROMBONE (a slide glissando blast), VS (a thunder crack) and TUBA (a tuba blast) slam in on beats 1-3 over
unison house stabs; the doors open on beat 4. Bar 1 is the 3 . 2 . 1 count-in, FIGHT! on the bar-2 downbeat (3.81 s).
The title slam is the frame-0 reset: the doors slide shut over the last T_CLOSE s with a riser and a whoosh into it.
HUD (first episode with it): each fighter's ability list under its HP bar (grey until unlocked, lit on every use).
Music, mood C: brass house. A dark minor house beat (four-on-the-floor kick, offbeat hats, claps, a filtered minor pad
and chord stabs) at loop_bpm(126) = 125.90 BPM (32 bars = 61 s). The bed has no bass synth and no brass: the TUBA
fighter is the bass line (oom = the root, pah = the fifth) and the TROMBONE plays the tenor lead above it (slides and
smears, F3-Ab4), so both sit on top. Leader fuller and louder, loser thinner and dirtier; the winner's solo over the
groove.

Beat sheet (61 s; events after 3.8 s are the run's own: the seed was chosen so they land near these targets):
    0-1.91    the VS title card: TROMBONE, VS, TUBA slam in; the doors open on beat 4
    1.91-3.81 balls rev in corners; giant 3 . 2 . 1 in the ring, FIGHT! at 3.81
    3.81-8    launch; lance thrusts vs the first inhales                 -
    8-30      reach grows on both sides; the four unlocks                SAD TROMBONE, OOMPAH, POWER SLIDE, BRASS WALL
    32-44     one dips below 25 HP                                       PEDAL TONE or SUBSONIC
    44-52     final exchange, HP in single digits                        -
    ~49-53    K.O.: hit-stop, slow-mo shatter (slide / tubing / bell)    K.O. -> (WINNER) WINS
    then-57.7 winner's solo under the spot                               COMMENT THE / NEXT MATCH-UP
    57.7-61   loser reassembles; both glide to their corners; the doors shut and the title slams at 61.0 = frame 0
"""
import math

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(126, DUR)        # 125.90: 32 bars in 61 s, so the groove is in phase at the loop point
BEAT = 60.0 / BPM               # 0.4766 s: one lance thrust, one kick
BAR = 4 * BEAT                  # 1.9063 s = the countdown = one oom-pah
N_BARS = 32
S16 = BEAT / 4                  # hits snap to the nearest 16th in the audio (<= 60 ms)

FIGHTERS = ('TROMBONE', 'TUBA')  # left corner (screen left), right corner (screen right)
SERIES = 'BALLFIGHT #10'
T_FIGHT = 2 * BAR               # FIGHT! on the bar-2 downbeat (3.81 s): the drop, the launch, the first thrust
COUNT = [(BAR, '3'), (BAR + BEAT, '2'), (BAR + 2 * BEAT, '1'), (2 * BAR, 'FIGHT!')]     # bar 1: the count-in

# ---------------------------------------------------------------- the VS title card on bar 0 (projects/ballfight/title.py)
TITLE = [(0.0, 'TROMBONE'), (BEAT, 'VS'), (2 * BEAT, 'TUBA')]   # one word slams in per beat (beat 1 = frame 0)
T_DOORS = (3 * BEAT + 0.10, BAR - 0.03)   # beat 4: the two panels slide apart like doors to reveal the ring
T_CLOSE = 0.30                  # the last 0.30 s of the file: the doors slide shut again (the run-up into the frame-0 slam)

# ---------------------------------------------------------------- frozen rules (crossover: the fighters as built)
DMG_SCALE = 1.0                 # the shared scale stays 1: the whole lever is the per-fighter handicap below
DMG_SIDE = (0.35, 1.55)         # damage each fighter deals (TROMBONE, TUBA): tuned once (tune.py, 300 seeds: 49 % / 51 %, median K.O. 50.8 s), frozen

# ---------------------------------------------------------------- story targets (seed_search.py scores runs against these)
T_KO = (49.0, 53.0)
MIN_LEAD_CHANGES = 2
MAX_WINNER_HP = 30.0
CAP_GAP = 4.0                   # never two ability captions within 4 s
MAX_GAP = 4.0                   # no stretch over 4 s without a hit (hard window)
CAP_TARGETS = {'SAD TROMBONE': 12, 'OOMPAH': 12, 'POWER SLIDE': 24, 'BRASS WALL': 22, 'PEDAL TONE': 40,
               'SUBSONIC': 40}

# ---------------------------------------------------------------- the scripted tail (after the honest K.O.)
SLOWMO = 0.7                    # s of picture time the last instant of the fight is stretched over (the K.O. hit)
SLOWMO_RATE = 0.25
T_REASSEMBLE = 30 * BAR + BEAT  # 57.66: the loser's pieces fly back together
T_GLIDE = 31 * BAR              # 59.09, bar 31: both glide back to their corners, HP refills, weapons reset
T_REV = T_GLIDE + 1.05          # in the corners, revving (same motion as the opening)


# ---------------------------------------------------------------- music (F minor, brass house)
KEY_ROOT = 41                   # F2: the tuba's home
SCALE = (0, 2, 3, 5, 7, 8, 10)  # F natural minor (E natural over the C chord)
# chords per bar (32 bars): the title card on the tonic, the count-in on the dominant, then the 4-bar house loop
# i - VI - iv - V (Fm . Db . Bbm . C) from FIGHT!; the last two bars Db . C resolve into the title slam's Fm on frame 0
CHORDS = ['Fm', 'C'] + ['Fm', 'Db', 'Bbm', 'C'] * 7 + ['Db', 'C']
CHORD_ROOT = {'Fm': 41, 'Db': 37, 'Bbm': 34, 'C': 36}       # the tuba's roots (oom); pah = the fifth above
CHORD_PCS = {'Fm': (5, 8, 0), 'Db': (1, 5, 8), 'Bbm': (10, 1, 5), 'C': (0, 4, 7)}
assert len(CHORDS) == N_BARS


def chord_at(t):
    b = int(max(0.0, t) // BAR) % N_BARS
    return CHORDS[b]


def scale_notes(lo, hi, chord=None):
    """F minor between midi lo..hi (over C the Eb becomes E: the dominant's leading tone)"""
    pcs = [(KEY_ROOT + s) % 12 for s in SCALE]
    if chord == 'C':
        pcs = [p if p != 3 else 4 for p in pcs]
    return [m for m in range(lo, hi + 1) if m % 12 in pcs]


def chord_tones(lo, hi, chord):
    return [m for m in range(lo, hi + 1) if m % 12 in CHORD_PCS[chord]]


# The trombone's tenor lead: one pitch per beat over the 4-bar loop (16 slots), F3-Ab4. Every thrust plays the slot it
# lands on (thrusts are on the beat grid; POWER SLIDE's 8th thrusts add passing notes), so the lance IS the hook.
TROMBONE_LINE = [65, 63, 60, 63,    # Fm: F4 Eb4 C4 Eb4
                 65, 61, 60, 56,    # Db: F4 Db4 C4 Ab3
                 61, 60, 58, 61,    # Bbm: Db4 C4 Bb3 Db4
                 64, 67, 60, 64]    # C:  E4 G4 C4 E4


def loop_beat(t):
    """beat index inside the 4-bar loop (bars 2.. follow it), snapped to the nearest beat"""
    k = int(round(t / BEAT))
    return (k - 8) % 16                                         # the loop starts on FIGHT! (bar 2)


def trombone_at(t):
    return TROMBONE_LINE[loop_beat(t)]


def tuba_root(t):
    """the tuba's oom: the bar's root (F2 Db2 Bb1 C2); its pah is the fifth above"""
    return CHORD_ROOT[chord_at(t)]


# the winner's solo (beat offsets from the solo's first bar, midi, beats): 3 bars over the groove
SOLO = {
    # a house hook on the slide: smears up into each phrase, a call and an answer, down to the low F
    'TROMBONE': [(0, 60, .5), (.5, 63, .5), (1, 65, .75), (1.75, 67, .25), (2, 68, 1.0), (3, 67, .5), (3.5, 65, .5),
                 (4, 63, .5), (4.5, 65, .5), (5, 60, 1.0), (6, 58, .5), (6.5, 60, .5), (7, 63, .5), (7.5, 65, .5),
                 (8, 68, .75), (8.75, 67, .25), (9, 65, .5), (9.5, 64, .5), (10, 65, 2.0)],
    # a strutting bass riff: octave jumps and the fifth, a chromatic walk up into the tonic
    'TUBA': [(0, 41, .5), (.5, 53, .25), (.75, 41, .25), (1, 48, .5), (1.5, 41, .5), (2, 44, .5), (2.5, 46, .5),
             (3, 48, .75), (3.75, 47, .25), (4, 46, .5), (4.5, 44, .5), (5, 41, .5), (5.5, 53, .5), (6, 48, .5),
             (6.5, 49, .5), (7, 50, .5), (7.5, 52, .5), (8, 53, 1.0), (9, 48, .5), (9.5, 41, .5), (10, 41, 2.0)],
}


def bar_t(b):
    return b * BAR


def beat_env(t, decay=0.12):
    return math.exp(-((t % BEAT) / decay))


def beat_pulse(t, decay=0.12, pre=0.12):
    """a pulse on every beat that ramps up over the last `pre` s before it (no one-frame snap), so the beat that lands on
    frame 0 doesn't jump at the loop seam (#34's 120 ms ramp)"""
    ph = t % BEAT
    up = min(1.0, max(0.0, (ph - (BEAT - pre)) / pre))
    return max(math.exp(-ph / decay), up * up * (3 - 2 * up))


def next_bar(t):
    return math.ceil(t / BAR - 1e-6) * BAR
