# 47. BALLFIGHT: Cello vs Contrabass

Board row: `docs/IDEAS.md` (#47). Series rules: `docs/ballfight/RULES.md`. Make it with the episode kit
(`projects/ballfight/README.md`, "Making an episode").

## 47. BALLFIGHT: Cello vs Contrabass — APPROVED
**Direction (user, Oct 28):** "Draft Cello vs Contrabass"; approved with mood A (dark tango), colours as proposed. All
BALLFIGHT series rules (VS title card without an episode tag, 3·2·1, ability-name captions, HUD ability list, colours,
honest tuned fight, 61.0 s, loop point = file boundary) plus consistent loudness from frame 0.
**Rivalry:** the low strings: the cello sings, the bass holds the orchestra up and never gets thanked. In Saint-Saëns'
*Carnival of the Animals* (1886, public domain) the cello is the Swan and the double bass is the Elephant: each
desperation move quotes its theme. Post caption: "The swan vs the elephant 🦢🐘 Comment the next match-up 👇"

| | CELLO (rust, left), new | CONTRABASS (steel blue, right), new |
|---|---|---|
| Style | The only grappler | The only juggernaut (no knockback while walking) |
| Signature | **THE EMBRACE**: every bar it rushes in, wraps its bow arm around the enemy, spins it and flings it into the ropes (grab -> hold ~1 beat -> throw; rope hit = extra damage) (new kind `grapple`) | **WALKING BASS**: one heavy step per beat, relentlessly toward the enemy, each step a small floor-quake; immune to knockback while walking (new kind `walk`) |
| Grows per landed hit | Grab reach + throw power | Step-quake size |
| LV2 | VIBRATO: a held enemy shakes and takes extra damage | SLAP BASS: a sharp percussive knockback burst (breaks a grab) |
| LV3 | PRELUDE: Bach's rocking cello prelude: the bow rolls across four strings, a ring of note-waves rocks outward | DOUBLE TIME: walks on 8ths |
| Below 25 HP | SWAN SONG: rises and glides across the ring like a swan, then dives; the music plays *The Swan* | THE ELEPHANT: an unstoppable stampede across the ring, floor shaking; the music plays *The Elephant* |

- **Likely shape (only if the honest fight gives it):** grappler vs unstoppable walker: throws are the cello's damage,
  but the bass can't be knocked back while walking, so the cello must catch it between steps; SLAP BASS breaks a grab;
  the two desperation moves are a matched pair from the same suite.
- **Colours (in the table):** CELLO rust (196, 84, 40), CONTRABASS steel blue (70, 110, 200); strong contrast (rust near
  the HORN's copper, steel blue near the PIANO's blue; neither in this fight).
- **Music, mood A (dark tango):** new kit mood `kit/moods/dark_tango.py`: bandoneón stabs, piano-less marcato rhythm
  (strings' chop played by a muted guitar/pizz pad, not cello or bass), dramatic minor, tango "stops" (a full-band cut
  for a beat) on the throws. The **contrabass plays the walking bass line** (its steps ARE the bass notes), the **cello
  the singing melody**; leader fuller and louder, loser thinner and dirtier; the winner's solo over the tango.
  `loop_bpm(118)` = 118.03 BPM (30 bars; bar = 2.034 s, FIGHT! at 4.07 s). Consistent loudness from frame 0 (stops are
  short and allowed as the one-beat cuts; `reelkit.levels` must still pass). Desperation quotes: Saint-Saëns' *The
  Swan* (SWAN SONG) and *The Elephant* (THE ELEPHANT), and Bach's Cello Suite No. 1 Prelude (PRELUDE), all public domain,
  arranged in the episode's key; nothing else quoted. (Not chosen: B symphonic metal; C dark jazz-hop.)
- **VS title card:** CELLO / VS / CONTRABASS: a cello swell, the thunder crack, a slapped bass hit.
- **Beat sheet:** series standard (bar 0 card · bar 1 count-in · FIGHT! at 4.07 s · unlocks 8–30 s · desperation 32–44 s ·
  K.O. ~49–53 s · solo + "COMMENT THE / NEXT MATCH-UP" · reassembly into frame 0).
- **Flags:** CELLO: the Pixel Pitch model + voice exist (`cello60_lowpoly`, bridge `CELLO`); new roster entry + kit module.
  **CONTRABASS: build the model from reference photos found with Claude in Chrome** (a real double bass: sloped
  shoulders, tall scroll, endpin; visibly bigger than the cello) as a new file in `series/instrument-history/`, own
  commit; a voice with bowed (arco) and plucked / slap (pizz) sounds, own commit; bridge entry. New kinds `grapple`,
  `walk` (+ the swan glide and the stampede) additive, own commits; every earlier episode replays identically. **Send
  `python -m ballfight preview CELLO --vs CONTRABASS` and `preview CONTRABASS --vs CELLO` before tuning.** Re-tune for
  40–60 % wins; honest seed search with `t_fight` = 2 bars. Kit `check` must pass.
- **Effort:** ~60–75 min. Folder: `projects/ballfight_cello_vs_contrabass/`.

## Board history

- Approved Oct 28 (ideas chat).
- Oct 28 (session_01CzTfqb): in production; kit episode `projects/ballfight_cello_vs_contrabass/`. CONTRABASS model from
  Commons reference photos (AGK front + side view, the Met's CC0 bass), voice (arco / pizz / slap) and bridge entry in
  their own commits; engine kinds `walk`, `grapple` + `vibrato`, `slap` + `prelude`, `swan` + `stampede` additive, own
  commits, replay guard identical each time; previews of both fighters sent before tuning; mood `dark_tango` (new).
  Tuned (quake reach 0.66, knock 0.2, scale 0.85: CELLO 46 %, median K.O. 49.3 s), seed 2455 (CONTRABASS wins at
  50.88 s with 0.1 HP, three lead changes; SWAN SONG then THE ELEPHANT 2 s apart, the latter in the HUD only by the 4 s
  rule). Sheet sent. Flagged to the user: The Swan's rhythm and The Elephant's tune are written without a score source.
  Details: `docs/log/ballfight_cello_vs_contrabass.md`.
