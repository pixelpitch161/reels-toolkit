# 40. BALLFIGHT #14: Bari Sax vs Bass Clarinet

Board row: `docs/IDEAS.md` (#40). Series rules: `docs/ballfight/RULES.md`.

## 40. BALLFIGHT #14: Bari Sax vs Bass Clarinet — APPROVED
**Direction (user, Oct 26):** "Draft Bari Sax vs Bass Clarinet"; approved with mood A (industrial construction site). A
crossover: BARI SAX as built in #39 v2, BASS CLARINET as built in #38. All BALLFIGHT series rules (VS title card, 3·2·1,
ability-name captions, HUD ability list, colours, honest fight, 61.0 s, loop point = file boundary), the **crossover
rules** (#36: fighters exactly as in `fighters.py`; the only per-episode lever is a per-fighter damage handicap tuned to
40–60 % wins; flag instead of changing a fighter; all earlier episodes replay identically) and the PLAYBOOK's
consistent loudness from frame 0 (`reelkit.levels` must PASS).
**Rivalry:** the real one vs the lookalike: the bass clarinet keeps getting called "the black saxophone" (same curved
metal neck, upturned bell, low growl). Post caption: "Clarinet or sax? Settle it 👇🎷"

| | BARI SAX (coral, left), from #39 v2 | BASS CLARINET (slate blue, right), from #38 |
|---|---|---|
| Signature | WRECKING BALL: every other bar a crane cable hooks it and it swings across the ring as a pendulum | ENDPIN ANCHOR: every bar it plants its floor peg and whirls around it like a hammer-thrower |
| Grows per landed hit | Cable length (wider, harder swings) | Orbit radius |
| LV2 | GROWL (gravelly shockwave) | SLAP TONGUE (heavy knockback smack) |
| LV3 | SKANK (a swing every bar, on the off-beats) | MULTIPHONICS (a ghost twin whirls on the opposite side) |
| Below 25 HP | CABLE SNAP (crashes down from the rafters onto the enemy's spot) | LIGHTS OUT (the arena dims, it sinks into shadow, then erupts) |

- **Likely shape (only if the honest fight gives it):** swing vs swing, one from the sky, one around a peg in the
  floor. Planting is a gamble: the bass clarinet can't move while it whirls (a target for the wrecking ball), but its
  whirl circle can knock the swinging bari off course. Dream finish: CABLE SNAP landing in the dark during LIGHTS OUT.
- **Sides:** first named = left, so BARI SAX left, BASS CLARINET right: the reverse of their own episodes. Swap the
  roster's default corners for this episode only (don't edit the roster's `corner` values; pass the sides from the
  timeline, or add an opt-in per-episode side override to the engine in its own commit).
- **Music, mood A (industrial construction site):** clanging metal percussion (anvils, pipes, sheet-metal hits),
  hydraulic hisses and air-wrench bursts as fills, a distorted minor bass groove, dark and driving; new to the series
  and matching the crane + wrecking ball. Both fighters are low voices: **the bari plays the honking bass line** on its
  swings (root on the swing, as in #39 v2), **the bass clarinet the growling counter-line an octave up** (chalumeau
  growl + slap pops); the bed has no bass instrument of its own. `loop_bpm(126)` = 125.90 BPM (32 bars; bar = 1.906 s,
  FIGHT! at 3.81 s). Consistent loudness from frame 0 (full groove on the title card; build with density). Leader
  fuller and louder, loser thinner and dirtier; the winner's solo over the machinery. (Not chosen: B dark disco; C
  horror suspense trap.)
- **VS title card:** BARI SAX / VS / BASS CLARINET (BASS CLARINET over two lines if needed, as #38 did): a bari honk, the
  thunder crack, a bass-clarinet growl + slap; existing panel patterns from #38 / #39.
- **Beat sheet (61 s):** bar 0 VS card · bar 1 count-in · FIGHT! at 3.81 s · 8–30 s the four unlocks (each named once;
  the HUD list lights on every use) · 32–44 s CABLE SNAP or LIGHTS OUT · 44–52 s final exchange · K.O. ~49–53 s (slow-mo
  shatter) · winner's solo + "COMMENT THE / NEXT MATCH-UP" · 58–61 s reassembly, doors shut, title slam = frame 0.
- **Flags:** honest seed search with `t_fight` = 2 bars and the usual hard windows. Both fighters swing in big arcs:
  keep them readable apart (coral cable + smear from above vs slate peg + orbit ring on the floor). LIGHTS OUT's dim
  must keep the cable and both balls readable; flashcheck must pass. Effects from `reelkit.fx`; safe-zone, flash, loop
  and levels checks must pass.
- **Effort:** low (~40–50 min): no new attacks, models or voices; the side swap, tuning + seed search, and the new music.
  Work in `projects/ballfight_barisax_vs_bassclarinet/`.

## Board history (moved here from the status board, Oct 27)

- Status: review (v2) · Session: session_01CzTfqe (2026-10-26/27) · Output: `ballfight_barisax_vs_bassclarinet.mp4` (v1: `_v1.mp4`, commit 8115d67)
- v2 delivered (user, Oct 27: bari model completely smooth; let it swing out over the ropes for a full swing; intro a little loud; stills sent first). Model v2 (567ced7, Pixel Pitch folder, from the Amati reference in Chrome): the keys had been buried inside the fattened sprite body; now cups with dark pad rings, pearls, rods, guards with diamond feet, bands, bow guard, engraving (all faces; voice identical). Engine opt-in `overhang` + `return` (df67dcd), the bari's roster standard (45c9190; #39 pinned off, all 70 earlier runs identical): ends may go 1.3 past the ring, never past a corner post; full swings 35 -> 76 %. Re-tuned x0.30 / x0.66 (48/52 %), new honest seed 128: hoisted out over the ropes on 11 of 15 swings, slung back in at 49.2 s, LIGHTS OUT + CABLE SNAP both land, bari wins at 50.80 s on 0.7 HP. Ropes pressed under the passing ball, springing back. Intro thinned by arrangement: K-weighted -1.4 LU vs the fight (card was +0.7). Checks: levels PASS (wav + mp4), band OK, safe zone OK, CoC 0 (sharp 0.26), flash PASS, loopcheck PASS (mp4 1.1x). Next: user feedback
