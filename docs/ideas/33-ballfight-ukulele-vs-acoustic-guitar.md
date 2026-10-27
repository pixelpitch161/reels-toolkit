# 33. BALLFIGHT #7: Ukulele vs Acoustic Guitar

Board row: `docs/IDEAS.md` (#33). Series rules: `docs/ballfight/RULES.md`.

## 33. BALLFIGHT #7: Ukulele vs Acoustic Guitar — APPROVED
**Rivalry:** little brother vs big brother. Hook "UKULELE / VS ACOUSTIC" (7 + 11). Emojis 🪕🎸. Call the second
fighter ACOUSTIC everywhere (HUD, captions) so it never reads as #27's GUITAR.
| | UKULELE (pink, left) | ACOUSTIC (wood tan, right) |
|---|---|---|
| Weapon (new kind) | **Body-slam dash**: every 2 beats it dashes at the enemy, damage on contact (new `dash` kind) | **Sound-hole blast**: a strummed cone of sound in front of it, aimed at the enemy, on the beat (new `cone` kind) |
| Grows per landed hit | Dash speed + distance | Cone length |
| Hit sound | Bright uke strums | Big open-chord strums |
| Unlock 1 | CHUNK (`shockwave`: the percussive chunk strum) | CAPO (`haste`: cone fires twice as often) |
| Unlock 2 | SPEED STRUM (`haste`: dashes every beat) | FINGERPICK (`arc`: a fan of string darts) |
| Below 25 HP | UKE-NADO (`wave`: spins into a tornado) | BIG STRUM (`slam`) |
- **Mood: spaghetti-western standoff**: galloping rhythm, whistled-style lead (synth), twangy baritone guitar bed
  kept below both fighters, minor. `loop_bpm(130)` = 129.84 BPM (33 bars; count-in 1.85 s).
- **Flags:** ACOUSTIC needs a bridge entry (`acoustic_lowpoly`). The bed's guitar must not mask the two fighters
  (band-level check).

## Board history (moved here from the status board, Oct 27)

- Status: review (v3) · Session: session_017mWr3z (2026-10-24) · Output: `ballfight_ukulele_vs_acoustic_v3.mp4` (v2: `_v2.mp4`, v1: `_v1.mp4`)
- v3 delivered (VS title card intro, series standard; early intro sheet sent first): bar 0 = the shared projects/ballfight/title.py card (taken over from #30 by the #35 session's move; added UKULELE surf-crest / ACOUSTIC wood-grain + six-string panel patterns): UKULELE slams in on frame 0 with a uke chord blast, VS cracks the divider with a thunder crack, ACOUSTIC on beat 3 with an open-chord blast, doors open on beat 4; 3-2-1 on bar 1, FIGHT! at 3.70 s, top caption dropped; chords re-laid (card on Am, count-in on E). Seed 1248 did not replay at 2 bars (K.O. 54.2 s), so the honest search was re-run (rules unchanged): new seed 1130 (6 fits in 2170; UKULELE 53 % / ACOUSTIC 47 % over 1779 decided): uke leads early, acoustic leads 35.6-48.7 s, UKE-NADO 44.4 s (captioned) and BIG STRUM 45.8 s trade, UKULELE wins at 50.97 s with 4.3 HP; replay guard on 1130. Loop: the title slam is the frame-0 reset (doors shut over the last 0.30 s + riser + whoosh). Checks: safe zone OK; CoC 0; loopcheck PASS render (0.9x) + mp4 (1.5x), audio step 2.7 dB, no click; flashcheck PASS (1 doors pair, worst 1/s); opening -19.3 dBFS, build +3.3 dB; fighters 8-14 dB over the bed. Waiting for feedback
