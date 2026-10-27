# 35. BALLFIGHT #9: Organ vs Harp

Board row: `docs/IDEAS.md` (#35). Series rules: `docs/ballfight/RULES.md`.

## 35. BALLFIGHT #9: Organ vs Harp — APPROVED
**Rivalry:** cathedral vs angels: heaven's instruments fight. Hook "ORGAN / VS HARP" (5 + 7). Emojis ⛪😇 (or 🎹🪉).
| | ORGAN (wine, left) | HARP (pearl, right) |
|---|---|---|
| Weapon (new kind) | **Erupting pipes**: on the beat a line of organ pipes bursts up from the floor from the organ toward the enemy (new `line` hazard) | **Tripwire strings**: glowing strings stretched across the ring that hurt whoever crosses them; the harp is immune (new `trap` kind; older strings fade) |
| Grows per landed hit | Line length | +1 string (max 7) |
| Hit sound | Big organ chords | Plucked arpeggios |
| Unlock 1 | CATHEDRAL (`shield`: an echoing dome) | PLUCK (`arc`: a fan of darts) |
| Unlock 2 | PULL ALL STOPS (`haste`) | ANGEL WINGS (`shield`) |
| Below 25 HP | TOCCATA (`slam`: a forest of pipes erupts) | 47 STRINGS (`wave`: every string on a concert harp snaps across the ring) |
- **Mood: gothic horror**: minor, choir "aah" pad, tolling bell, slow-burn tension into a pounding back half.
  `loop_bpm(126)` = 125.90 BPM (32 bars; count-in 1.91 s).
- **Flags:** ORGAN needs a bridge entry (`organ60_lowpoly`). Organ chords are sustained: keep its hits short stabs,
  not held chords, so the bed doesn't turn to mush.

## Board history (moved here from the status board, Oct 27)

- Status: review (v4) · Session: session_011cXc3S (2026-10-24) · Output: `ballfight_organ_vs_harp_v4.mp4` (v3: `_v3.mp4`, v2: `_v2.mp4`, v1: `ballfight_organ_vs_harp.mp4`)
- v4 delivered (HUD ability list, series rule Oct 25; early sheet + HUD close-ups sent first): same fight (seed 237, replays identically on the latest engine), title card and soundtrack; `fight_hud` abilities (#36) list ORGAN's and HARP's LV2 / LV3 / LOW under the bars: grey until unlocked (glow sweep), the fighter's colour after, lit on a pill on every use (CATHEDRAL / PULL ALL STOPS / ANGEL WINGS held while active, TOCCATA until it lands), back to grey on the glide home; no LV pips or status line. The taller HUD (top y 448) reached the front corner post, so the ring is nudged up 8 px (camera SY 288 -> 280): the fight stays >= 14 px clear of it. Checks: safezone OK (0.25 s steps); subject CoC 0 on all 1830 frames; loopcheck PASS render (0.3x) + mp4 (0.9x), audio step 3.1 dB; flashcheck PASS (the one doors pair, worst 1 / s). v3 kept (`render_v3_backup.py`, `_v3.mp4`). Waiting for feedback
