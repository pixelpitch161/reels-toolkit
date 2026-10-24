"""#30 BALLFIGHT #4: Bagpipes vs Accordion - ONE timeline for picture and sound.

Idea: the "most hated instrument" final on #27's engine, ring, HUD and count-in. v2 (BALLFIGHT v2 redesign, user Oct 24):
the bag and the bellows are the weapons. BAGPIPES (left corner): BAG INFLATE - every bar the bag swells over 2 beats to
~1.6x size and mass (heavy, slow, body-slams), then lets go and rockets around the ring like a released balloon,
zig-zagging and squealing (contact damage; every landed hit grows the inflation and the flight). ACCORDION (right
corner): BELLOWS BITE - every 2 beats the end boards fly apart with the pleated bellows stretched between them like
jaws and snap shut a 16th-grid later (a landed bite grows the jaw reach). Abilities: WAR CRY (tartan shockwave),
HIGHLAND FLING (two flights per bar), BAG BURST (the bag pops); POLKA (bites every beat), SQUEEZEBOX (folds shut,
buttons glowing), FULL BELLOWS (the bellows stretch across the whole ring and clamp). Honest simulated fight
(projects/ballfight/fight.py, rules re-tuned for v2 with tune.py, then frozen), the seed picked for pacing, never
edited. v1 (drone aura vs bellows rings, seed 1581) is in git history and *_v1_backup.py.
Hook: the giant 3 . 2 . 1 . FIGHT! in the ring = the band's count-in (stomp + clap + a chanter blast and an accordion
chord per count, the drop lands on FIGHT); top caption "BAGPIPES / VS ACCORDION" with the BALLFIGHT #4 tag.
Music, mood: dark sea shanty. A dorian (the chanter's A, so the drone sits in it; dorian = the shanty's own minor),
stomping, a low male-choir hum pad, foot stomps and claps, no fiddle, at loop_bpm(122) = 121.97 BPM (31 bars = 61 s;
1 bar = the countdown). The bed has no lead: the bagpipes and the accordion are the band's two leads.

Beat sheet (61 s; events after 2 s are the run's own: the seed was chosen so they land near these targets):
    0-1.97   balls rev in corners; giant 3 . 2 . 1 in the ring          BAGPIPES / VS ACCORDION, then FIGHT!
    1.97-8   launch; the bag swells and rockets vs the snapping jaws      -
    8-30     the bag grows, the flights get longer, the jaws reach further; WAR CRY, POLKA, HIGHLAND FLING, SQUEEZEBOX
    32-44    one dips below 25 HP                                        BAG BURST or FULL BELLOWS
    44-52    final exchange, HP in single digits                         -
    ~49-53   K.O.: hit-stop, slow-mo shatter (pipes + tartan / buttons + bellows pleats)   K.O. -> (WINNER) WINS
    then-58  winner's solo under the spot                                COMMENT THE / NEXT MATCH-UP
    58-61    loser reassembles; both glide to their corners; HP refills; "3" lands at 61.0 = frame 0
"""
import math

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(122, DUR)        # 121.97: 31 bars in 61 s, so the groove is in phase at the loop point
BEAT = 60.0 / BPM               # 0.4919 s
BAR = 4 * BEAT                  # 1.9677 s = the countdown
N_BARS = 31
S16 = BEAT / 4                  # hits snap to the nearest 16th in the audio (<= 61 ms)

FIGHTERS = ('BAGPIPES', 'ACCORDION')   # left corner, right corner
SERIES = 'BALLFIGHT #4'
T_FIGHT = BAR                   # FIGHT! lands on the bar-1 downbeat: the drop, the launch, the first push
COUNT = [(0.0, '3'), (BEAT, '2'), (2 * BEAT, '1'), (BAR, 'FIGHT!')]

# ---------------------------------------------------------------- frozen rules (tune.py; the same for every seed)
DMG_SCALE = 0.76  # tuned once (tune.py) so the median K.O. sits near 50 s with each side winning about half

# ---------------------------------------------------------------- story targets (seed_search.py scores runs against these)
T_KO = (49.0, 53.0)
MIN_LEAD_CHANGES = 2
MAX_WINNER_HP = 30.0
CAP_GAP = 4.0                   # never two ability captions within 4 s
CAP_TARGETS = {'WAR CRY': 10, 'POLKA': 15, 'HIGHLAND FLING': 22, 'SQUEEZEBOX': 28, 'BAG BURST': 40, 'FULL BELLOWS': 40}

# ---------------------------------------------------------------- the scripted tail (after the honest K.O.)
SLOWMO = 0.7                    # s of picture time the last instant of the fight is stretched over (the K.O. hit)
SLOWMO_RATE = 0.25
T_REASSEMBLE = 29 * BAR + 2 * BEAT   # 58.05: the loser's pieces fly back together
T_GLIDE = 30 * BAR              # 59.03, bar 30: both glide back to their corners, HP refills, weapons reset
T_REV = T_GLIDE + 1.05          # in the corners, revving (same motion as the opening)


# ---------------------------------------------------------------- music (A dorian, dark sea shanty)
KEY_ROOT = 45                   # A
SCALE = (0, 2, 3, 5, 7, 9, 10)  # A dorian: A B C D E F# G (the chanter's A; the raised 6th is the shanty's colour)
# chords per bar (31 bars): the count-in on the dominant E, then the 4-bar loop i - VII - i - v (Am . G . Am . Em),
# ending on Am + E (bar 30) so the loop seam is two bars of E resolving on FIGHT!
CHORDS = ['E'] + ['Am', 'G', 'Am', 'Em'] * 7 + ['Am', 'E']
CHORD_ROOT = {'Am': 45, 'G': 43, 'Em': 40, 'E': 40}
CHORD_PCS = {'Am': (9, 0, 4), 'G': (7, 11, 2), 'Em': (4, 7, 11), 'E': (4, 8, 11, 2)}
assert len(CHORDS) == N_BARS


def chord_at(t):
    b = int(max(0.0, t) // BAR) % N_BARS
    return CHORDS[b]


def scale_notes(lo, hi, chord=None):
    """A dorian between midi lo..hi (over E the G is raised to G#: the dominant's leading note)"""
    pcs = [(KEY_ROOT + s) % 12 for s in SCALE]
    if chord == 'E':
        pcs = [p if p != 7 else 8 for p in pcs]
    return [m for m in range(lo, hi + 1) if m % 12 in pcs]


# The chanter's line: an original stomping shanty tune, one pitch per 8th over the 4-bar loop (32 slots), in the
# chanter's own range (G4 .. A5). The drone aura samples it: every tick plays the line's note at that moment, so a
# short contact is a phrase fragment and a long stay in the ring plays the tune.
_M = {  # (bar in loop, 8th) -> midi; slots in between hold the last note
    (0, 0): 69, (0, 1): 69, (0, 2): 72, (0, 3): 74, (0, 4): 76, (0, 5): 74, (0, 6): 72, (0, 7): 71,
    (1, 0): 67, (1, 1): 67, (1, 2): 71, (1, 3): 74, (1, 4): 79, (1, 5): 78, (1, 6): 74, (1, 7): 71,
    (2, 0): 69, (2, 1): 72, (2, 2): 76, (2, 3): 81, (2, 4): 79, (2, 5): 76, (2, 6): 74, (2, 7): 72,
    (3, 0): 71, (3, 1): 71, (3, 2): 67, (3, 3): 71, (3, 4): 76, (3, 5): 74, (3, 6): 71, (3, 7): 68,
}
MELODY = []
_last = 69
for _b in range(4):
    for _s in range(8):
        _last = _M.get((_b, _s), _last)
        MELODY.append(_last)


def melody_at(t):
    """the chanter line's pitch at time t (bars 1.. follow the 4-bar loop)"""
    k = int(round(t / (2 * S16)))
    b = (k // 8 - 1) % 4
    return MELODY[b * 8 + k % 8]


# the winner's solo (beat offsets from the solo's first bar, midi, beats)
SOLO = {
    # a bagpipe lament-into-reel: the long A with a birl, a climb to the high A, a dotted run down, the low A
    'BAGPIPES': [(0, 69, 1.0), (1, 71, .5), (1.5, 72, .5), (2, 74, .75), (2.75, 76, .25), (3, 76, 1.0), (4, 81, 1.5),
                 (5.5, 79, .5), (6, 78, .75), (6.75, 76, .25), (7, 74, .5), (7.5, 72, .5), (8, 71, .75), (8.75, 72, .25),
                 (9, 74, .5), (9.5, 71, .5), (10, 69, 2.0)],
    # an accordion musette run: a minor arpeggio up, a chromatic turn, chord stabs, the tonic
    'ACCORDION': [(0, 69, .5), (.5, 72, .5), (1, 76, .5), (1.5, 81, .5), (2, 80, .25), (2.25, 81, .25), (2.5, 83, .5),
                  (3, 84, 1.0), (4, 83, .25), (4.25, 81, .25), (4.5, 79, .25), (4.75, 78, .25), (5, 76, .5), (5.5, 74, .5),
                  (6, 72, .5), (6.5, 74, .25), (6.75, 72, .25), (7, 71, .5), (7.5, 68, .5), (8, 69, .5), (8.5, 72, .5),
                  (9, 76, .5), (9.5, 71, .5), (10, 69, 2.0)],
}


def bar_t(b):
    return b * BAR


def beat_env(t, decay=0.12):
    return math.exp(-((t % BEAT) / decay))


def beat_pulse(t, decay=0.12, pre=0.07):
    """a pulse on every beat that ramps up over the last `pre` s before it (no one-frame snap), so the beat that lands on
    frame 0 doesn't jump at the loop seam"""
    ph = t % BEAT
    up = min(1.0, max(0.0, (ph - (BEAT - pre)) / pre))
    return max(math.exp(-ph / decay), up * up * (3 - 2 * up))


def next_bar(t):
    return math.ceil(t / BAR - 1e-6) * BAR
