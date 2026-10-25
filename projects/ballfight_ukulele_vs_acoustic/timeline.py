"""#33 BALLFIGHT #7: Ukulele vs Acoustic Guitar - ONE timeline for picture and sound.

Idea: little brother vs big brother on #27's engine, ring, HUD and count-in. UKULELE (left corner, pink) fights up
close: every 2nd beat it throws itself at the acoustic in a body-slam dash (new weapon kind `dash`: speed + distance
grow per landed dash) and strums as it lands. ACOUSTIC (right corner, wood tan) keeps it at range: on every beat it
strums a cone of sound out of its sound hole, aimed at the uke (new weapon kind `cone`: the cone gets longer per landed
strum). So the fight IS the strumming: the uke's off-beat slams against the acoustic's steady downstrokes. Levels from
damage dealt unlock two abilities each; below 25 HP each gets one desperation move. Honest simulated fight
(projects/ballfight/fight.py, rules re-tuned once for this pair with tune.py, then frozen), the seed picked for pacing,
never edited.
Hook (v3, series standard from Oct 24): bar 0 is the VS TITLE CARD (projects/ballfight/title.py): the frame splits
diagonally into a pink UKULELE panel (surf crests) and a wood-tan ACOUSTIC panel (grain + six strings), each with its
ball big and close; UKULELE slams in on beat 1 (frame 0) with a uke chord blast, VS cracks the divider on beat 2 with a
thunder crack, ACOUSTIC slams in on beat 3 with an open-chord blast, all over unison band hits; beat 4 the doors slide
apart. Bar 1: the giant 3 . 2 . 1 = the band's count-in (a whip crack + both fighters' strums per count); FIGHT! on the
bar-2 downbeat. No top caption (the card replaces it). The doors slide shut over the last T_CLOSE s with a riser and a
whoosh, so the title slam on frame 0 is the loop's reset. The second fighter is called ACOUSTIC everywhere.
Music, spaghetti-western standoff: A minor, the descending Am - G - F - E cadence, a galloping rhythm, a whistled-style
lead (synth), a twangy baritone guitar bed kept below both fighters, at loop_bpm(130) = 129.84 BPM (33 bars = 61 s;
1 bar = the countdown).

Beat sheet (61 s; events after the count-in are the run's own: the seed was chosen so they land near these targets):
    0-1.85   VS title card: UKULELE . VS . ACOUSTIC slam in, the doors open on beat 4
    1.85-3.7 giant 3 . 2 . 1 in the ring (the count-in)                   then FIGHT! on bar 2 (3.70 s)
    3.7-8    launch; first waves and rake strokes, HP ticking            -
    8-30     dash gets faster and longer, cone longer; the unlocks       CHUNK, CAPO, SPEED STRUM, FINGERPICK
    32-44    one dips below 25 HP                                        UKE-NADO or BIG STRUM
    44-52    final exchange, HP in single digits                         -
    ~49-53   K.O.: hit-stop, slow-mo shatter (wood, strings, frets)      K.O. -> (WINNER) WINS
    then-58  winner's solo under the spot                                COMMENT THE / NEXT MATCH-UP
    58-61    loser reassembles; both glide to their corners; HP refills; the doors shut: the title slam = frame 0
"""
import math

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(130, DUR)        # 129.84: 33 bars in 61 s, so the groove is in phase at the loop point
BEAT = 60.0 / BPM               # 0.4621 s: one acoustic strum
BAR = 4 * BEAT                  # 1.8485 s = the countdown
N_BARS = 33
S16 = BEAT / 4                  # hits snap to the nearest 16th in the audio (<= 58 ms)

FIGHTERS = ('UKULELE', 'ACOUSTIC')  # left corner (screen left), right corner (screen right)
SERIES = 'BALLFIGHT #7'         # posting order on the board (#29 = #3 ... #33 = #7); renumber if posted out of order
T_FIGHT = 2 * BAR               # v3: FIGHT! lands on the bar-2 downbeat (3.70 s): the drop, the launch, the first strum
COUNT = [(BAR, '3'), (BAR + BEAT, '2'), (BAR + 2 * BEAT, '1'), (2 * BAR, 'FIGHT!')]     # bar 1: the count-in
# the VS title card (projects/ballfight/title.py) on bar 0
TITLE = [(0.0, 'UKULELE'), (BEAT, 'VS'), (2 * BEAT, 'ACOUSTIC')]   # one word slams in per beat (beat 1 = frame 0)
T_DOORS = (3 * BEAT + 0.10, BAR - 0.03)  # beat 4: the panels slide apart like doors to reveal the ring
T_CLOSE = 0.30                  # the last 0.30 s of the file: the doors slide shut again (the run-up into the frame-0 slam)

# ---------------------------------------------------------------- frozen rules (tune.py; the same for every seed)
DMG_SCALE = 0.72                # v2 re-tuned once (tune.py, 160 seeds a cell), then frozen

# ---------------------------------------------------------------- story targets (seed_search.py scores runs against these)
T_KO = (49.0, 53.0)
MIN_LEAD_CHANGES = 2
MAX_WINNER_HP = 30.0
CAP_GAP = 4.0                   # never two ability captions within 4 s
CAP_TARGETS = {'CHUNK': 10, 'CAPO': 15, 'SPEED STRUM': 22, 'FINGERPICK': 28, 'UKE-NADO': 40, 'BIG STRUM': 40}

# ---------------------------------------------------------------- the scripted tail (after the honest K.O.)
SLOWMO = 0.7                    # s of picture time the last instant of the fight is stretched over (the K.O. hit)
SLOWMO_RATE = 0.25
T_REASSEMBLE = 31 * BAR + BEAT  # 57.77: the loser's pieces fly back together
T_GLIDE = 32 * BAR              # 59.15, bar 32: both glide back to their corners, HP refills, weapons reset
T_REV = T_GLIDE + 1.05          # in the corners, revving (same motion as the opening)


# ---------------------------------------------------------------- music (A minor, spaghetti-western standoff)
KEY_ROOT = 45                   # A
SCALE = (0, 2, 3, 5, 7, 8, 10)  # A natural minor; over E the G is raised to G# (A harmonic minor's dominant)
# chords per bar (33 bars), v3: the title card on the tonic (bar 0), the count-in on the dominant (bar 1), the descending
# cadence Am - G - F - E seven times from FIGHT! (bar 2), then Am - F - E, so bar 32 (E) resolves into the card's slam
CHORDS = ['Am', 'E'] + ['Am', 'G', 'F', 'E'] * 7 + ['Am', 'F', 'E']
CHORD_ROOT = {'Am': 45, 'G': 43, 'F': 41, 'E': 40}
CHORD_PCS = {'Am': (9, 0, 4), 'G': (7, 11, 2), 'F': (5, 9, 0), 'E': (4, 8, 11)}
assert len(CHORDS) == N_BARS
# voicings: the ukulele's reentrant GCEA shapes (Am 2000, G 0232, F 2010, E 4442) and the acoustic's open chords
UKE_CHORD = {'Am': (69, 60, 64, 69), 'G': (67, 62, 67, 71), 'F': (69, 60, 65, 69), 'E': (71, 64, 68, 71)}
AC_CHORD = {'Am': (45, 52, 57, 60, 64), 'G': (43, 47, 50, 55, 59, 67), 'F': (41, 48, 53, 57, 60, 65),
            'E': (40, 47, 52, 56, 59, 64)}


def chord_at(t):
    b = int(max(0.0, t) // BAR) % N_BARS
    return CHORDS[b]


def scale_notes(lo, hi, chord=None):
    """A minor between midi lo..hi (over E the G is raised to G#)"""
    pcs = [(KEY_ROOT + s) % 12 for s in SCALE]
    if chord == 'E':
        pcs = [p if p != 7 else 8 for p in pcs]
    return [m for m in range(lo, hi + 1) if m % 12 in pcs]


# The whistle: an original standoff theme over two turns of the cadence (8 bars): (bar in phrase, beat, midi, beats).
# Long held notes with a wide leap, sparse, so the fighters' strums stay on top.
WHISTLE = [
    (0, 0, 76, 1.5), (0, 1.5, 81, .5), (0, 2, 79, .5), (0, 2.5, 76, 1.5),
    (1, 0, 74, 2.0), (1, 2, 71, .5), (1, 2.5, 74, .5), (1, 3, 79, 1.0),
    (2, 0, 77, 1.5), (2, 1.5, 76, .5), (2, 2, 72, 2.0),
    (3, 0, 71, 1.0), (3, 1, 68, 1.0), (3, 2, 64, 2.0),
    (4, 0, 69, .5), (4, .5, 72, .5), (4, 1, 76, 1.0), (4, 2, 81, 2.0),
    (5, 0, 83, 1.0), (5, 1, 81, .5), (5, 1.5, 79, .5), (5, 2, 74, 2.0),
    (6, 0, 84, 1.5), (6, 1.5, 81, .5), (6, 2, 77, 1.0), (6, 3, 76, 1.0),
    (7, 0, 80, 1.0), (7, 1, 77, 1.0), (7, 2, 76, 2.0),
]

# the winner's solo (beat offsets from the solo's first bar, midi or a strum (tuple), beats)
SOLO = {
    # the uke: a bright strum-and-pick flourish up the cadence, a triple strum, ending on a high Am
    'UKULELE': [(0, UKE_CHORD['Am'], .5), (.5, 76, .25), (.75, 79, .25), (1, 81, .5), (1.5, 79, .25), (1.75, 76, .25),
                (2, UKE_CHORD['G'], .5), (2.5, 74, .25), (2.75, 76, .25), (3, 79, .5), (3.5, 74, .5),
                (4, UKE_CHORD['F'], .25), (4.25, UKE_CHORD['F'], .25), (4.5, 77, .25), (4.75, 76, .25), (5, 72, .5),
                (5.5, 76, .5), (6, UKE_CHORD['E'], .5), (6.5, 80, .25), (6.75, 83, .25), (7, 80, .5), (7.5, 76, .5),
                (8, UKE_CHORD['Am'], .25), (8.25, UKE_CHORD['Am'], .25), (8.5, UKE_CHORD['Am'], .25), (8.75, 81, .25),
                (9, 84, .5), (9.5, 81, .5), (10, 80, .5), (10.5, 76, .5), (11, (69, 72, 76, 81), 1.0)],
    # the acoustic: rasgueado bursts and bass runs down the cadence, ending on a full Am
    'ACOUSTIC': [(0, AC_CHORD['Am'], .5), (.5, 64, .25), (.75, 65, .25), (1, 64, .25), (1.25, 62, .25), (1.5, 60, .5),
                 (2, AC_CHORD['G'], .25), (2.25, AC_CHORD['G'], .25), (2.5, AC_CHORD['G'], .25), (2.75, AC_CHORD['G'], .25),
                 (3, 59, .5), (3.5, 57, .5), (4, AC_CHORD['F'], .5), (4.5, 57, .25), (4.75, 60, .25), (5, 65, .5),
                 (5.5, 64, .5), (6, AC_CHORD['E'], .25), (6.25, AC_CHORD['E'], .25), (6.5, AC_CHORD['E'], .25),
                 (6.75, AC_CHORD['E'], .25), (7, 52, .25), (7.25, 53, .25), (7.5, 56, .25), (7.75, 59, .25),
                 (8, AC_CHORD['Am'], .5), (8.5, 69, .25), (8.75, 72, .25), (9, 76, .5), (9.5, 72, .25), (9.75, 69, .25),
                 (10, AC_CHORD['E'], .25), (10.25, AC_CHORD['E'], .25), (10.5, 64, .25), (10.75, 68, .25),
                 (11, (45, 52, 57, 60, 64, 69), 1.0)],
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
