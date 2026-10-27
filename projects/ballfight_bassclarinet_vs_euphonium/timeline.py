"""#38 BALLFIGHT #12: Bass Clarinet vs Euphonium - ONE timeline for picture and sound.

Idea: the two instruments nobody can name, "a black saxophone" vs "a small tuba", unsung heroes of their sections.
BASS CLARINET (left corner, slate blue) is the only anchor-and-pivot fighter: ENDPIN ANCHOR - every bar it drives its
floor peg into the canvas and whirls round it like a hammer-thrower, its long body smashing anything in the circle,
then pulls the peg and is flung off along the tangent (the orbit grows per landed smash). EUPHONIUM (right corner,
amber) is the only artillery: BELL MORTAR - every 2 beats the upright bell lobs brass shells that arc high and land on
the enemy's predicted spot two beats later; growing shadow circles telegraph them; splash damage (+ half a shell per
landed salvo). Game rules in the shared engine (projects/ballfight/fight.py: 'anchor', 'mortar', 'barrage', 'twin',
'copy', 'lights_out'); the audio follows them. Honest simulated fight (rules tuned once with tune.py, then frozen),
seed picked for pacing, never edited.
Hook: bar 0 is the VS title card (projects/ballfight/title.py): slate / amber panels split diagonally, both balls big
and close; BASS / CLARINET (two lines: 13 letters is the longest name yet), VS and EUPHONIUM slam in on beats 1-3 over
unison drumline hits (a bass-clarinet growl + slap, the thunder crack, a euphonium blast); the doors open on beat 4.
Bar 1 is the 3 . 2 . 1 count-in (the snare line's), FIGHT! on the bar-2 downbeat (3.81 s). The title slam is the
frame-0 reset: the doors slide shut over the last T_CLOSE s into it.
HUD: each fighter's ability list under its HP bar (grey until unlocked, lit on every use; series rule Oct 25).
Music, mood A: dark drumline. A minor marching-band battle cadence: snare line, quads / tenors, five pitched bass drums,
cymbal line, a front-ensemble pit (marimba ostinato, timpani) for the harmony; no low brass or woodwinds in the bed.
loop_bpm(126) = 125.90 BPM (32 bars = 61 s), C minor. The BASS CLARINET takes the rumbling bottom (a chalumeau note per
half turn of its whirl, slap-tongue pops on the smashes), the EUPHONIUM the warm tenor melody (one note per shell
fired, a fat chord tone when a salvo lands). Consistent loudness from frame 0 (project rule, Oct 26): the drumline is
at full level on the title card; it builds with density (more drums, rolls, cymbals), never level. Leader fuller and
louder, loser thinner and dirtier; the winner's solo over the cadence.

Beat sheet (61 s; events after 3.8 s are the run's own: the seed was chosen so they land near these targets):
    0-1.91    the VS title card: BASS CLARINET, VS, EUPHONIUM slam in; the doors open on beat 4
    1.91-3.81 balls rev in corners; giant 3 . 2 . 1 in the ring, FIGHT! at 3.81
    3.81-8    launch; the first whirls vs the first shells                -
    8-30      orbit / salvo grow; the four unlocks                       SLAP TONGUE, MARCHING ORDERS, MULTIPHONICS,
                                                                         IDENTITY CRISIS
    32-44     one dips below 25 HP                                       LIGHTS OUT or NOT A TUBA
    44-52     final exchange, HP in single digits                        -
    ~49-53    K.O.: hit-stop, slow-mo shatter (keys / neck / bell / valves)   K.O. -> (WINNER) WINS
    then-57.2 winner's solo under the spot                               COMMENT THE / NEXT MATCH-UP
    57.2-61   loser reassembles; both glide to their corners; the doors shut and the title slams at 61.0 = frame 0
"""
import math

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(126, DUR)        # 125.90: 32 bars in 61 s, so the cadence is in phase at the loop point
BEAT = 60.0 / BPM               # 0.4766 s
BAR = 4 * BEAT                  # 1.9063 s = the countdown = one whirl cycle
N_BARS = 32
S16 = BEAT / 4                  # hits snap to the nearest 16th in the audio (<= 60 ms)

FIGHTERS = ('BASS CLARINET', 'EUPHONIUM')  # left corner (screen left), right corner (screen right)
SERIES = 'BALLFIGHT #12'
T_FIGHT = 2 * BAR               # FIGHT! on the bar-2 downbeat (3.81 s): the launch
COUNT = [(BAR, '3'), (BAR + BEAT, '2'), (BAR + 2 * BEAT, '1'), (2 * BAR, 'FIGHT!')]     # bar 1: the count-in

# ---------------------------------------------------------------- the VS title card on bar 0 (projects/ballfight/title.py)
TITLE = [(0.0, 'BASS CLARINET'), (BEAT, 'VS'), (2 * BEAT, 'EUPHONIUM')]   # one word slams in per beat (beat 1 = frame 0)
TITLE_LINES = {'BASS CLARINET': ('BASS', 'CLARINET')}   # 13 letters: two lines at scale 6 (one line would be scale 4)
TITLE_BALLS = [(74, 250, 54), (282, 396, 56)]           # the left ball a little lower: room for the second line
T_DOORS = (3 * BEAT + 0.10, BAR - 0.03)   # beat 4: the two panels slide apart like doors to reveal the ring
T_CLOSE = 0.30                  # the last 0.30 s of the file: the doors slide shut again (the run-up into the frame-0 slam)

# ---------------------------------------------------------------- frozen rules (tuned once with tune.py)
DMG_SCALE = 0.68                # tuned once (tune.py), frozen

# ---------------------------------------------------------------- story targets (seed_search.py scores runs against these)
T_KO = (49.0, 53.0)
MIN_LEAD_CHANGES = 2
MAX_WINNER_HP = 30.0
CAP_GAP = 4.0                   # never two ability captions within 4 s
MAX_GAP = 4.0                   # no stretch over 4 s without a hit (hard window)
CAP_TARGETS = {'SLAP TONGUE': 12, 'MARCHING ORDERS': 12, 'MULTIPHONICS': 24, 'IDENTITY CRISIS': 24, 'LIGHTS OUT': 40,
               'NOT A TUBA': 40}

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


# ---------------------------------------------------------------- music (C minor, dark drumline)
KEY_ROOT = 36                   # C2
SCALE = (0, 2, 3, 5, 7, 8, 10)  # C natural minor (B natural over G: the leading tone)
# chords per bar (32 bars): the title card on the tonic, the count-in on the dominant, then the 4-bar battle loop from
# FIGHT! (Cm Ab Fm G, seven times); the last two bars Ab . G fall into the title slam's Cm on frame 0
CHORDS = ['Cm', 'G'] + ['Cm', 'Ab', 'Fm', 'G'] * 7 + ['Ab', 'G']
CHORD_ROOT = {'Cm': 36, 'Ab': 44, 'Fm': 41, 'G': 43}          # timpani / bass-drum roots (C2 Ab2 F2 G2)
CHORD_PCS = {'Cm': (0, 3, 7), 'Ab': (8, 0, 3), 'Fm': (5, 8, 0), 'G': (7, 11, 2)}
assert len(CHORDS) == N_BARS


def chord_at(t):
    b = int(max(0.0, t) // BAR) % N_BARS
    return CHORDS[b]


def chord_tones(lo, hi, chord):
    return [m for m in range(lo, hi + 1) if m % 12 in CHORD_PCS[chord]]


# The BASS CLARINET's rumble (the bottom): one chalumeau note per 8th over the 4-bar loop (32 slots), C2-G3; every
# 8th of a whirl plays its slot (the rumble while it spins), the peg going in plays the bar's root, smashes pop a slap
# tongue on the slot's pitch.
BASS_8THS = [36, 36, 43, 36, 46, 36, 43, 39,      # Cm
             44, 44, 51, 44, 39, 44, 48, 51,      # Ab
             41, 41, 48, 41, 44, 41, 48, 44,      # Fm
             43, 43, 50, 43, 47, 43, 50, 47]      # G (B natural: the leading tone)
# The EUPHONIUM's tenor melody: one note per beat over the loop (16 slots), G3-C5; every salvo plays the beat it is
# fired on (MARCHING ORDERS: a salvo every beat = the whole melody), a landed salvo adds a fat chord tone under it.
MELODY = [67, 68, 67, 63,     # Cm
          60, 63, 68, 72,     # Ab
          72, 68, 65, 68,     # Fm
          67, 62, 59, 62]     # G


def loop_beat(t):
    """beat index inside the 4-bar loop (bars 2.. follow it), snapped to the nearest beat"""
    return (int(round(t / BEAT)) - 8) % 16


def loop_8th(t):
    return (int(round(t / (BEAT / 2))) - 16) % 32


def bass_at(t):
    return BASS_8THS[loop_8th(t)]


def melody_at(t):
    return MELODY[loop_beat(t)]


def bass_root(t):
    return CHORD_ROOT[chord_at(t)]


# the winner's solo (beat offsets from the solo's first bar, midi, beats): 2 bars over the cadence (the K.O. at 51.7 s
# puts it on bars 28-29: Fm, G), ending on the leading chord into the reassembly
SOLO = {
    # a chalumeau climb with slap pops (marked by the 0.25-beat notes), tumbling back down to the G
    'BASS CLARINET': [(0, 41, .5), (.5, 48, .5), (1, 53, .5), (1.5, 56, .5), (2, 55, .75), (2.75, 53, .25), (3, 51, .5),
                      (3.5, 48, .5), (4, 47, .5), (4.5, 50, .5), (5, 55, .5), (5.5, 53, .5), (6, 50, .5), (6.5, 47, .5),
                      (7, 43, 1.0)],
    # a singing tenor line, hymn-like, over the cadence
    'EUPHONIUM': [(0, 65, 1.0), (1, 68, .5), (1.5, 72, .5), (2, 70, 1.0), (3, 68, .5), (3.5, 65, .5), (4, 67, 1.0),
                  (5, 62, .5), (5.5, 65, .5), (6, 67, 2.0)],
}
