# Log: `projects/ballfight_organ_vs_harp`

Append new entries at the end (newest last).

## BALLFIGHT #9: Organ vs Harp (`projects/ballfight_organ_vs_harp`)
- Ninth BALLFIGHT episode (#35), produced in parallel with #29-#34 on the shared engine. ORGAN erupts rows of organ pipes
  out of the canvas toward the harp on every second beat (new weapon kind `line`: the row runs out at 6 u/s, each pipe up
  0.55 s, one hit per row, grows its length); HARP stretches tripwire strings rope to rope across the ring (new kind
  `trap`: crossing an enemy string cuts, the owner passes freely, 1 -> 7 strings, the oldest fades). Abilities: CATHEDRAL
  (dome shield) / PULL ALL STOPS (haste) / TOCCATA (slam: a pipe forest); PLUCK (arc) / ANGEL WINGS (wings + halo shield)
  / 47 STRINGS (wave). Mood: gothic horror, D minor, 125.90 BPM (32 bars). ORGAN bridge entry = organ60_lowpoly's
  Saint-Sulpice case + reed voice (the modern "cage" organ reads as a frame in an acrylic box at ball size).
- Honest run, seed 1181 (10 fits in 1,179 runs; ORGAN 52 % / HARP 48 % over 1,042 decided): organ leads early, the harp's
  web takes over at 33.4 s, lead flips at 38.1 / 43.9 s, the organ wins on its last hits at 51.66 s with 4.5 HP.
- Engine lessons: hazards reported as events (`line`, `string`, `string_off`; hits carry the hazard id) need no new
  recorded arrays, so the change stays tiny and replays of #27 / #28 are untouched (tools/ballfight_replay_hash.py before
  and after every commit, and again after merging the other sessions' engine commits: rebases conflict in the same
  places - Fight.__init__ state, the on-beat branch, the per-substep calls, the roster tail - keep both sides).
- Rules lessons: fast, wide rows landed ~80 % and won by ~28 s; slower, thinner rows with a short start make the organ an
  early, close-range bully, and a one-string harp that grows per landed crossing makes the natural "bully vs web" story
  (~0.6-0.8 lead changes per run). Fit rate ~1 % with #27's hard windows: an early-stopping search is ~2 min.
- Loop lesson: the pearl corner light is bright, so its beat pulse moved enough of the frame to fail the seam (2.7x with
  #27's 70 ms pre-ramp); a 0.16 s pre-ramp in this episode's `beat_pulse` -> 1.3x. The balls' sprite yaw ignores the
  corner rev shake (no 6-degree yaw-step flicker at the seam).
- Sound: first balance had the organ stabs only +2-5 dB over the bed (short stabs every 2 beats read low on average);
  organ bus x2 -> +8-11 dB, harp +8-14 dB. The organ synth caches notes by (scene t0, event index): the bridge clears
  that cache per voice() call (added by #31's session), or every organ note comes back as the first one.
- Checks: safe zone OK (0.25 s steps); subject CoC 0 on all 1830 frames (pipe tops and strings included); loopcheck PASS
  render + mp4 (seam 1.7x); flash 0 general (max 7.6 % at the TOCCATA); opening -18.5 dBFS, build +3.8 dB. Render 716 s
  on 2 cores + mux (19 min total).
- v2 (BALLFIGHT v2 redesign, user Oct 24): built the shared effects kit `reelkit/fx.py` first (five small commits, pushed
  before the re-render so #29-#34 could start): Flares (point lights in the fighter's colour fed into the scene's
  Lights, so the canvas / floor passes, ropes, posts and stands light up; `tint()` for the crowd), Bursts (1-2 frame ink
  starbursts; FlashBudget keeps bright cores to 3 / s), Shake (capped 2.5 px), shimmer / fringe (local warps),
  Particles (analytic drag + gravity + wobble, trails = the same particle sampled earlier, so any frame renders alone),
  Marks (per-pixel floor marks in the caller's floor coords), Moments (eased <= 30 % dim with the fighters kept lit +
  speed lines). Everything is built once from the run's events, so patch renders match the full render.
- v2 lessons: a pearl (near-white) fighter's light whitens the canvas fast: scale its colour by ~0.9 and keep peaks
  <= 0.75; first pass at peaks 1.0-1.8 plus the old per-hit glows blew out the hit spots. The flash test on the two
  busiest stretches before the full render (render 5-7 s of frames, `flashcheck.analyse`) matched the final mp4
  (18.6 % vs 18.5 % max). Same audio (the events didn't change). Render 18 min.
- v3 (VS title card intro, series standard Oct 24): moved #30's `title.py` into `projects/ballfight/` (its own commit)
  and generalised it: the word layout comes from the names (biggest scale <= 7 that fits; #30 reproduces exactly),
  panel patterns by fighter (`PATTERNS`; other sessions have since added theirs, plus opt-in CLOSEUP / GLASS
  registries), the sprite cache in the episode's own check/. Verified by rendering 10 of #30's title frames with the
  old and new file: pixel-identical. Bar 0 = the card, bar 1 = 3-2-1, FIGHT! on bar 2; chords and sections moved a bar
  (title on the tonic, count-in on the dominant, bar 31's dominant resolves on the title slam).
- v3 seed: the search re-run with the fight at 2 bars (rules frozen) ranked seed 237 first (6 fits in 999); old caches
  (1-bar runs) moved aside first so `sim.load` couldn't return a stale run. Flash: the doors pair is the one counted
  flash (worst 1 / s), as #30. Render 17 min.
- v4 (HUD ability list, series rule Oct 25; the user asked for #35's re-render): #36's `fight_hud(abilities=...)` with
  #36's row builder (same hold conventions, so the list behaves the same in every episode). Measured the overlap first
  (screen extents of balls, pipe tops, ring corners over the whole video): the HUD top moved 466 -> 448 and cut across
  the front corner post (444-450) while the fight itself stayed above 438, so a small 8 px camera nudge (SY 288 -> 280)
  was enough; the back post top lands at 122, just under the one-line captions (100-121). Pearl (near-white) unlocked
  rows still read apart from the grey locked ones. Same audio.wav (no events or captions changed). Render 21 min.
- Feedback: (pending)
