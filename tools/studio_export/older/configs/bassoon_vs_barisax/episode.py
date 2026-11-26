"""#39 BALLFIGHT #13: Bassoon vs Bari Sax (v4), as a kit config (plan m4 Task 5, Table O3 row 39). Delivered from
projects/ballfight_bassoon_vs_barisax/ (left as delivered, only read; the kit's examples/bassoon_vs_barisax is the same
fight under a stand-in bed). Today's roster (the BARI SAX with `steer`, owner default Q3), the series-standard beat
sheet, the episode's theme (ska punk), tempo, key, scale, chords, the bassoon's line, both solos, roles, handicap,
damage scale and seed. #39 v4 already fights from bar 2 (timeline.py 57), so its seed replays #39's fight."""

FIGHTERS = ('BASSOON', 'BARI SAX')     # timeline.py 55
SEED = 77                               # seed.txt
DMG_SCALE = 0.34                        # timeline.py 66
DMG_SIDE = (1.0, 0.8)                   # timeline.py 67: the BARI SAX's hits x0.8

BPM = 170                               # timeline.py 49: loop_bpm(170) = 169.18, 43 bars
SHARD_SEED, FX_SEED = 371, 39           # render.py 576 (the shatter), 1224 (the particles)

MOOD = 'ska_punk'                       # Table O2 (audio.py's bed)
KEY, SCALE = 'E', 'minor'               # timeline.py 91-92: KEY_ROOT 40, SCALE (0, 2, 3, 5, 7, 8, 10)
LOOP = ['Em', 'C', 'D', 'B']            # timeline.py 95
CHORDS = ['Em', 'B'] + LOOP * 10 + ['B']        # timeline.py 95, as it is
ROLES = {'BASSOON': 'melody', 'BARI SAX': 'bass'}   # as the kit's examples/bassoon_vs_barisax
LINES = {
    # timeline.py 121-124: BASSOON_8THS, one pitch per 8th
    'BASSOON': [64, 67, 71, 67, 64, 66, 67, 69,
                64, 67, 72, 67, 64, 67, 71, 67,
                66, 69, 74, 69, 66, 69, 72, 69,
                66, 71, 75, 71, 66, 69, 71, 63],
}
SOLO = {
    # timeline.py 154-157
    'BASSOON': [(0, 64, 0.25), (0.5, 67, 0.25), (1, 71, 0.25), (1.5, 69, 0.25), (2, 67, 0.25), (2.5, 66, 0.25),
                (3, 64, 0.25), (3.5, 71, 0.25), (4, 76, 0.25), (4.5, 74, 0.25), (5, 71, 0.25), (5.5, 67, 0.25),
                (6, 64, 0.5), (7, 40, 0.75)],
    # timeline.py 158-160
    'BARI SAX': [(0, 40, 0.5), (1, 52, 0.5), (1.5, 50, 0.25), (2, 47, 0.5), (3, 45, 0.25), (3.5, 43, 0.5), (4, 40, 0.5),
                 (5, 47, 0.25), (5.5, 45, 0.25), (6, 40, 1.5)],
}
