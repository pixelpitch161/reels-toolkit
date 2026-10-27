# 30. BALLFIGHT #4: Bagpipes vs Accordion

Board row: `docs/IDEAS.md` (#30). Series rules: `docs/ballfight/RULES.md`.

## 30. BALLFIGHT #4: Bagpipes vs Accordion — APPROVED
**Rivalry:** the "most hated instrument" final, pure comedy bait. Hook "BAGPIPES / VS ACCORDION" (8 + 12). Emojis 🏴󠁧󠁢󠁳󠁣󠁴󠁿🪗.
| | BAGPIPES (tartan green, left) | ACCORDION (magenta, right) |
|---|---|---|
| Weapon (new kind) | **Drone aura**: a ring around the ball that hurts while the enemy is inside it | **Bellows**: on alternate bars a blast that pushes the enemy away, then a suction that pulls it in |
| Grows per landed hit | Aura radius | Bellows reach + force |
| Hit sound | The drone swells; chanter grace-note squeals | Wheezy chord stabs on push and pull |
| Unlock 1 | WAR CRY (`shockwave`) | POLKA (`haste`: push/pull every beat) |
| Unlock 2 | DOUBLE DRONE (`haste`: aura damage x2) | SQUEEZEBOX (`shield`: folds shut) |
| Below 25 HP | BAG BURST (`wave`: the bag deflates in one blast) | FULL BELLOWS (`wave` or `slam`; differ from BAG BURST on screen) |
- **Mood: dark sea shanty**: stomping minor shanty, low male-choir "hum" pad, fiddle-less, foot stomps and claps.
  Pitch the key to the bagpipe chanter (A mixolydian / A minor) so the drone sits in it. `loop_bpm(122)` = 121.97 BPM
  (31 bars; count-in 1.97 s).
- **Flag:** the bagpipe drone is a constant tone: keep it under the mix except when the aura lands hits.

**v3 intro test (user, Oct 24):** "We need a better intro to grab people's attention. Instead of starting with just the
'3 2 1', start the video with X vs X in big text covering the screen, coming in with impact, and then do the usual 3 2 1
FIGHT. Let's test it with just #30." If it works, it rolls out to the whole series (other episodes unchanged for now).
- **Bar 0 (0–1.97 s): the VS TITLE CARD.** The frame splits diagonally into two panels in the fighters' colours
  (tartan green top-left, magenta bottom-right) over the dimmed arena, each with its fighter's ball big and close, like a
  fighting-game versus screen; small "BALLFIGHT #4" tag at the top. The words slam in one per beat:
  - beat 1 (frame 0): **BAGPIPES** slams down from huge to size with overshoot, motion streaks, camera shake, a dust
    burst and a local light in its colour; sound = big stomp + boom + a bagpipe drone blast;
  - beat 2: **VS** slams into the centre and cracks the diagonal divider (crack decal, sparks); sound = crash / thunder hit;
  - beat 3: **ACCORDION** slams in the same way; sound = boom + an accordion chord blast;
  - beat 4: everything holds with a pulse, then the two panels slide apart like doors to reveal the ring; pickup fill.
  - Text: the biggest pixel-font sizes that fit the safe zone's width (ACCORDION's 9 letters across x 20–340 internal),
    names in fighter colours with a thick dark outline and highlight, VS largest in white-gold; panels and effects
    cover the full screen, the words stay inside the safe zone so no app UI hides them.
- **Bar 1 (1.97–3.94 s): the usual 3 · 2 · 1**, **FIGHT! on the bar-2 downbeat (3.94 s)**. The old top caption "BAGPIPES /
  VS ACCORDION" is dropped (the title card replaces it).
- **The fight shifts exactly one bar later.** Keep seed 1442 if the run replays identically, just one bar later (every
  event stays on the same beat; the K.O. moves from 50.43 to ~52.40 s, inside the 49–53 window, leaving a ~5 s solo before
  the 58.05 reassembly). If it doesn't replay identically, re-run the search with the fight starting at 2 bars.
- **Music:** bar 0 = big unison band hits on beats 1–3 (one per word, each with that instrument on top) and a pickup on
  beat 4; bar 1 = the existing count-in; the fight section one bar shorter. Never quiet.
- **Loop:** the title slam is the frame-0 reset. The run-up (reassembly, glide to corners) leads into it; ramp in the last
  ~0.15 s (panel edges / streaks sliding in, as #27's "3" fly-in did) plus a riser into the slam so `reelkit.loopcheck`
  passes.
- **Flash:** no full-screen white; the coloured panels appear once and leave once ~2 s later (well under 3 changes per
  second); `reelkit.flashcheck` must pass.

## Board history (moved here from the status board, Oct 27)

- Status: review (v3) · Session: session_011GUQnH (2026-10-24) · Output: `ballfight_bagpipes_vs_accordion_v3.mp4` (v2: `_v2.mp4`, v1: `_v1.mp4`)
- v3 intro test delivered (early intro sheet sent first): bar 0 = the VS title card (`title.py`: diagonal tartan-green / magenta panels, big balls, BAGPIPES / VS / ACCORDION slamming in on beats 1-3 with shake, streaks, dust, local light; drone blast, thunder crack as VS splits the divider, accordion chord; doors slide apart on beat 4), bar 1 = 3-2-1, FIGHT! at 3.94 s, old top caption dropped. Seed 1442 did not replay identically one bar later (float drift from ~31 s, K.O. 51.41), so the honest search was re-run at 2 bars (rules unchanged): 1442's new run ranked first (6 fits in 1444; BAGPIPES 57 % / ACCORDION 43 % over 1212 decided): bagpipes lead by 20, FULL BELLOWS lands at 45.8 s, 5-5 at 50 s, ACCORDION wins at 51.41 s with 0.24 HP. Loop: the title slam is the frame-0 reset (doors shut over the last 0.30 s + riser + whoosh, a ghost BAGPIPES lands on frame 0). Checks: safe zone OK; subject CoC 0; loopcheck PASS render (0.9x) + mp4 (1.5x), audio step 4.0 dB; flashcheck PASS (1 doors pair, worst 1/s). Waiting for feedback
