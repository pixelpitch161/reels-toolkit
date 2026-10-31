# 55. BALLFIGHT: Bass Clarinet vs Bassoon

Board row: `docs/IDEAS.md` (#55). Series rules: `docs/ballfight/RULES.md`. Make it with the episode kit
(`projects/ballfight/README.md`, "Making an episode").

## 55. BALLFIGHT: Bass Clarinet vs Bassoon — APPROVED
**Direction (user, Oct 31):** "draft Bass Clarinet vs Bassoon"; approved with mood A (horror suspense trap). A viewer
request (`docs/ideas/requests.md`: TikTok, "Gotta get bass clarinet and bassoon", 6 likes). A crossover: BASS CLARINET as
built in #38 / #40, BASSOON as built in #39 / #52 (in the kit). All BALLFIGHT series rules (VS title card without an
episode tag, 3·2·1, ability-name captions, HUD ability list, colours, honest tuned fight, 61.0 s, loop point = file
boundary, **no approval waits**), the crossover rules (fighters exactly as built; the only per-episode lever is a
per-fighter damage handicap, 40–60 % wins) and consistent loudness from frame 0.
**Rivalry:** the bottom of the woodwind section: both hold down the bass line, both get mistaken for something else,
their players sit side by side. Post caption: "The low woodwinds settle it 🖤 Comment the next match-up 👇"

| | BASS CLARINET (slate blue, left) | BASSOON (forest green, right) |
|---|---|---|
| Signature | ENDPIN ANCHOR (plants its floor peg every bar and whirls around it) | TIPTOE (staccato sneak-hops, a stomp on each landing, double damage from behind) |
| Grows per landed hit | Orbit radius | Hop distance + stomp size |
| LV2 | SLAP TONGUE (heavy knockback smack) | STACCATO (hops twice as fast) |
| LV3 | MULTIPHONICS (a ghost twin whirls on the opposite side) | BOCAL HOOK (hooks the enemy, spins it to face away) |
| Below 25 HP | LIGHTS OUT (the arena dims, it sinks into shadow, then erupts) | MOUNTAIN KING (accelerating hops around the enemy, Grieg's theme) |

- **Likely shape (only if the honest fight gives it):** the whirl circle swats a hopper out of the air, but a planted bass
  clarinet can't turn (open to backstabs); MULTIPHONICS covers its back; dream finish: MOUNTAIN KING sneaking around in
  LIGHTS OUT's darkness (keep both balls and the HUD readable in the dim).
- **Sides:** BASS CLARINET left, BASSOON right (its usual side). **Colours:** slate blue vs forest green, both darkish:
  check on the first sheet that they read apart (especially during LIGHTS OUT); if not, a stronger edge glow on each
  ball, never a colour change.
- **Music, mood A (horror suspense trap):** new kit mood `kit/moods/horror_trap.py`: half-time trap drums (sliding 808,
  skittering hats), a creepy detuned music-box motif, creeping low strings, reversed swells into the big hits; no
  woodwinds of its own. The **bass clarinet plays the growling bass line** (chalumeau growl + slap pops), the **bassoon
  the comic staccato line** above it (its hops = its notes); leader fuller and louder, loser thinner and dirtier; LIGHTS
  OUT drops the bed to the music box + 808 for its bar (still at the series' loudness: the one allowed dip at most);
  MOUNTAIN KING quotes Grieg's public-domain theme over the trap beat, accelerating. `loop_bpm(140)` = 141.64 BPM (36
  bars; bar = 1.694 s, FIGHT! at 3.39 s). Original otherwise. (Not chosen: B heist; C dark tango.)
- **VS title card:** BASS CLARINET / VS / BASSOON (BASS CLARINET over two lines if needed, as #38): a bass-clarinet growl,
  the thunder crack, a bassoon blat.
- **Beat sheet:** series standard (bar 0 card · bar 1 count-in · FIGHT! at 3.39 s · unlocks 8–30 s · desperation
  32–44 s · K.O. ~49–53 s · solo + "COMMENT THE / NEXT MATCH-UP" · reassembly into frame 0).
- **Flags:** BASSOON is in the kit. BASS CLARINET: if not in the kit yet, port it from #40 (`kit/fighters/bass_clarinet.py`,
  checked against #40 with the hash and a few frames) and note it in `docs/log/ballfight_studio.md`. Engine changes
  additive; every earlier episode replays identically. LIGHTS OUT's dim is eased (no flash); kit `check` must pass.
- **Effort:** ~40–50 min. Folder: `projects/ballfight_bassclarinet_vs_bassoon/`.

## Board history

- Approved Oct 31 (ideas chat).
