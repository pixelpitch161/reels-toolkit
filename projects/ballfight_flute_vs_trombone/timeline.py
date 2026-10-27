"""#41 BALLFIGHT #15: Flute vs Trombone - ONE timeline for picture and sound.

Idea: the front row vs the back row. A real band-room feud: the flutes sit up front, the trombones behind them poke them
with their slides; now the flute answers. A crossover (series rule, #36): both fighters come exactly as built in the
shared roster (projects/ballfight/fighters.py): FLUTE as in #32 v2 (SNAKE CHARMER: it drops woven baskets, up to 2;
silver cobras rise, sway and strike on the beat; FLUTTER TONGUE, SNAKE DANCE, PICCOLO), TROMBONE as in #34 (SLIDE
LANCE: aims and shoots the slide out on every beat, swats projectiles it passes through; SAD TROMBONE, POWER SLIDE,
PEDAL TONE). The only per-episode lever is a damage handicap per fighter (Rules(dmg_side), tuned once with tune.py,
frozen below). Sides for this episode only (first named = left): FLUTE left, TROMBONE right, both swapped from their
own episodes; the engine seats fighters by the order they are passed (FIGHTERS below), so the roster is untouched.
Lance vs cobras: the engine didn't define it, so it is the opt-in rule `Rules(swat_cobras=True)` (fight.py, own
commit): a live lance that passes through a risen cobra knocks it flat (a strike in the air misses, no strikes for
2 beats); no damage, no growth.
Honest simulated fight, seed picked for pacing, never edited.
Hook: bar 0 is the VS title card (projects/ballfight/title.py): silver / mint panels split diagonally, both balls big
and close; FLUTE (a flute flurry), VS (the thunder crack) and TROMBONE (a trombone smear) slam in on beats 1-3 over
unison band hits; the doors open on beat 4. Bar 1 is the 3 . 2 . 1 count-in, FIGHT! on the bar-2 downbeat (3.59 s).
The title slam is the frame-0 reset: the doors slide shut over the last T_CLOSE s into it.
HUD: each fighter's ability list under its HP bar (grey until unlocked, lit on every use; series rule Oct 25).
Music, mood A: sinister big top. loop_bpm(132) = 133.77 BPM (34 bars = 61 s), E minor (harmonic: D# over B7): a dark
circus with a band-organ / calliope on the off-beat chords, a bass drum on the "oom", snare rolls, cymbal crashes, a
glockenspiel glint; the bed has no brass and no flute of its own. The TROMBONE is the bass and the smears (each thrust
= the circus bass's root / fifth with a smear into it; POWER SLIDE walks 8ths; SAD TROMBONE = the clown's wah-wah);
the FLUTE is the high melody (each cobra strike plays the melody's note at that 8th; FLUTTER TONGUE flutters it).
Consistent loudness from frame 0 (project rule, Oct 26): the full band on the title card; it builds with density, not
level. Leader fuller and louder, loser thinner and dirtier; the winner's solo over the big top.

Beat sheet (61 s; events after 3.59 s are the run's own: the seed was chosen so they land near these targets):
    0-1.79    the VS title card: FLUTE, VS, TROMBONE slam in; the doors open on beat 4
    1.79-3.59 balls rev in corners; giant 3 . 2 . 1 in the ring, FIGHT! at 3.59
    3.59-8    launch; the first baskets vs the first thrusts                  -
    8-30      strike reach / lance reach grow; the four unlocks              FLUTTER TONGUE, SAD TROMBONE, SNAKE DANCE, POWER SLIDE
    32-44     one dips below 25 HP                                            PICCOLO or PEDAL TONE
    44-52     final exchange, HP in single digits                             -
    ~49-53    K.O.: hit-stop, slow-mo shatter                                 K.O. -> (WINNER) WINS
    then-57.4 winner's solo under the spot                                    COMMENT THE / NEXT MATCH-UP
    57.4-61   loser reassembles; both glide to their corners; the doors shut and the title slams at 61.0 = frame 0
"""
import math

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(132, DUR)        # 133.77: 34 bars in 61 s, so the groove is in phase at the loop point
BEAT = 60.0 / BPM               # 0.4485 s: one lance thrust, one cobra strike
BAR = 4 * BEAT                  # 1.7941 s = the countdown
N_BARS = 34
S16 = BEAT / 4                  # hits snap to the nearest 16th in the audio (<= 56 ms)

FIGHTERS = ('FLUTE', 'TROMBONE')  # this episode's sides: left corner (screen left), right corner (screen right)
SERIES = 'BALLFIGHT #15'
T_FIGHT = 2 * BAR               # FIGHT! on the bar-2 downbeat (3.59 s): the launch
COUNT = [(BAR, '3'), (BAR + BEAT, '2'), (BAR + 2 * BEAT, '1'), (2 * BAR, 'FIGHT!')]     # bar 1: the count-in

# ---------------------------------------------------------------- the VS title card on bar 0 (projects/ballfight/title.py)
TITLE = [(0.0, 'FLUTE'), (BEAT, 'VS'), (2 * BEAT, 'TROMBONE')]   # one word slams in per beat (beat 1 = frame 0)
T_DOORS = (3 * BEAT + 0.10, BAR - 0.03)   # beat 4: the two panels slide apart like doors to reveal the ring
T_CLOSE = 0.30                  # the last 0.30 s of the file: the doors slide shut again (the run-up into the frame-0 slam)

# ---------------------------------------------------------------- frozen rules (crossover: the damage handicap only)
DMG_SCALE = 1.0                 # the shared scale stays 1.0; each fighter's damage is its own handicap below
DMG_SIDE = (2.0, 0.40)          # FLUTE, TROMBONE: tuned once (tune.py, 300 seeds: FLUTE 47 % / TROMBONE 53 % of 277
                                # decided, median K.O. 49.6 s), frozen. The lance lands ~62 hits a fight, the cobras ~36.
SWAT_COBRAS = True              # the opt-in lance-vs-cobra rule (fight.py #41): a live lance knocks a cobra flat

# ---------------------------------------------------------------- story targets (seed_search.py scores runs against these)
T_KO = (49.0, 53.0)
MIN_LEAD_CHANGES = 2
MAX_WINNER_HP = 30.0
CAP_GAP = 4.0                   # never two ability captions within 4 s
MAX_GAP = 4.0                   # no stretch over 4 s without a hit (hard window)
CAP_TARGETS = {'FLUTTER TONGUE': 12, 'SAD TROMBONE': 12, 'SNAKE DANCE': 24, 'POWER SLIDE': 24, 'PICCOLO': 40,
               'PEDAL TONE': 40}

# ---------------------------------------------------------------- the scripted tail (after the honest K.O.)
SLOWMO = 0.7                    # s of picture time the last instant of the fight is stretched over (the K.O. hit)
SLOWMO_RATE = 0.25
T_REASSEMBLE = 32 * BAR         # 57.41: the loser's pieces fly back together
T_GLIDE = 33 * BAR              # 59.21, bar 33: both glide back to their corners, HP refills, weapons reset
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


# ---------------------------------------------------------------- music (E minor, sinister big top)
KEY_ROOT = 40                   # E2: the trombone's lowest slide-position note, the bass's home
SCALE = (0, 2, 3, 5, 7, 8, 11)  # E harmonic minor (C natural, D#): the carnival / sideshow colour
# chords per bar (34 bars): the title card on the tonic, the count-in on the dominant, then an 8-bar loop from FIGHT!
# (Em Am B7 Em | C Am B7 B7, four times); its last two bars (B7 B7) fall into the title slam's Em on frame 0
LOOP = ['Em', 'Am', 'B7', 'Em', 'C', 'Am', 'B7', 'B7']
CHORDS = ['Em', 'B7'] + LOOP * 4
CHORD_ROOT = {'Em': 40, 'Am': 45, 'B7': 47, 'C': 48}       # the TROMBONE's bass roots (E2 A2 B2 C3: inside its register)
CHORD_FIFTH = {'Em': 47, 'Am': 40, 'B7': 42, 'C': 43}      # the circus bass's "other" note (fifth below / above)
CHORD_PCS = {'Em': (4, 7, 11), 'Am': (9, 0, 4), 'B7': (11, 3, 6, 9), 'C': (0, 4, 7)}
assert len(CHORDS) == N_BARS


def chord_at(t):
    b = int(max(0.0, t) // BAR) % N_BARS
    return CHORDS[b]


def chord_tones(lo, hi, chord):
    return [m for m in range(lo, hi + 1) if m % 12 in CHORD_PCS[chord]]


def bass_root(t):
    return CHORD_ROOT[chord_at(t)]


def bass_note(t):
    """the circus bass on the beat: root on beats 1 + 3, the fifth on 2 + 4 (oom . pah-less: the calliope plays the pah
    chords on the off-beats)"""
    c = chord_at(t)
    b = int(round(t / BEAT)) % 4
    return CHORD_ROOT[c] if b in (0, 2) else CHORD_FIFTH[c]


# POWER SLIDE walks 8ths: root, 3rd, 5th, chromatic approach into the next bar's root (8 per bar)
def walk_note(t):
    c = chord_at(t)
    i = int(round(t / (BEAT / 2))) % 8
    r = CHORD_ROOT[c]
    nxt = CHORD_ROOT[chord_at(t + BAR * 0.51)] if i >= 6 else r
    pcs = CHORD_PCS[c]
    third = r + ((pcs[1] - pcs[0]) % 12)
    fifth = r + 7
    table = [r, r, third, third, fifth, fifth, nxt + 2 if nxt + 2 <= 65 else nxt - 1, nxt + 1]
    m = table[i]
    return max(40, min(65, m))


# The FLUTE's melody: one note per 8th over the 8-bar loop (64 slots), E harmonic minor around E5-E6: a sinister
# sideshow line (the augmented second C-D# and the D# leading tone give the charmer colour; original, not the cliché
# tune). Every cobra strike plays the note of its 8th; FLUTTER TONGUE flutters it.
MEL_8THS = [76, 79, 83, 79, 76, 75, 76, 71,      # Em: E G B G E D# E B
            81, 84, 83, 81, 79, 76, 77, 76,      # Am: A C B A G E F E (the F: a passing phrygian sting)
            75, 78, 81, 78, 83, 81, 78, 75,      # B7: D# F# A F# B A F# D#
            76, 71, 76, 79, 83, 84, 83, 79,      # Em: E B E G B C B G
            84, 83, 79, 76, 72, 76, 79, 84,      # C: C B G E C E G C
            81, 76, 72, 76, 81, 84, 88, 84,      # Am: A E C E A C E C
            83, 87, 86, 87, 90, 87, 83, 81,      # B7: B D# D D# F# D# B A
            78, 75, 71, 75, 78, 81, 83, 75]      # B7: F# D# B D# F# A B D# (leads home)
assert len(MEL_8THS) == 64


def loop_8th(t):
    """8th index inside the 8-bar loop (bars 2.. follow it), snapped to the nearest 8th"""
    return (int(round(t / (BEAT / 2))) - 16) % 64


def mel_at(t):
    return MEL_8THS[loop_8th(t)]


# the winner's solo (beat offsets from the solo's first bar, midi, beats) over the big top: from the first bar line
# 1.6 s after the K.O. up to the reassembly (seed 392: bars 29-31).
SOLO = {
    # the cobras' charmer line, high and ornamented, falling to the E
    'FLUTE': [(0, 83, .5), (.5, 84, .25), (.75, 83, .25), (1, 81, .5), (1.5, 79, .5), (2, 76, .5), (2.5, 75, .25),
              (2.75, 76, .25), (3, 79, .5), (3.5, 83, .5), (4, 88, .75), (4.75, 87, .25), (5, 84, .5), (5.5, 83, .5),
              (6, 76, 1.5)],
    # the clown's victory, 3 bars (Em . C . Am): a big smear up, oom-pah struts, a fanfare over C, the walk down to A
    'TROMBONE': [(0, 40, .5), (.5, 47, .5), (1, 52, 1.0), (2, 55, .5), (2.5, 52, .5), (3, 47, .5), (3.5, 51, .5),
                 (4, 48, .5), (4.5, 55, .5), (5, 60, .75), (5.75, 59, .25), (6, 55, .5), (6.5, 52, .5), (7, 48, .5),
                 (7.5, 47, .5), (8, 45, .5), (8.5, 52, .5), (9, 57, .75), (9.75, 55, .25), (10, 52, .5), (10.5, 48, .5),
                 (11, 45, 1.0)],
}
