"""#39 BALLFIGHT #13: Bassoon vs Bari Sax - ONE timeline for picture and sound.

Idea: the low reeds. The orchestra's clown vs the band's chaos agent: both hold down the bottom, sound hilarious and
get no respect. BASSOON (left corner, forest green) is the only sneak, where position matters: TIPTOE - it doesn't
glide, it tiptoes: exaggerated staccato hops on the beat (stop-and-go, squash and stretch), each to a spot BEHIND the
enemy (against its direction of travel), each landing a small stomp that does double damage from behind (the
backstab). BARI SAX (right corner, coral) v2 (Oct 26: "the bari sax is really boring"): so heavy it wears a harness:
WRECKING BALL - every other bar a crane cable drops from the spot's rig, hooks the harness, hoists it back and swings it
across the ring as a wrecking ball in a pendulum arc, smashing what's in front of it (harder the faster it's going),
then unhooks and rolls on; the cable grows per landed smash (a wider swing that hits harder). Both are game rules in
the engine (projects/ballfight/fight.py: 'tiptoe', 'hook', 'mountain', 'wrecking', 'offbeat', 'snap'); the audio
follows them. v1 (WINDOW RATTLER, LOW A) is frozen in v1/. v3 (Oct 27): the detailed bari model and the swing out
over the ropes (the roster's, from #40 v2); v2 (the swing kept inside the ring) is frozen in v2/. v4 (Oct 27): a full
swing near the corners too (the roster's 'steer'); v3 is frozen in v3/. Honest simulated
fight (the rules tuned once with tune.py, then frozen), seed picked for pacing, never edited.
Hook: bar 0 is the VS title card (projects/ballfight/title.py): forest-green / coral panels split diagonally, both
balls big and close; BASSOON (a bassoon blat), VS (the thunder crack) and BARI SAX (a bari honk) slam in on beats 1-3
over unison band hits; the doors open on beat 4. Bar 1 is the 3 . 2 . 1 count-in, FIGHT! on the bar-2 downbeat
(2.84 s). The title slam is the frame-0 reset: the doors slide shut over the last T_CLOSE s into it.
HUD: each fighter's ability list under its HP bar (grey until unlocked, lit on every use; series rule Oct 25).
Music, mood A: dark ska-punk. Fast minor ska-punk at loop_bpm(170) = 169.18 BPM (43 bars = 61 s, the fastest in the
series), E minor: driving punk drums, upstroke guitar skanks on every off-beat, an organ bubble; no horns of its own.
The BASSOON plays sneaky staccato lines up high (one dot per hop landing, an accent on a backstab); the BARI SAX is
the honking bass, riding the swings: every swing one huge honk on the bar's bass root, swelling to the bottom of the
arc and bent like a doppler pass; SKANK's swings honk up the octave, their bottoms on the off-beats; GROWL growls. MOUNTAIN KING: Grieg's In the Hall of the Mountain King theme (1875, public domain) arranged in E minor,
one note per hop, accelerating with the hops; nothing else quoted. Consistent loudness from frame 0 (project rule,
Oct 26): the band is at full level on the title card; it builds with density, not level. Leader fuller and louder,
loser thinner and dirtier; at 169 BPM every hit note stays short so the mix doesn't smear; the winner's solo over the
skank.

Beat sheet (61 s; events after 2.84 s are the run's own: the seed was chosen so they land near these targets):
    0-1.42    the VS title card: BASSOON, VS, BARI SAX slam in; the doors open on beat 4
    1.42-2.84 balls rev in corners; giant 3 . 2 . 1 in the ring, FIGHT! at 2.84
    2.84-8    launch; the first tiptoes vs the first swings                    -
    8-30      hop reach / cable length grow; the four unlocks                  STACCATO, GROWL, BOCAL HOOK, SKANK
    32-44     one dips below 25 HP                                             MOUNTAIN KING or CABLE SNAP
    44-52     final exchange, HP in single digits                              -
    ~49-53    K.O.: hit-stop, slow-mo shatter (maple joints / reed / keys / bell)   K.O. -> (WINNER) WINS
    then-57.45 winner's solo under the spot                                     COMMENT THE / NEXT MATCH-UP
    57.45-61  loser reassembles; both glide to their corners; the doors shut and the title slams at 61.0 = frame 0
"""
import math

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(170, DUR)        # 169.18: 43 bars in 61 s, so the skank is in phase at the loop point
BEAT = 60.0 / BPM               # 0.3547 s: one hop
BAR = 4 * BEAT                  # 1.4186 s = the countdown = one honk
N_BARS = 43
S16 = BEAT / 4                  # hits snap to the nearest 16th in the audio (<= 44 ms)

FIGHTERS = ('BASSOON', 'BARI SAX')  # left corner (screen left), right corner (screen right)
SERIES = 'BALLFIGHT #13'
T_FIGHT = 2 * BAR               # FIGHT! on the bar-2 downbeat (2.84 s): the launch
COUNT = [(BAR, '3'), (BAR + BEAT, '2'), (BAR + 2 * BEAT, '1'), (2 * BAR, 'FIGHT!')]     # bar 1: the count-in

# ---------------------------------------------------------------- the VS title card on bar 0 (projects/ballfight/title.py)
TITLE = [(0.0, 'BASSOON'), (BEAT, 'VS'), (2 * BEAT, 'BARI SAX')]   # one word slams in per beat (beat 1 = frame 0)
T_DOORS = (3 * BEAT + 0.08, BAR - 0.03)   # beat 4: the two panels slide apart like doors to reveal the ring
T_CLOSE = 0.30                  # the last 0.30 s of the file: the doors slide shut again (the run-up into the frame-0 slam)

# ---------------------------------------------------------------- frozen rules (tuned once with tune.py)
DMG_SCALE = 0.34              # v3 (Oct 27: the bari's swing may go out over the ropes): re-tuned once with tune.py, frozen;
DMG_SIDE = (1.0, 0.8)         # the BARI SAX's hits x0.8 (the roster's numbers are shared with #40, so the handicap is the
                              # episode's own): 400 seeds BASSOON 53 % / BARI SAX 47 %, median K.O. 50.8 s (without it the
                              # full swings won the bari 83 %). v2 was 0.32 with no handicap (v2/), v1 0.45 (v1/).
                              # v4 (the roster's 'steer': full swings near the corners too) kept these: 400 seeds
                              # BASSOON 56 % / BARI SAX 44 %, median K.O. 50.9 s

# ---------------------------------------------------------------- story targets (seed_search.py scores runs against these)
T_KO = (49.0, 53.0)
MIN_LEAD_CHANGES = 2
MAX_WINNER_HP = 30.0
CAP_GAP = 4.0                   # never two ability captions within 4 s
MAX_GAP = 4.0                   # no stretch over 4 s without a hit (hard window)
CAP_TARGETS = {'STACCATO': 12, 'GROWL': 12, 'BOCAL HOOK': 24, 'SKANK': 24, 'MOUNTAIN KING': 40, 'CABLE SNAP': 40}

# ---------------------------------------------------------------- the scripted tail (after the honest K.O.)
SLOWMO = 0.7                    # s of picture time the last instant of the fight is stretched over (the K.O. hit)
SLOWMO_RATE = 0.25
T_REASSEMBLE = 40.5 * BAR       # 57.45: the loser's pieces fly back together (the K.O. at 51.16 + 3.65 s leaves the
                                # COMMENT THE NEXT MATCH-UP caption 2.6 s)
T_GLIDE = 41 * BAR + 2 * BEAT   # 58.87: both glide back to their corners, HP refills, weapons reset
T_REV = T_GLIDE + 1.05          # in the corners, revving (same motion as the opening)


# ---------------------------------------------------------------- music (E minor, dark ska-punk)
KEY_ROOT = 40                   # E2
SCALE = (0, 2, 3, 5, 7, 8, 10)  # E natural minor (D# over the B chord: the leading tone)
# chords per bar (43 bars): the title card on the tonic, the count-in on the dominant, then the 4-bar ska-punk loop
# from FIGHT! (Em C D B, ten times); the last bar's B falls into the title slam's Em
CHORDS = ['Em', 'B'] + ['Em', 'C', 'D', 'B'] * 10 + ['B']
CHORD_ROOT = {'Em': 40, 'C': 36, 'D': 38, 'B': 35}        # bass roots (E2 C2 D2 B1)
CHORD_PCS = {'Em': (4, 7, 11), 'C': (0, 4, 7), 'D': (2, 6, 9), 'B': (11, 3, 6)}
assert len(CHORDS) == N_BARS


def chord_at(t):
    b = int(max(0.0, t) // BAR) % N_BARS
    return CHORDS[b]


def scale_notes(lo, hi, chord=None):
    """E minor between midi lo..hi (over B the D becomes D#)"""
    pcs = [(KEY_ROOT + s) % 12 for s in SCALE]
    if chord == 'B':
        pcs = [p if p != 2 else 3 for p in pcs]
    return [m for m in range(lo, hi + 1) if m % 12 in pcs]


def chord_tones(lo, hi, chord):
    return [m for m in range(lo, hi + 1) if m % 12 in CHORD_PCS[chord]]


# The bassoon's sneaky staccato line, up high (E3-E5): one pitch per 8th over the 4-bar loop (32 slots); every hop
# landing plays the slot it lands on (STACCATO's 16th landings take the passing note between), a backstab is an accent
# up the octave's chord tone.
BASSOON_8THS = [64, 67, 71, 67, 64, 66, 67, 69,      # Em: tiptoeing up and back
                64, 67, 72, 67, 64, 67, 71, 67,      # C
                66, 69, 74, 69, 66, 69, 72, 69,      # D
                66, 71, 75, 71, 66, 69, 71, 63]      # B: D# leads home
# The ropes' bass strings: per chord, the four sides' pitches (root, fifth, octave, the fifth above): more sides
# rattling = a fuller bass chord (the bari's own honk plays the root)
ROPE_NOTES = {'Em': (40, 47, 52, 59), 'C': (36, 43, 48, 55), 'D': (38, 45, 50, 57), 'B': (35, 42, 47, 54)}
HONK_NOTE = {'Em': 40, 'C': 36, 'D': 38, 'B': 42}       # the bari's honk: the bass root (B's on its fifth: below low C)

# MOUNTAIN KING: Grieg, In the Hall of the Mountain King (1875, public domain), the theme's opening (B minor in the
# original, where the bassoons introduce it) arranged in E minor up in the bassoon's tenor register: one note per hop,
# so the theme accelerates with the hops (20 hops, 20 notes)
MK_THEME = [64, 66, 67, 69, 71, 67, 71,     # E F# G A B G B
            70, 66, 70,                     # A# F# A#
            69, 65, 69,                     # A F A
            64, 66, 67, 69, 71, 67, 71]     # E F# G A B G B


def loop_8th(t):
    """8th index inside the 4-bar loop (bars 2.. follow it), snapped to the nearest 8th"""
    k = int(round(t / (BEAT / 2)))
    return (k - 16) % 32


def bassoon_at(t):
    return BASSOON_8THS[loop_8th(t)]


def bass_root(t):
    return CHORD_ROOT[chord_at(t)]


# the winner's solo (beat offsets from the solo's first bar, midi, beats): 2 bars over the skank, ending on the tonic
SOLO = {
    # a sneaky staccato run that tiptoes up, peeks, and lands home with a comic low blat
    'BASSOON': [(0, 64, .25), (.5, 67, .25), (1, 71, .25), (1.5, 69, .25), (2, 67, .25), (2.5, 66, .25), (3, 64, .25),
                (3.5, 71, .25), (4, 76, .25), (4.5, 74, .25), (5, 71, .25), (5.5, 67, .25), (6, 64, .5), (7, 40, .75)],
    # big honks: root, the octave scoop, a growled riff, home
    'BARI SAX': [(0, 40, .5), (1, 52, .5), (1.5, 50, .25), (2, 47, .5), (3, 45, .25), (3.5, 43, .5), (4, 40, .5),
                 (5, 47, .25), (5.5, 45, .25), (6, 40, 1.5)],
}


def bar_t(b):
    return b * BAR


def beat_env(t, decay=0.12):
    return math.exp(-((t % BEAT) / decay))


def beat_pulse(t, decay=0.12, pre=0.10):
    """a pulse on every beat that ramps up over the last `pre` s before it (no one-frame snap), so the beat that lands on
    frame 0 doesn't jump at the loop seam"""
    ph = t % BEAT
    up = min(1.0, max(0.0, (ph - (BEAT - pre)) / pre))
    return max(math.exp(-ph / decay), up * up * (3 - 2 * up))


def next_bar(t):
    return math.ceil(t / BAR - 1e-6) * BAR
