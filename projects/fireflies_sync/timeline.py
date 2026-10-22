"""Fireflies That Sync (IDEAS #15): ONE timeline for picture AND sound.

Science (see README): Photinus carolinus males flash 5-8 times ~0.55 s apart, then go dark ~8 s.
A lone male has no clear rhythm; synchrony appears at ~20 males. In big swarms each burst rolls
across the forest at ~0.5 m/s. Females answer, dimmer, from the ground in the males' dark pauses.

Music: one flash = one bell note (each fly its own pentatonic pitch). The flash spacing sets the tempo:
0.55 s per flash = 109 BPM (calc), so a burst of 8 flashes = 2 bars and a dark pause of 16 beats = 4 bars
(8.8 s, inside the "~8 s" range). Burst onsets are anchored to bar downbeats:

    burst A (lock-in, ~20 flies)   12.19 s   (grid beat -16)
    burst 1 (dozens)               21.00 s   (beat 0)
    burst 2 (hundreds, wave L->R)  34.21 s   (beat 24)
    burst 3 (valley, waves meet)   47.42 s   (beat 48)

The first gap (16.6-21.0 s) is a short 8-beat gap so the first big drop lands at 21 s; every gap after
the drop is the full 16 beats. The video is exactly 109 beats long, so it loops on the random sparkle."""
import math

FPS = 30
DUR = 60.0
NF = int(FPS * DUR)

BPM = 109.0
BEAT = 60.0 / BPM               # 0.5505 s = the real flash spacing (~0.55 s)
BAR = 4 * BEAT
FLASHES = 8                     # flashes per burst (NPS: 5-8)
DARK_BEATS = 16                 # 8.8 s of dark (captioned as "~8 s")
CYCLE = (FLASHES + DARK_BEATS) * BEAT

TB1 = 21.0                      # burst 1 = beat 0 of the grid


def grid(b):
    """time of grid beat b (flash beats once synced)"""
    return TB1 + b * BEAT


T_A = grid(-16)                 # 12.193  lock-in burst (~20)
T_A_END = grid(-8)              # 16.597
T_B1, T_B1_END = grid(0), grid(8)          # 21.000 - 25.404
T_B2, T_B2_END = grid(24), grid(32)        # 34.211 - 38.615
T_B3, T_B3_END = grid(48), grid(56)        # 47.422 - 51.826
BURSTS = [T_A, T_B1, T_B2, T_B3]

T_PUSH = 3.8                    # push in on the lone male
T_MACRO = 6.2
T_NEIGH = 8.0                   # neighbours join
T_RANDOM_END = 12.0             # opening random flashers fall silent / join
T_RET = 56.0                    # camera returns, random sparkle restarts
T_RANDOM_BACK = 55.6

V_WAVE = 0.5                    # m/s, burst propagation (Sarfati et al. 2021)
WAVE_HW = 0.69                  # each fly flashes while the front is within this (= 5 flashes)

# ---------------------------------------------------------------- pitch
# D major pentatonic bells, D5..D7: mid-high, survives phone speakers
BELLS = [74, 76, 78, 81, 83, 86, 88, 90, 93, 95, 98]
FEMALE_NOTES = [62, 66, 69, 71, 74]        # soft low answers (D4..D5)

# chord per bar, 6-bar cycle locked to the bursts: burst = D, Bm ; dark = G, Em, G, Asus4
CHORDS = {
    'D': dict(bass=38, pad=[50, 57, 62, 66]),
    'Bm': dict(bass=35, pad=[47, 54, 59, 62]),
    'G': dict(bass=43, pad=[43, 50, 55, 59]),
    'Em': dict(bass=40, pad=[52, 55, 59, 62]),
    'Asus': dict(bass=33, pad=[45, 52, 57, 62]),
}
CYCLE_CHORDS = ['D', 'Bm', 'G', 'Em', 'G', 'Asus']


def bar_index(t):
    return math.floor((t - TB1) / BAR + 1e-9)


def chord_at(t):
    return CYCLE_CHORDS[bar_index(t) % 6]


def phase(t):
    """('random'|'neigh'|'burst'|'dark', burst index, beats into the phase)"""
    if t < T_NEIGH or t >= T_RET:
        return ('random', -1, 0.0)
    if t < T_A:
        return ('neigh', -1, (t - T_NEIGH) / BEAT)
    for k, tb in enumerate(BURSTS):
        if tb <= t < tb + FLASHES * BEAT:
            return ('burst', k, (t - tb) / BEAT)
    prev = max(k for k, tb in enumerate(BURSTS) if tb <= t)
    return ('dark', prev, (t - BURSTS[prev] - FLASHES * BEAT) / BEAT)


def dark_progress(t):
    """0..1 through the current dark pause (for the HUD bar), None outside one"""
    ph, k, b = phase(t)
    if ph != 'dark':
        return None
    span = 8 if k == 0 else DARK_BEATS
    if k == 3:
        span = (T_RET - T_B3_END) / BEAT
    return min(1.0, b / span)


# ---------------------------------------------------------------- captions
# (t0, t1, [lines], [per-line delay s]); *word* = amber, ~word~ = green
CAPTIONS = [
    (0.0, 3.9, ["1 FLY: *RANDOM*", "20 FLIES: *SYNC*"], [0.0, 0.0]),
    (4.1, 7.9, ["ALONE:", "*NO RHYTHM*"], [0.0, 0.0]),
    (8.1, 11.95, ["EACH ONE *WATCHES*", "ITS NEIGHBOURS"], [0.0, 0.0]),
    (12.15, 16.5, ["AT ≈20:", "THEY *LOCK IN*"], [0.0, 0.0]),
    (16.7, 20.85, ["*8 FLASHES*", "≈0.5 S APART"], [0.0, 0.0]),
    (25.6, 29.6, ["≈8 S OF *DARK*..."], [0.0]),
    (29.7, 34.05, ["~FEMALES~", "~ANSWER~"], [0.0, 0.0]),
    (34.35, 38.5, ["THE FLASH *ROLLS*", "AT ≈0.5 M/S"], [0.0, 0.0]),
    (38.8, 47.2, ["WHY SYNC?", "*STILL A MYSTERY*"], [0.0, 3.6]),
    (52.0, 55.8, ["SEE IT: SMOKY MTNS", "*LATE MAY-JUNE*"], [0.0, 0.0]),
    (56.05, 59.65, ["EVERY NIGHT IT", "*STARTS RANDOM*"], [0.0, 0.0]),
]

# HUD panel (slides up from the bottom)
HUD_IN, HUD_OUT = 8.4, 56.2
