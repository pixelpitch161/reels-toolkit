"""#37 BALLFIGHT #11: Clarinet vs Oboe - ONE timeline for picture and sound.

Idea: the woodwinds' reed war, single reed vs double reed. CLARINET (left corner, lavender) is the only teleporter:
TWELFTH JUMP - the register key makes the clarinet jump a twelfth, so on every beat the ball vanishes and reappears
past the enemy, and the lavender zip line between its two spots cuts whatever it crosses. OBOE (right corner, olive)
is the only debuffer: TUNING A - every bar it sounds its A as a narrow tuning beam; a hit knocks the enemy OUT OF TUNE
for 2 bars (it wobbles, its hits play flat and land weaker, and every one of the oboe's nasal 8th notes stings it).
Both are game rules in the engine (projects/ballfight/fight.py: 'jump', 'tune' + 'detune'); the audio follows them.
Honest simulated fight (the rules tuned once with tune.py, then frozen), seed picked for pacing, never edited.
Hook: bar 0 is the VS title card (projects/ballfight/title.py): lavender / olive panels split diagonally, both balls
big and close; CLARINET (a clarinet glissando blast), VS (the thunder crack) and OBOE (the oboe's A, held) slam in on
beats 1-3 over unison storm hits; the doors open on beat 4. Bar 1 is the 3 . 2 . 1 count-in, FIGHT! on the bar-2
downbeat (3.39 s). The title slam is the frame-0 reset: the doors slide shut over the last T_CLOSE s into it.
HUD: each fighter's ability list under its HP bar (grey until unlocked, lit on every use; series rule Oct 25).
Music, mood A: Baroque storm. Harpsichord continuo + a racing minor string ostinato, stormy presto, all original, at
loop_bpm(140) = 141.64 BPM (36 bars = 61 s), D minor (the oboe's A is the dominant: its tuning note IS the storm's
pedal). The bed has no woodwinds: the CLARINET plays the lower, liquid line (chalumeau: one note per jump, the twelfth
above on every landed cut) and the OBOE the higher, nasal one (the A every bar, running 8ths while the enemy is out of
tune: the stings). Consistent loudness from frame 0 (project rule, Oct 26): the storm is at full level on the title
card; it builds with density, not level. Leader fuller and louder, loser thinner and sour (out of tune = flat, about a
quarter-tone at full depth); the winner's solo over the storm.

Beat sheet (61 s; events after 3.4 s are the run's own: the seed was chosen so they land near these targets):
    0-1.69    the VS title card: CLARINET, VS, OBOE slam in; the doors open on beat 4
    1.69-3.39 balls rev in corners; giant 3 . 2 . 1 in the ring, FIGHT! at 3.39
    3.39-8    launch; the first jumps vs the first A beams                -
    8-30      jump reach / detune depth grow; the four unlocks           SQUEAK, DOUBLE REED, CLARION, ENDLESS NOTE
    32-44     one dips below 25 HP                                       RHAPSODY or ILL WIND
    44-52     final exchange, HP in single digits                        -
    ~49-53    K.O.: hit-stop, slow-mo shatter (keys / reed / bell)       K.O. -> (WINNER) WINS
    then-57.6 winner's solo under the spot                               COMMENT THE / NEXT MATCH-UP
    57.6-61   loser reassembles; both glide to their corners; the doors shut and the title slams at 61.0 = frame 0
"""
import math

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(140, DUR)        # 141.64: 36 bars in 61 s, so the storm is in phase at the loop point
BEAT = 60.0 / BPM               # 0.4236 s: one jump
BAR = 4 * BEAT                  # 1.6944 s = the countdown = one A
N_BARS = 36
S16 = BEAT / 4                  # hits snap to the nearest 16th in the audio (<= 53 ms)

FIGHTERS = ('CLARINET', 'OBOE')  # left corner (screen left), right corner (screen right)
SERIES = 'BALLFIGHT #11'
T_FIGHT = 2 * BAR               # FIGHT! on the bar-2 downbeat (3.39 s): the launch
COUNT = [(BAR, '3'), (BAR + BEAT, '2'), (BAR + 2 * BEAT, '1'), (2 * BAR, 'FIGHT!')]     # bar 1: the count-in

# ---------------------------------------------------------------- the VS title card on bar 0 (projects/ballfight/title.py)
TITLE = [(0.0, 'CLARINET'), (BEAT, 'VS'), (2 * BEAT, 'OBOE')]   # one word slams in per beat (beat 1 = frame 0)
T_DOORS = (3 * BEAT + 0.10, BAR - 0.03)   # beat 4: the two panels slide apart like doors to reveal the ring
T_CLOSE = 0.30                  # the last 0.30 s of the file: the doors slide shut again (the run-up into the frame-0 slam)

# ---------------------------------------------------------------- frozen rules (tuned once with tune.py)
DMG_SCALE = 0.55              # tuned once (tune.py, 200 seeds: CLARINET 47 % / OBOE 53 %, median K.O. 50.8 s), frozen

# ---------------------------------------------------------------- story targets (seed_search.py scores runs against these)
T_KO = (49.0, 53.0)
MIN_LEAD_CHANGES = 2
MAX_WINNER_HP = 30.0
CAP_GAP = 4.0                   # never two ability captions within 4 s
MAX_GAP = 4.0                   # no stretch over 4 s without a hit (hard window)
CAP_TARGETS = {'SQUEAK': 12, 'DOUBLE REED': 12, 'CLARION': 24, 'ENDLESS NOTE': 24, 'RHAPSODY': 40, 'ILL WIND': 40}

# ---------------------------------------------------------------- the scripted tail (after the honest K.O.)
SLOWMO = 0.7                    # s of picture time the last instant of the fight is stretched over (the K.O. hit)
SLOWMO_RATE = 0.25
T_REASSEMBLE = 34 * BAR         # 57.61: the loser's pieces fly back together
T_GLIDE = 35 * BAR              # 59.31, bar 35: both glide back to their corners, HP refills, weapons reset
T_REV = T_GLIDE + 1.05          # in the corners, revving (same motion as the opening)


# ---------------------------------------------------------------- music (D minor, Baroque storm)
KEY_ROOT = 38                   # D2
SCALE = (0, 2, 3, 5, 7, 8, 10)  # D natural minor (C# over the A chord: the leading tone)
# chords per bar (36 bars): the title card on the tonic, the count-in on the dominant, then the 8-bar circle-of-fifths
# storm loop from FIGHT! (Dm Gm C F Bb Gm A A, four times); the last two bars Gm . A fall into the title slam's Dm
CHORDS = ['Dm', 'A'] + ['Dm', 'Gm', 'C', 'F', 'Bb', 'Gm', 'A', 'A'] * 4 + ['Gm', 'A']
CHORD_ROOT = {'Dm': 38, 'Gm': 43, 'C': 36, 'F': 41, 'Bb': 46, 'A': 45}      # continuo bass roots (D2 G2 C2 F2 Bb2 A2)
CHORD_PCS = {'Dm': (2, 5, 9), 'Gm': (7, 10, 2), 'C': (0, 4, 7), 'F': (5, 9, 0), 'Bb': (10, 2, 5), 'A': (9, 1, 4)}
assert len(CHORDS) == N_BARS


def chord_at(t):
    b = int(max(0.0, t) // BAR) % N_BARS
    return CHORDS[b]


def scale_notes(lo, hi, chord=None):
    """D minor between midi lo..hi (over A the C becomes C#)"""
    pcs = [(KEY_ROOT + s) % 12 for s in SCALE]
    if chord == 'A':
        pcs = [p if p != 0 else 1 for p in pcs]
    return [m for m in range(lo, hi + 1) if m % 12 in pcs]


def chord_tones(lo, hi, chord):
    return [m for m in range(lo, hi + 1) if m % 12 in CHORD_PCS[chord]]


# The clarinet's chalumeau line: one note per beat over the 8-bar loop (32 slots), D3-A4, a liquid broken-chord line;
# every jump plays the slot it lands on (CLARION's 8th jumps add the passing note between), a landed cut adds the
# twelfth above (the register key) as a grace.
CLARINET_LINE = [62, 57, 53, 57,    # Dm: D4 A3 F3 A3
                 58, 55, 50, 55,    # Gm: Bb3 G3 D3 G3
                 60, 55, 52, 55,    # C:  C4 G3 E3 G3
                 57, 53, 48, 53,    # F:  A3 F3 C3 F3
                 58, 53, 50, 53,    # Bb: Bb3 F3 D3 F3
                 55, 58, 62, 58,    # Gm: G3 Bb3 D4 Bb3
                 57, 61, 64, 61,    # A:  A3 C#4 E4 C#4
                 57, 52, 49, 52]    # A:  A3 E3 C#3 E3
# The oboe's running 8ths (the stings while the enemy is out of tune), A4-F6: one pitch per 8th over the 8-bar loop
OBOE_8THS = [74, 77, 81, 77, 74, 76, 77, 81,     # Dm
             79, 82, 86, 82, 79, 77, 74, 77,     # Gm
             76, 79, 84, 79, 76, 77, 79, 76,     # C
             77, 81, 84, 81, 77, 79, 81, 77,     # F
             77, 82, 86, 82, 77, 81, 82, 86,     # Bb
             79, 82, 86, 84, 82, 81, 79, 77,     # Gm
             76, 81, 85, 81, 76, 79, 81, 85,     # A
             81, 79, 77, 76, 73, 76, 79, 81]     # A
A_NOTE = 81                     # the tuning A (A5): one held A per bar from the oboe's beam


def loop_beat(t):
    """beat index inside the 8-bar loop (bars 2.. follow it), snapped to the nearest beat"""
    k = int(round(t / BEAT))
    return (k - 8) % 32


def loop_8th(t):
    k = int(round(t / (BEAT / 2)))
    return (k - 16) % 64


def clarinet_at(t):
    return CLARINET_LINE[loop_beat(t)]


def oboe_8th_at(t):
    return OBOE_8THS[loop_8th(t)]


def bass_root(t):
    return CHORD_ROOT[chord_at(t)]


# the winner's solo (beat offsets from the solo's first bar, midi, beats): 2 bars over the storm, ending on the tonic
SOLO = {
    # a chalumeau run that leaps a twelfth (the register key) and tumbles home
    'CLARINET': [(0, 50, .5), (.5, 53, .5), (1, 57, .5), (1.5, 62, .5), (2, 74, 1.0), (3, 72, .25), (3.25, 70, .25),
                 (3.5, 69, .25), (3.75, 67, .25), (4, 65, .5), (4.5, 64, .5), (5, 61, .5), (5.5, 64, .5),
                 (6, 62, 2.0)],
    # the A, then a nasal descending sequence over the storm, resolving on D
    'OBOE': [(0, 81, 1.0), (1, 82, .5), (1.5, 81, .5), (2, 79, .5), (2.5, 77, .5), (3, 76, .5), (3.5, 77, .5),
             (4, 74, .5), (4.5, 76, .5), (5, 73, .5), (5.5, 76, .5), (6, 74, 2.0)],
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
