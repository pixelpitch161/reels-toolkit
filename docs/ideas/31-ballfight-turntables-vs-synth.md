# 31. BALLFIGHT #5: Turntables vs Synth

Board row: `docs/IDEAS.md` (#31). Series rules: `docs/ballfight/RULES.md`.

## 31. BALLFIGHT #5: Turntables vs Synth — APPROVED
**Rivalry:** DJ vs producer, electronic civil war. Hook "TURNTABLES / VS SYNTH" (10 + 8). Emojis 🎧🎹.
| | TURNTABLES (lime, left) | SYNTH (cyan, right) |
|---|---|---|
| Weapon (new kind) | **Vinyl frisbees** that ricochet off the ropes (volley with a `bounce` option: up to 2 rope bounces) | **Arpeggiator satellites**: glowing notes orbiting the ball that hit what they touch (new `orbit` kind) |
| Grows per landed hit | +1 disc per landed volley (half-step, as #27) | +1 satellite (max 8) |
| Hit sound | Scratch + a chopped vocal-free stab | Arpeggio notes, one per satellite hit |
| Unlock 1 | SCRATCH (`haste`: discs every 8th for a bar) | LFO (`haste`: the orbit wobbles and speeds up) |
| Unlock 2 | REWIND (**new kind**: undoes its last 2 s of damage taken; the ball zips back along its path, VHS wobble) | SAWTOOTH (`arc`: flings the satellites outward, they return) |
| Below 25 HP | BASS DROP (`slam`) | WHITE NOISE (`wave`) |
- **Mood: dark breakbeat**: chopped breaks, a reese bass, cold pads, minor. `loop_bpm(138)` = 137.70 BPM (35 bars;
  count-in 1.74 s).
- **Flags:** REWIND must stay honest: it is a rule in the simulation (recorded damage log), never an edit. Bridge
  entries needed for TURNTABLES and SYNTH. No vocal samples (all synthesised).

## Board history (moved here from the status board, Oct 27)

- Status: review (v3) · Session: session_01JYtAwR (2026-10-24) · Output: `ballfight_turntables_vs_synth_v3.mp4` (v2: `_v2.mp4`, v1: `ballfight_turntables_vs_synth.mp4`)
- v3 delivered (VS title card intro, series rule; early intro sheet sent first): bar 0 = the shared title card (projects/ballfight/title.py, used unchanged: panel patterns registered in TT.PATTERNS, pitched sprites in TT._SPR): lime record-groove and cyan circuit-board panels, big balls, TURNTABLES / VS / SYNTH slamming in on beats 1-3 (scratch flare + organ stab, thunder crack, saw chord + supersaw sweep), doors open on beat 4; 3-2-1 on bar 1, FIGHT! at 3.49 s; top caption dropped; HUD hidden while the card is shut. Honest seed search re-run with the fight at 2 bars (rules unchanged): 6 fits in 1339, TURNTABLES 617 / SYNTH 665 of 1282 decided; seed 1339: the decks lead for 24 s, the lead swaps back and forth 28-42 s (WHITE NOISE 40.5 + BASS DROP 41.0), a 9.9 HP REWIND keeps the decks ahead to the wire, the cables' pulses take it at 50.05 s: SYNTH wins at 50.79 s with 2.8 HP (5 snaps, 3 REWINDs). Captions: SCRATCH 12.6, LFO 17.0, REWIND 21.4, SAWTOOTH 26.1, WHITE NOISE 40.5. Loop: the title slam is the frame-0 reset (doors shut over the last 0.30 s, TURNTABLES ghost on frame 0, riser into the seam). Checks: safe zone OK; subject CoC 0; loopcheck PASS render (0.6x) + mp4 (1.1x), audio step 3.2 dB; flashcheck PASS (the doors pair, worst 1/s); opening -16.4 dBFS, build +3.1 dB. No engine change (#27 / #28 untouched). v2 kept (`*_v2_backup.py`, `_v2.mp4`, seed 781). Waiting for feedback
