"""#30 BALLFIGHT #4: Bagpipes vs Accordion (v3), as a kit config (plan m4 Task 5, Table O3 row 33). Delivered from
projects/ballfight_bagpipes_vs_accordion/ (left as delivered, only read). Today's roster, the series-standard beat
sheet, the episode's theme (sea shanty), tempo, key, scale, chords, the chanter's line, both solos, damage scale and
seed. #30 v3 already fights from bar 2 (timeline.py 49), so its seed replays #30's fight."""

FIGHTERS = ('BAGPIPES', 'ACCORDION')   # timeline.py 47
SEED = 1442                             # seed.txt
DMG_SCALE = 0.76                        # timeline.py 58

BPM = 122                               # timeline.py 41: loop_bpm(122) = 121.97, 31 bars
SHARD_SEED, FX_SEED = 271, 30           # render.py 397 (the shatter), 835 (the particles)

MOOD = 'sea_shanty'                     # Table O2 (audio.py's bed)
KEY, SCALE = 'A', 'dorian'              # timeline.py 76-77: KEY_ROOT 45, SCALE (0, 2, 3, 5, 7, 9, 10)
LOOP = ['Am', 'G', 'Am', 'Em']          # timeline.py 81
CHORDS = ['Am', 'E'] + LOOP * 7 + ['E']  # timeline.py 81, as it is
LINES = {
    # timeline.py 103-112: MELODY, the chanter's line, one pitch per 8th
    'BAGPIPES': [69, 69, 72, 74, 76, 74, 72, 71,
                 67, 67, 71, 74, 79, 78, 74, 71,
                 69, 72, 76, 81, 79, 76, 74, 72,
                 71, 71, 67, 71, 76, 74, 71, 68],
}
SOLO = {
    # timeline.py 125-130
    'BAGPIPES': [(0, 69, 1.0), (1, 71, 0.5), (1.5, 72, 0.5), (2, 74, 0.75), (2.75, 76, 0.25), (3, 76, 1.0),
                 (4, 81, 1.5), (5.5, 79, 0.5), (6, 78, 0.75), (6.75, 76, 0.25), (7, 74, 0.5), (7.5, 72, 0.5),
                 (8, 71, 0.75), (8.75, 72, 0.25), (9, 74, 0.5), (9.5, 71, 0.5), (10, 69, 2.0)],
    # timeline.py 131-134
    'ACCORDION': [(0, 69, 0.5), (0.5, 72, 0.5), (1, 76, 0.5), (1.5, 81, 0.5), (2, 80, 0.25), (2.25, 81, 0.25),
                  (2.5, 83, 0.5), (3, 84, 1.0), (4, 83, 0.25), (4.25, 81, 0.25), (4.5, 79, 0.25), (4.75, 78, 0.25),
                  (5, 76, 0.5), (5.5, 74, 0.5), (6, 72, 0.5), (6.5, 74, 0.25), (6.75, 72, 0.25), (7, 71, 0.5),
                  (7.5, 68, 0.5), (8, 69, 0.5), (8.5, 72, 0.5), (9, 76, 0.5), (9.5, 71, 0.5), (10, 69, 2.0)],
}
