# 19. The Mountain King Escape

Board row: `docs/IDEAS.md` (#19). Series rules: none.

## 19. The Mountain King Escape — APPROVED
**Direction (user, Oct 22):** satisfying-first; engagement brief: chaos and sound from frame 0, never quiet,
build to a satisfying payoff, exaggerate.
**Trend basis (TikTok / Instagram research, Oct 22):** "can the ball escape within N seconds?" countdown sims and
"melody played by bounces" videos are everywhere; recognisable tunes drive comments ("Do you know this song?").
This combines both on a public-domain piece that accelerates by itself.
- **Hook:** "CAN IT ESCAPE" / "BEFORE IT ENDS?" (13 + 15 chars, in the first 2 s).
- **What you see:** a glowing ball trapped inside 12 concentric rings floating in a vast low-poly crystal cavern
  (the Mountain King's hall). Each ring spins opposite to its neighbour and has one gap. When the ball slips
  through a gap, that ring shatters into pixel shards and the ball is loose in the next, bigger ring. HUD:
  countdown timer + "RINGS LEFT" counter.
- **How it plays:** every bounce off a ring wall plays the next note of Grieg's *In the Hall of the Mountain King*
  (1875). The ball's speed follows the tempo, so the visible acceleration IS the music's acceleration. The
  instrument climbs as the piece builds: low pizzicato bass → bassoon → strings → brass → full orchestra synth for
  the last rings. Each ring shatter lands a timpani + cymbal hit on a phrase ending; near-misses (the ball
  skimming a gap's edge) get a little sting. The last three rings break in quick succession in the prestissimo;
  the final ring shatters on the last chord. Our own synthesised arrangement of the public-domain score.
- **Scene:** crystal cavern: glowing crystal clusters pulse on each note; torches on the ledges (warm local
  lights); a shaft of light from a crack in the cavern roof (key light) catching dust and drifting cave moths;
  static troll statues watching from the ledges. Ring and ball shadows land on the cavern floor below; ring
  colours shift with each ring broken.
- **Payoff / ending (loop):** the ball bursts out of ring 12 on the final chord and flies up the light shaft; the
  camera zooms out to show the whole cavern is itself inside a giant ring with a gap; the ball lands in a new
  innermost ring identical to the opening: "trapped again", loop.

**Beat sheet (60 s; the arrangement skips the near-silent opening: frame 0 is already mid-build at a lively tempo
with accompaniment):**
| Time | Visual | Caption | Sound |
|---|---|---|---|
| 0–3 | Ball pinging inside 12 spinning rings, crystals flashing; countdown starts at 60 | CAN IT ESCAPE / BEFORE IT ENDS? | Theme as bouncy pizzicato at a lively tempo, bass under it |
| 3–8 | Camera eases back: the cavern and all 12 rings | 12 RINGS / 60 SECONDS | Theme repeats; bassoon joins |
| 8–12 | Near-miss on the ring 1 gap, then through; ring 1 shatters | 11 LEFT | Sting, then cymbal on the phrase ending |
| 12–20 | Rings 2–3 break; ball faster; crystals pulse brighter | EVERY BOUNCE / = 1 NOTE | Theme up a fifth; strings join; tempo climbing |
| 20–28 | Rings 4–5 break; ball trail leaves light streaks | 7 LEFT | Louder; octave doubling; timpani on breaks |
| 28–36 | Rings 6–7; ball rattles fast; troll statues' eyes glow | IT'S SPEEDING UP… | Brass enters, tempo racing |
| 36–44 | Rings 8–9; two near-misses in a row | 3 LEFT / 16 SECONDS | Full orchestra synth, frantic |
| 44–52 | Prestissimo: rings 10 → 11 shatter back to back; cavern shakes, shards everywhere | 1 LEFT… | Fastest bounces; snare rolls |
| 52–55 | Climax: ring 12 shatters on the final chord; ball rockets up the light shaft | ESCAPED! | Final chords + crash |
| 55–60 | Zoom out: cavern inside a giant ring; ball drops into a new innermost ring; loop | TRAPPED AGAIN? | Low theme restarts into the opening |

**Flags:**
- Copyright: the composition is public domain (Grieg, 1875; died 1907). Recordings and modern arrangements are
  not: transcribe from a public-domain score (e.g. IMSLP) and synthesise our own arrangement.
- The ending is scripted (bounce times are planned to hit the melody notes, standard for this format). The hook is
  a tease, not a fair contest; never claim it's a real physics simulation.
- Pacing: the real piece runs ~2½ min. Condense to 60 s by cutting repeats and starting mid-build; frame 0 must
  already be lively (never quiet).
- Note density at the prestissimo ≈ 8–9 bounces/s (calc), fine for the ear; keep bounce notes crisp and short.
- Troll statues stay generic (no likeness to any film or game trolls); the "trapped again" zoom-out is ours.
  Source: [Wikipedia: In the Hall of the Mountain King](https://en.wikipedia.org/wiki/In_the_Hall_of_the_Mountain_King).

**Feasibility:** no physics engine: the ball moves in straight segments between planned wall hits, each timed to
a melody note; segment speed comes from the gap between notes, so acceleration is automatic; gaps rotate so they
line up exactly when an escape is due. Rings = extruded low-poly tori with a gap; shatter = pre-computed shard
particles; crystals and torches = simple glow meshes. One note list (from the transcribed score) drives bounces,
crystal pulses, ring breaks and audio. The cavern (walls, ledges, crystal clusters, statues) is moderate work;
portrait suits the tall cavern and light shaft; lock focus on the rings. The zoom-out loop needs one extra wide
set of the cavern inside the giant ring; the last frame must match frame 0.

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_01NxecfC (2026-10-22) · Output: `mountain_king_escape.mp4`
- v1 delivered with caption + tags (early sheet sent first). Theme transcribed from the CC0 Mutopia engraving of Grieg's own piano arrangement (verify_score.py diffs it bar by bar). Beat sheet followed: 267 notes = 267 bounces, 152 -> 265 BPM (8.8 bounces/s at the end), rings break at 8.8 / 15.0 / 18.2 / 23.7 / 26.0 / 31.0 / 33.7 / 38.3 / 42.6 / 46.7 / 48.4, ring 12 on the final chord (52.1). One change: the "two near-misses in a row" skim on bounces i and i+2 (consecutive skims are geometrically impossible with long chords). Checks: rings + ball CoC 0 on all frames; flash test 1 isolated general flash (ring-1 break, 28 %), worst 1/s; bounce voice 10-26 dB over the accompaniment; -9.9 LUFS, never below -17 dBFS; loop seam 4.8 vs 5.6 normal step. Waiting for feedback
