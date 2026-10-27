# 25. Battle of the Bands: Genre Wars

Board row: `docs/IDEAS.md` (#25). Series rules: none.

## 25. Battle of the Bands: Genre Wars — APPROVED
**Direction (user, Oct 23):** Battle of the Bands is doing well, so make more episodes. Standing notes: 61.0 s; names +
instruments on screen; music not always super uplifting; can open sparse; impact captions; something changes every
4–5 s (lesson from #21).
**Concept:** a Battle of the Bands episode on the existing engine (`projects/battle_of_the_bands/battle.py`). The four
teams are genres, and the song itself shifts toward whoever is winning, ending fully in the winner's style.
- **Hook:** "4 GENRES" / "1 STAGE" (8 + 7 chars, impact style, in the first 2 s). Post caption: "Comment your genre 👇🎧"
- **Series look:** keep the round LED wall, speaker stacks and floating festival stage so it reads as Battle of the
  Bands, with a title tag ("BATTLE OF THE BANDS · GENRE WARS"). Optional: start an all-time win tally (A3) that
  carries into later episodes.
- **The four genres (each reuses one existing engine team, so no new simulation code):**

| Genre | Strategy (engine team) | Its sound | Stage lights when it leads |
|---|---|---|---|
| ROCK (red) | Smashing ricochets (BALL) | Distorted guitar + kit | Pyro flashes, red wash |
| JAZZ (blue) | Improvised wandering lines (VINE) | Sax, upright bass, ride, piano comping | Smoky blue spots |
| EDM (cyan) | Pulses on the beat (RIPPLE) | Saw lead, sub bass, four-on-the-floor | Lasers |
| ORCHESTRA (gold) | A cascading wall of sound (SAND) | Strings, brass, timpani | Warm gold glow |

- **Music:** all four genres play the same original progression and tempo, each arranged in its own style, so the
  stems blend: dramatic minor (Am–F–Dm–E), ~118 BPM (`loop_bpm(118)` = 118.03: 30 bars in 61 s), tense rather than
  sugary. Every move steps that genre's part on the beat; each stem's volume follows its share of the stage, so the
  song audibly morphs as the lead changes; an eliminated genre drops out; the winner's style takes over the final
  bars. Original and synthesised.
- **Instruments on screen:** `reelkit.ui.share_bar` under the wall with each genre name + its lead instrument in small
  text ("ROCK · GUITAR", "JAZZ · SAX", "EDM · SYNTH", "ORCHESTRA · STRINGS"); labels pop up at each base in the
  first 2 s (impact style).
- **Beat sheet (61 s):** #20's proven shape: burst at frame 0, the stage fills by ~21 s, eliminations ~31 / 42 / 51 s,
  the winner's genre solo, the wall cracks back to empty exactly at 61.0 s. Captions: "4 GENRES / 1 STAGE" (0–2),
  "THE SONG FOLLOWS / THE LEADER" (8–14), "(GENRE) TAKES OVER" on lead changes, "(GENRE) IS OUT", "(GENRE) WINS",
  "WHICH GENRE / ARE YOU?" (winner solo). Something must change every 4–5 s.
- **Flags:** honest seed (existing `seed_search.py --stop 5`; re-tune the search's story shape for 61 s), never
  hand-edited. Genre stereotypes played as fun, not mockery. Each stem must sound good alone (the winner's solo is
  that stem). Loop point = file boundary; safe-zone, flash (pyro / lasers) and loop checks must pass.
- **Effort:** mostly new audio (four genre arrangements of one progression) + lighting moods; engine, stage and UI
  reused: roughly an episode-sized job plus the music. Work in a new project folder (e.g. `projects/botb_genre_wars/`)
  that imports the engine; don't edit #20's files.

## Board history (moved here from the status board, Oct 27)

- Status: review · Session: session_01CRdGiC (2026-10-23) · Output: `botb_genre_wars.mp4`
- v1 delivered with caption + tags (early sheet sent first). New folder projects/botb_genre_wars/ importing #20's battle.py unchanged (ORCHESTRA=SAND, ROCK=BALL, EDM=RIPPLE, JAZZ=VINE). Honest seed 189 (61 s search, 5 fits in 192 runs): EDM leads, JAZZ takes over 18.6, bell 20.33 (bar 10), ROCK takes over 21.1, EDM retakes 28.6; ORCHESTRA out 33.7, ROCK 43.3, JAZZ 52.85; EDM (3rd at the bell) wins and solos; wall cracks to empty on 61.0 = frame 0. One original song (Am-F-Dm-E, 118.03 BPM, 8-bar melody) in four arrangements; bass/harmony follow share gently, drums and melody steeply, so the song morphs to the leader; stinger in the new leader's style + stage mood (pyro / lasers / blue haze + spot cones / gold motes) on every lead change. Series title as an impact caption at 2.65 s (in the scoreboard panel it clipped the wall). Checks: safe zone OK (0.25 s steps); wall + props CoC 0 on all 1830 frames; flash 0 general flashes (max 13.9 %); levels: opening -14.3 dBFS, build +3.1 dB, 0 quiet windows; loopcheck render.py audio.wav PASS; loopcheck mp4: picture seam 1.7x OK, click OK, audio-level line FAILs only from AAC end padding (640 samples; container says audio = 61.000 s; same open question as #22). Waiting for feedback
