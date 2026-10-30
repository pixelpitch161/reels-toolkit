"""#34 BALLFIGHT #8: Trombone vs French Horn (v5), rebuilt as an episode-kit config to check the HORN port
(kit/fighters/horn.py, HORN v3 = ECHO CALL + FINAL FLICK, made for #54 Horn vs Trombone, THE REMATCH) and the
brass_trailer mood (kit/moods/brass_trailer.py). Delivered from projects/ballfight_trombone_vs_horn/ (left as delivered);
this config replays the same fight (seed 140: the same replay-guard lines as #34's own sim).

The TROMBONE is the kit's (#34's drawing, #41's sound), so compare the horn's layers and its bus and the bed's buses,
not the trombone's bus or the whole mix. The kit's tail is the series standard (the reassembly two bars before the end;
#34's ran half a bar later): compare frames up to the shatter. The card shows the roster name HORN (#34: FRENCH HORN)."""

FIGHTERS = ('TROMBONE', 'HORN')
SEED = 140                               # #34 v5's seed
DMG_SCALE = 0.72                         # #34 v5's frozen damage scale

BPM = 122                                # loop_bpm -> 121.97: 31 bars, FIGHT! at 3.94 s
SERIES = 'BALLFIGHT #8'                  # (#34 predates the no-tag rule)
SHARD_SEED, FX_SEED = 271, 34            # #34's own seeds for the shatter and the particles

MOOD = 'brass_trailer'                   # #34's bed, ported
KEY, SCALE = 'D', 'minor'
LOOP = ['Dm', 'Bb', 'Gm', 'A']
CHORDS = ['Dm', 'A'] + LOOP * 7 + ['A']
ROLES = {'TROMBONE': 'riff', 'HORN': 'melody'}
LINES = {
    'TROMBONE': [50, 50, 57, 55, 46, 46, 53, 50, 55, 55, 58, 57, 57, 57, 61, 52],     # #34's TROMBONE_LINE (beats)
    'HORN': [62, 69, 65, 69, 62, 65, 70, 65, 62, 67, 70, 67, 61, 64, 69, 64],         # #34's HORN_CALL (a call a beat)
    'HORN THEME': [62, 69, 70, 65, 67, 70, 69, 73]}                                   # #34's HORN_LINE (STOPPED HORN)
SOLO = {                                 # #34's, unchanged
    'TROMBONE': [(0, 50, .5), (.5, 53, .5), (1, 57, 1.0), (2, 55, .5), (2.5, 53, .5), (3, 50, 1.0), (4, 45, .5),
                 (4.5, 48, .5), (5, 50, .75), (5.75, 53, .25), (6, 57, 1.0), (7, 60, .5), (7.5, 58, .5), (8, 57, .5),
                 (8.5, 53, .5), (9, 50, 1.5)],
    'HORN': [(0, 62, .5), (.5, 69, .5), (1, 74, 1.0), (2, 72, .5), (2.5, 70, .5), (3, 69, 1.0), (4, 65, .5), (4.5, 69, .5),
             (5, 74, .75), (5.75, 72, .25), (6, 70, .5), (6.5, 69, .5), (7, 73, 1.0), (8, 74, 2.0)]}

CAPTION = 'The brass section pecking order 🎺📯 Comment the next match-up 👇'
