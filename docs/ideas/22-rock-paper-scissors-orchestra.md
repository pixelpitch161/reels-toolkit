# 22. Rock Paper Scissors Orchestra

Board row: `docs/IDEAS.md` (#22). Series rules: none.

## 22. Rock Paper Scissors Orchestra — APPROVED
**Direction (user, Oct 22–23):** satisfying-first; not every video upbeat (this one: **mood A, ominous cinematic
orchestra**, chosen by the user); the music can open sparse but tense rather than in full chaos; share bar keeps
team names in small text with leader lines; captions with a very slight "impact"; 61.0 s (TikTok monetisation).
**Concept:** the three teams are the sections of an orchestra, chasing each other around a round stage. When a piece
catches the piece it beats, the caught piece joins the winner's team, and its section plays. **The twist (drives
comments):** the moment one team is wiped out, the winner is already decided, and it's never the team that did the
wiping out (if scissors finish off paper, rock and scissors remain, and rock beats scissors: the killer dooms
itself).
- **Hook:** "ROCK. PAPER." / "SCISSORS." (each word lands on its own beat with the impact caption style). Post
  caption: "Who are you rooting for? 🎧"
- **The three sections:**

| Team | Beats | Plays |
|---|---|---|
| ROCK (slate grey) | Scissors | Timpani and taiko |
| PAPER (ivory) | Rock | Strings: pizzicato that turns to tremolo as the section grows |
| SCISSORS (silver/red) | Paper | Brass stabs |

- **How it works:** 120 pieces (40 per team), low-poly tokens (stones, folded sheets, little scissor blades), bounce
  off each other and the stage edge with real collision physics. Each piece steers towards the nearest piece it beats
  and away from the nearest one that beats it, so chases become swirling waves. A catch converts the caught piece.
  Populations swing back and forth (comebacks are built in: when a team nearly dies out, its predator runs short of
  targets). Once a team is out, the last two play out a fixed result while the stragglers try to evade.
- **Music, mood A (ominous cinematic orchestra):** tense minor-key film score at ~110.2 BPM (`loop_bpm(110)`: 28
  bars in 61 s). Opens sparse but uneasy (one low timpani pulse, a held cello note) and builds as the chases heat up.
  Every capture plays the next note of the winning section's part, on the beat (grouped per sixteenth). Each
  section's volume follows its headcount; a wiped-out section falls silent. The winner gets a dark, triumphant
  ending, not a cheerful one. Original and synthesised, not built on any famous piece. (Alternatives offered, not
  chosen: B creepy music box; C chamber melancholy.)
- **Scene:** a low-poly opera house, dim, seen from slightly above: the round stage is the arena, ringed by red
  velvet curtains and gilded boxes; a big chandelier above (key light), warm footlights at the stage edge (local
  light); chandelier crystals shimmer, dust drifts in the beams, pawn-figure silhouettes in a couple of boxes (small
  life). Tokens cast real shadows on the stage boards.
- **UI:** the share bar with names (ROCK, PAPER, SCISSORS + shares in small text under the bar, each joined to its
  colour by a leader line) inside the safe zone, and impact-style captions. #21's session is building both as
  reusable `reelkit.ui` pieces: reuse them (if they aren't in reelkit yet, coordinate rather than build a second
  copy).
- **Ending (loop at the file boundary):** the winning team alone on stage; in the last seconds all its pieces gather
  into a tight clump at centre stage; at 61.0 s = frame 0 the clump bursts out into three colours again.

**Beat sheet (61 s; events after 8 s are targets: the real run decides, so choose a seed whose run fits):**
| Time | Visual | Caption (impact) | Sound (mood A) |
|---|---|---|---|
| 0–3 | Burst: 120 pieces fly out from centre stage into three colours | ROCK. / PAPER. / SCISSORS. (one word per beat) | Timpani hit per word, low cello |
| 3–8 | First catches; small rule icons appear by the bar | HIT WHAT YOU BEAT / IT JOINS YOU | Sparse pizzicato and brass replies |
| 8–20 | Chases swirl into waves; the share bar sways | (no caption) | Sections trade phrases; pulse tightens |
| 20–28 | One team crashes to a handful | (TEAM) IS DYING… | Its section thins to single notes |
| 28–36 | Comeback: its predator runs out of targets, so it surges back | (no caption) | Its section swells back; tension rising |
| ~36–38 | A team is wiped out | (TEAM) IS OUT | Its section stops dead |
| 38–42 | The twist | SCISSORS KILLED PAPER… / AND DOOMED ITSELF (example) | Low brass sting, then a ticking pulse |
| 42–50 | Mop-up: the doomed team's last pieces dodge around the stage edge | (no caption) | Rising strings, fewer and fewer stabs |
| ~50 | Last straggler caught; one colour fills the stage | ROCK WINS (example) | Full orchestra hit, minor-key fanfare |
| 50–57 | Winner's section plays its theme alone; the chandelier blazes | WHO WERE YOU / ROOTING FOR? | Dark triumphant theme |
| 57–61 | All pieces gather into a clump at centre stage; at 61.0 they burst = frame 0 | (no caption) | Swell into the opening timpani |

**Flags:**
- Make it ours, not the viral one: emoji rock-paper-scissors simulations went viral before. Ours: own low-poly tokens
  (never emoji artwork), the orchestra mapping, the opera house, the twist caption.
- Honest simulation: the chase rules are stylised physics. Pick seeds and balance, never hand-edit who wins; tune so
  each team wins some runs. Early-stopping search (`--stop 5`).
- The twist is always true with three teams in a cycle: when one is out, the survivor that beats the other wins. The
  caption can state it with confidence.
- Story shape: prefer seeds with a visible comeback before the first elimination and stragglers who dodge for a few
  seconds in the mop-up; avoid runs that end too early.
- Length 61.0 s (`DUR = 61.0`), tempo with whole bars (~110.2 BPM = 28 bars). Loop point = file boundary (the burst
  at frame 0); `reelkit.loopcheck` must pass.
- Keep the mood dark and cinematic, not upbeat.

**Feasibility:** sim: 120 discs in a round arena with elastic collisions, a seek/flee steering force, damping and a
speed cap; a capture converts the piece; a simple grid speeds up neighbour searches. Run offline and cache; reuse the
seed-search pattern from `projects/battle_of_the_bands/seed_search.py` with `--stop`. Render: three instanced
low-poly token models that turn to face their direction of travel, on a round stage with shadows; the opera house is
moderate modelling (curtains, boxes, chandelier) and can be simplified with a dark auditorium beyond the stage. UI:
share bar with names + impact captions from `reelkit.ui` (once #21 adds them). Audio: capture events grouped per
sixteenth step each section's part; volume follows headcount; a wiped-out section stops. One event list drives the
picture, the bar and the mix.

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_01Kd27hS (2026-10-23; took over from session_018AnKYU, which pushed nothing) · Output: `rock_paper_scissors_orchestra.mp4`
- v1 delivered with caption + tags (early sheet sent first). Honest chase run, seed 60 (fits are ~3 % of seeds; balance over 102 logged runs: ROCK 24 / PAPER 23 / SCISSORS 26 wins): ROCK crashes to 5 at 30 s, swells to 110 and wipes out SCISSORS at 38.5 s (dooming itself), tiny PAPER eats the giant and wins at 46.6 s; victory swirl + stadium-wave hop; the winner gathers into the frame-0 clump, which bursts into three colours at 61.0 = frame 0. Changes vs backlog: catches start at bar 1 (2.18 s), so the hook is the three colour wedges fanning out; a panic-sprint rule (+30 % speed with a predator close, same for everyone) keeps comebacks and dodging alive; rule legend ROCK > SCISSORS > PAPER > ROCK at 3.6-7.9 s; the win came at 46.6 (target ~50), so the winner section runs ~10 s. Checks: safe zone OK; stage + tokens CoC 0 on all frames; 0 general flashes; picture seam 2.2x; opening -15.7 dBFS, build +3.0 dB, no quiet windows; sections 15-19 dB over the bed (1-5 kHz). Open: mp4 loopcheck flags the audio seam: AAC pads 61.0 s at 48 kHz with 13 ms of silence (the wav seam passes; every 61.0 s reel has this) - user to choose: loopcheck trims decoded audio to the nominal length, or reels move to 61.333 s (1840 frames) like Pixel Pitch. Waiting for feedback
