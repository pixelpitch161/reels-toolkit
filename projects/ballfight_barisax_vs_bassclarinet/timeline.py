"""#40 BALLFIGHT #14: Bari Sax vs Bass Clarinet - ONE timeline for picture and sound.

Idea: the real one vs the lookalike. The bass clarinet keeps getting called "the black saxophone" (same curved metal
neck, upturned bell, low growl). A crossover (series rule, #36): both fighters come exactly as built in the shared
roster (projects/ballfight/fighters.py): BARI SAX as in #39 v2 (WRECKING BALL: every other bar a crane cable hooks
its harness and it swings across the ring as a pendulum; GROWL, SKANK, CABLE SNAP), BASS CLARINET as in #38 (ENDPIN
ANCHOR: every bar it plants its floor peg and whirls round it like a hammer-thrower; SLAP TONGUE, MULTIPHONICS, LIGHTS
OUT). The only per-episode lever is a damage handicap per fighter (Rules(dmg_side), tuned once with tune.py, frozen
below). Sides for this episode only (first named = left): BARI SAX left, BASS CLARINET right, the reverse of their own
episodes; the engine seats fighters by the order they are passed (FIGHTERS below), so the roster is untouched. (The
engine mirrors spin by side, as for every right-corner fighter: the endpin whirl turns clockwise here.)
Honest simulated fight, seed picked for pacing, never edited.
Hook: bar 0 is the VS title card (projects/ballfight/title.py): coral / slate-blue panels split diagonally, both balls
big and close; BARI SAX (a bari honk), VS (the thunder crack) and BASS / CLARINET (a growl + slap) slam in on beats 1-3
over unison band hits; the doors open on beat 4. Bar 1 is the 3 . 2 . 1 count-in, FIGHT! on the bar-2 downbeat
(3.81 s). The title slam is the frame-0 reset: the doors slide shut over the last T_CLOSE s into it.
HUD: each fighter's ability list under its HP bar (grey until unlocked, lit on every use; series rule Oct 25).
Music, mood A: industrial construction site. loop_bpm(126) = 125.90 BPM (32 bars = 61 s), D minor: clanging metal
percussion (anvils, pipes, sheet-metal hits), hydraulic hisses and air-wrench bursts as fills, a distorted minor groove
with no bass instrument of its own. The BARI SAX is the honking bass line on its swings (the bar's root on every swing,
as in #39 v2); the BASS CLARINET plays the growling counter-line an octave up (a chalumeau growl through every whirl,
slap pops on smashes). Consistent loudness from frame 0 (project rule, Oct 26): full groove on the title card; it builds
with density, not level. Leader fuller and louder, loser thinner and dirtier; the winner's solo over the machinery.

Beat sheet (61 s; events after 3.81 s are the run's own: the seed was chosen so they land near these targets):
    0-1.91    the VS title card: BARI SAX, VS, BASS CLARINET slam in; the doors open on beat 4
    1.91-3.81 balls rev in corners; giant 3 . 2 . 1 in the ring, FIGHT! at 3.81
    3.81-8    launch; the first swings vs the first whirls                    -
    8-30      cable length / orbit grow; the four unlocks                     GROWL, SLAP TONGUE, SKANK, MULTIPHONICS
    32-44     one dips below 25 HP                                            CABLE SNAP or LIGHTS OUT
    44-52     final exchange, HP in single digits                             -
    ~49-53    K.O.: hit-stop, slow-mo shatter                                 K.O. -> (WINNER) WINS
    then-57.2 winner's solo under the spot                                    COMMENT THE / NEXT MATCH-UP
    57.2-61   loser reassembles; both glide to their corners; the doors shut and the title slams at 61.0 = frame 0
"""
import math

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(126, DUR)        # 125.90: 32 bars in 61 s, so the groove is in phase at the loop point
BEAT = 60.0 / BPM               # 0.4766 s
BAR = 4 * BEAT                  # 1.9063 s = the countdown = one whirl cycle
N_BARS = 32
S16 = BEAT / 4                  # hits snap to the nearest 16th in the audio (<= 60 ms)

FIGHTERS = ('BARI SAX', 'BASS CLARINET')  # this episode's sides: left corner (screen left), right corner (screen right)
SERIES = 'BALLFIGHT #14'
T_FIGHT = 2 * BAR               # FIGHT! on the bar-2 downbeat (3.81 s): the launch
COUNT = [(BAR, '3'), (BAR + BEAT, '2'), (BAR + 2 * BEAT, '1'), (2 * BAR, 'FIGHT!')]     # bar 1: the count-in

# ---------------------------------------------------------------- the VS title card on bar 0 (projects/ballfight/title.py)
TITLE = [(0.0, 'BARI SAX'), (BEAT, 'VS'), (2 * BEAT, 'BASS CLARINET')]   # one word slams in per beat (beat 1 = frame 0)
TITLE_LINES = {'BASS CLARINET': ('BASS', 'CLARINET')}   # 13 letters: two lines (as #38 did; here name B, the card's
                                                        # #40 opt-in)
TITLE_BALLS = [(78, 222, 56), (302, 352, 48)]          # the right ball up and right: clear of VS and the two lines
T_DOORS = (3 * BEAT + 0.10, BAR - 0.03)   # beat 4: the two panels slide apart like doors to reveal the ring
T_CLOSE = 0.30                  # the last 0.30 s of the file: the doors slide shut again (the run-up into the frame-0 slam)

# ---------------------------------------------------------------- frozen rules (crossover: the damage handicap only)
DMG_SCALE = 1.0                 # the shared scale stays 1.0; each fighter's damage is its own handicap below
DMG_SIDE = (0.30, 0.66)         # BARI SAX, BASS CLARINET: v2 (the bari swings out over the ropes, #40 v2 Oct 27): re-tuned
                                # once (tune.py, 250 seeds: 48 % / 52 % of 239 decided, median K.O. 50.8 s), frozen.
                                # v1: (0.35, 0.68). For reference: 0.32 in #39 v2, 0.68 in #38

# ---------------------------------------------------------------- story targets (seed_search.py scores runs against these)
T_KO = (49.0, 53.0)
MIN_LEAD_CHANGES = 2
MAX_WINNER_HP = 30.0
CAP_GAP = 4.0                   # never two ability captions within 4 s
MAX_GAP = 4.0                   # no stretch over 4 s without a hit (hard window)
CAP_TARGETS = {'GROWL': 12, 'SLAP TONGUE': 12, 'SKANK': 24, 'MULTIPHONICS': 24, 'CABLE SNAP': 40, 'LIGHTS OUT': 40}

# ---------------------------------------------------------------- the scripted tail (after the honest K.O.)
SLOWMO = 0.7                    # s of picture time the last instant of the fight is stretched over (the K.O. hit)
SLOWMO_RATE = 0.25
T_REASSEMBLE = 30 * BAR         # 57.19: the loser's pieces fly back together
T_GLIDE = 31 * BAR              # 59.10, bar 31: both glide back to their corners, HP refills, weapons reset
T_REV = T_GLIDE + 1.05          # in the corners, revving (same motion as the opening)


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


# ---------------------------------------------------------------- music (G minor, industrial construction site)
KEY_ROOT = 43                   # G2
SCALE = (0, 2, 3, 5, 7, 8, 10)  # G natural minor (F# over D: the leading tone)
# chords per bar (32 bars): the title card on the tonic, the count-in on the dominant, then the 4-bar loop from FIGHT!
# (Gm Eb Cm D, seven times); the last two bars Eb . D fall into the title slam's Gm on frame 0
CHORDS = ['Gm', 'D'] + ['Gm', 'Eb', 'Cm', 'D'] * 7 + ['Eb', 'D']
CHORD_ROOT = {'Gm': 43, 'Eb': 39, 'Cm': 36, 'D': 38}       # the BARI's bass roots (G2 Eb2 C2 D2: all inside its range)
CHORD_PCS = {'Gm': (7, 10, 2), 'Eb': (3, 7, 10), 'Cm': (0, 3, 7), 'D': (2, 6, 9)}
assert len(CHORDS) == N_BARS


def chord_at(t):
    b = int(max(0.0, t) // BAR) % N_BARS
    return CHORDS[b]


def chord_tones(lo, hi, chord):
    return [m for m in range(lo, hi + 1) if m % 12 in CHORD_PCS[chord]]


def bass_root(t):
    return CHORD_ROOT[chord_at(t)]


# The BARI SAX's bass line: the bar's root on every swing (a huge doppler honk, swelling to the bottom of the arc;
# SKANK's off-beat swings honk up the octave), and while it rolls between swings it pumps the root + fifth on the
# beats (an engine idling), BARI_PUMP over the 4-bar loop (16 beats).
BARI_PUMP = [43, 43, 50, 43,      # Gm: G2 . D3 .
             39, 39, 46, 39,      # Eb: Eb2 . Bb2
             36, 36, 43, 36,      # Cm: C2 . G2
             38, 38, 45, 42]      # D: D2 . A2, F#2 leads home
# The BASS CLARINET's counter-line, an octave above the bass (G3-D4 chalumeau growl): one note per 8th over the loop
# (32 slots); every 8th of a whirl plays its slot, the peg going in plays the bar's root an octave up, a smash pops a
# slap tongue on the slot's pitch.
COUNTER_8THS = [55, 55, 62, 55, 58, 55, 62, 57,      # Gm
                51, 51, 58, 51, 55, 51, 58, 55,      # Eb
                48, 48, 55, 48, 51, 48, 55, 51,      # Cm
                50, 50, 57, 50, 54, 50, 57, 54]      # D (F#: the leading tone)


def loop_beat(t):
    """beat index inside the 4-bar loop (bars 2.. follow it), snapped to the nearest beat"""
    return (int(round(t / BEAT)) - 8) % 16


def loop_8th(t):
    return (int(round(t / (BEAT / 2))) - 16) % 32


def pump_at(t):
    return BARI_PUMP[loop_beat(t)]


def counter_at(t):
    return COUNTER_8THS[loop_8th(t)]


# the winner's solo (beat offsets from the solo's first bar, midi, beats): 2 bars over the machinery. Seed 617's K.O.
# at 50.55 s puts it on bars 28-29 (Cm, D), so both outline Cm then D and land on D (the dominant into the reassembly).
SOLO = {
    # big honks: the low C, a scoop up the chord, a growled run down, the low D held
    'BARI SAX': [(0, 36, .75), (1, 43, .5), (1.5, 48, .5), (2, 51, .25), (2.5, 48, .5), (3, 46, .5), (3.5, 43, .25),
                 (4, 38, .5), (5, 50, .25), (5.25, 48, .25), (5.5, 45, .25), (5.75, 42, .25), (6, 38, 1.5)],
    # a chalumeau climb with slap pops (the 0.25-beat notes), falling back to the D
    'BASS CLARINET': [(0, 48, .5), (.5, 51, .5), (1, 55, .5), (1.5, 58, .5), (2, 60, .75), (2.75, 58, .25),
                      (3, 55, .5), (3.5, 51, .5), (4, 50, .5), (4.5, 54, .5), (5, 57, .5), (5.5, 54, .25),
                      (5.75, 50, .25), (6, 50, 1.5)],
}
