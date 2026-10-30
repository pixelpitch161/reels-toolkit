# 54. BALLFIGHT: Horn vs Trombone, THE REMATCH

Board row: `docs/IDEAS.md` (#54). Series rules: `docs/ballfight/RULES.md`. Make it with the episode kit
(`projects/ballfight/README.md`, "Making an episode").

## 54. BALLFIGHT: Horn vs Trombone, THE REMATCH — APPROVED
**Direction (user, Oct 30):** "draft Horn vs Trombone Rematch. Instead of having 'VS' in the middle of the title card,
replace it with 'REMATCH'. Could be diagonal to give room for the instrument names"; approved with mood A (brass
trailer: the sequel). A viewer request (`docs/ideas/requests.md`: on Instagram's #34 reel, "RIGGEDD!! RUN IT BACK!!" (60
likes), "Horn is superior to trombone bro" (209)). A crossover of HORN v3 (ECHO CALL, as built in #34 v5) and TROMBONE
(in the kit). All BALLFIGHT series rules (no episode tag, 3·2·1, ability-name captions, HUD ability list, colours, honest
tuned fight, 61.0 s, loop point = file boundary), the crossover rules (fighters exactly as built; the only per-episode
lever is a per-fighter damage handicap, 40–60 % wins) and consistent loudness from frame 0.
**Post caption:** "You said rigged. We ran it back. 📯🎺 Comment the next match-up 👇"

- **The title card: REMATCH instead of VS (user).** REMATCH runs **along the diagonal crack** between the two panels,
  angled like the divider, slamming in on beat 2 (where VS was) and cracking the divider with the thunder hit; gold like
  VS, the largest scale that fits the diagonal inside the safe zone; the corners stay free for big HORN and TROMBONE
  names (in their panels). Build it as an opt-in option on the shared `projects/ballfight/title.py` (a custom middle word
  + an angle, e.g. `MIDDLE = ('REMATCH', angle)` in the config), own commit; without it every card draws exactly as
  before (all earlier title frames pixel-identical). Pixel text rotated with nearest-neighbour so it stays crisp; check
  it reads at 1080x1920 on the early intro sheet (frames at 0.0, 0.5, 1.0, 1.5 s).

| | HORN (copper, left), v3 from #34 v5 | TROMBONE (mint, right), from the kit |
|---|---|---|
| Signature | ECHO CALL (round blast on every beat; copper echo ghosts along its path repeat it a beat later) | SLIDE LANCE (thrust on every beat, long range; swats what it passes through) |
| Grows per landed hit | +1 echo ghost (max 4) | Lance reach |
| LV2 | STOPPED HORN (copper hand shield) | SAD TROMBONE (shockwave + 1 s slow) |
| LV3 | HORN RIP (echoes come back faster) | POWER SLIDE (thrusts on 8ths) |
| Below 25 HP | FINAL FLICK (a giant glove flicks the enemy into the ropes) | PEDAL TONE (a giant bell drops and stomps) |

- **Likely shape (only if the honest fight gives it):** chasing the horn means running through its echoes, but the lance
  hits from outside them; STOPPED HORN blocks the lance completely. **Honest as always:** the seed is picked for pacing,
  never for the winner. #34 v5 already played these exact fighters, so prefer a seed whose story differs from #34 v5's
  (different lead changes / finish), whoever wins.
- **Sides:** first named = left: HORN left, TROMBONE right (the reverse of the first fight, which also reads as round 2).
- **Music, mood A (brass trailer: the sequel):** port #34's `brass_trailer` bed into the kit (`kit/moods/brass_trailer.py`,
  checked against #34's audio), then play it darker and bigger for the rematch: low-string ostinato, bigger braams,
  taiko hits, minor; no brass of its own. The **horn plays the noble melody** (echo ghosts = quieter delayed repeats of
  its call), the **trombone the punchy low riffs and smears**; leader fuller and louder, loser thinner and dirtier; the
  winner's solo over the trailer. `loop_bpm(122)` = 121.97 BPM (31 bars; bar = 1.968 s, FIGHT! at 3.94 s). Consistent
  loudness from frame 0. Original. (Not chosen: B mambo; C second line.)
- **VS title card hits:** HORN (a horn call blast) · REMATCH (the thunder crack + a braam) · TROMBONE (a trombone smear).
- **Beat sheet:** series standard (bar 0 card · bar 1 count-in · FIGHT! at 3.94 s · unlocks 8–30 s · desperation
  32–44 s · K.O. ~49–53 s · solo + "COMMENT THE / NEXT MATCH-UP" · reassembly into frame 0, the REMATCH card slamming in
  on the seam as usual).
- **Flags:** TROMBONE is in the kit. HORN: port v3 from #34 v5 into `kit/fighters/horn.py` (checked against #34 with the
  hash and a few frames); keep THE HUNT's `pack` kind in the engine. Engine / title changes additive, own commits; every
  earlier episode and title card replays identically. Kit `check` must pass.
- **Effort:** ~40–55 min. Folder: `projects/ballfight_horn_vs_trombone_rematch/`.

## Board history

- Approved Oct 30 (ideas chat).
- Delivered Oct 30 (session_01CzTfqd) for review: `ballfight_horn_vs_trombone_rematch.mp4`. REMATCH card (title.py
  `MIDDLE`, scale 7 along the crack), HORN v3 ported (checked against #34), brass_trailer ported + sequel option,
  DMG_SIDE (1.04, 1.08), seed 630: FINAL FLICK 45.8 s, PEDAL TONE lands 47.2 s, HORN wins 50.28 s with 2.8 HP. Details:
  `docs/log/ballfight_horn_vs_trombone_rematch.md`.
