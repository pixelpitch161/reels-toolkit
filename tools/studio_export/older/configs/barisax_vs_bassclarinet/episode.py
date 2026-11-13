"""#40 BALLFIGHT #14: Bari Sax vs Bass Clarinet (v2), as a kit config (plan m4 Task 5, Table O3 row 40). Delivered from
projects/ballfight_barisax_vs_bassclarinet/ (left as delivered, only read). Today's roster: the BARI SAX of #39 v4 with
`steer` (owner default Q3), so unlike #40's own sim.py (22-24, which pins `steer` off for its process) nothing is pinned
here and the fight is a new fight (plan question Q-E, Table O3's note). The series-standard beat sheet, the episode's
theme (industrial), tempo, key, scale, chords, both lines, both solos, roles, handicap, damage scale and the title
card's two-line name.

SEED is not #40's 128: under today's BARI SAX seed 128 reaches no K.O. (the replay guard's kit line `128 None None 978
bfc2a20ce87b9fe3`; #40's own sim gives `128 1 49.59583333333333 905 4e35a2802e9b37d2`), so, as for the stand-ins, SEED
is the first seed of the kit's search (search.py's `one`, from seed 1) that fits, at #40's handicap (reported for a
ruling: task-05-report.md)."""

FIGHTERS = ('BARI SAX', 'BASS CLARINET')   # timeline.py 49
SEED = 75   # the first fit of the kit's search from seed 1 (#40's seed.txt 128: no K.O.)
DMG_SCALE = 1.0                         # timeline.py 63
DMG_SIDE = (0.30, 0.66)                 # timeline.py 64: the frozen handicap

BPM = 126                               # timeline.py 43: loop_bpm(126) = 125.90, 32 bars
SHARD_SEED, FX_SEED = 401, 40           # render.py 518 (the shatter), 1160 (the particles)
TITLE_LINES = {'BASS CLARINET': ('BASS', 'CLARINET')}   # timeline.py 56
TITLE_BALLS = [(78, 222, 56), (302, 352, 48)]           # timeline.py 58

MOOD = 'industrial'                     # Table O2 (audio.py's bed)
KEY, SCALE = 'G', 'minor'               # timeline.py 105-106: KEY_ROOT 43, SCALE (0, 2, 3, 5, 7, 8, 10)
LOOP = ['Gm', 'Eb', 'Cm', 'D']          # timeline.py 109
CHORDS = ['Gm', 'D'] + LOOP * 7 + ['Eb', 'D']   # timeline.py 109, as it is
ROLES = {'BARI SAX': 'bass', 'BASS CLARINET': 'counter'}   # as the kit's examples/barisax_vs_bassclarinet
LINES = {
    # timeline.py 131-134: BARI_PUMP, root + fifth on the beats
    'BARI SAX': [43, 43, 50, 43,
                 39, 39, 46, 39,
                 36, 36, 43, 36,
                 38, 38, 45, 42],
    # timeline.py 138-141: COUNTER_8THS, one pitch per 8th
    'BASS CLARINET': [55, 55, 62, 55, 58, 55, 62, 57,
                      51, 51, 58, 51, 55, 51, 58, 55,
                      48, 48, 55, 48, 51, 48, 55, 51,
                      50, 50, 57, 50, 54, 50, 57, 54],
}
SOLO = {
    # timeline.py 163-166
    'BARI SAX': [(0, 36, 0.75), (1, 43, 0.5), (1.5, 48, 0.5), (2, 51, 0.25), (2.5, 48, 0.5), (3, 46, 0.5),
                 (3.5, 43, 0.25), (4, 38, 0.5), (5, 50, 0.25), (5.25, 48, 0.25), (5.5, 45, 0.25), (5.75, 42, 0.25),
                 (6, 38, 1.5)],
    # timeline.py 167-170
    'BASS CLARINET': [(0, 48, 0.5), (0.5, 51, 0.5), (1, 55, 0.5), (1.5, 58, 0.5), (2, 60, 0.75), (2.75, 58, 0.25),
                      (3, 55, 0.5), (3.5, 51, 0.5), (4, 50, 0.5), (4.5, 54, 0.5), (5, 57, 0.5), (5.5, 54, 0.25),
                      (5.75, 50, 0.25), (6, 50, 1.5)],
}
