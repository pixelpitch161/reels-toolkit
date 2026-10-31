"""#36 BALLFIGHT #10: Trombone vs Tuba, rebuilt as an episode-kit config to check the TUBA port (kit/fighters/tuba.py:
OOM-PAH SWALLOW, OOMPAH, BRASS WALL, SUBSONIC, as #36 drew and played it). Delivered from
projects/ballfight_trombone_vs_tuba/ (left as delivered); this config replays the same fight (seed 358: the same
replay-guard lines as #36's own sim).

The TROMBONE is the kit's (#34's drawing, #41's sound; here its opt-in 'riff' role on #36's TROMBONE_LINE, not #36's
own tenor-lead idiom), and the bed is a stand-in (#36's brass house is not a kit mood; big_top, like it, has no brass
and no bass of its own and leaves the low end to the fighters), so compare the tuba's layers and its bus (+ the solo
bus: the tuba wins), not the trombone's bus or the whole mix. Checked: replay hash identical, 10 fight frames
pixel-identical with #36's render.py (UI on and off), the tuba and solo buses bit-identical before the master.
The kit's tail is the series standard (#36's too: the reassembly two bars before the end)."""

FIGHTERS = ('TROMBONE', 'TUBA')
SEED = 358                               # #36's seed
DMG_SCALE = 1.0
DMG_SIDE = (0.35, 1.55)                  # #36's frozen handicap (timeline.DMG_SIDE)

BPM = 126                                # loop_bpm -> 125.90: 32 bars, FIGHT! at 3.81 s
SERIES = 'BALLFIGHT #10'                 # (#36 predates the no-tag rule)
SHARD_SEED, FX_SEED = 271, 36            # #36's own seeds for the shatter and the particles

MOOD = 'big_top'                         # a stand-in: #36's brass house is not a kit mood
KEY, SCALE = 'F', 'minor'
LOOP = ['Fm', 'Db', 'Bbm', 'C']
CHORDS = ['Fm', 'C'] + LOOP * 7 + ['Db', 'C']
ROLES = {'TROMBONE': 'riff', 'TUBA': 'bass'}
LINES = {
    'TROMBONE': [65, 63, 60, 63, 65, 61, 60, 56, 61, 60, 58, 61, 64, 67, 60, 64]}   # #36's TROMBONE_LINE (beats)
SOLO = {                                 # #36's, unchanged
    'TROMBONE': [(0, 60, .5), (.5, 63, .5), (1, 65, .75), (1.75, 67, .25), (2, 68, 1.0), (3, 67, .5), (3.5, 65, .5),
                 (4, 63, .5), (4.5, 65, .5), (5, 60, 1.0), (6, 58, .5), (6.5, 60, .5), (7, 63, .5), (7.5, 65, .5),
                 (8, 68, .75), (8.75, 67, .25), (9, 65, .5), (9.5, 64, .5), (10, 65, 2.0)],
    'TUBA': [(0, 41, .5), (.5, 53, .25), (.75, 41, .25), (1, 48, .5), (1.5, 41, .5), (2, 44, .5), (2.5, 46, .5),
             (3, 48, .75), (3.75, 47, .25), (4, 46, .5), (4.5, 44, .5), (5, 41, .5), (5.5, 53, .5), (6, 48, .5),
             (6.5, 49, .5), (7, 50, .5), (7.5, 52, .5), (8, 53, 1.0), (9, 48, .5), (9.5, 41, .5), (10, 41, 2.0)]}

CAPTION = 'Who owns the low end? 🎺 Comment the next match-up 👇'
