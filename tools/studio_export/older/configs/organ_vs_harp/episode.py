"""#35 BALLFIGHT #9: Organ vs Harp (v4), as a kit config (plan m4 Task 5, Table O3 row 37). Delivered from
projects/ballfight_organ_vs_harp/ (left as delivered, only read). Today's roster, the series-standard beat sheet, the
episode's theme (gothic horror), tempo, key, scale, chords, both solos, damage scale and seed. #35 already fights from
bar 2 (timeline.py 48), so its seed replays #35's fight (Gate O's ORGAN self-proof runs the same line).

Two values here have no form in a format 2 fight file (task-05-report.md, for a ruling): CHORDS is #35's three sections
(bars 2-13 i i VI V, 14-21 i iv VI V, 22-29 i VI iv V; timeline.py 76-79), which no single 4- or 8-chord loop gives, so
LOOP is the first section's; and the solos hold #35's named entries, ORGAN's ('dim', root) and (chord, root) and HARP's
('gliss', top)."""

FIGHTERS = ('ORGAN', 'HARP')           # timeline.py 46
SEED = 237                              # seed.txt
DMG_SCALE = 0.6                         # timeline.py 56

BPM = 126                               # timeline.py 40: loop_bpm(126) = 125.90, 32 bars
SHARD_SEED, FX_SEED = 271, 35           # render.py 339 (the shatter), 836 (the particles)

MOOD = 'gothic_horror'                  # Table O2 (audio.py's bed)
KEY, SCALE = 'D', 'minor'               # timeline.py 74-75: KEY_ROOT 38, SCALE (0, 2, 3, 5, 7, 8, 10)
LOOP = ['Dm', 'Dm', 'Bb', 'A']          # timeline.py 79: the slow burn's loop (bars 2-13)
# timeline.py 79, as it is: the burn (LOOP x3), the climb, the pound, then i V into the seam
CHORDS = ['Dm', 'A'] + LOOP * 3 + ['Dm', 'Gm', 'Bb', 'A'] * 2 + ['Dm', 'Bb', 'Gm', 'A'] * 2 + ['Dm', 'A']
SOLO = {
    # timeline.py 118-122: ('dim', r) a diminished seventh on r, (chord, r) that chord on r
    'ORGAN': [(0, 69, 0.25), (0.25, 67, 0.25), (0.5, 69, 1.0), (1.5, 67, 0.25), (1.75, 65, 0.25), (2, 64, 0.25),
              (2.25, 62, 0.25), (2.5, 61, 0.75), (3.25, 62, 0.75), (4, ('dim', 61), 1.0), (5, ('dim', 64), 1.0),
              (6, ('dim', 67), 0.5), (6.5, ('dim', 70), 0.5), (7, ('Dm', 62), 1.0), (8, ('Dm', 50), 2.5)],
    # timeline.py 123-127: ('gliss', top) a glissando up to top
    'HARP': [(0, 50, 0.25), (0.25, 57, 0.25), (0.5, 62, 0.25), (0.75, 65, 0.25), (1, 69, 0.25), (1.25, 74, 0.25),
             (1.5, 77, 0.5), (2, 81, 0.5), (2.5, 79, 0.25), (2.75, 77, 0.25), (3, 76, 0.25), (3.25, 74, 0.25),
             (3.5, 73, 0.5), (4, 46, 0.25), (4.25, 53, 0.25), (4.5, 58, 0.25), (4.75, 62, 0.25), (5, 65, 0.25),
             (5.25, 70, 0.25), (5.5, 74, 0.5), (6, ('gliss', 86), 1.0), (7, 74, 0.5), (7.5, 81, 0.5), (8, 86, 2.5)],
}
TITLE_HITS = {'ORGAN': (26, 38, 50, 57, 62, 65, 69, 74)}   # audio.py 321: the full organ, pedal to mixture
