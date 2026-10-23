"""Grieg, *In the Hall of the Mountain King* (Peer Gynt, Op. 23 1875 / Suite No. 1 Op. 46 No. 4, 1888).
The composition is public domain (Grieg died 1907).

Transcribed for this reel from Grieg's own piano arrangement in the public-domain (CC0) Mutopia engraving
"Mutopia-2013/12/07-1888" (typeset by Coyau from The University Society, New York, 1918, plate 1136-4-CB;
scan: IMSLP #00581), parsed note by note from its LilyPond source and re-written here in our own format.
No recording or modern arrangement was used. Bar numbers below refer to that engraving (bar 0 = the horn
fermata).

Format: each phrase is a list of (pitch, duration in eighths). Pitches are MIDI numbers in a reference
register (A starts on B2 = 47; B starts on F#3 = 54, "up a fifth", as in bars 13-16). The arrangement
transposes by whole octaves only.

Source checks
- Phrase A = bars 1-4 / 5-8 (melody B C# D E F# D F# | E# C# E# E C E | B C# D E F# D F# B | A F# D F# A).
- Phrase B = bars 9-12 / 13-16 (F# G# A# B C# A# C# | D A# D C# A# C# | ... | D A# D C#).
- Phrase B# = bars 61-64 (stretto, second time): D# replaces D (both hands in bar 62; the left hand also
  has D# in bar 64).
- The engraving's right hand in bars 63-64 is note-for-note bars 59-60 (F#+G natural, D natural) while its
  left hand in bar 64 has D#: a copy-paste slip. We follow the left hand (D# in bar 4 of B#) and use G#
  in the run, as in all six other statements of phrase B (bars 9-16, 33-40, 57, 61).
  (`python verify_score.py` re-parses the engraving and diffs it against these lists.)
- Coda = bars 73-76: (rest) grace G# A A# -> B over D, twice; then B C# D E F# D F# B | A# F# A# C# | B.
- Bass (left hand, quarters): A bars B-F#-B-F# x3 then D-A-D-A (bars 1-8); B bars F#-C#-F#-C# |
  D-A#-F#-C# (bars 9-16). Off-beat chords: bars 29-40 (right hand, lower voice).
- Piu vivo left hand (bars 49-72, eighths in octaves): A bars B A G F# (x2), last bar D C# B A (x2);
  B bars F# E D C# (x2) | D C# D E F# E D C#; B# bars with E#/D# (F# E# D# C# ...).
- Tempo marks: Alla marcia e molto marcato (q = 138 in the engraving), Piu vivo (q = 154) at bar 49,
  "sempre stretto sin al fine" at bar 57. Our 60 s cut starts mid-build and accelerates 152 -> 265.
"""

B2, C3, Cs3, D3, Ds3, E3, Es3, F3, Fs3, G3, Gs3, A3, As3, B3 = 47, 48, 49, 50, 51, 52, 53, 53, 54, 55, 56, 57, 58, 59
Fs2, Gs2, As2 = 42, 44, 46
Cs4, D4, Ds4 = 61, 62, 63

# ---------------------------------------------------------------- melody (bars 5-8, 13-16, 61-64)
A = [  # B C# D E F# D F#(q) | E# C# E#(q) E C E(q) | B C# D E F# D F# B | A F# D F# A(h)
    (B2, 1), (Cs3, 1), (D3, 1), (E3, 1), (Fs3, 1), (D3, 1), (Fs3, 2),
    (Es3, 1), (Cs3, 1), (Es3, 2), (E3, 1), (C3, 1), (E3, 2),
    (B2, 1), (Cs3, 1), (D3, 1), (E3, 1), (Fs3, 1), (D3, 1), (Fs3, 1), (B3, 1),
    (A3, 1), (Fs3, 1), (D3, 1), (Fs3, 1), (A3, 4),
]
B = [  # F# G# A# B C# A# C#(q) | D A# D(q) C# A# C#(q) | F# G# A# B C# A# C#(q) | D A# D(q) C#(h)  (up a fifth)
    (Fs3, 1), (Gs3, 1), (As3, 1), (B3, 1), (Cs4, 1), (As3, 1), (Cs4, 2),
    (D4, 1), (As3, 1), (D4, 2), (Cs4, 1), (As3, 1), (Cs4, 2),
    (Fs3, 1), (Gs3, 1), (As3, 1), (B3, 1), (Cs4, 1), (As3, 1), (Cs4, 2),
    (D4, 1), (As3, 1), (D4, 2), (Cs4, 4),
]
BS = [(Ds4 if p == D4 else p, d) for p, d in B]       # bars 61-64: D# for D
# coda (bars 73-76): rest, stab, rest | rest, stab, rest | run | A# F# A# C# | final B
# None = rest. 'stab' notes carry the G# A A# grace notes in the arrangement.
CODA = [
    (None, 2), (B3, 2), (None, 4),
    (None, 2), (B3, 2), (None, 4),
    (B2, 1), (Cs3, 1), (D3, 1), (E3, 1), (Fs3, 1), (D3, 1), (Fs3, 1), (B3, 1),
    (As3, 1), (Fs3, 1), (As3, 1), (Cs4, 1), (B3, 4),
]
CODA_STABS = (1, 4)            # indices in CODA of the two stabs (grace notes G# A A#)
GRACE = (Gs3, A3, As3)

PHRASES = {'A': A, 'B': B, 'B#': BS, 'coda': CODA}

# ---------------------------------------------------------------- accompaniment (per bar of a phrase)
# bass in quarters (reference octave B1 = 35)
BASS = {
    'A': [[35, 42, 35, 42]] * 3 + [[38, 45, 38, 45]],
    'B': [[42, 49, 42, 49], [38, 46, 42, 49], [42, 49, 42, 49], [38, 46, 42, 49]],
}
BASS['B#'] = [[42, 49, 42, 49], [39, 46, 42, 49], [42, 49, 42, 49], [39, 46, 42, 49]]
# piu vivo running bass, eighths (reference octave B2)
RUN = {
    'A': [[47, 45, 43, 42] * 2] * 3 + [[50, 49, 47, 45] * 2],
    'B': [[54, 52, 50, 49] * 2, [50, 49, 50, 52, 54, 52, 50, 49]] * 2,
    'B#': [[54, 53, 51, 49] * 2, [51, 49, 51, 53, 54, 53, 51, 49]] * 2,
}
# harmony per half bar (chord tones for off-beat stabs / pads / brass): two entries per bar
HARM = {
    'A': [[47, 50, 54]] * 6 + [[50, 54, 57]] * 2,
    'B': [[54, 58, 61], [54, 58, 61], [50, 54, 58], [54, 58, 61]] * 2,
    'B#': [[54, 58, 61], [54, 58, 61], [51, 54, 58], [54, 58, 61]] * 2,
}
# coda chords (bars 73-76, left hand): B octave + D E# G# B on the stabs; final B minor
STAB_CHORD = [35, 47, 50, 53, 56, 59]
FINAL_CHORD = [23, 35, 47, 50, 54, 59, 62, 66, 71]


def phrase_len(name):
    return sum(d for _, d in PHRASES[name])        # in eighths
