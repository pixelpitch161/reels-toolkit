"""#34 BALLFIGHT #8: Trombone vs French Horn - ONE timeline for picture and sound.

Idea: the brass section's pecking order, on #27's engine, ring, HUD and count-in. TROMBONE (left corner, mint) points
its slide at the enemy and shoots it out like a lance on every beat (blade `mode='thrust'`; reach grows per landed
hit), so its thrusts ARE the low-brass ostinato: every thrust is a slide stab, every landed one a glissando accent.
HORN (right corner, copper) sounds a hunting call every two beats: slow arrows that home in on the enemy (volley with
`homing`; +half an arrow and stronger homing per landed volley), so its volleys are the noble horn theme. Levels from
damage dealt unlock two abilities each; below 25 HP each gets one desperation move. Honest simulated fight
(projects/ballfight/fight.py, rules tuned once for this pair with tune.py, then frozen), seed picked for pacing, never
edited.
Hook: the giant 3 . 2 . 1 . FIGHT! in the ring = the band's count-in (a braam + taiko per count, the drop lands on FIGHT);
top caption "TROMBONE / VS FRENCH HORN" with the BALLFIGHT #8 tag. HUD names TROMBONE / HORN.
Music, mood: cinematic brass trailer. D minor, low-string 16th ostinato, braams, taiko-style hits, heroic-dark, at
loop_bpm(122) = 121.97 BPM (31 bars = 61 s; 1 bar = the countdown). The two fighters are the brass section: the bed has
strings, braams and drums but no brass of its own. Registers apart so hits read: trombone low (A2-C4), horn mid (C4-D5).

Beat sheet (61 s; events after 2 s are the run's own: the seed was chosen so they land near these targets):
    0-1.97   balls rev in corners; giant 3 . 2 . 1 in the ring          TROMBONE / VS FRENCH HORN, then FIGHT!
    1.97-8   launch; thrusts vs homing arrows, HP ticking                -
    8-30     reach grows, arrows multiply; unlocks                       SAD TROMBONE, STOPPED HORN, POWER SLIDE, HORN RIP
    32-44    one dips below 25 HP                                        PEDAL TONE or TALLY-HO
    44-52    final exchange, HP in single digits                         -
    ~49-53   K.O.: hit-stop, slow-mo shatter (brass tubing, valves)      K.O. -> (WINNER) WINS
    then-58  winner's solo under the spot                                COMMENT THE / NEXT MATCH-UP
    58-61    loser reassembles; both glide to their corners; HP refills; "3" lands at 61.0 = frame 0
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
TITLE = ('TROMBONE', 'VS FRENCH HORN')
SERIES = 'BALLFIGHT #8'
T_FIGHT = BAR                   # FIGHT! lands on the bar-1 downbeat: the drop, the launch and the first thrust
COUNT = [(0.0, '3'), (BEAT, '2'), (2 * BEAT, '1'), (BAR, 'FIGHT!')]

# ---------------------------------------------------------------- frozen rules (tune.py; the same for every seed)
DMG_SCALE = 0.60                # v2 (THE HUNT), tuned once (tune.py, 200 seeds): median K.O. ~51 s, each side 40-60 %

# ---------------------------------------------------------------- story targets (seed_search.py scores runs against these)
T_KO = (49.0, 53.0)
MIN_LEAD_CHANGES = 2
MAX_WINNER_HP = 30.0
CAP_GAP = 4.0                   # never two ability captions within 4 s
CAP_TARGETS = {'SAD TROMBONE': 10, 'STOPPED HORN': 15, 'POWER SLIDE': 22, 'HORN RIP': 28, 'PEDAL TONE': 40,
               'TALLY-HO': 40}

# ---------------------------------------------------------------- the scripted tail (after the honest K.O.)
SLOWMO = 0.7                    # s of picture time the last instant of the fight is stretched over (the K.O. hit)
SLOWMO_RATE = 0.25
T_REASSEMBLE = 29 * BAR + 2 * BEAT   # 58.05: the loser's pieces fly back together
T_GLIDE = 30 * BAR              # 59.03, bar 30: both glide back to their corners, HP refills, weapons reset
T_REV = T_GLIDE + 1.05          # in the corners, revving (same motion as the opening)


# ---------------------------------------------------------------- music (D minor, cinematic brass trailer)
KEY_ROOT = 38                   # D
SCALE = (0, 2, 3, 5, 7, 8, 10)  # D natural minor (C# over the A chord)
# chords per bar (31 bars): the count-in on the dominant, then the 4-bar loop i - VI - iv - V (Dm . Bb . Gm . A),
# ending on Bb . A (bar 30 + the count-in bar 0), so the loop seam is the dominant resolving on FIGHT!
CHORDS = ['A'] + ['Dm', 'Bb', 'Gm', 'A'] * 7 + ['Bb', 'A']
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


def loop_beat(t):
    """beat index inside the 4-bar loop (bars 1.. follow it), snapped to the nearest beat"""
    k = int(round(t / BEAT))
    return (k - 4) % 16


def trombone_at(t):
    return TROMBONE_LINE[loop_beat(t)]


def horn_at(t):
    return HORN_LINE[loop_beat(t) // 2]


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
