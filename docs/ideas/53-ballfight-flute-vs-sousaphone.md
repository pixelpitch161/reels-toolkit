# 53. BALLFIGHT: Flute vs Sousaphone

Board row: `docs/IDEAS.md` (#53). Series rules: `docs/ballfight/RULES.md`. Make it with the episode kit
(`projects/ballfight/README.md`, "Making an episode").

## 53. BALLFIGHT: Flute vs Sousaphone — APPROVED
**Direction (user, Oct 30):** a viewer request from the TikTok comments (`docs/ideas/requests.md`:
"Now put flute vs a sousaphone"). "Draft Flute vs Sousaphone. Make sure it uses chrome to get images for the
Sousaphone"; approved with mood A (dark marching-band fight song). All BALLFIGHT series rules (VS title card without an
episode tag, 3·2·1, ability-name captions, HUD ability list, colours, honest tuned fight, 61.0 s, loop point = file
boundary), the crossover rules for the FLUTE side (exactly as in the kit; the only per-episode lever is a per-fighter
damage handicap) and consistent loudness from frame 0.
**Rivalry:** the smallest vs the biggest in the marching band: flutes and piccolos lead from the front, sousaphones
bring up the rear. Post caption: "You asked for it 👀 Flute vs Sousaphone. Comment the next match-up 👇"

| | FLUTE (silver, left), from the kit | SOUSAPHONE (marching crimson, right), new |
|---|---|---|
| Style | Trapper | The only marcher (marching-band drill paths) |
| Signature | SNAKE CHARMER (baskets up to 2; cobras strike on the beat) | **DRILL SET**: every 2 bars it marches a crisp straight drill path across the ring on the beat (8 steps), trampling anything in its way (baskets included), halts at the end of the set and fires a huge **bell blast** straight ahead from its forward-facing bell (new kind `drill`) |
| Grows per landed hit | Cobra reach + speed | Bell-blast length |
| LV2 | FLUTTER TONGUE (cobras strike twice as fast) | HORN FLASH: spins the whole horn 360°, a blast sweeping all around (new `flash_spin`) |
| LV3 | SNAKE DANCE (cobras chase for a bar) | DOUBLE TIME (`haste`: marches twice as fast) |
| Below 25 HP | PICCOLO (piercing ring-wide note) | COMPANY FRONT: a full-width wall of ghost sousaphones marches across the whole ring (new `company_front`) |

- **Likely shape (only if the honest fight gives it):** predictable but unstoppable drill paths that trample the
  baskets; the flute has to set its cobras along the lines, not in the way. Best finish: COMPANY FRONT vs PICCOLO (see
  music).
- **Colour (in the table):** SOUSAPHONE marching crimson (210, 30, 60) (band-uniform red; near the KAZOO's red, not in
  this fight) vs FLUTE silver.
- **Sides:** FLUTE left (its usual side), SOUSAPHONE right.
- **Music, mood A (dark marching-band fight song):** port #38's `dark_drumline` bed into the kit (`kit/moods/dark_drumline.py`,
  checked against #38's audio as the porting steps say) and use it as a minor fight song: snare line, tenors, pitched
  bass drums, cymbal line, big unison hits; no low brass or flutes of its own. The **sousaphone plays the bass line**
  (its steps and blasts on the beat), the **flute the high melody**; leader fuller and louder, loser thinner and dirtier;
  the winner's solo over the drumline. `loop_bpm(126)` = 125.90 BPM (32 bars; bar = 1.906 s, FIGHT! at 3.81 s). COMPANY
  FRONT quotes the trio of Sousa's *The Stars and Stripes Forever* (1896, public domain) in the episode's key; if
  PICCOLO fires during it, the flute's part plays the famous piccolo obbligato from the same trio (also public domain).
  Nothing else quoted. Consistent loudness from frame 0. (Not chosen: B dark second line; C villain march.)
- **VS title card:** FLUTE / VS / SOUSAPHONE: a flute flurry, the thunder crack, a sousaphone blast; panel pattern:
  the coiled tubing + huge bell (crimson-lit).
- **Beat sheet:** series standard (bar 0 card · bar 1 count-in · FIGHT! at 3.81 s · unlocks 8–30 s · desperation
  32–44 s · K.O. ~49–53 s · solo + "COMMENT THE / NEXT MATCH-UP" · reassembly into frame 0).
- **Flags:**
  - **SOUSAPHONE model: build it from reference photos found with Claude in Chrome** (user's instruction): a real
    sousaphone's wraparound coil (worn around the player), the huge forward-facing bell (fiberglass-white or brass), the
    lead pipe and valves; it must read as a sousaphone at ball size and look nothing like the TUBA. A new file in
    `series/instrument-history/`, own commit; no school logos on the bell. Voice: start from the bridge's `TUBA` voice,
    brighter and punchier for marching, own commit; bridge entry.
  - New roster entry + `kit/fighters/sousaphone.py`; new kinds additive, own commits; every earlier episode replays
    identically. **Send `python -m ballfight preview SOUSAPHONE --vs FLUTE` before tuning.** Re-tune for 40–60 % wins
   ; honest seed search with `t_fight` = 2 bars. Kit `check` must pass.
- **Effort:** ~50–65 min. Folder: `projects/ballfight_flute_vs_sousaphone/`.

## Board history

- Approved Oct 30 (ideas chat).
