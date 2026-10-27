# 29. BALLFIGHT #3: Trumpet vs Sax

Board row: `docs/IDEAS.md` (#29). Series rules: `docs/ballfight/RULES.md`.

## 29. BALLFIGHT #3: Trumpet vs Sax — APPROVED
**Rivalry:** jazz's oldest argument: who's the lead horn. Hook "TRUMPET / VS SAX" (7 + 6). Post caption emojis 🎺🎷.
| | TRUMPET (yellow, left) | SAX (teal, right) |
|---|---|---|
| Weapon (new kind) | **Beam**: charges for a beat (bell glows), then fires a straight piercing blast | **Smoke rings**: wobbly rings that curve toward the enemy (volley with a `curve` option) |
| Grows per landed hit | Beam length + width | +1 ring per landed volley (half a ring, as #27's keys, if it snowballs) |
| Hit sound | Bright stab, higher with each hit | Breathy growl notes, bluesy bends |
| Unlock 1 | FANFARE (`shockwave`) | SMOKE SCREEN (`shield`) |
| Unlock 2 | DOUBLE TONGUE (`haste`: fires twice as often) | BEBOP RUN (`arc`) |
| Below 25 HP | HIGH C (`wave`: a piercing ring-wide blast) | HONK (`slam`: a bell blast at the predicted spot) |
- **Mood: noir jazz**, fast and smoky: minor, walking bass, brushed drums, muted piano comping. `loop_bpm(132)` =
  133.77 BPM (34 bars; count-in 1.79 s).

## Board history (moved here from the status board, Oct 27)

- Status: review (v3) · Session: session_012TuUcx (2026-10-24) · Output: `ballfight_trumpet_vs_sax.mp4` (v3)
- v3 delivered (BALLFIGHT v2 redesign, early sheet sent first): TRUMPET VALVE COMBO (new kind `combo`: 3 pistons punch out 1-2-3 on the 8ths, steam; reach grows) vs SAX STRAP FLAIL (new kind `flail`: momentum swing, wraps + reels in + yanks, smoke trail; strap grows); HIGH C = new ability kind `beam` (v2's bell blast as the finisher, heat shimmer), BEBOP RUN now haste, FANFARE banners, HONK smoke-ring crater; effects on reelkit.fx. Engine kinds in 3 small commits + roster commit; #27 seed 88 / #28 seed 568 and every guarded episode replay identically; guard entry for #29 -> seed 191. Re-tuned (100 seeds): TRUMPET 47 % / SAX 53 %, median K.O. 50.1 s. New honest seed 191 (6 fits in 604): sax leads most of the way, HIGH C lands 43.1 s and puts the trumpet ahead, sax retakes 45.4 s (HONK 43.5), SAX wins 49.16 s with 8.6 HP. Captions: SMOKE SCREEN 10.8, FANFARE 17.5, BEBOP RUN 23.8, DOUBLE TONGUE 29.2, HIGH C 42.6 (HONK in the status line). Music: combo = 3 notes of one valve fingering; flail half turns play an original blues line. Checks: safe zone OK; CoC 0 all frames; loopcheck PASS render + mp4 (seam 1.7x); flash 0 general (peak 16.5 %); opening -16.8 dBFS, build +3.4 dB. v1/v2 history: v1 beam vs smoke rings (seed 337); v2 trumpet model rebuilt + blast out of the bell. Waiting for feedback
