"""ONE timeline drives picture AND sound. Put every event time here and import it in both
render.py and audio.py so cuts, captions, beats and the science stay in sync.

Proven shape, 61 s (TikTok monetisation needs >= 1 min; the extra second is a safety net):
    0-4 s    hook: the most striking image + a question/claim caption, no slow intro
    4-8 s    reveal the setup / environment (one wide establishing shot)
    8-35 s   explain in 3-5 beats, each a new visual payoff every 2-4 s (sweep, slow-mo, counter...)
    35-52 s  the music/song payoff: the science drives the music (tempo, notes, rhythm)
    52-61 s  takeaway + "try it" call to action; the loop point is the file boundary (61.0 s = frame 0)
"""
FPS = 30
DUR = 61.0
NF = int(FPS * DUR)

from reelkit.config import loop_bpm
BPM = loop_bpm(96, DUR)       # 94.43: a whole number of bars in 61 s, so the beat is in phase at the loop point
BEAT = 60.0 / BPM
T_REVEAL = 4.0
T_SONG = 35.0
T_END = 52.0

CAPTIONS = [
    (0.0, 4.0, ["WHAT DOES A", "*SOUND* LOOK LIKE?"]),
    (4.1, 8.0, ["A *TEMPLATE* SCENE", "FOR THE NEXT VIDEO"]),
    (8.1, 20.0, ["SWAP IN YOUR", "*SCIENCE* HERE"]),
    (20.1, 35.0, ["EVERY *BEAT* PULSES", "THE LANTERN"]),
    (35.1, 52.0, ["THE *SONG* PAYOFF"]),
    (52.1, 59.0, ["TRY IT *TONIGHT*!"]),
]


def beat_env(t, decay=0.12):
    """0..1 pulse on every beat after the song starts"""
    import math
    if t < T_SONG:
        return 0.0
    ph = (t - T_SONG) % BEAT
    return math.exp(-ph / decay)
