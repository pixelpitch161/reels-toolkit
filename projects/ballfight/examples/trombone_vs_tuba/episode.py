"""#36 BALLFIGHT #10: Trombone vs Tuba, rebuilt as an episode-kit config (the check of the TUBA port, kit/fighters/
tuba.py, and the brass-house mood, kit/moods/brass_house.py). Delivered from projects/ballfight_trombone_vs_tuba/ (left
as delivered); this config replays the same fight (seed 358).

Who owns the low end? The first crossover: TROMBONE (left, mint) vs TUBA (right, indigo), each as last built. The TUBA
is #36's own; the TROMBONE is the one last delivered (#41: kit/fighters/trombone.py), so its part is #41's circus bass
on the thrusts, not #36's tenor hook (LINES below keeps #36's), and its title blast, PEDAL TONE, fall and reassembly
are #41's. Music, mood C: brass house in F minor; the bed has no bass synth and no brass: the TUBA is the
bass line (oom = the root, pah = the fifth)."""

# ---------------------------------------------------------------- the fight
FIGHTERS = ('TROMBONE', 'TUBA')          # first named = left corner (screen left)
SEED = 358                               # seed_search at 2 bars: 6 fits in 416 (106 ranked first, its 52.9 s K.O. too late)
DMG_SIDE = (0.35, 1.55)                  # the crossover's only lever (tune: 300 seeds, 49 % / 51 %, median K.O. 50.8 s)

# ---------------------------------------------------------------- tempo, title (only to match the delivered #36)
BPM = 126                                # loop_bpm -> 125.90: 32 bars in 61 s, FIGHT! at 3.81 s
SERIES = 'BALLFIGHT #10'                 # (#36 predates the no-tag rule)
FX_SEED = 36                             # #36's own particle seed (the shatter's is the default 271)
REASSEMBLE_AT = (30, 1)                  # #36's reassembly: bar 30 beat 2 (57.66 s; the kit default is bar 30's downbeat)

# ---------------------------------------------------------------- music: mood C, brass house
MOOD = 'brass_house'
KEY, SCALE = 'F', 'minor'                # E natural over the C chord (the dominant's leading tone)
LOOP = ['Fm', 'Db', 'Bbm', 'C']          # the 4-bar house loop i - VI - iv - V from FIGHT!
CHORDS = ['Fm', 'C'] + LOOP * 7 + ['Db', 'C']     # the card on the tonic, the count-in on the dominant; the last two
                                                  # bars Db . C resolve into the title slam's Fm on frame 0
ROLES = {'TUBA': 'bass', 'TROMBONE': 'melody'}
LINES = {
    # #36's tenor hook, one pitch per beat over the loop (F3-Ab4): #36's TROMBONE played it on its thrusts; the TROMBONE
    # as last delivered (#41) plays the bass line on them instead and ignores it (kept for a later trombone part)
    'TROMBONE': [65, 63, 60, 63,         # Fm: F4 Eb4 C4 Eb4
                 65, 61, 60, 56,         # Db: F4 Db4 C4 Ab3
                 61, 60, 58, 61,         # Bbm: Db4 C4 Bb3 Db4
                 64, 67, 60, 64]}        # C:  E4 G4 C4 E4
SOLO = {
    # a house hook on the slide: smears up into each phrase, a call and an answer, down to the low F
    'TROMBONE': [(0, 60, .5), (.5, 63, .5), (1, 65, .75), (1.75, 67, .25), (2, 68, 1.0), (3, 67, .5), (3.5, 65, .5),
                 (4, 63, .5), (4.5, 65, .5), (5, 60, 1.0), (6, 58, .5), (6.5, 60, .5), (7, 63, .5), (7.5, 65, .5),
                 (8, 68, .75), (8.75, 67, .25), (9, 65, .5), (9.5, 64, .5), (10, 65, 2.0)],
    # a strutting bass riff: octave jumps and the fifth, a chromatic walk up into the tonic
    'TUBA': [(0, 41, .5), (.5, 53, .25), (.75, 41, .25), (1, 48, .5), (1.5, 41, .5), (2, 44, .5), (2.5, 46, .5),
             (3, 48, .75), (3.75, 47, .25), (4, 46, .5), (4.5, 44, .5), (5, 41, .5), (5.5, 53, .5), (6, 48, .5),
             (6.5, 49, .5), (7, 50, .5), (7.5, 52, .5), (8, 53, 1.0), (9, 48, .5), (9.5, 41, .5), (10, 41, 2.0)],
}
TITLE_HITS = {'TROMBONE': (41, 53),      # (#41's blast: the low F + a big smear up to F3; #36 slid four notes up a fifth)
              'TUBA': (29, 41)}          # a low blast, F1 under F2 (the TUBA's default: the key's root + its octave below)
CAP_TARGETS = {'SAD TROMBONE': 12, 'OOMPAH': 12, 'POWER SLIDE': 24, 'BRASS WALL': 22, 'PEDAL TONE': 40,
               'SUBSONIC': 40}

# ---------------------------------------------------------------- the post
CAPTION = 'Comment the next match-up 👇🎺📯'
