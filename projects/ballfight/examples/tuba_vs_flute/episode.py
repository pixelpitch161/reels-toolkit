"""#32 BALLFIGHT #6: Tuba vs Flute (v3), rebuilt as an episode-kit config (the check of the villain-march mood,
kit/moods/villain_march.py, and the first time the TUBA loses in a kit episode). Delivered from
projects/ballfight_tuba_vs_flute/ (left as delivered); this config replays the same fight (seed 514: FLUTE wins at
51.28 s with 5.7 HP).

Biggest vs smallest, lowest vs highest. TUBA (left, indigo) and FLUTE (right, silver), both built in #32 v2; each is
the one last delivered: the TUBA as #36 drew and played it (kit/fighters/tuba.py), the FLUTE as #41 did
(kit/fighters/flute.py), so their parts and effects differ from #32's where those episodes changed them. #32 v3 had the
VS card and the v2 effects but no HUD ability list (the kit draws one). Music: villain march in C minor; the bed has no
bass and no lead: the TUBA is the bass line (oom = the root, pah = the fifth), the FLUTE the high charmer line.

Checked against #32 v3's own renderer and audio: the replay hash is identical; frames are pixel-identical outside the
HUD rows on the card, the count-in, 30.3 s, 55.5 s (the TUBA's shatter: the first time it loses in a delivered run),
58.0 s and 60.9 s; the rest differ only in the fighters' #36 / #41 changes and the series' later shared effects (the
K.O. flare x0.9 and the effects clock held through the hit-stop, the fringe and speed-line windows). The bed's buses
(drums, brass, pad, music box, fx) are sample-identical once the fighters' own placements are set aside."""

# ---------------------------------------------------------------- the fight
FIGHTERS = ('TUBA', 'FLUTE')             # first named = left corner (screen left)
SEED = 514                               # seed_search at 2 bars: 6 fits in 753, ranked first (as at 1 bar in v2)
DMG_SCALE = 1.06                         # tuned once (tune.py): median K.O. near 50 s, TUBA 53 % / FLUTE 47 %

# ---------------------------------------------------------------- tempo, title (only to match the delivered #32 v3)
BPM = 126                                # loop_bpm -> 125.90: 32 bars in 61 s, FIGHT! at 3.81 s
SERIES = 'BALLFIGHT #6'                  # (#32 predates the no-tag rule)
FX_SEED = 32                             # #32's own particle seed (the shatter's is the default 271)
REASSEMBLE_AT = (30, 1)                  # #32's reassembly: bar 30 beat 2 (57.66 s; the kit default is bar 30's downbeat)

# ---------------------------------------------------------------- music: villain march
MOOD = 'villain_march'
KEY, SCALE = 'C', 'harmonic minor'       # C D Eb F G Ab B
LOOP = ['Cm', 'Ab', 'Db7', 'G7']         # the 4-bar march loop i - VI - N6 - V from FIGHT!; the Neapolitan with the B
                                         # natural on top (Db F Ab B, written Db7) is the villain's snarl into G7
CHORDS = ['Cm', 'G7'] + LOOP * 7 + ['Ab', 'G7']   # the card on the tonic, the count-in on the dominant; the last two
                                                  # bars Ab . G7 resolve into the title slam's Cm on frame 0
ROLES = {'TUBA': 'bass', 'FLUTE': 'melody'}
LINES = {
    # the FLUTE's snake-charmer line (#32 v2: sinuous, C harmonic minor with the augmented second Ab-B, a phrygian-
    # dominant turn over G), one note per 8th over the loop (#32 played it on 16ths with grace notes between; these
    # are its notes on the 8ths)
    'FLUTE': [79, 80, 83, 84, 83, 80, 77, 75,      # Cm:  G Ab B C B Ab F Eb
              72, 75, 77, 80, 84, 83, 80, 79,      # Ab:  C Eb F Ab C B Ab G
              77, 80, 85, 84, 85, 83, 80, 77,      # Db7: F Ab Db C Db B Ab F
              79, 79, 83, 86, 89, 86, 83, 80]}     # G7:  G G B D F D B Ab
SOLO = {
    # the tuba's victory march: oom-pah with a menacing chromatic climb, a low pedal and a final growl on the low C
    'TUBA': [(0, 36, .75), (1, 43, .5), (2, 36, .75), (3, 43, .5), (3.5, 42, .5), (4, 41, .75), (5, 39, .5), (6, 38, .5),
             (6.5, 37, .5), (7, 36, 1.0), (8, 44, .75), (9, 43, .5), (9.5, 44, .25), (9.75, 46, .25), (10, 47, .5),
             (10.5, 48, .5), (11, 36, 1.0)],
    # the flute's cadenza: a flurry up the harmonic minor, a shrill trill, a sinister augmented-2nd fall, the low C
    'FLUTE': [(0, 72, .25), (.25, 74, .25), (.5, 75, .25), (.75, 77, .25), (1, 79, .25), (1.25, 80, .25), (1.5, 83, .25),
              (1.75, 84, .25), (2, 87, .75), (2.75, 86, .125), (2.875, 87, .125), (3, 86, .125), (3.125, 87, .125),
              (3.25, 86, .125), (3.375, 87, .125), (3.5, 89, .5), (4, 91, .5), (4.5, 89, .25), (4.75, 87, .25),
              (5, 86, .25), (5.25, 84, .25), (5.5, 83, .5), (6, 80, .5), (6.5, 79, .25), (6.75, 80, .25), (7, 83, .5),
              (7.5, 84, .5), (8, 91, .25), (8.25, 89, .25), (8.5, 87, .25), (8.75, 86, .25), (9, 84, .25),
              (9.25, 83, .25), (9.5, 80, .25), (9.75, 79, .25), (10, 75, .5), (10.5, 74, .5), (11, 72, 1.0)],
}
TITLE_HITS = {'TUBA': (36, 31),                  # #32's blast: the low C over the low G under it
              'FLUTE': (79, 84, 87, 91)}         # a shrieking flurry up to a flutter-tongued high G
CAP_TARGETS = {'OOMPAH': 10, 'FLUTTER TONGUE': 15, 'BRASS WALL': 22, 'SNAKE DANCE': 28, 'SUBSONIC': 40,
               'PICCOLO': 40}

# ---------------------------------------------------------------- the post
CAPTION = 'Lowest vs highest 📯🪈 Comment the next match-up 👇'
