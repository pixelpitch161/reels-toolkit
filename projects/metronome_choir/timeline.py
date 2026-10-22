"""The Metronome Choir (IDEAS #16, "100 Metronomes, part 2") - the single source of times.

Picture (render.py) and sound (audio.py) both import this file, sim.py and motion.py.

Arc (beat sheet in docs/IDEAS.md #16; 120 BPM grid, 1 beat = 0.5 s = one metronome tick):
    0-3     cold open: 199 metronomes on 5 fixed risers + the golden soloist, all ticking at random
    3-6     sweep the 5 colour-coded sections                       "5 SECTIONS / 5 VOICES"
    6-13    bass riser onto 2 soda cans (clunk), locks row by row   its two-note line rises out of the wash
    13-17   tenor riser onto cans, locks
    17-21   alto + soprano together                                 "4 OF 5"
    21-25   high bells lock; 5 sections in sync, out of step         flammed voices
    25-30   all 5 risers hoisted onto one platform on giant cans    "NOW: ONE BIG BOARD"
    30-34   sections pull together, counter climbs, kick enters on the pulse
    34-38   199 locked, spotlight on the coral soloist (melody pickup drags against the beat)
    38      the soloist locks = the drop (LED tip show, original song in D major)
    52-58   wide shot, final chorus, "TRY IT"
    58-60   giant cans kicked out, platform thumps, everyone jolted back to random -> loop
"""
import math

FPS = 30
DUR = 60.0
NF = int(FPS * DUR)

BPM = 120
BEAT = 60.0 / BPM            # one tick of a locked metronome (it swings at 1 Hz, ticks at both ends)
BAR = 4 * BEAT


def grid(b):
    return b * BEAT


# ---------------------------------------------------------------- the choir
NSEC = 5
ROWS, COLS = 4, 10
SEC_NAMES = ['BASS', 'TENOR', 'ALTO', 'SOPRANO', 'HIGH BELLS']
SEC_SHORT = ['BASS', 'TENR', 'ALTO', 'SOPR', 'HIGH']
SEC_COL = [(92, 110, 232), (162, 102, 226), (236, 96, 160), (248, 158, 68), (84, 200, 240)]
GOLD = (246, 198, 84)
EMPTY = (0, 0, 4)            # (section, row, col): the soloist's own seat (bass riser, front row) stays empty

# bells: (left-swing note, right-swing note) per section, D major.
# left ticks stack into D major (D3 F#3 A3 D4 F#4), right ticks into Asus2 (A2 E3 B3 E4 A4)
SEC_NOTES = [(50, 45), (54, 52), (57, 59), (62, 64), (66, 69)]

# ---------------------------------------------------------------- events
# board k: (lift starts, settles on its cans = the board's coupling switches on)
LIFT = [(6.0, 7.5), (12.8, 14.0), (16.8, 18.0), (16.8, 18.0), (20.4, 21.6)]
PLAT = (25.0, 27.6)          # whole loft hoisted, settles on the giant cans (global coupling on)
T_DROP = 38.0                # the soloist locks in = the drop (beat 76, a bar line)
T_KICK = 57.6                # the giant cans get kicked out
T_JOLT = 58.0                # platform thump: every metronome thrown back to random
SONG_BARS = int(round((T_JOLT - T_DROP) / BAR))       # 10 bars

# ---------------------------------------------------------------- music (original, D major, 120 BPM)
# song chords per bar from the drop: I vi IV V  I vi IV V  IV V  (the thump lands where the I would be)
SONG_CHORDS = ['D', 'Bm', 'G', 'A', 'D', 'Bm', 'G', 'A', 'G', 'A']
CHORD_NOTES = {'D': (50, [62, 66, 69]), 'Bm': (47, [59, 62, 66]), 'G': (43, [59, 62, 67]), 'A': (45, [57, 59, 64])}     # A chord kept sus2 (no C#): the bells' left ticks ring D
# the soloist's bell carousel: one slot per tick, None = a rest (just the tick). Slot 0 rings on the drop.
MELODY = [74, 78, 81, 83,   81, 78, 74, 78,   79, 78, 76, 74,   76, None, 81, 76,
          74, 78, 81, 83,   86, 83, 81, 78,   79, 81, 83, 86,   83, None, 81, None,
          83, 81, 79, 83,   81, 79, 78, 76]
assert len(MELODY) == 4 * SONG_BARS


def song_bar(t):
    """bar index of the song (0..9) or None"""
    if t < T_DROP or t >= T_JOLT:
        return None
    return int((t - T_DROP) // BAR)


def beat_env(t, decay=0.09):
    """0..1 pulse on every beat of the song"""
    if t < T_DROP - 0.01 or t >= T_JOLT:
        return 0.0
    u = (t - T_DROP) % BEAT
    return math.exp(-u / decay)


# ---------------------------------------------------------------- captions (top, max 18 chars per line)
CAPTIONS = [
    (-0.5, 3.0, ["200 METRONOMES", "*= 1 SONG*"]),
    (3.05, 6.0, ["5 SECTIONS", "*5 VOICES*"]),
    (6.05, 9.55, ["BASS GOES ON", "*2 SODA CANS*"]),           # locks (sim.py): bass 9.6, tenor 15.9,
    (9.6, 12.95, ["BASS:", "~LOCKED~"]),                         #   alto + soprano 20.0, high bells 23.5
    (13.0, 15.9, ["TENOR,", "*YOUR TURN*"]),
    (15.95, 17.6, ["TENOR:", "~LOCKED~"]),
    (17.65, 19.95, ["ALTO + SOPRANO", "*TOGETHER*"]),
    (20.0, 21.9, ["*4 OF 5*", "~LOCKED~"]),
    (21.95, 25.0, ["EACH IN SYNC...", "*NOT* WITH EACH", "OTHER"]),
    (25.05, 30.0, ["NOW:", "*ONE BIG BOARD*"]),
    (30.05, 34.0, ["NOBODY IS", "*IN CHARGE*"]),
    (34.05, 38.0, ["199 IN SYNC...", "*ONE SOLOIST*"]),
    (40.6, 45.6, ["THE BOARD IS", "*THE CONDUCTOR*"]),
    (52.1, 57.9, ["TRY IT:", "*2 METRONOMES*", "+ 2 SODA CANS"]),
    (59.5, 60.5, ["200 METRONOMES", "*= 1 SONG*"]),
]
BIG_TEXT = (T_DROP, T_DROP + 2.4)          # "200/200" slams in on the drop
