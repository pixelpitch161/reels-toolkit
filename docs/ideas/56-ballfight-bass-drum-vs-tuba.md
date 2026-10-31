# 56. BALLFIGHT: Bass Drum vs Tuba

Board row: `docs/IDEAS.md` (#56). Series rules: `docs/ballfight/RULES.md`. Make it with the episode kit
(`projects/ballfight/README.md`, "Making an episode").

## 56. BALLFIGHT: Bass Drum vs Tuba — APPROVED
**Direction (user, Oct 31):** "draft Bass Drum vs Tuba"; approved with mood A (dark drumline, from the kit); **no Tuba
redesign** (the TUBA fights as built, despite the comments criticising it: `docs/ideas/requests.md`). A viewer request
(TikTok, "Base drum vs tuba", twice). All BALLFIGHT series rules (VS title card without an episode tag, 3·2·1,
ability-name captions, HUD ability list, colours, honest tuned fight, 61.0 s, loop point = file boundary, **no approval
waits**), the crossover rules for the TUBA side (exactly as built; the only per-episode lever is a per-fighter damage
handicap, 40–60 % wins) and consistent loudness from frame 0.
**Rivalry:** who owns the BOOM: the marching band's two biggest low sounds. Post caption: "Who owns the low end? 🥁📯
Comment the next match-up 👇"

| | BASS DRUM (gunmetal, left), new | TUBA (indigo, right), as built (#32 v3 / #36) |
|---|---|---|
| Style | The only roller | Swallow and launch |
| Signature | **STEAMROLLER**: every bar it tips onto its side and rolls across the ring like a wheel toward the enemy, flattening anything in its path; a dust trail telegraphs the line; every impact is a huge BOOM (new kind `roll_charge`) | OOM-PAH SWALLOW ("oom" inhales and swallows, "pah" spits the enemy into the ropes) |
| Grows per landed hit | Roll speed + drum size | Suction reach |
| LV2 | HEARTBEAT: two quick booms (lub-dub), back-to-back shockwaves | OOMPAH (swallows twice as often) |
| LV3 | DRUM ROLL: rolls nonstop for a bar, zig-zagging, with a thundering roll | BRASS WALL (brass dome shield) |
| Below 25 HP | 1812 OVERTURE: five cannon-booms blast across the ring (bass drums often stand in for the cannons in Tchaikovsky's *1812 Overture*) | SUBSONIC (the floor ripples like water) |

- **Likely shape (only if the honest fight gives it):** the tuba can swallow the bass drum mid-roll and spit it into the
  ropes; the roll is telegraphed, so the tuba can line up the inhale; HEARTBEAT punishes a tuba that gets close; 1812 vs
  SUBSONIC is the loudest finish. If the engine doesn't define a rolling fighter being swallowed (does the roll stop?),
  add it as an opt-in rule (own commit) and note it; don't change the tuba.
- **Colour (in the table):** BASS DRUM gunmetal (128, 128, 140) (dark metal grey with a bright rim; distinct from the
  FLUTE's silver and the BASS CLARINET's slate blue) vs TUBA indigo.
- **Sides:** BASS DRUM left, TUBA right (the tuba's usual side).
- **Music, mood A (dark drumline, kit):** the kit's `dark_drumline` bed **with its own bass drums removed** (the fighter
  IS the bass drum; keep snares, tenors, cymbals). The **bass drum fighter is the kick and the booms** (rolls and impacts
  on the beat), the **tuba plays the bass line** (oom = root, pah = the fifth, as in #32); leader fuller and louder, loser
  thinner and dirtier; the winner's solo over the drumline. `loop_bpm(126)` = 125.90 BPM (32 bars; bar = 1.906 s, FIGHT!
  at 3.81 s). 1812 OVERTURE quotes the finale of Tchaikovsky's *1812 Overture* (1880, public domain) in the episode's key,
  with the five booms as the cannons; nothing else quoted. Consistent loudness from frame 0.
- **VS title card:** BASS DRUM / VS / TUBA: a huge boom, the thunder crack, a tuba blast.
- **Beat sheet:** series standard (bar 0 card · bar 1 count-in · FIGHT! at 3.81 s · unlocks 8–30 s · desperation 32–44 s ·
  K.O. ~49–53 s · solo + "COMMENT THE / NEXT MATCH-UP" · reassembly into frame 0).
- **Flags:**
  - **BASS DRUM model: build it from reference photos found with Claude in Chrome** (a real marching or concert bass drum:
    the big cylinder, two heads, hoops, tension rods, a felt mallet), a new file in `series/instrument-history/`, own
    commit; no logos on the heads; it must read as a drum that can roll (the cylinder on its side). Voice: a deep boom +
    mallet hits + a roll, own commit; bridge entry. New roster entry + `kit/fighters/bass_drum.py`.
  - TUBA: if not in the kit yet, port it from #36 / #32 v3 (`kit/fighters/tuba.py`, checked with the hash and a few frames)
    and note it in `docs/log/ballfight_studio.md`.
  - New kinds additive, own commits; every earlier episode replays identically. Camera shake on booms stays small;
    no full-screen flashes; kit `check` must pass. Re-tune for 40–60 % wins; honest seed search with `t_fight` = 2 bars.
- **Effort:** ~50–65 min. Folder: `projects/ballfight_bassdrum_vs_tuba/`.

## Board history

- Approved Oct 31 (ideas chat).
