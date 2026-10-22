# Fireflies That Sync (IDEAS #15)

*Photinus carolinus* in the Great Smoky Mountains (Elkmont): each male flashes 5-8 times ~0.55 s apart, then
goes dark ~8 s. A lone male has no clear rhythm; synchrony appears at ~20 males, each watching its
neighbours. In big swarms each burst rolls through the forest at ~0.5 m/s (the signal moves, not the
insects). Females, much dimmer, answer from the ground in the males' dark pauses.

How the science drives the music (one flash schedule in `sim.py` drives picture, HUD and sound):
- **Every male flash is a bell note** at that fly's own D-major-pentatonic pitch (left = low, right = high),
  at the exact flash time. The opening's ~60 random flashers = a chaotic sparkle (~60 notes/s).
- **Lock-in = smear -> flam -> unison:** a Kuramoto mean-field model in the frame of the beat grid. Over
  burst A the 20 flies' spread shrinks 274 -> 203 -> 126 -> 67 -> 32 -> 14 -> 8 ms, so you hear their notes
  converge into one chord. A soft pulse rises with the model's coherence.
- **Tempo from the flash spacing:** 0.55 s = 109 BPM (calc). A burst of 8 flashes = 2 bars; a dark pause of
  16 beats = 4 bars (8.8 s, captioned "~8 S"). Bursts land on downbeats: 12.19 (A), 21.00, 34.21, 47.42 s.
  The video is exactly 109 beats.
- **The wave:** in burst 2 each fly flashes while the burst front (moving +x at 0.5 m/s) is within 0.69 m
  (5 flashes), so a lit band slides left -> right: the chord climbs and pans, doubled by an arpeggio sweep.
  In the climax two fronts converge on the hero and meet on beat 4; the valley (2500 flies) erupts.
- **Females answer** with soft kalimba notes in the dark pauses (quantised to 8ths); the groove holds the
  pauses (the build) and each burst is the drop.

Build order:

```bash
python sim.py                   # -> sim.npz (per-frame flash intensities + event lists); prints lock-in spread
python audio.py                 # -> audio.wav + stems/ ; prints section levels
python audio_check.py           # 2 s windows, bells vs music (1-5 kHz), female answers vs music, loop seam
python focus_check.py           # hero CoC = 0 on all 1800 frames (no rendering)
python -m reelkit.sheet render.py test/sheet.png 0.6 7.06 10 14.95 22.1 28.16 35.31 37.51 45.6 50.17 53.4 59.9
python -m reelkit.video render.py audio.wav fireflies_sync.mp4
python flash_check.py --mp4 fireflies_sync.mp4      # photosensitivity (general-flash test)
```

| file | what |
|---|---|
| `timeline.py` | single source of times: 109 BPM grid, bursts, phases, chords, captions, HUD window |
| `scene.py` | cove forest: 46 tulip poplars, creek + mossy log + stones, mossy boulders, rhododendron, 120 ferns, red-lit trail with seated visitors, barred owl, meadow trees, three ridge layers, stars; moon at az -2 / el 11 |
| `sim.py` | populations (hero, 19 neighbours, opening flashers, newcomers rising from ferns, near swarm, valley, females), random flashing, Kuramoto lock-in, wave bursts, female answers, per-frame intensities |
| `render.py` | log-distance camera with hero tracking, per-pixel leaf litter (moon shadow stripes, creek glints, red pools, splatted flash light), objects, low-poly *P. carolinus* hero for the macro, locked DOF, flashes as glows or octagonal bokeh, HUD raster + pips + dark bar |
| `audio.py` | bells, kalimba answers, drone, pads, groove, bass, arps, melody, risers; tail folded onto the start |
| `audio_check.py`, `focus_check.py`, `flash_check.py` | the verification passes |

Stylisation (as flagged on the board): the real display is in full darkness; the moonlit look keeps the
playbook's lit level. The first gap (16.6-21 s) is a short 8-beat gap so the first drop lands at 21 s; every
gap after it is the full 16 beats. Flash glows use a perceptual curve (I^0.55) so a 100-150 ms flash reads at
30 fps; flash timing is unchanged. The valley erupting on one beat is stylised (at 0.5 m/s distant parts of a
real valley sit at different points of their bursts). Female answer timing is quantised to the beat.
Responsible viewing: nobody catches fireflies; visitors' lights are dim, red and pointed at the ground.

Sources: [NPS: Synchronous Fireflies](https://www.nps.gov/grsm/learn/nature/fireflies.htm),
[Sarfati et al. 2021, Science Advances](https://pmc.ncbi.nlm.nih.gov/articles/PMC8262802),
[CU Boulder (Sarfati et al. 2020)](https://www.colorado.edu/today/synchronous-fireflies),
[NPS 2025 lottery release](https://home.nps.gov/grsm/learn/news/synchronous-firefly-lottery-and-viewing-dates-for-2025.htm).
