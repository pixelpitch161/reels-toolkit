"""#31 BALLFIGHT #5: Turntables vs Synth - ONE timeline for picture and sound.

Idea: DJ vs producer, electronic civil war, on #27's engine, ring, HUD and count-in. TURNTABLES (lime, left corner)
throws vinyl frisbees that ricochet off the ropes (a volley with `bounce`: up to 2 rope bounces; +1 disc every second
landed volley), and every throw is a scratch. SYNTH (cyan, right corner) is an arpeggiator: glowing note satellites
orbit its ball and hit what they touch (new weapon kind `orbit`; +1 satellite per landed hit, max 8), and every
satellite passing the top of its orbit plays the next arp note, so more satellites = a denser arpeggio. Abilities:
SCRATCH (discs on the 8ths for a bar) / REWIND (new kind: undoes its last 2 s of damage taken, zipping back along its
own path) / BASS DROP (slam); LFO (the orbit speeds up and wobbles) / SAWTOOTH (an arc of saw-tooth notes) / WHITE
NOISE (wave). Honest simulated fight (projects/ballfight/fight.py, rules tuned once for this pair with tune.py, then
frozen), the seed picked for pacing, never edited. REWIND is a rule in the simulation (the ball's own record), not an
edit.
Hook: the giant 3 . 2 . 1 . FIGHT! in the ring = the band's count-in (a scratch + a synth stab per count, the drop
lands on FIGHT); top caption "TURNTABLES / VS SYNTH" with the BALLFIGHT #5 tag.
Music, mood: dark breakbeat. F minor, chopped breaks, a reese bass, cold pads, at loop_bpm(138) = 137.70 BPM (35 bars
= 61 s; 1 bar = the countdown). The two fighters are the leads: the decks scratch and stab, the synth arpeggiates.
Whoever is losing, their half of the track falls apart.

Beat sheet (61 s; events after 2 s are the run's own: the seed was chosen so they land near these targets):
    0-1.74   balls rev in corners; giant 3 . 2 . 1 in the ring          TURNTABLES / VS SYNTH, then FIGHT!
    1.74-8   launch; discs ricochet vs the orbit, HP ticking              -
    8-30     discs multiply, satellites fill the orbit; unlocks           SCRATCH, LFO, REWIND, SAWTOOTH
    32-44    one dips below 25 HP                                         BASS DROP or WHITE NOISE
    44-52    final exchange, HP in single digits                          -
    ~49-53   K.O.: hit-stop, slow-mo shatter (vinyl / keys)               K.O. -> (WINNER) WINS
    then-58  winner's solo under the spot                                 COMMENT THE / NEXT MATCH-UP
    58-61    loser reassembles; both glide to their corners; HP refills; "3" lands at 61.0 = frame 0
"""
import math

FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(138, DUR)        # 137.70: 35 bars in 61 s, so the groove is in phase at the loop point
BEAT = 60.0 / BPM               # 0.4357 s
BAR = 4 * BEAT                  # 1.7429 s = the countdown
N_BARS = 35
S16 = BEAT / 4                  # hits snap to the nearest 16th in the audio (<= 54 ms)

FIGHTERS = ('TURNTABLES', 'SYNTH')   # left corner (screen left), right corner (screen right)
SERIES = 'BALLFIGHT #5'
T_FIGHT = BAR                   # FIGHT! lands on the bar-1 downbeat: the drop, the launch and the first disc
COUNT = [(0.0, '3'), (BEAT, '2'), (2 * BEAT, '1'), (BAR, 'FIGHT!')]

# ---------------------------------------------------------------- frozen rules (tune.py; the same for every seed)
DMG_SCALE = 0.6                 # v2: tuned once (tune.py) so the median K.O. sits near 50 s with each side winning about half

# ---------------------------------------------------------------- story targets (seed_search.py scores runs against these)
T_KO = (49.0, 53.0)
MIN_LEAD_CHANGES = 2
MAX_WINNER_HP = 30.0
CAP_GAP = 4.0                   # never two ability captions within 4 s
CAP_TARGETS = {'SCRATCH': 10, 'LFO': 15, 'REWIND': 24, 'SAWTOOTH': 28, 'BASS DROP': 40, 'WHITE NOISE': 40}

# ---------------------------------------------------------------- the scripted tail (after the honest K.O.)
SLOWMO = 0.7                    # s of picture time the last instant of the fight is stretched over (the K.O. hit)
SLOWMO_RATE = 0.25
T_REASSEMBLE = 33 * BAR + BEAT  # 57.95: the loser's pieces fly back together
T_GLIDE = 34 * BAR              # 59.26, bar 34: both glide back to their corners, HP refills, weapons reset
T_REV = T_GLIDE + 1.05          # in the corners, revving (same motion as the opening)


# ---------------------------------------------------------------- music (F minor, dark breakbeat)
KEY_ROOT = 41                   # F
SCALE = (0, 2, 3, 5, 7, 8, 10)  # F natural minor: F G Ab Bb C Db Eb (E natural over C)
# chords per bar (35 bars): the count-in on the dominant, then the 4-bar loop i - VI - iv - V (Fm . Db . Bbm . C),
# ending on Fm . C so the loop seam is the dominant (two bars of C) resolving on FIGHT!
CHORDS = ['C'] + ['Fm', 'Db', 'Bbm', 'C'] * 8 + ['Fm', 'C']
CHORD_ROOT = {'Fm': 41, 'Db': 37, 'Bbm': 46, 'C': 36}
CHORD_PCS = {'Fm': (5, 8, 0), 'Db': (1, 5, 8), 'Bbm': (10, 1, 5), 'C': (0, 4, 7, 10)}
assert len(CHORDS) == N_BARS

# The arpeggiator: satellite j plays ARP[chord][j] when it passes the top of the orbit, so 2 satellites play a two-note
# figure and 8 play the whole climbing arpeggio (chord tones over two octaves, a 9th / 11th colour on top)
ARP = {
    'Fm': (65, 72, 68, 77, 75, 80, 79, 84),
    'Db': (65, 68, 73, 72, 77, 80, 75, 85),
    'Bbm': (65, 70, 73, 77, 72, 82, 80, 85),
    'C': (64, 67, 72, 70, 76, 79, 74, 82),
}


def chord_at(t):
    b = int(max(0.0, t) // BAR) % N_BARS
    return CHORDS[b]


def scale_notes(lo, hi, chord=None):
    """F minor between midi lo..hi (over C the Eb is raised to E: the dominant's leading tone)"""
    pcs = [(KEY_ROOT + s) % 12 for s in SCALE]
    if chord == 'C':
        pcs = [p if p != 3 else 4 for p in pcs]
    return [m for m in range(lo, hi + 1) if m % 12 in pcs]


# the winner's solo (beat offsets from the solo's first bar)
SOLO = {
    # a synth lead: a saw line that climbs out of the arp, a long bend, a fall back to the root
    'SYNTH': [(0, 72, .5), (.5, 75, .25), (.75, 77, .25), (1, 80, .5), (1.5, 79, .5), (2, 77, .25), (2.25, 75, .25),
              (2.5, 77, 1.0), (3.5, 72, .5), (4, 73, .5), (4.5, 77, .25), (4.75, 80, .25), (5, 85, 1.0), (6, 84, .5),
              (6.5, 80, .5), (7, 82, .25), (7.25, 80, .25), (7.5, 79, .5), (8, 76, .5), (8.5, 79, .25), (8.75, 82, .25),
              (9, 84, .75), (9.75, 82, .25), (10, 79, .5), (10.5, 76, .5), (11, 77, 1.0)],
    # a scratch routine: (beat, kind, length in beats) with kind a scratch ('baby', 'chirp', 'trans', 'flare', 'tear',
    # 'scrib', 'back') or 'stab' (an organ stab on the chord); 'cut' = a crossfader cut of the break under it
    'TURNTABLES': [(0, 'stab', .5), (0, 'back', 1.0), (1, 'baby', .5), (1.5, 'baby', .5), (2, 'chirp', .25),
                   (2.25, 'chirp', .25), (2.5, 'trans', .5), (3, 'stab', .25), (3.25, 'stab', .25), (3.5, 'flare', .5),
                   (4, 'stab', .5), (4.5, 'tear', .5), (5, 'scrib', .5), (5.5, 'chirp', .25), (5.75, 'chirp', .25),
                   (6, 'trans', 1.0), (7, 'stab', .25), (7.5, 'stab', .25), (8, 'stab', .5), (8, 'flare', .5),
                   (8.5, 'baby', .25), (8.75, 'baby', .25), (9, 'tear', .5), (9.5, 'chirp', .25), (9.75, 'chirp', .25),
                   (10, 'scrib', .5), (10.5, 'trans', .5), (11, 'stab', 1.0), (11, 'back', 1.0)],
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
