"""Domino Symphony (IDEAS #17) - the ONE timeline for picture and sound.

Everything sits on a 120 BPM grid (beat 0.5 s, bar 2 s, 16th 0.125 s, 32nd 0.0625 s). The domino
schedule itself lives in layout.py (built from the times here) and is cached in layout.npz; render.py
and audio.py both read that cache, so every click you hear is a domino you see falling.

Beat sheet (60 s):
    0-1    cat's paw taps domino #1                         10,000 DOMINOES / 1 PUSH
    1-4    line splits every ~0.5 s: 1 -> 64 rays, out of step (chaos wash)
    4-8    rays funnel back together in pairs; clicks line up    64 LINES... / 1 RHYTHM
    8-14   bell lane peels off, every 4th domino rings a chime    EVERY 4TH ONE / RINGS
    14-20  bell lane splits into 4 tuned lanes = chords, kick     4 LANES / = CHORDS
    20-26  4,096-domino field falls in a chevron sweep, painted faces reveal a pixel music note
    26-32  2-lane spiral winds inward, 16ths -> triplets -> 32nds (build); cat steps over a lane
    32-38  holdout on the plank ledge: wobbles, settles, wobbles, teeters    WILL IT FALL?
    38-40  it tips: growing-domino staircase booms, each step 1.5x    EACH ONE / 1.5x BIGGER
    40-52  giant (1.28 m) slams = drop; 8 frame lanes race round the whole layout in sync,
           chimes play the song; camera cranes up over the giant pixel note
    52-56  the frame closes at the bottom on the final chord          10,000 / IN 52 SECONDS
    56-60  fast rewind: everything stands back up, paw hovers over #1 -> loop
"""
import math

FPS = 30
DUR = 60.0
NF = int(FPS * DUR)

BPM = 120
BEAT = 60.0 / BPM          # 0.5 s
BAR = 4 * BEAT             # 2.0 s
S16 = BEAT / 4             # 0.125 s
S32 = BEAT / 8             # 0.0625 s

# ---------------------------------------------------------------- section times
T_TAP = 0.0
T_FAN = 1.0                # first split
T_FUNNEL = 4.0
T_LOCK = 7.0               # 4 lanes exactly on the grid from here
T_BELL = 8.0
T_CHORD = 14.0
T_FIELD = 20.0
T_SPIRAL = 26.0
T_HOLD = 32.0              # inner spiral lane hits the ledge block
T_HOLD2 = 35.5             # outer spiral lane hits it again
T_TIP = 38.0               # holdout goes over
T_DROP = 40.0              # giant lands
T_END = 52.0               # frame closes, final chord
T_REW = 56.0               # rewind starts
T_REW_END = 59.6           # rewind done: paw hovering, real time again

# staircase: contacts on 8ths, giant struck at 39.5 and flat on the floor at 40.0
STAIR_CONTACTS = [38.25, 38.5, 38.75, 39.0, 39.25, 39.5]
STAIR_BOOMS = [(38.25, 50), (38.5, 49), (38.75, 47), (39.0, 45), (39.25, 43), (39.5, 42), (39.75, 40), (40.0, 38)]


def scene_time(t):
    """scene clock tau: real time until the rewind, then 56 -> -0.3 s backwards (accelerating, easing out),
    then real time again so the paw lands on domino #1 exactly at the loop point (tau = 0 at t = 60)."""
    if t <= T_REW:
        return t
    if t >= T_REW_END:
        return -0.3 + (t - T_REW_END) * (0.3 / (DUR - T_REW_END))
    u = (t - T_REW) / (T_REW_END - T_REW)
    e = u * u * u * (u * (6 * u - 15) + 10)
    return T_REW + (-0.3 - T_REW) * e


# ---------------------------------------------------------------- harmony (D major)
# one chord per 2 s bar; bar b starts at 2b
CHORD_OF_BAR = ['D', 'D', 'D', 'A',            # 0-8   chaos -> resolve
                'D', 'A', 'Bm', 'G',           # 8-16  melody
                'D', 'A',                      # 16-20 chords + kick
                'Bm', 'G', 'D',                # 20-26 field swell
                'Em', 'G', 'A',                # 26-32 spiral build
                'A', 'A', 'A',                 # 32-38 holdout suspense (A pedal)
                'A',                           # 38-40 staircase booms
                'D', 'A', 'Bm', 'G', 'D', 'A',  # 40-52 full song
                'D', 'D',                      # 52-56 final chord
                'D', 'D']                      # 56-60 rewind
PAD = {'D': [50, 57, 62, 66, 69], 'A': [45, 52, 57, 61, 64], 'Bm': [47, 54, 59, 62, 66], 'G': [43, 50, 55, 59, 62],
       'Em': [40, 47, 52, 55, 59]}
ROOT = {'D': 38, 'A': 33, 'Bm': 35, 'G': 31, 'Em': 40}
ARP = {'D': [69, 74, 78], 'A': [69, 73, 76], 'Bm': [71, 74, 78], 'G': [71, 74, 79], 'Em': [71, 76, 79]}

MELODY_BARS = {
    4: [78, 81, 86, 88], 5: [85, 81, 76, 81], 6: [86, 83, 78, 83], 7: [86, 85, 83, 81],
    8: [78, 81, 86, 90], 9: [88, 85, 81, 85],
    20: [78, 81, 86, 88], 21: [85, 81, 76, 81], 22: [86, 83, 78, 83], 23: [86, 85, 83, 81],
    24: [78, 81, 86, 90], 25: [88, 85, 83, 85],
}
FINAL_NOTE = 86


def chord_at(t):
    b = int(max(0, min(len(CHORD_OF_BAR) - 1, t // BAR)))
    return CHORD_OF_BAR[b]


def melody_at(t):
    """melody note on the beat at time t (or None)"""
    b = int(t // BAR)
    k = int(round((t - b * BAR) / BEAT))
    if b in MELODY_BARS and 0 <= k < 4:
        return MELODY_BARS[b][k]
    return None


def arp_at(t, lane):
    """chord tone for arp lane 1..3 at time t"""
    return ARP[chord_at(t)][lane - 1]


# ---------------------------------------------------------------- captions (start the hook early so frame 0 shows it)
CAPTIONS = [
    (-0.5, 4.0, ["10,000 DOMINOES", "*1 PUSH*"]),
    (4.05, 8.0, ["64 LINES...", "*1 RHYTHM*"]),
    (8.05, 14.0, ["EVERY 4TH ONE", "*RINGS*"]),
    (14.05, 20.0, ["4 LANES", "= *CHORDS*"]),
    (32.05, 38.0, ["WILL IT *FALL?*"]),
    (38.05, 40.0, ["EACH ONE", "*1.5×* BIGGER"]),
    (52.05, 56.0, ["*10,000*", "IN 52 SECONDS"]),
    (59.5, 60.5, ["10,000 DOMINOES", "*1 PUSH*"]),
]


def beat_env(t, decay=0.12, t0=T_CHORD, t1=T_END + 0.5):
    """0..1 pulse on every beat (for lamp/glow pumping)"""
    if t < t0 or t > t1:
        return 0.0
    ph = (t - t0) % BEAT
    return math.exp(-ph / decay)
