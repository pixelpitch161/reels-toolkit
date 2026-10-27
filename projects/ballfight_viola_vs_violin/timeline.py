"""#42 BALLFIGHT #16: Viola vs Violin - ONE timeline for picture and sound.

Idea: the orchestra's oldest joke. The violin gets every melody; the viola gets the rests and the punchlines. This is the
viola's revenge. VIOLA (teal, left) is new: COUNTING RESTS (fight.py weapon kind 'rests'): it rests for 3 beats (a giant
quarter-rest over it, a small "1 2 3" counting above, half damage taken), then on beat 4 a giant whole note appears and
rolls at the violin like a heavy wheel for a whole bar, crushing it once and flattening its pizzicato darts; every
landed note makes the next one bigger and heavier. DOUBLE STOP (two whole notes at once: at the violin and where it is
heading), MELODY AT LAST (for 2 bars the rests shrink to 1 beat: a whole note every other beat, and the music hands the
viola the melody), VIOLA JOKE below 25 HP (a rimshot and the whole crowd laughs; the violin freezes for a beat while
the biggest whole note drops on it). VIOLIN (violet, right) comes exactly as built in #28 (crossover rule): the bow
spun like a rapier, faster per landed hit; PIZZICATO, TREMOLO, SCREECH. The only per-episode lever is the damage
handicap per fighter (Rules(dmg_side), tuned once with tune.py, frozen below).
Honest simulated fight, seed picked for pacing, never edited.
Hook: bar 0 is the VS title card (projects/ballfight/title.py): teal / violet panels split diagonally, both balls big and
close; VIOLA (a dark viola chord), VS (the thunder crack) and VIOLIN (a violin flourish) slam in on beats 1-3 over
unison band hits; the doors open on beat 4. Bar 1 is the 3 . 2 . 1 count-in, FIGHT! on the bar-2 downbeat (3.39 s).
The title slam is the frame-0 reset: the doors slide shut over the last T_CLOSE s into it.
HUD: each fighter's ability list under its HP bar (grey until unlocked, lit on every use; series rule Oct 25).
Music, mood A: orchestral drill. loop_bpm(140) = 141.64 BPM (36 bars = 61 s), C minor: a UK drill beat (sliding 808s,
skittering hi-hat rolls, a half-time snare on beat 3) under a dark staccato string bed (cellos + basses only: no violin
or viola in the bed). The VIOLIN plays the melody (its bow turns sample the line, as in #28); the VIOLA plays the inner
harmony a fifth lower (each whole note = one long sustained viola note over the beat; the counting rests are a soft
muted pluck on 1-2-3) until MELODY AT LAST, when the arrangement flips and the viola leads for its 2 bars. VIOLA JOKE =
a rimshot (ba-dum-tss) + the crowd laughing, then the big note. Consistent loudness from frame 0 (project rule,
Oct 26): the drill beat is full on the title card; it builds with density, not level. Leader fuller and louder, loser
thinner and dirtier; the winner's solo over the drill beat.

Beat sheet (61 s; events after 3.39 s are the run's own: the seed was chosen so they land near these targets):
    0-1.69    the VS title card: VIOLA, VS, VIOLIN slam in; the doors open on beat 4
    1.69-3.39 balls rev in corners; giant 3 . 2 . 1 in the ring, FIGHT! at 3.39
    3.39-8    launch; the viola counts its first rests, the first whole note rolls; the bow chips
    8-30      notes grow, the bow speeds up; the four unlocks                DOUBLE STOP, PIZZICATO, MELODY AT LAST, TREMOLO
    32-44     one dips below 25 HP                                            VIOLA JOKE or SCREECH
    44-52     final exchange, HP in single digits                             -
    ~49-53    K.O.: hit-stop, slow-mo shatter (scroll / strings / bridge)     K.O. -> (WINNER) WINS
    then-58   winner's solo under the spot                                    COMMENT THE / NEXT MATCH-UP
    58-61     loser reassembles; both glide to their corners; the doors shut and the title slams at 61.0 = frame 0
"""
import math

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(140, DUR)        # 141.64: 36 bars in 61 s, so the groove is in phase at the loop point
BEAT = 60.0 / BPM               # 0.4236 s: one counted rest
BAR = 4 * BEAT                  # 1.6944 s = the countdown, one whole note
N_BARS = 36
S16 = BEAT / 4                  # hits snap to the nearest 16th in the audio (<= 53 ms)

FIGHTERS = ('VIOLA', 'VIOLIN')  # this episode's sides: left corner (screen left), right corner (screen right)
SERIES = 'BALLFIGHT #16'
T_FIGHT = 2 * BAR               # FIGHT! on the bar-2 downbeat (3.39 s): the launch
COUNT = [(BAR, '3'), (BAR + BEAT, '2'), (BAR + 2 * BEAT, '1'), (2 * BAR, 'FIGHT!')]     # bar 1: the count-in

# ---------------------------------------------------------------- the VS title card on bar 0 (projects/ballfight/title.py)
TITLE = [(0.0, 'VIOLA'), (BEAT, 'VS'), (2 * BEAT, 'VIOLIN')]   # one word slams in per beat (beat 1 = frame 0)
T_DOORS = (3 * BEAT + 0.10, BAR - 0.03)   # beat 4: the two panels slide apart like doors to reveal the ring
T_CLOSE = 0.30                  # the last 0.30 s of the file: the doors slide shut again (the run-up into the frame-0 slam)

# ---------------------------------------------------------------- frozen rules (crossover side: the damage handicap only)
DMG_SCALE = 1.0                 # the shared scale stays 1.0; each fighter's damage is its own handicap below
DMG_SIDE = (0.75, 2.1)          # VIOLA, VIOLIN: tuned once (tune.py, 150 seeds: VIOLA 49 % / VIOLIN 51 % of 148 decided,
                                # median K.O. 49.5 s), frozen. The rests halve the bow most of the time: hence x2.1.

# ---------------------------------------------------------------- story targets (seed_search.py scores runs against these)
T_KO = (49.0, 53.0)
MIN_LEAD_CHANGES = 2
MAX_WINNER_HP = 30.0
CAP_GAP = 4.0                   # never two ability captions within 4 s
MAX_GAP = 4.0                   # no stretch over 4 s without a hit (hard window): the rests must not stall the fight
CAP_TARGETS = {'DOUBLE STOP': 12, 'PIZZICATO': 12, 'MELODY AT LAST': 24, 'TREMOLO': 24, 'VIOLA JOKE': 40,
               'SCREECH': 40}

# ---------------------------------------------------------------- the scripted tail (after the honest K.O.)
SLOWMO = 0.7                    # s of picture time the last instant of the fight is stretched over (the K.O. hit)
SLOWMO_RATE = 0.25
T_REASSEMBLE = 34 * BAR         # 57.61: the loser's pieces fly back together
T_GLIDE = 35 * BAR              # 59.31, bar 35: both glide back to their corners, HP refills, weapons reset
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


# ---------------------------------------------------------------- music (C minor, orchestral drill)
KEY_ROOT = 36                   # C2: the 808's home
SCALE = (0, 2, 3, 5, 7, 8, 11)  # C harmonic minor (B natural on G)
# chords per bar (36): the title card on the tonic, the count-in on the dominant, then an 8-bar loop from FIGHT!
# (Cm Cm Ab Ab Fm Fm G G: drill's slow two-bar changes) four times; bars 34-35 (Ab G) fall into the title slam's Cm on
# frame 0
LOOP = ['Cm', 'Cm', 'Ab', 'Ab', 'Fm', 'Fm', 'G', 'G']
CHORDS = ['Cm', 'G'] + LOOP * 4 + ['Ab', 'G']
CHORD_ROOT = {'Cm': 36, 'Ab': 32, 'Fm': 29, 'G': 31}       # the 808's roots (C2 Ab1 F1 G1)
CHORD_PCS = {'Cm': (0, 3, 7), 'Ab': (8, 0, 3), 'Fm': (5, 8, 0), 'G': (7, 11, 2)}
assert len(CHORDS) == N_BARS


def chord_at(t):
    b = int(max(0.0, t) // BAR) % N_BARS
    return CHORDS[b]


def chord_tones(lo, hi, chord):
    return [m for m in range(lo, hi + 1) if m % 12 in CHORD_PCS[chord]]


# The VIOLIN's line: an original drill-string melody (staccato, minor, a phrygian Db sting), one pitch per 16th over a
# 4-bar phrase that follows the 2-bar chords (Cm . Ab or Fm . G). The bow samples it, as in #28: every quarter turn plays
# the line's note at that moment (snapped to the 16th), so a slow bow plays a few strokes of the tune and a fast bow
# (TREMOLO) the whole thing as frantic spiccato.
_M = {  # (bar in phrase, 16th) -> midi; slots in between hold the last note
    (0, 0): 72, (0, 3): 75, (0, 6): 74, (0, 8): 72, (0, 10): 71, (0, 12): 72, (0, 14): 67,
    (1, 0): 75, (1, 3): 79, (1, 6): 77, (1, 8): 75, (1, 10): 74, (1, 12): 73, (1, 14): 72,
    (2, 0): 68, (2, 3): 72, (2, 6): 75, (2, 8): 80, (2, 10): 79, (2, 12): 77, (2, 14): 75,
    (3, 0): 74, (3, 3): 71, (3, 6): 74, (3, 8): 77, (3, 10): 79, (3, 12): 83, (3, 14): 84,
}
MELODY = []
_last = 72
for _b in range(4):
    for _s in range(16):
        _last = _M.get((_b, _s), _last)
        MELODY.append(_last)


def melody_at(t):
    """the violin line's pitch at time t (the phrase restarts every 4 bars from FIGHT!), fitted to the bar's chord: in
    the Ab / Fm half of the loop the phrase is a third lower where it would clash"""
    k = int(round(t / S16))
    b = (k // 16 - 2) % 4
    m = MELODY[b * 16 + k % 16]
    c = chord_at(k * S16)
    if c in ('Ab', 'Fm') and (m % 12) in (2, 7, 11):          # D / G / B against Ab / Fm: step to a chord tone
        m -= 1 if (m % 12) != 7 else 2
    return m


def viola_inner(t):
    """the VIOLA's inner voice: a chord tone a fifth (or so) under the violin's line, in the viola's warm range (C3-G4)"""
    c = chord_at(t)
    tgt = melody_at(t) - 7
    cands = chord_tones(48, 67, c)
    return min(cands, key=lambda m: (abs(m - tgt), m))


# MELODY AT LAST: the music hands the viola the melody for its 2 bars (8 beats from the beat after it fires): a long,
# singing line (beats, midi, beats), fitted to the chord at each note; the violin drops to soft harmony meanwhile
VIOLA_LEAD = [(0, 67, 1.0), (1, 70, 0.5), (1.5, 68, 0.5), (2, 67, 1.5), (3.5, 65, 0.5), (4, 63, 1.0), (5, 65, 0.75),
              (5.75, 67, 0.25), (6, 62, 1.0), (7, 60, 1.0)]


def fit(m, chord, lo=48, hi=79):
    """nudge a melody note onto the chord's scale (C harmonic minor; over Ab / Fm the B becomes Bb)"""
    pcs = [(KEY_ROOT + s) % 12 for s in SCALE]
    if chord in ('Ab', 'Fm', 'Cm'):
        pcs = [p if p != 11 else 10 for p in pcs]
    while m % 12 not in pcs:
        m -= 1
    return max(lo, min(hi, m))


# the winner's solo (beat offsets from the solo's first bar, midi, beats) over the drill beat: from the first bar line
# 1.6 s after the K.O. up to the reassembly (seed 8219: bars 31-33)
SOLO = {
    # the viola, at last: a dark, singing line down to its open C string
    'VIOLA': [(0, 67, 1.0), (1, 68, 0.5), (1.5, 67, 0.5), (2, 72, 1.5), (3.5, 70, 0.5), (4, 68, 1.0), (5, 67, 0.5),
              (5.5, 65, 0.5), (6, 63, 1.0), (7, 62, 0.5), (7.5, 59, 0.5), (8, 60, 1.0), (9, 55, 1.0), (10, 48, 2.0)],
    # the violin: a spiccato flurry up the harmonic minor, a cry, sighs down, the low G
    'VIOLIN': [(0, 72, .25), (.25, 74, .25), (.5, 75, .25), (.75, 77, .25), (1, 79, .5), (1.5, 80, .25), (1.75, 83, .25),
               (2, 84, .75), (2.75, 83, .25), (3, 80, .5), (3.5, 79, .5), (4, 77, .25), (4.25, 79, .25), (4.5, 77, .25),
               (4.75, 75, .25), (5, 74, .5), (5.5, 71, .5), (6, 72, .25), (6.25, 74, .25), (6.5, 72, .25), (6.75, 74, .25),
               (7, 75, .5), (7.5, 74, .25), (7.75, 71, .25), (8, 68, .5), (8.5, 67, .5), (9, 71, .25), (9.25, 74, .25),
               (9.5, 77, .25), (9.75, 80, .25), (10, 83, .5), (10.5, 79, .5), (11, 67, 1.0)],
}
