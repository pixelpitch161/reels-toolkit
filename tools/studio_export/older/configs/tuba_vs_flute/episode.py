"""#32 BALLFIGHT #6: Tuba vs Flute (v3), as a kit config (plan m4 Task 5, Table O3 row 35). Delivered from
projects/ballfight_tuba_vs_flute/ (left as delivered, only read). Both fighters are the kit's (TUBA as #36 built it,
FLUTE as #32 v2 built it); the episode's theme (villain march), tempo, key, scale, chords, the flute's line, both solos,
damage scale and seed. #32 v3 already fights from bar 2 (timeline.py 47), so its seed replays #32's fight. The music
box's counter-line (timeline.py 106 MUSIC_BOX) is the bed's, not a fighter's."""

FIGHTERS = ('TUBA', 'FLUTE')           # timeline.py 45
SEED = 514                              # seed.txt
DMG_SCALE = 1.06                        # timeline.py 55

BPM = 126                               # timeline.py 39: loop_bpm(126) = 125.90, 32 bars
SHARD_SEED, FX_SEED = 271, 32           # render.py 525 (the shatter), 757 (the particles)

MOOD = 'villain_march'                  # Table O2 (audio.py's bed)
KEY, SCALE = 'C', 'harmonic minor'      # timeline.py 74-75: KEY_ROOT 36, SCALE (0, 2, 3, 5, 7, 8, 11)
LOOP = ['Cm', 'Ab', 'Db', 'G']          # timeline.py 79
CHORDS = ['Cm', 'G'] + LOOP * 7 + ['Ab', 'G']   # timeline.py 79, as it is
ROLES = {'TUBA': 'bass', 'FLUTE': 'melody'}     # timeline.py 74 (the key on the tuba's home C2); the flute leads
LINES = {
    # timeline.py 117-132: the charmer line (CHARMER), the note sounding on each 16th of the loop (charmer_note: a note
    # holds to the next slot)
    'FLUTE': [79, 79, 80, 79, 83, 83, 84, 84, 83, 83, 80, 79, 77, 77, 75, 75,
              72, 72, 75, 75, 77, 79, 80, 80, 84, 84, 83, 83, 80, 80, 79, 79,
              77, 77, 80, 80, 85, 85, 84, 84, 85, 84, 83, 83, 80, 80, 77, 77,
              79, 80, 79, 79, 83, 83, 86, 86, 89, 89, 86, 86, 83, 83, 80, 80],
}
SOLO = {
    # timeline.py 136-140
    'TUBA': [(0, 36, 0.75), (1, 43, 0.5), (2, 36, 0.75), (3, 43, 0.5), (3.5, 42, 0.5), (4, 41, 0.75), (5, 39, 0.5),
             (6, 38, 0.5), (6.5, 37, 0.5), (7, 36, 1.0), (8, 44, 0.75), (9, 43, 0.5), (9.5, 44, 0.25), (9.75, 46, 0.25),
             (10, 47, 0.5), (10.5, 48, 0.5), (11, 36, 1.0)],
    # timeline.py 141-147
    'FLUTE': [(0, 72, 0.25), (0.25, 74, 0.25), (0.5, 75, 0.25), (0.75, 77, 0.25), (1, 79, 0.25), (1.25, 80, 0.25),
              (1.5, 83, 0.25), (1.75, 84, 0.25), (2, 87, 0.75), (2.75, 86, 0.125), (2.875, 87, 0.125), (3, 86, 0.125),
              (3.125, 87, 0.125), (3.25, 86, 0.125), (3.375, 87, 0.125), (3.5, 89, 0.5), (4, 91, 0.5), (4.5, 89, 0.25),
              (4.75, 87, 0.25), (5, 86, 0.25), (5.25, 84, 0.25), (5.5, 83, 0.5), (6, 80, 0.5), (6.5, 79, 0.25),
              (6.75, 80, 0.25), (7, 83, 0.5), (7.5, 84, 0.5), (8, 91, 0.25), (8.25, 89, 0.25), (8.5, 87, 0.25),
              (8.75, 86, 0.25), (9, 84, 0.25), (9.25, 83, 0.25), (9.5, 80, 0.25), (9.75, 79, 0.25), (10, 75, 0.5),
              (10.5, 74, 0.5), (11, 72, 1.0)],
}
TITLE_HITS = {'TUBA': (36, 31),                 # audio.py 303-304: the low C with the low G under it
              'FLUTE': (79, 84, 87, 91)}        # audio.py 313-315: the flurry up to the flutter-tongued G
