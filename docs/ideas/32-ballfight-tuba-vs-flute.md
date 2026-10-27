# 32. BALLFIGHT #6: Tuba vs Flute

Board row: `docs/IDEAS.md` (#32). Series rules: `docs/ballfight/RULES.md`.

## 32. BALLFIGHT #6: Tuba vs Flute — APPROVED
**Rivalry:** biggest vs smallest, lowest vs highest. Hook "TUBA / VS FLUTE" (4 + 8). Emojis 📯🪈.
| | TUBA (indigo, left) | FLUTE (silver, right) |
|---|---|---|
| Weapon (new kind) | **Big pulse + gravity**: a slow, huge shockwave every bar (#28's `pulse` with `every` in bars), and a constant gentle **pull** toward the tuba (new `pull` kind) | **Darts**: rapid tiny fast projectiles every beat, low damage (volley tuned fast and light) |
| Grows per landed hit | Pulse radius + pull strength | +1 dart per landed volley (half-step) |
| Hit sound | Deep oom notes (the bass line) | Quick flutter notes, high register |
| Unlock 1 | OOMPAH (`haste`: pulses every 2 beats) | FLUTTER TONGUE (`haste`) |
| Unlock 2 | BRASS WALL (`shield`) | TRILL (`arc`) |
| Below 25 HP | SUBSONIC (`slam`) | PICCOLO (`wave`: a piercing high ring-wide blast) |
- **Mood: villain march**: heavy minor march, snare ostinato, low brass bed (the tuba fighter supplies the bass), a
  sinister music-box counter-line. `loop_bpm(126)` = 125.90 BPM (32 bars; count-in 1.91 s).
- **Flag:** same ball size and speed for both (fairness, as #27); "biggest vs smallest" lives in the weapons and the
  sound, not in the physics.

## Board history (moved here from the status board, Oct 27)

- Status: review (v3) · Session: session_01T3eDfB (2026-10-24) · Output: `ballfight_tuba_vs_flute_v3.mp4` (v2: `_v2.mp4`, v1: `_v1.mp4`)
- v3 delivered (VS title card intro, series rule; early intro sheet sent first): bar 0 = the shared title card (projects/ballfight/title.py; added opt-in CLOSEUP / GLASS registries for the pixel-art flute, #30 pixel-identical): indigo bell-ring and steel-silver key-cup panels, big balls, TUBA / VS / FLUTE slamming in on beats 1-3 over unison band hits (tuba blast, thunder crack, flute flurry), doors open on beat 4; 3-2-1 on bar 1, FIGHT! at 3.81 s; top caption dropped; HUD hidden while the card is shut. Honest seed search re-run with the fight at 2 bars (rules unchanged): 6 fits in 753, TUBA 53 % / FLUTE 47 % over 586 decided; seed 514 ranked first again (its run at 2 bars matches v2 event for event, one bar later): FLUTE wins at 51.28 s with 5.7 HP. Loop: the title slam is the frame-0 reset (doors shut over the last 0.30 s, TUBA ghost on frame 0, riser + doors whoosh). Checks: safe zone OK; subject CoC 0; loopcheck PASS render (0.4x) + mp4 (1.1x), audio step 2.0 dB; flashcheck PASS (the doors pair, worst 1/s); opening -17.1 dBFS, build +3.7 dB. v2 kept (`*_v2_backup.py`, `_v2.mp4`). Waiting for feedback
