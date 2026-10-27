# 13. Count the Thunder

Board row: `docs/IDEAS.md` (#13). Series rules: none.

## 13. Count the Thunder — APPROVED
- **Hook:** "SECONDS ÷ 5 = MI" / "SECONDS ÷ 3 = KM" (16 + 16 chars, in the first 2 s; both units for a global
  audience).
- **Science:** light arrives essentially instantly (~300,000 km/s); sound travels ~0.3 km/s. Count from flash
  to thunder: ÷ 5 for miles (NWS), ÷ 3 for km (Environment Canada). Thunder can be heard ~10 miles away (NWS)
  or 8–20 km depending on conditions (ECCC). Air absorbs high frequencies as sound travels, so distant thunder
  is low and rumbling; a strike to your side gives a sharp clap, while a channel running toward or away from
  you rumbles because its sound arrives over a stretch of time. Safety (NWS): if you can hear thunder you're
  within striking distance, there's no safe place outside, and you wait 30 min after the last thunder.
  Sources: [NWS: science of thunder](https://www.weather.gov/safety/lightning-science-thunder),
  [NWS: lightning safety](https://www.weather.gov/safety/lightning-safety-overview),
  [ECCC: how thunder sounds](https://www.canada.ca/en/environment-climate-change/services/lightning/science/how-thunder-sounds.html).
- **Music:** distance drives rhythm AND timbre. First half (real time): the count is the click track
  ("1… 2… 3…" at 60 BPM) and each thunder lands where the physics says. Back half: 120 BPM groove, one flash per
  bar on the downbeat (bright snare); thunder = the kick, delayed by the real flash-to-bang time (2 s = a bar
  late, then 1, ½, ¼, ⅛ s), walking back toward its flash and fusing on the downbeat at the overhead strike.
  The thunder's low-pass opens as it nears (rumble → clap). Rain = shaker. All synthesised and original.
- **Scene:** low-poly prairie cabin cut-away at night. Inside: warm lamp + fireplace (local lights), cat on the
  windowsill, rain streaking the glass. Moon = key light until the storm swallows it; then each flash throws
  window-frame shadows across the floor. Outside: fence posts with distance markers, a windmill spinning faster
  in the rising wind, a storm cell (anvil cloud + rain curtain) crossing from the horizon to the cabin.
- **Payoff / ending:** the overhead strike hits the barn's lightning rod: flash and clap on the same downbeat
  with the full groove. The storm passes, the moon returns, and it loops to the calm opening (one distant flash).

**Beat sheet (60 s; first-half delays in real time, distances from the on-screen rules, calc):**
| Time | Visual | Caption | Sound |
|---|---|---|---|
| 0–4 | Calm cabin; distant flash at 0.5 s; counter starts | SECONDS ÷ 5 = MI / SECONDS ÷ 3 = KM | Count clicks (60 BPM) |
| 4–8 | Counter climbs | LIGHT: INSTANT / SOUND: 1 KM / 3 S | Clicks, light rain |
| 8–12 | Thunder arrives at ~10 s | 10 S = 2 MI | Low, muffled rumble |
| 12–20 | Flash at 12 s → thunder at 19.5 s | FAR = RUMBLE / AIR EATS HIGHS | Rumble, a bit brighter |
| 20–30 | Flashes 21 → 26 s and 27 → 30 s; storm crosses the fence posts | 5 S = 1 MI · 3 S = 1 KM | Rumbles turning into claps |
| 30–35 | Moon swallowed; cat moves off the sill | HEAR THUNDER? / GO INDOORS | Groove fades in under rain |
| 35–44 | One flash per bar; thunder 2 → 1 → ½ s late; distance HUD shrinks | 0.4 MI… 0.2 MI… | Kick walks back across the bar |
| 44–50 | Thunder ¼ → ⅛ s late; window shadows flash | 100 M… 40 M… | Flams tighten; filter opens |
| 50–52 | Climax: overhead strike on the lightning rod | 0 S | Flash + clap fuse with full groove |
| 52–60 | Storm passes, moon returns; loop to opening | TRY IT: COUNT / FROM INSIDE | Rain thins to one distant rumble |

**Flags:**
- Safety: the character is indoors throughout and the ending says "from inside". Never show or imply counting
  outdoors.
- Photosensitivity: at most 3 flashes per second (we're at ~0.5/s) and no full-frame white flashes. Flash the
  sky, window and floor shadows while the warm lamps keep the room lit (also matches the "too dark" lesson).
- The rules are approximations: at 343 m/s it's ~4.7 s/mile and ~2.9 s/km, and it varies with temperature.
  Fine for captions; don't claim precision.
- Hearing range varies (NWS ~10 miles, ECCC 8–20 km): no single "max distance" number on screen.
- No stock thunder recordings: synthesise everything.

**Feasibility:** easy: lightning (jagged polyline + glow), dithered low-poly cloud blobs, rain-streak
particles, fence posts, windmill. Reuse the #7 cut-away room lessons (narrow room ~3.4 units; moon pinned in
the window as a painted backdrop). Medium: flash-lit window shadows on the floor (short key-light spike through
the window mask; cheap with the existing shadow pipeline). Audio: thunder as filtered noise (sharp N-wave
transient for close claps, long low-passed tail for far rumbles); carve the music's low end under each hit and
add a click so it reads on phone speakers (#2 lesson). Font: add "÷" as a runtime glyph from the project (like
♭ and ≈ on #2).

#### 13 — v2 rework (APPROVED, supersedes the v1 beat sheet above)
**User feedback on v1:** not engaging enough, especially at the start. Engagement brief: constantly
stimulating; open on chaos with sound from frame 0; never go quiet; build to a satisfying lock-in (the 100
Metronomes arc); exaggerate where the phenomenon is calmer.
**Audio check of v1 (rendered audio.py, 2 s windows, vs 100 Metronomes):** first 10 s ≈ −27 dBFS and ~1 hit/s
(Metronomes: −18.5 dBFS, ~4.4 hits/s); first 30 s is count ticks + 4 thunders; groove only from 30 s; climax
(−19) barely louder than the start; whole mix 6–9 dB below the other reels.

**v2 in one line:** a storm throws many strikes at different distances, so far away every thunder lands a
different number of seconds after its flash: flashes and rumbles overlap out of sync (natural chaos). As the
storm closes in, every delay shrinks and the thunder snaps onto the flashes until they fuse: chaos → lock-in,
straight from the physics.
- **Cold open on the climax:** the overhead strike on the barn rod (flash + clap fused, downpour, full groove),
  seen from inside the cabin. Then "HOW FAR WAS THAT?" rewinds the storm back to the horizon.
- **Never quiet:** rain + wind + a low-passed groove from frame 0; the bed's filter opens as the storm nears.
  The counting rides on the beat (count ticks = hats), not over silence.
- **Make the mismatch visible:** each flash spawns its sound ring crawling across the prairie (v1 already has
  these). Many strikes = many rings at once, crossing each other; each rumble sounds exactly when its ring
  reaches the cabin, so you can see why the rumbles don't match the flashes. As the storm nears, the rings
  shorten and each lands right after its own flash.
- **Lock-in (back half):** keep v1's best idea: flashes on the downbeat (snare), each thunder = the kick, late
  by 2 → 1 → ½ → ¼ → ⅛ s, fusing at the overhead strike. The overlapping rumble smear thins out as each strike's
  thunder pulls onto its own flash.
- **Ending builds, doesn't fade:** a second cell races in: "THE NEXT ONE'S CLOSER", count 3… 2… 1…, and the
  strike lands on frame 0 (the cold open) for the loop.
- **Level:** start ≥ about −20 dBFS short-term with ≥3 hits/s, build ~8 dB to the climax, and no 2 s window
  more than 12 dB below the peak except a deliberate ≤1 s tape-stop/rewind gap.

**v2 beat sheet (60 s; every displayed delay/distance stays physically consistent, ÷5 mi / ÷3 km):**
| Time | Visual | Caption | Sound |
|---|---|---|---|
| 0–3 | Cold open: overhead strike on the barn rod, downpour, cat bolts off the sill | SECONDS ÷ 5 = MI / SECONDS ÷ 3 = KM | Flash + clap fused, full groove |
| 3–5 | "Rewind": rain streaks reverse, storm sucks back to the horizon | HOW FAR / WAS THAT? | Reverse whoosh + rewind scratch |
| 5–9 | Horizon storm: many small flashes (≤2/s), rings crawling over the prairie | LIGHT: INSTANT / SOUND: 1 KM / 3 S | Rain/wind + low-passed groove; distant rumbles smear in |
| 9–15 | One flash is tagged; counter runs with its ring | COUNT IT: / 1… 2… 3… | Count ticks as hats over the groove; overlapping rumbles |
| 15–20 | Tagged ring reaches the cabin | 15 S = 3 MI / ≈ 5 KM | Its rumble lands: low, muffled |
| 20–25 | Rings cross each other; rumbles arrive "wrong" | FAR = RUMBLE / AIR EATS HIGHS | Rumble smear, brighter; filter opens a notch |
| 25–30 | Storm crosses the fence-post markers; rings shorten | IT'S COMING | Delays ~5 → 3 s; rumbles turning into claps |
| 30–35 | Moon swallowed; cat moves off the sill; flashes start landing on downbeats | HEAR THUNDER? / GO INDOORS | Groove opens to full (120 BPM) |
| 35–44 | One flash per bar; thunder 2 → 1 → ½ s late; distance HUD shrinks | 0.4 MI… 0.2 MI… | Kick walks back across the bar; smear thinning |
| 44–50 | Thunder ¼ → ⅛ s late; window shadows flash | ≈80 M… ≈40 M… | Flams tighten; low-pass fully open |
| 50–52 | Climax: overhead strike (same image as the cold open) | 0 S / RIGHT ON TOP | Flash + clap fused, everything at once |
| 52–57 | Second cell races in from the horizon | TRY IT: COUNT / FROM INSIDE | Groove keeps driving; rain swells |
| 57–60 | Count 3… 2… 1… with its ring; strike lands on frame 0 (loop) | THE NEXT ONE'S / CLOSER | Riser into the cold-open clap |

**v2 flags:**
- Safety unchanged: indoors throughout, including the cold open; ending says "from inside".
- Photosensitivity: at most 3 flashes per second overall (≤2/s in the horizon chaos), distant bolts small, no
  full-frame white flashes. Re-run v1's flash_check.py on v2.
- Strikes landing on downbeats is stylisation (real lightning isn't rhythmic), but each strike's thunder delay
  must still match its displayed distance; never caption a distance that disagrees with its delay.
- The second cell in 52–60 is a staged loop device; keep its count consistent with its distance too.
- Still synthesise all thunder (no stock recordings).

**v2 feasibility:** a re-cut, not a rebuild. Keep v1's scene.py / render.py (cabin cut-away, cat, bolts, sound
rings, HUD, window shadows) and its checks (focus_check, flash_check, audio_check). Rewrite timeline.py (a strike
list with many distances, the rewind, the second cell) and audio.py (continuous rain/wind + filtered groove from
0 s, overlapping distance-filtered rumbles, rewind and riser FX, louder master). Many simultaneous rings: cap
visible rings (~8) and fade the oldest. Keep a `*_backup.py` of v1 before changing anything.

## Board history (moved here from the status board, Oct 27)

- Status: review (v2) · Session: session_01QAsUpz (v1); session_016ZWSrN (v2, 2026-10-21) · Output: `count_the_thunder_v2.mp4` (v1: `count_the_thunder.mp4`)
- v1 feedback: not engaging enough at the start. v2 delivered (v1 kept as `*_backup.py`): cold open on the rod strike, rewind, ~1 flash/s storm chaos with out-of-step rumbles + tagged 15 s count, lock-in, second cell 3/2/1 s into frame 0. Audio: -12.1 LUFS (v1 -18), chaos -17…-19 dBFS -> climax -12, >=3 hits/s in all but 3 of 30 windows (2-2.5); flash check 0 general flashes; focus 0 px on all frames. Next: user feedback
