# Marble Waterfall Organ (IDEAS #18)

A glowing brass-and-glass marble machine floats on a stone plinth in a vibrant abstract void (no room,
no ground). It opens with every gate open: ~120 marble hits a second spray off every tier, ping random
notes and streak away into space. Gates click shut one by one on the downbeat; each routes its stream
onto one instrument, so a part of the song rises out of the chaos. The last (melody) gate sticks, judders,
fails twice, holds its breath for half a beat, then snaps shut: the drop. At 57 s a lever flips, every gate
springs open and the wash floods back into the opening chaos (the file loops).

Satisfying-first (science is optional flavour); the void physics is fantasy (gravity only inside the
machine, escapees drift away weightless) and no caption claims real physics.

| gate | part | instrument | shuts |
|---|---|---|---|
| 1 | kick | big drum skin (+2 toms only the spray hits) | 6 s |
| 2 | snare + hats | snare skin + 3 glass discs | 10 s |
| 3 | bass | 10 tuned pipes ("bloops") | 14 s (pickup bar on A) |
| 4 | chords | 7 glass bowls | 18 s |
| 5 | stabs | 7 tuned bars (marimba) | 20 s |
| 6 | arpeggio | 1.25-turn spiral ramp of 16 bars; one marble per bar rings all 16 on 16ths | 24 s (lift doubles speed) |
| 7 | cymbals | crash + ride discs | 32 s |
| 8 | melody | 10 golden bell bars | sticks 35 s, snaps at 40 s |

Music: original, 120 BPM, | Bm | G | D | A | from bar 8 so the drop (40 s) and the final chord (56 s) land on Bm.
Full song: ~33 marble hits/s. Opening wash: random D-major notes (each spray contact rings the element it hits).

## Build

```bash
python sim.py                     # -> marbles.npz: every marble path, every hit, per-frame element flashes
python audio.py                   # -> audio.wav (+ 2 s levels, marble-vs-bed margins, melody presence, quietest window)
python focus_check.py             # machine CoC on all 1800 frames (expect 0)
python sheet_par.py test/sheet.png 0.5 4.5 9 12 16 21 27 33 37.05 40.2 45 50 54 58.5
python -m reelkit.video render.py audio.wav marble_waterfall_organ.mp4
python flash_check.py marble_waterfall_organ.mp4
python engagement_check.py audio.wav
```

| file | what |
|---|---|
| `timeline.py` | the grid, gate times + gate motion (snap, stuck judder, lever spring), harmony, every routed note, spray rate, lift speed, captions |
| `machine.py` | the machine geometry (lift cage, crown, 8 tiers, spiral, plinth, basin, lanterns, lever) + the 59 playable elements + hop maths |
| `sim.py` | builds every marble path once (routed: chute roll -> exact ballistic hop onto the element at the note time -> bounce into the basin; spray: flap deflection -> 1-3 random contacts -> escape -> weightless drift), wraps the 60 s seam, caches flashes |
| `scene.py` | the void: gradient, chord-tinted nebula textures, 3 parallax star layers, floating crystals/rings, comets, the orb (key light) |
| `render.py` | void -> far shapes -> per-pixel plinth top (orb shadows of the whole machine, lantern/hit light, AO) -> machine + near shapes (one painter list, cached orb visibility) -> z-tested marbles -> locked DOF -> glows, trails, sparks -> god rays, bloom, grade -> HUD + captions |
| `audio.py` | one voice per element kind, every hit from `marbles.npz`, looping drone + chord pad, riser, gate clacks, lever; periodic buses, circular reverb, drop-referenced master |
| `focus_check.py`, `flash_check.py`, `engagement_check.py` | checks without / after rendering |
| `sheet_par.py` | contact sheet on 2 processes |

The orb is drawn at az 2, el 7 (so it shows beside the crown in the wide shots); shading/shadows use a slightly
higher key (az -4, el 28) so the machine's shadows actually land on the plinth top.
