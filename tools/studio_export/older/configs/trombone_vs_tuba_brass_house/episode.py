"""#36 BALLFIGHT #10: Trombone vs Tuba with #36's own music, as a kit config (plan m4 Task 5, Table O3 row 38; owner
default Q5: brass house is this pair's starting music, the big top of the kit's examples/trombone_vs_tuba stays a
preset). Delivered from projects/ballfight_trombone_vs_tuba/ (left as delivered, only read). Both fighters are the
kit's; the episode's theme (brass house), tempo, key, scale, chords, the trombone's line, both solos, roles, handicap,
damage scale and seed. #36 already fights from bar 2 (timeline.py 45), so its seed replays #36's fight."""

FIGHTERS = ('TROMBONE', 'TUBA')        # timeline.py 43
SEED = 358                              # seed.txt
DMG_SCALE = 1.0                         # timeline.py 54
DMG_SIDE = (0.35, 1.55)                 # timeline.py 55: the frozen handicap

BPM = 126                               # timeline.py 37: loop_bpm(126) = 125.90, 32 bars
SHARD_SEED, FX_SEED = 271, 36           # render.py 429 (the shatter), 952 (the particles)

MOOD = 'brass_house'                    # Table O2 (audio.py's bed)
KEY, SCALE = 'F', 'minor'               # timeline.py 75-76: KEY_ROOT 41, SCALE (0, 2, 3, 5, 7, 8, 10)
LOOP = ['Fm', 'Db', 'Bbm', 'C']         # timeline.py 79
CHORDS = ['Fm', 'C'] + LOOP * 7 + ['Db', 'C']   # timeline.py 79, as it is
ROLES = {'TROMBONE': 'riff', 'TUBA': 'bass'}    # as the kit's examples/trombone_vs_tuba plays #36's line
LINES = {
    # timeline.py 104-107: TROMBONE_LINE, one pitch per beat
    'TROMBONE': [65, 63, 60, 63,
                 65, 61, 60, 56,
                 61, 60, 58, 61,
                 64, 67, 60, 64],
}
SOLO = {
    # timeline.py 126-130
    'TROMBONE': [(0, 60, 0.5), (0.5, 63, 0.5), (1, 65, 0.75), (1.75, 67, 0.25), (2, 68, 1.0), (3, 67, 0.5),
                 (3.5, 65, 0.5), (4, 63, 0.5), (4.5, 65, 0.5), (5, 60, 1.0), (6, 58, 0.5), (6.5, 60, 0.5), (7, 63, 0.5),
                 (7.5, 65, 0.5), (8, 68, 0.75), (8.75, 67, 0.25), (9, 65, 0.5), (9.5, 64, 0.5), (10, 65, 2.0)],
    # timeline.py 131-134
    'TUBA': [(0, 41, 0.5), (0.5, 53, 0.25), (0.75, 41, 0.25), (1, 48, 0.5), (1.5, 41, 0.5), (2, 44, 0.5),
             (2.5, 46, 0.5), (3, 48, 0.75), (3.75, 47, 0.25), (4, 46, 0.5), (4.5, 44, 0.5), (5, 41, 0.5),
             (5.5, 53, 0.5), (6, 48, 0.5), (6.5, 49, 0.5), (7, 50, 0.5), (7.5, 52, 0.5), (8, 53, 1.0), (9, 48, 0.5),
             (9.5, 41, 0.5), (10, 41, 2.0)],
}
