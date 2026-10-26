"""#34 BALLFIGHT #8: Trombone vs French Horn - ONE timeline for picture and sound.

Idea: the brass section's pecking order, on #27's engine, ring, HUD and count-in. TROMBONE (left corner, mint) points
its slide at the enemy and shoots it out like a lance on every beat (blade `mode='thrust'`; reach grows per landed
hit), so its thrusts ARE the low-brass ostinato: every thrust is a slide stab, every landed one a glissando accent.
HORN (right corner, copper) sounds ECHO CALL (v5, HORN v3; v2-v4: THE HUNT): a round blast on every beat, repeated by
copper echo ghosts trailing on its own past path (+1 ghost per landed blast, max 4), so its calls are the horn theme and
the echoes its delay. Levels from
damage dealt unlock two abilities each; below 25 HP each gets one desperation move. Honest simulated fight
(projects/ballfight/fight.py, rules tuned once for this pair with tune.py, then frozen), seed picked for pacing, never
edited.
Hook (v3, VS title card intro = series rule, Oct 24): bar 0 is the shared VS title card (projects/ballfight/title.py):
mint / copper panels split diagonally, both balls big and close; TROMBONE, VS and FRENCH HORN slam in on beats 1-3 (a
slide blast, a thunder crack as VS splits the divider, a horn call, over unison band hits), the doors slide apart on
beat 4. Bar 1 is the band's count-in 3 . 2 . 1 (a braam + taiko per count), FIGHT! on the bar-2 downbeat (3.94 s). No
top caption any more. The title slam is the frame-0 reset: the doors slide shut over the last T_CLOSE s with a riser
and a whoosh into it. HUD names TROMBONE / HORN.
Music, mood: cinematic brass trailer. D minor, low-string 16th ostinato, braams, taiko-style hits, heroic-dark, at
loop_bpm(122) = 121.97 BPM (31 bars = 61 s; 1 bar = the countdown). The two fighters are the brass section: the bed has
strings, braams and drums but no brass of its own. Registers apart so hits read: trombone low (A2-C4), horn mid (C4-D5).

Beat sheet (61 s; events after 2 s are the run's own: the seed was chosen so they land near these targets):
    0-1.97   the VS title card: TROMBONE, VS, FRENCH HORN slam in; the doors open on beat 4
    1.97-3.94 balls rev in corners; giant 3 . 2 . 1 in the ring, FIGHT! at 3.94
    3.94-10  launch; thrusts vs the first echo calls, HP ticking          -
    10-32    reach grows, the pack grows; unlocks                        STOPPED HORN, SAD TROMBONE, HORN RIP, POWER SLIDE
    32-44    one dips below 25 HP                                        PEDAL TONE or FINAL FLICK
    44-52    final exchange, HP in single digits                         -
    ~49-53   K.O.: hit-stop, slow-mo shatter (brass tubing, valves)      K.O. -> (WINNER) WINS
    then-58  winner's solo under the spot                                COMMENT THE / NEXT MATCH-UP
    58-61    loser reassembles; both glide to their corners; HP refills; the doors shut and the title slams at 61.0 = frame 0
"""
import math

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(122, DUR)        # 121.97: 31 bars in 61 s, so the groove is in phase at the loop point
BEAT = 60.0 / BPM               # 0.4919 s: one slide thrust
BAR = 4 * BEAT                  # 1.9677 s = the countdown
N_BARS = 31
S16 = BEAT / 4                  # hits snap to the nearest 16th in the audio (<= 61 ms)

FIGHTERS = ('TROMBONE', 'HORN')  # left corner (screen left), right corner (screen right)
SERIES = 'BALLFIGHT #8'
T_FIGHT = 2 * BAR               # v3: FIGHT! lands on the bar-2 downbeat (3.94 s): the drop, the launch, the first thrust
COUNT = [(BAR, '3'), (BAR + BEAT, '2'), (BAR + 2 * BEAT, '1'), (2 * BAR, 'FIGHT!')]     # bar 1: the band's count-in

# ---------------------------------------------------------------- v3: the VS title card on bar 0 (projects/ballfight/title.py)
TITLE = [(0.0, 'TROMBONE'), (BEAT, 'VS'), (2 * BEAT, 'FRENCH HORN')]   # one word slams in per beat (beat 1 = frame 0)
T_DOORS = (3 * BEAT + 0.10, BAR - 0.03)  # beat 4: the two panels slide apart like doors to reveal the ring
T_CLOSE = 0.30                  # the last 0.30 s of the file: the doors slide shut again (the run-up into the frame-0 slam)

# ---------------------------------------------------------------- frozen rules (tune.py; the same for every seed)
DMG_SCALE = 0.72                # v5 (ECHO CALL), tuned once (tune.py, 400 seeds): median K.O. 51.5 s, 49 / 51 % decided
                                # (v2 THE HUNT: 0.60)

# ---------------------------------------------------------------- story targets (seed_search.py scores runs against these)
T_KO = (49.0, 53.0)
MIN_LEAD_CHANGES = 2
MAX_WINNER_HP = 30.0
CAP_GAP = 4.0                   # never two ability captions within 4 s
CAP_TARGETS = {'SAD TROMBONE': 12, 'STOPPED HORN': 17, 'POWER SLIDE': 24, 'HORN RIP': 30, 'PEDAL TONE': 42,
               'FINAL FLICK': 42}      # v3: +1 bar (the fight starts at 2 bars); v5: FINAL FLICK replaces TALLY-HO

# ---------------------------------------------------------------- the scripted tail (after the honest K.O.)
SLOWMO = 0.7                    # s of picture time the last instant of the fight is stretched over (the K.O. hit)
SLOWMO_RATE = 0.25
T_REASSEMBLE = 29 * BAR + 2 * BEAT   # 58.05: the loser's pieces fly back together
T_GLIDE = 30 * BAR              # 59.03, bar 30: both glide back to their corners, HP refills, weapons reset
T_REV = T_GLIDE + 1.05          # in the corners, revving (same motion as the opening)


# ---------------------------------------------------------------- music (D minor, cinematic brass trailer)
KEY_ROOT = 38                   # D
SCALE = (0, 2, 3, 5, 7, 8, 10)  # D natural minor (C# over the A chord)
# chords per bar (31 bars), v3: the title card on the tonic (bar 0), the count-in on the dominant (bar 1), then the 4-bar
# loop i - VI - iv - V (Dm . Bb . Gm . A) from FIGHT! (bar 2), ending on A (bar 30): the seam resolves V -> i on the
# title slam
CHORDS = ['Dm', 'A'] + ['Dm', 'Bb', 'Gm', 'A'] * 7 + ['A']
CHORD_ROOT = {'Dm': 38, 'Bb': 34, 'Gm': 43, 'A': 45}
CHORD_PCS = {'Dm': (2, 5, 9), 'Bb': (10, 2, 5), 'Gm': (7, 10, 2), 'A': (9, 1, 4)}
assert len(CHORDS) == N_BARS


def chord_at(t):
    b = int(max(0.0, t) // BAR) % N_BARS
    return CHORDS[b]


def scale_notes(lo, hi, chord=None):
    """D minor between midi lo..hi (over A the C becomes C#: the dominant's leading tone)"""
    pcs = [(KEY_ROOT + s) % 12 for s in SCALE]
    if chord == 'A':
        pcs = [p if p != 0 else 1 for p in pcs]
    return [m for m in range(lo, hi + 1) if m % 12 in pcs]


def chord_tones(lo, hi, chord):
    return [m for m in range(lo, hi + 1) if m % 12 in CHORD_PCS[chord]]


# The trombone's ostinato: one pitch per beat over the 4-bar loop (16 slots). Every thrust plays the slot it lands on
# (thrusts are on the beat grid; POWER SLIDE's 8th thrusts add passing notes), so the lance IS the low-brass riff.
TROMBONE_LINE = [50, 50, 57, 55,    # Dm: D D A G
                 46, 46, 53, 50,    # Bb: Bb Bb F D
                 55, 55, 58, 57,    # Gm: G G Bb A
                 57, 57, 61, 52]    # A:  A A C# E
# The horn's theme: one note per 2 beats (8 slots over the 4-bar loop): a noble hunting call, 4ths and 5ths.
HORN_LINE = [62, 69,    # Dm: D4 A4
             70, 65,    # Bb: Bb4 F4
             67, 70,    # Gm: G4 Bb4
             69, 73]    # A:  A4 C#5 (-> D5 on the loop)

# v5 ECHO CALL: one call per beat (16 slots over the 4-bar loop), chord tones in the horn's middle register, so the
# delayed repeats (the same notes one beat later, quieter and darker) stack into the bar's chord
HORN_CALL = [62, 69, 65, 69,    # Dm: D4 A4 F4 A4
             62, 65, 70, 65,    # Bb: D4 F4 Bb4 F4
             62, 67, 70, 67,    # Gm: D4 G4 Bb4 G4
             61, 64, 69, 64]    # A:  C#4 E4 A4 E4


def loop_beat(t):
    """beat index inside the 4-bar loop (bars 2.. follow it), snapped to the nearest beat"""
    k = int(round(t / BEAT))
    return (k - 8) % 16                                         # v3: the loop starts on FIGHT! (bar 2)


def trombone_at(t):
    return TROMBONE_LINE[loop_beat(t)]


def horn_at(t):
    return HORN_LINE[loop_beat(t) // 2]


def call_at(t):
    return HORN_CALL[loop_beat(t)]


# the winner's solo (beat offsets from the solo's first bar, midi, beats)
SOLO = {
    # a dark slide solo: low and greasy, glissandi up into each phrase, down to the pedal D
    'TROMBONE': [(0, 50, .5), (.5, 53, .5), (1, 57, 1.0), (2, 55, .5), (2.5, 53, .5), (3, 50, 1.0), (4, 45, .5),
                 (4.5, 48, .5), (5, 50, .75), (5.75, 53, .25), (6, 57, 1.0), (7, 60, .5), (7.5, 58, .5), (8, 57, .5),
                 (8.5, 53, .5), (9, 50, 1.5)],
    # a heroic horn cadenza: the call leaps up a 4th and 5th, falls, climbs to the leading tone and resolves high
    'HORN': [(0, 62, .5), (.5, 69, .5), (1, 74, 1.0), (2, 72, .5), (2.5, 70, .5), (3, 69, 1.0), (4, 65, .5), (4.5, 69, .5),
             (5, 74, .75), (5.75, 72, .25), (6, 70, .5), (6.5, 69, .5), (7, 73, 1.0), (8, 74, 2.0)],
}


def bar_t(b):
    return b * BAR


def beat_env(t, decay=0.12):
    return math.exp(-((t % BEAT) / decay))


def beat_pulse(t, decay=0.12, pre=0.12):
    """a pulse on every beat that ramps up over the last `pre` s before it (no one-frame snap), so the beat that lands on
    frame 0 doesn't jump at the loop seam (120 ms here: this fight's typical frame step is low, so #28's 70 ms ramp left
    the seam at 2.6x a normal step; 120 ms -> under 2x)"""
    ph = t % BEAT
    up = min(1.0, max(0.0, (ph - (BEAT - pre)) / pre))
    return max(math.exp(-ph / decay), up * up * (3 - 2 * up))


def next_bar(t):
    return math.ceil(t / BAR - 1e-6) * BAR
