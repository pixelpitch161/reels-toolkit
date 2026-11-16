# Nine Studio themes (2026-11-16)

Aedan approved all nine themes proposed in https://claude.ai/artifact/8yQaWfbpHYwqDrLa9RkFTQ: they fill the thin
categories (Rock & Punk 3 to 6, Folk & World 4 to 7) and give airy, high and bright-brass fighters themes made for
them. 26 themes become 35.

| Theme | Category | Tempo | Key | Made for |
|---|---|---|---|---|
| surf_rock | Rock & Punk | 150-168 | E minor | ELECTRIC v UKULELE |
| pep_band_punk | Rock & Punk | 150-168 | A major | MELLOPHONE v SNARE DRUM |
| drumline_metal | Rock & Punk | 132-160 | D minor | TIMPANI v BASS DRUM |
| celtic_reel | Folk & World | 112-132 | D mixolydian | FLUTE v BAGPIPES |
| mariachi | Folk & World | 130-160 | G major | TRUMPET v MELLOPHONE |
| highlife | Folk & World | 112-128 | C major | CORNET v SOUSAPHONE |
| samba_bateria | Jazz & Latin | 98-112 | A harmonic minor | MARIMBA v SNARE DRUM |
| heroic_fanfare | Cinematic & Dark | 100-128 | Bb major | TRUMPET v HORN |
| liquid_dnb | Electronic & Club | 160-168 | F minor | FLUTE v ALTO SAX |

Approach (builder's call, for speed): these themes have no kit mood to reproduce, so they share one frame
(`bf-content/src/themes/fresh`) that plays the classic bed's sections (VS card, count-in, FIGHT!, K.O., stinger, the
bars after the K.O., the build and all five hooks) with the theme's own `drums`, `bass` and `chords` layers. Each
theme is one file: its card, preset, layers (built from the existing themes' instruments) and two tables. Classic is
a layered bed on a plain steady table with no story cues; Composed is the full cued table. Every Studio theme has all
five hooks, so every pair is offered them. The DnB and punk tempos stay at or under 168 bpm (43 bars), the most the
existing bar ranges allow. Levels were balanced by stem loudness against the fighters', "close enough".

Tests: `bf-harness/tests/studio_themes.rs` (both arrangements of each theme play a whole song for its own pair and
pass the levels check), plus the count updates in the registry, starters and app tests.
