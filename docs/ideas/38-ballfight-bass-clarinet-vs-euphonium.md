# 38. BALLFIGHT #12: Bass Clarinet vs Euphonium

Board row: `docs/IDEAS.md` (#38). Series rules: `docs/ballfight/RULES.md`.

## 38. BALLFIGHT #12: Bass Clarinet vs Euphonium — APPROVED
**Direction (user, Oct 26):** "Draft Bass Clarinet vs Euphonium"; approved with mood A (dark drumline), colours as
proposed. All BALLFIGHT series rules apply (VS title card, 3·2·1, ability-name captions, HUD ability list, colours,
honest tuned fight, 61.0 s, loop point = file boundary) plus the PLAYBOOK's **consistent loudness from frame 0**
(`reelkit.levels` must PASS).
**Rivalry:** the two instruments nobody can name: "a black saxophone" vs "a small tuba". Unsung heroes of their
sections; every band kid who played one shows up in the comments. Post caption: "Name both without googling 😤
Comment the next match-up 👇"

| | BASS CLARINET (slate blue, left) | EUPHONIUM (amber, right) |
|---|---|---|
| Style | The only anchor-and-pivot fighter | The only artillery |
| Signature | **ENDPIN ANCHOR**: every bar it drives its floor peg into the canvas and whirls around it like a hammer-thrower, its long body smashing anything in the circle, then pulls the peg and rolls on (new kind `anchor`) | **BELL MORTAR**: the upright bell fires brass shells that arc high on the beat (every 2 beats) and land on the enemy's predicted spot; growing shadow circles show where they'll hit; splash damage (new kind `mortar`) |
| Grows per landed hit | Orbit radius | +1 shell per salvo (half a shell per hit if it snowballs, as #27's keys) |
| LV2 | SLAP TONGUE: a short, heavy knockback smack in front | MARCHING ORDERS (`haste`: shells every beat) |
| LV3 | MULTIPHONICS: two notes at once: a ghost twin orbits the opposite side of the peg (new `twin`) | IDENTITY CRISIS: copies the enemy's last-used ability (same kind and numbers) and fires it back in amber (new `copy`) |
| Below 25 HP | LIGHTS OUT: the arena lights dim around it (eased, never a flash), it sinks into shadow for a bar, then erupts with a crushing slam (new `lights_out`) | NOT A TUBA: a full barrage of shells rains across the ring |

- **Likely shape (only if the honest fight gives it):** the euphonium wants distance so shells have time to land; the
  bass clarinet wants to get close, plant and whirl; every shell telegraphs its spot, so the anchor's question is when
  it's safe to plant. IDENTITY CRISIS adds surprises (a copied SLAP TONGUE, MULTIPHONICS or even LIGHTS OUT).
- **Colours (new, in the table):** BASS CLARINET slate blue (110, 132, 170), EUPHONIUM amber (240, 172, 32). Amber sits
  between Drums' orange and Trumpet's yellow; keep it reading as brass amber (a touch deeper if it looks orange).
- **Music, mood A (dark drumline):** a minor marching-band battle cadence (snare line, quads/tenors, pitched bass
  drums, cymbal line) with low brass hits; intense, the euphonium's home turf. Both fighters are low voices: the **bass
  clarinet takes the rumbling bottom** (chalumeau growl, slap-tongue pops), the **euphonium the warm tenor melody**; the
  bed has no low brass or woodwinds of its own. `loop_bpm(126)` = 125.90 BPM (32 bars; bar = 1.906 s, FIGHT! at 3.81 s).
  Consistent loudness: the drumline is at full level on frame 0; build with density (more drums, rolls, cymbals).
  Leader fuller and louder, loser thinner and dirtier; the winner's solo over the cadence. (Not chosen: B horror
  suspense trap; C low-end funk.)
- **VS title card:** BASS CLARINET / VS / EUPHONIUM. "BASS CLARINET" (13 letters) is the longest name yet: split it
  over two lines on the card if one line would be too small, staying inside the safe zone. Hits: a bass-clarinet growl
  + slap, the thunder crack, a euphonium blast; panel patterns: keys + curved metal neck (slate-lit), upright bell +
  four valves (amber-lit).
- **Captions:** IDENTITY CRISIS (15 chars) and MULTIPHONICS (12) use the small size where needed. NOT A TUBA is the
  desperation caption. HUD names: "BASS CLARINET" may need "BASS CL." if it doesn't fit beside the bar; the list shows
  the full ability names.
- **Beat sheet (61 s):** bar 0 VS card · bar 1 count-in · FIGHT! at 3.81 s · 8–30 s the four unlocks (each named once;
  the HUD list lights on every use) · 32–44 s LIGHTS OUT or NOT A TUBA · 44–52 s final exchange · K.O. ~49–53 s (slow-mo
  shatter into keys / neck / bell / valves) · winner's solo + "COMMENT THE / NEXT MATCH-UP" · 58–61 s reassembly, doors
  shut, title slam = frame 0.
- **Flags:** both instruments are all new: no Pixel Pitch model or voice for either, so build both from reference
  photos (as the oboe was: `series/instrument-history/`, one new file each, own commits; don't edit other instruments)
  plus their synth voices and bridge entries. New engine kinds additive, own commits; all earlier episodes replay
  identically. IDENTITY CRISIS copies the real last-used ability as a rule (if the enemy hasn't used one yet, it copies
  the enemy's signature attack once). LIGHTS OUT dims slowly (eased over ~0.3 s, the arena never goes black, both balls
  and the HUD stay readable); flashcheck must pass. Mortar shells read in 3D: real arc height + a growing ground shadow.
  Re-tune for 40–60 % wins; honest seed search with `t_fight` = 2 bars and the usual hard windows. Effects from
  `reelkit.fx`; no full-screen flashes; safe-zone, flash, loop and levels checks must pass.
- **Effort:** ~75–90 min (two new models + voices, four new mechanics). Work in
  `projects/ballfight_bassclarinet_vs_euphonium/`.

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_01RYTMVg (2026-10-26) · Output: `ballfight_bassclarinet_vs_euphonium.mp4`
- v1 delivered (early sheet sent first). Both instruments new in the Pixel Pitch folder, own commits, from reference photos found in Chrome (Commons): BASS CLARINET (Yamaha YCL-622II front shot scaled to a 40 mm body + Buffet Prestige 3/4; neck loop, keys, U-bow + tilted bell, floor peg; chalumeau reed voice + slap tongue; `ball` scene radii x2) and EUPHONIUM (Yamaha YEP-321 front shot scaled with its 280 mm bell / 664 mm height + B&S 3171A side; 3 top pistons + diagonal 4th valve; warm voice; tubing x1.35); bridge entries in their own commit. Engine kinds in 5 small commits (+ a per-arm cooldown fix; all 11 finished episodes replay identically each time): weapon `anchor` (ENDPIN ANCHOR), weapon `mortar` + ability `barrage` (BELL MORTAR / NOT A TUBA), `twin` (MULTIPHONICS), `copy` (IDENTITY CRISIS: the enemy's real last ability; optional `trigger(ab=, quiet=)`), `lights_out` (eased dim, never a flash); SLAP TONGUE reuses `snip`. Title card: opt-in `TITLE_LINES` / `TITLE_BALLS` (BASS / CLARINET on two lines at scale 6; older cards pixel-identical) + both panel patterns. Tuned once at damage scale 0.68 (150 seeds 52 / 48 %, median K.O. 50.6 s). Honest search at 2 bars: 8 fits in 2,171 (BASS CLARINET 54 % / EUPHONIUM 46 % over 2,092 decided); seed **1269** (ranked 2nd; #23's 52.90 s K.O. left ~0.6 s for COMMENT): IDENTITY CRISIS copies SLAP TONGUE 32.9 s and MULTIPHONICS 42.4 s, NOT A TUBA 43.85 s puts the euphonium ahead (45.3 s), LIGHTS OUT erupts under it and swings it back (47.8 s), BASS CLARINET wins at 51.66 s with 7.9 HP. Captions SLAP TONGUE 16.2, MARCHING ORDERS 21.4, MULTIPHONICS 27.2, IDENTITY CRISIS 32.9, NOT A TUBA 43.8 (LIGHTS OUT in the HUD list: 4 s rule). Music mood A (dark drumline, C minor, 125.90 BPM, no low brass / woodwinds in the bed): bass clarinet = the rumble (8ths through every whirl, slap pops on smashes), euphonium = the tenor melody (a note per salvo). Checks: `reelkit.levels` PASS on wav + mp4 (opening +0.2 dB vs ending, spread 3.1 dB, no dips); band check bass clarinet +7.0..+9.2 dB, euphonium +7.2..+11.9 dB over the bed; no silent 0.5 s windows in either fighter's stem; safe zone OK; CoC 0 on all 1830 frames; loopcheck PASS render 0.3x / mp4 0.9x (audio 5.0 dB, no click); flashcheck PASS (1 = the doors). Replay guard #38 -> seed 1269. Waiting for feedback
