# 37. BALLFIGHT #11: Clarinet vs Oboe

Board row: `docs/IDEAS.md` (#37). Series rules: `docs/ballfight/RULES.md`.

## 37. BALLFIGHT #11: Clarinet vs Oboe — APPROVED
**Direction (user, Oct 26):** "Draft Clarinet vs Oboe"; approved with mood A (Baroque storm). Same day, a project-wide
rule: **consistent loudness from frame 0** ("the videos start out too quiet. The ending loudness is fine, but it should
be more consistent overall"): see PLAYBOOK, `reelkit.levels` must PASS. All BALLFIGHT series rules apply (VS title card,
3·2·1, ability-name captions, HUD ability list, colours, honest tuned fight, 61.0 s, loop point = file boundary).
**Rivalry:** the woodwinds' reed war, single reed vs double reed. Oboists carve their own reeds and the whole
orchestra tunes to their A; clarinettists squeak and jazz. Comment bait: in *Peter and the Wolf* the clarinet is the cat
and the oboe is the duck. Post caption: "The cat finally gets the duck? 🐈🦆 Comment the next match-up 👇"

| | CLARINET (lavender, left) | OBOE (olive, right) |
|---|---|---|
| Style | The only teleporter | The only debuffer |
| Signature | **TWELFTH JUMP**: the register key makes the clarinet jump a twelfth, so the clarinet jumps too: on each beat it vanishes and reappears beside the enemy; the lavender zip line from where it was to where it lands cuts anything it crosses (new kind `jump`) | **TUNING A**: every bar it sounds its A as a narrow tuning beam; a hit knocks the enemy **OUT OF TUNE** for 2 bars: it wobbles (doubled, warped outline), its hits audibly play flat and do less damage, and each of the oboe's nasal 8th notes stings it (new weapon `tune` + status `detune`) |
| Grows per landed hit | Jump distance | Detune depth (more sting, weaker enemy hits) |
| LV2 | SQUEAK: the infamous squeak as a shrill burst that stuns the enemy for 0.5 s (new `stun`) | DOUBLE REED: the reed's two blades scissor out and snip anything close (new `snip`) |
| LV3 | CLARION (`haste`: jumps twice as often) | ENDLESS NOTE (circular breathing: the A beam held for a whole bar) |
| Below 25 HP | RHAPSODY: a chain of five rising jumps zig-zagging across the ring, each a cut | ILL WIND (`wave`: a sour gust across the ring, the enemy fully out of tune) |

- **Likely shape (only if the honest fight gives it):** the oboe must land the beam before it hurts, and the clarinet
  is the hardest thing in the ring to hit; once it's out of tune, every jump is weaker. You hear who's losing when
  their notes go sour.
- **OUT OF TUNE is a game rule** in the simulation; the audio follows it. Keep the detune obvious but musical (about a
  quarter-tone at full depth), sour rather than broken.
- **Music, mood A (Baroque storm):** harpsichord continuo + a racing minor string ostinato, stormy-presto energy, all
  original (no quoting Vivaldi or anyone). Both fighters are woodwinds: the **clarinet plays the lower, liquid line**
  (chalumeau register), the **oboe the higher, nasal line**; the bed has no woodwinds of its own. `loop_bpm(140)` =
  141.64 BPM (36 bars; bar = 1.694 s, FIGHT! at 3.39 s). **Consistent loudness** from the title card to the end (the
  new rule): the storm is already at full level on frame 0; build with density, not level. Leader fuller and louder,
  loser thinner and sour; the winner's solo over the storm. (Not chosen: B spy caper; C dark klezmer.)
- **VS title card:** CLARINET / VS / OBOE: a clarinet gliss blast, the thunder crack, the oboe's A as a held blast;
  panel patterns: clarinet keys / ebony grain (lavender-lit), oboe double reed / tuning-fork motif (olive-lit).
- **Beat sheet (61 s):** bar 0 VS card · bar 1 count-in · FIGHT! at 3.39 s · 8–30 s the four unlocks (each named once;
  the HUD list lights on every use) · 32–44 s RHAPSODY or ILL WIND · 44–52 s final exchange · K.O. ~49–53 s (slow-mo
  shatter into keys / reed / bell) · winner's solo + "COMMENT THE / NEXT MATCH-UP" · 58–61 s reassembly, doors shut, title
  slam = frame 0.
- **Flags:** the OBOE is all new: no Pixel Pitch oboe yet, so build the model from reference photos (as the rebuilt
  trumpet / trombone were) and a nasal double-reed synth voice, each in its own commit (series folder: add, don't edit
  other instruments); the CLARINET's model and voice exist (`clarinet60_lowpoly`, bridge entry CLARINET). New engine
  kinds additive, own commits; all earlier episodes replay identically. Re-tune for 40–60 % wins; honest seed search
  with `t_fight` = 2 bars and the usual hard windows. Teleports must read: a short lavender vanish / appear burst +
  the zip line, never a hard pop that looks like a glitch. Effects from `reelkit.fx`; no full-screen flashes;
  safe-zone, flash, loop and levels checks must pass.
- **Effort:** ~60–75 min (the oboe model + voice and two new weapon kinds). Work in
  `projects/ballfight_clarinet_vs_oboe/`.

## Board history (moved here from the status board, Oct 27)

- Status: review (v1 audio fix) · Session: session_019oLg15 (2026-10-26) · Output: `ballfight_clarinet_vs_oboe.mp4`
- Audio fix delivered (user, Oct 26: "in the last couple of seconds of the fight, the clarinet stops making noise"): the HP thinning's hash (from #36) was linear, so on CLARION's consecutive 8th jumps every value fell under the threshold and the clarinet's last ~3 s went silent (the oboe's stings had 1 s holes at 41 and 50 s too). New splitmix hash + never two dropped notes in a row, max 30 %: both lines play to the K.O.; remaining oboe gaps are honest (clarinet in tune, between A beams). Re-muxed only (same frames); levels PASS (opening -0.1 dB vs ending, spread 4.3 dB), loopcheck PASS mp4 1.0x. Waiting for feedback. Previous: v1 delivered (early sheet sent first). OBOE built new in the Pixel Pitch folder, own commits: model from Commons reference photos (Yamaha YOB-831 front shot measured at 590 mm, Lorée with reed, a reed against a ruler; Chrome unreachable, built-in browser used), `ball` scene = radii x1.6 for sprites; nasal double-reed voice (formants ~1.2 / ~3 kHz); bridge entry OBOE. New engine kinds in 4 small commits + roster commit (all 10 finished episodes replay identically; guard entry #37 -> seed 654): weapon `jump` (TWELFTH JUMP teleport + zip-line cut) + ability `rhapsody`, ability `stun` (SQUEAK; every weapon dispatch checks it), weapon `tune` + status `detune` (OUT OF TUNE: hits x (1 - 0.6 x depth), stings on the 8ths) + ability `endless` + wave option `detune` (ILL WIND), ability `snip` (DOUBLE REED). Tuned once at damage scale 0.55 (200 seeds CLARINET 47 % / OBOE 53 %, median K.O. 50.8 s). Honest search at 2 bars: 8 fits in 654 (51 / 49); seed **654**: even to 20 s, the oboe's detune grinds a lead from 22 s, RHAPSODY 42.8 / ILL WIND 43.6 trade, 18-18 at 46 s, the clarinet retakes the lead 0.2 s before its K.O. at 51.90 s with 6.5 HP. Captions SQUEAK 12.3, DOUBLE REED 17.0, ENDLESS NOTE 23.7, CLARION 28.4, RHAPSODY 42.8 (ILL WIND in the HUD list only: 4 s rule). Title panels: CLARINET ebony grain + ring keys, OBOE tuning forks + double reeds + the A's sine (shared title.py, own commit). Music mood A (Baroque storm, D minor, 141.64 BPM, no woodwinds in the bed): clarinet = chalumeau note per jump + the twelfth on landed cuts, flat by up to a quarter-tone while OUT OF TUNE; oboe = the A per beam, running 8ths = the stings. Checks: `reelkit.levels` PASS on wav + mp4 (opening +0.3 dB vs ending, spread 4.3 dB, no dips; one master gain, no automation); band check clarinet +7.3..+9.2 dB, oboe +9.5..+12.1 dB over the bed; safe zone OK; CoC 0 on all 1830 frames; loopcheck PASS render 0.5x / mp4 1.0x (audio 3.1 dB, no click); flashcheck PASS (1 = the doors). Waiting for feedback
