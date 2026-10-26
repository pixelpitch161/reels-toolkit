"""BALLFIGHT roster: every fighter's look, weapon, scaling, abilities and sound in ONE table, so a new episode is mostly a
new entry here (or a new ability pair) on the same engine (fight.py) and renderer.

A fighter is a glass ball with a low-poly Pixel Pitch instrument inside. Every fighter has:
  weapon      one of the engine's weapon kinds (fight.py):
                'blade'   a rigid arm spinning around the ball (sword); grows by `grow` per landed hit up to `max_len`;
                          optional `spin_grow` / `spin_max`: spins faster per landed hit instead (or as well)
                'volley'  projectiles fired at the opponent every `every` beats; +`grow` keys per volley that lands (a
                          fraction adds up: 0.5 = one more key every second landed volley)
                'pulse'   a ring blasting out from the ball every `every` beats (on the 16th grid, so haste can roll it);
                          reach `base` beyond the surface, +`grow` per landed pulse up to `max_reach`; haste pulses hit
                          for `sub_dmg` x and don't grow it; `clears`: it swats enemy projectiles in its path
                'beam'    charges for `charge_beats` (aim locked at the opponent's spot predicted `lead` x the charge
                          ahead; the bell tracks the opponent at `track` rad/s between shots), then fires a straight
                          blast every `every` beats: length `base` beyond the surface, width `width`, live for `hold` s,
                          hits once; +`grow` length / +`grow_w` width per landed beam up to `max_len` / `max_w`; pierces
                          (clears) enemy projectiles in its path
                'orbit'   `n0` satellites circling the ball (`base` beyond its surface, `spin` rad/s, hit radius `size`)
                          that hit what they touch (`dmg`, `knock`; each rests `cooldown` s after a hit); +`grow`
                          satellites per landed hit up to `n_max` (<= 8); `parry`: they destroy enemy projectiles they
                          touch (rest `parry_cool` s); haste speeds the orbit, `wobble` (fraction, `wobble_beats` per
                          cycle) also swings its radius while haste lasts
                'scratch_floor'  (#31 v2) the ring is the DJ's record: every `every` beats (haste divides it) the floor
                          jerks forward then back over `dur_beats`; each jerk flings the opponent around the ring centre
                          (tangent + `out` x outward, knock = the strength, `base` .. `max`), the DJ's own ball stays
                          anchored; the first rope the opponent hits within `window` s is a slam: `dmg` x impact speed /
                          the shared speed (capped at `cap`); +`grow` strength per landed slam
                'tether'  (#31 v2) patch cables: every `every` beats a plug flies at the opponent (`plug_speed`,
                          `plug_r`) while fewer than its slots (`n0` .. `n_max`, +`grow` per landed plug) are in; a plug
                          that lands jabs (`plug_dmg`) and plugs a cable in for `life_beats`; pulses run down every
                          cable every `volt16` 16ths (haste divides it) at `volt_speed`, `volt_dmg` each; the leash keeps
                          the opponent within `length` (it bounces off) unless it is flung harder than `snap` x cables
              volley option `curve` (deg/s): projectiles home in on the opponent at that turn rate and fade after
                          `life` s; `hit_r`: their hit radius (default Rules.KEY_R)
              volley option `bounce` (N): projectiles ricochet off the ropes up to N times before they leave
                'line'    a row of pipes erupts from the floor toward the opponent every `every` beats: front speed
                          `speed`, each pipe up `linger` s, half-width `width` / 2, stops at the ropes, hits once; length
                          `base`, +`grow` per landed row up to `max_len`
                'trap'    every `every` beats a string is stretched rope to rope, square to the line between the two
                          balls at `at` of the way; crossing an enemy string costs `dmg` (per string once per `cool` s;
                          `knock` = a stumble, no turn); the owner passes freely; keeps int(n_vol) strings (`n0`, +`grow`
                          per landed crossing up to `n_max`; the oldest drops)
  abilities   unlock1 (at LV2), unlock2 (at LV3), low (once, when HP drops below Rules.LOW_HP). Kinds:
                'shockwave'  expanding ring: knocks back + damages what it passes, clears enemy projectiles
                'shield'     blocks all damage for `dur` s (a blade hitting it bounces and reverses)
                'haste'      weapon speed x `mult` for `dur` s (blade spin / volley rate / pulse rate)
                'rewind'     undo the last `window` s of damage taken: HP back to its value then and the ball zips back
                             along its own path in `zip_beats` beats (intangible meanwhile); waits until that would undo
                             >= `min_undo` HP
                'lash'       (#31 v2) for `beats` beats the cables lash side to side in a saw-tooth: n arms (the
                             tether's slots, >= `n_min`) of length `len`, `spread` deg apart around the aim, sweep `sweep`
                             deg every `period_beats` then snap back (arm j lags `lag`); a forward sweep that crosses the
                             opponent cuts (`dmg`, `knock`), each arm resting `cool` s
              durations: `dur` in s, or `dur_beats` in beats at the episode's tempo
                'arc'        a sweeping arc of `n` projectiles across the ring over `beats` beats
                'wave'       a big slow shockwave across the whole ring (like 'shockwave', harder, clears everything)
                'slam'       telegraphed area hit at the opponent's predicted spot after `tele` beats
              unlock abilities recur every `every_bars` bars after they unlock (only the first use gets a caption)
  look        colour (rim, HUD, captions), corner, the instrument's best view inside the ball (yaw centre, swing, roll),
              weapon style for the renderer
  sound       pp: the Pixel Pitch voice (projects/which_instrument_wins/pp_bridge.py roster name), register, hit idiom

Add a fighter: copy an entry, give it a weapon + 3 abilities from the kinds above (or add a kind to fight.py), a colour
and an idiom; the next episode's timeline.py names the two fighters."""

FIGHTERS = {
    'GUITAR': dict(
        color=(255, 84, 72), dark=(150, 34, 40), corner='red',
        view=(12, 24, 30),
        weapon=dict(kind='blade', style='neck',           # the guitar's own neck, frets and headstock, swung like a sword
                    base=0.62, grow=0.058, max_len=1.95,  # reach beyond the ball's surface: 5 frets + headstock -> 24 frets
                    frets0=5, spin=6.80, dmg=6.5, cooldown=0.30, parry=True, knock=0.9),
        abilities=dict(
            unlock1=dict(name='POWER CHORD', kind='shockwave', speed=7.5, radius=3.4, dmg=3.0, knock=1.2, every_bars=4),
            unlock2=dict(name='SHRED', kind='haste', mult=2.0, dur=4.0, every_bars=6),
            low=dict(name='FEEDBACK', kind='wave', speed=4.5, radius=9.0, dmg=8.0, knock=1.5)),
        pp='GUITAR', register=(52, 76), idiom='power_chord'),
    'PIANO': dict(
        color=(84, 156, 255), dark=(30, 60, 150), corner='blue',
        view=(225, 20, 0),
        weapon=dict(kind='volley', style='keys',           # piano keys fired on the beat, each one a note up the scale
                    n0=1, grow=0.5, n_max=14, every=2, speed=5.4, dmg=0.5,
                    spread=7.0, max_fan=64.0, size=(0.30, 0.13)),
        abilities=dict(
            unlock1=dict(name='SUSTAIN', kind='shield', dur=2 * 1.8485, every_bars=6),
            unlock2=dict(name='GLISSANDO', kind='arc', n=15, sweep=150.0, beats=1, speed=5.6, dmg=0.5, every_bars=4),
            low=dict(name='GRAND SLAM', kind='slam', tele=2, radius=1.35, dmg=11.0, knock=1.6)),
        pp='PIANO', register=(48, 84), idiom='keys'),

    # ---- #28 BALLFIGHT #2: brute force vs finesse (numbers tuned once with ballfight_drums_vs_violin/tune.py, frozen)
    'DRUMS': dict(
        color=(255, 146, 40), dark=(160, 84, 16), corner='red',      # orange (series colour table, IDEAS)
        view=(12, 30, 0),
        weapon=dict(kind='pulse', style='drumhead',        # a drumhead ring blasts out on every beat: the fight's groove
                    every=1, base=0.9, grow=0.07, max_reach=2.3, speed=9.0, dmg=2.5, sub_dmg=0.45, knock=0.75,
                    clears=True),
        abilities=dict(
            unlock1=dict(name='DRUM ROLL', kind='haste', mult=4.0, dur_beats=4, every_bars=4),
            unlock2=dict(name='CRASH CYMBAL', kind='shield', dur_beats=6, every_bars=6),
            low=dict(name='DOUBLE KICK', kind='slam', tele=2, radius=1.35, dmg=11.0, knock=1.6)),
        pp='DRUMS', register=(36, 50), idiom='kit'),
    'VIOLIN': dict(
        color=(176, 98, 255), dark=(96, 48, 168), corner='blue',     # violet (series colour table, IDEAS)
        view=(12, 26, 28),
        weapon=dict(kind='blade', style='bow',             # the bow, spun like a rapier: long, thin, light; faster per hit
                    base=1.8, grow=0.0, max_len=1.8, frets0=0, spin=2.4, spin_grow=0.7, spin_max=14.0,
                    dmg=1.8, cooldown=0.18, parry=False, knock=0.55),
        abilities=dict(
            unlock1=dict(name='PIZZICATO', kind='arc', n=5, sweep=64.0, beats=1, speed=6.0, dmg=1.4, every_bars=3),
            unlock2=dict(name='TREMOLO', kind='haste', mult=2.5, dur=3.0, every_bars=6),
            low=dict(name='SCREECH', kind='wave', speed=4.5, radius=9.0, dmg=8.0, knock=1.5)),
        pp='VIOLIN', register=(62, 88), idiom='bowed'),

    # ---- #30 BALLFIGHT #4: the "most hated instrument" final. v2 (BALLFIGHT v2 redesign, user Oct 24): the bag and the
    # bellows are the weapons (numbers tuned once with ballfight_bagpipes_vs_accordion/tune.py, frozen). v1 (drone aura
    # vs bellows rings: kinds 'aura' / 'bellows', still in fight.py) is in git history (seed 1581).
    #   'inflate'  every `every` beats the ball swells to `scale0` x R (+`grow` per landed hit, up to `scale_max`) over
    #              `inflate_beats` (mass x size, speed x `slow`: heavy body slams for up to `slam_dmg`), then lets go and
    #              rockets like a released balloon for `fly_beats` (+`fly_grow` per landed hit, up to `fly_max`) at
    #              `fly_speed` x V, zig-zagging (a seeded turn of `zig_turn` deg every `zig` s, alternating sides, `home`
    #              blend toward the enemy) while it deflates; `fly_dmg` on contact; `cool` s between contact hits.
    #              Haste divides the cycle (HIGHLAND FLING: two flights per bar)
    #   'bite'     every `every` beats the two halves fly apart with the bellows stretched between them like jaws (aimed
    #              at the enemy's spot `lead` x the opening ahead), and snap shut `open16` 16ths later: a hit if the enemy
    #              is within R + reach and `half` deg of the aim; +`grow` reach per landed main bite (haste bites hit for
    #              `sub_dmg` x and don't grow it)
    #   ability 'clamp' (FULL BELLOWS): the bellows stretch across the whole ring for `tele` beats, then clamp: a
    #              ring-wide bite (lands unless shielded), knocking the enemy toward the accordion
    'BAGPIPES': dict(
        color=(64, 196, 88), dark=(26, 104, 42), corner='red',       # tartan green (series colour table, IDEAS)
        view=(42, 22, 0),
        weapon=dict(kind='inflate', style='balloon',       # BAG INFLATE: swell, then rocket around like a released balloon
                    every=4, inflate_beats=2, scale0=1.4, grow=0.015, scale_max=1.65, mass_k=1.0, slow=0.55,
                    fly_beats=1.25, fly_grow=0.06, fly_max=1.75, fly_speed=2.6, fly_fade=0.35, zig=(0.10, 0.17),
                    zig_turn=(35.0, 70.0), home=0.35, aim_wobble=20.0, slam_dmg=5.0, slam_min=0.3, fly_dmg=6.8,
                    cool=0.3, knock=1.2, rest_beats=0.25),
        abilities=dict(
            unlock1=dict(name='WAR CRY', kind='shockwave', speed=7.5, radius=3.4, dmg=3.0, knock=1.2, every_bars=4),
            unlock2=dict(name='HIGHLAND FLING', kind='haste', mult=2.0, dur_beats=8, every_bars=6),
            low=dict(name='BAG BURST', kind='wave', speed=4.5, radius=9.0, dmg=8.0, knock=1.5)),
        pp='BAGPIPES', register=(67, 81), idiom='drone'),
    'ACCORDION': dict(
        color=(236, 82, 214), dark=(140, 34, 124), corner='blue',    # magenta (series colour table, IDEAS)
        view=(72, 22, 0),
        weapon=dict(kind='bite', style='jaws',             # BELLOWS BITE: the halves open like jaws and snap shut
                    every=2, base=0.9, grow=0.07, max_reach=2.0, half=38.0, open16=2, lead=0.6, dmg=2.6, sub_dmg=0.6,
                    knock=0.9),
        abilities=dict(
            unlock1=dict(name='POLKA', kind='haste', mult=2.0, dur_beats=8, every_bars=5),
            unlock2=dict(name='SQUEEZEBOX', kind='shield', dur_beats=6, every_bars=6),
            low=dict(name='FULL BELLOWS', kind='clamp', tele=1, dmg=9.0, knock=1.5)),
        pp='ACCORDION', register=(55, 79), idiom='reeds'),
    # ---- #32 BALLFIGHT #6: biggest vs smallest, lowest vs highest. v2 (BALLFIGHT v2 redesign, user Oct 24): the bell
    # swallows (OOM-PAH SWALLOW), the flute charms cobras (SNAKE CHARMER); numbers tuned once with
    # ballfight_tuba_vs_flute/tune.py, frozen
    'TUBA': dict(
        color=(104, 92, 236), dark=(52, 44, 140), corner='red',      # indigo (series colour table, IDEAS)
        view=(0, 32, 0),
        weapon=dict(kind='swallow', style='bell',          # OOM-PAH SWALLOW: oom = inhale + swallow, pah = spit it out
                    every=4, inhale=1.0, pah=2.0, base=1.8, grow=0.12, max_reach=3.0, suck=26.0, catch=0.12,
                    aim_rate=6.0, dmg=3.0, dmg_spit=5.0, dmg_rope=5.0, spit_boost=2.2, spit_t=0.9, jitter=25.0),
        abilities=dict(
            unlock1=dict(name='OOMPAH', kind='haste', mult=2.0, dur_beats=8, every_bars=4),
            unlock2=dict(name='BRASS WALL', kind='shield', dur_beats=6, every_bars=6),
            low=dict(name='SUBSONIC', kind='slam', tele=2, radius=1.5, dmg=11.0, knock=1.6)),
        pp='TUBA', register=(33, 55), idiom='oompah'),
    'FLUTE': dict(
        color=(222, 232, 246), dark=(120, 134, 162), corner='blue',  # silver (series colour table, IDEAS)
        view=(0, 18, 32),
        weapon=dict(kind='striker', style='cobra',         # SNAKE CHARMER: cobras rise from dropped baskets, strike on the beat
                    every=1, drop_every=4, max_baskets=2, rise=1.0, base=1.4, grow=0.07, max_reach=2.4, slack=0.25,
                    t_strike=0.12, t_min=0.07, speed_grow=0.003, head_r=0.14, dmg=1.08, knock=0.5, slither=2.2),
        abilities=dict(
            unlock1=dict(name='FLUTTER TONGUE', kind='haste', mult=2.0, dur_beats=8, every_bars=4),
            unlock2=dict(name='SNAKE DANCE', kind='dance', dur_beats=4, every_bars=5),
            low=dict(name='PICCOLO', kind='wave', speed=6.0, radius=9.0, dmg=8.0, knock=1.5)),
        pp='FLUTE', register=(72, 93), idiom='charmer'),

    # ---- #34 BALLFIGHT #8: the brass section's pecking order (tuned once with ballfight_trombone_vs_horn/tune.py, frozen;
    # v2 redesign: HORN = THE HUNT, re-tuned at damage scale 0.60; v3 HORN = ECHO CALL + FINAL FLICK (Oct 25), re-tuned
    # at damage scale 0.72, the TROMBONE unchanged; THE HUNT's 'pack' kind stays in the engine)
    'TROMBONE': dict(
        color=(120, 240, 184), dark=(44, 150, 104), corner='red',     # mint (series colour table, IDEAS)
        view=(205, 18, 30),
        weapon=dict(kind='blade', mode='thrust', style='slide',  # the slide shoots out at the enemy on the beat, pulls back
                    every=1, rest=0.30, base=0.95, grow=0.08, max_len=2.5, frets0=1,
                    t_out=0.08, t_hold=0.06, t_back=0.18, aim_rate=7.0, dmg=3.5, knock=0.9, parry=True),
        abilities=dict(
            unlock1=dict(name='SAD TROMBONE', kind='shockwave', speed=6.0, radius=3.4, dmg=3.0, knock=1.0,
                         slow=dict(mult=0.6, dur=1.0), every_bars=4),
            unlock2=dict(name='POWER SLIDE', kind='haste', mult=2.0, dur_beats=8, every_bars=6),
            low=dict(name='PEDAL TONE', kind='slam', tele=2, radius=1.35, dmg=24.0, knock=1.6)),
        pp='TROMBONE', register=(40, 65), idiom='slide'),
    'HORN': dict(
        color=(200, 112, 64), dark=(120, 58, 30), corner='blue',     # copper (series colour table, IDEAS)
        view=(0, 22, 0),
        weapon=dict(kind='echo', style='echo_call',        # v3 ECHO CALL: a round blast on every beat, echo ghosts repeat it
                    every=1, base=0.9, speed=7.0, dmg=4.3, knock=0.5, n0=0, n_max=4, grow=1.0, decay=0.85,
                    echo_reach=0.85),
        abilities=dict(
            unlock1=dict(name='STOPPED HORN', kind='shield', dur_beats=6, every_bars=6),     # a copper hand plugs the bell
            unlock2=dict(name='HORN RIP', kind='haste', mult=2.0, dur_beats=8, every_bars=6),  # half-beat echoes
            low=dict(name='FINAL FLICK', kind='flick', wind_beats=1, reach=2.4, glove_r=0.4, dmg=14.0, boost=2.6,
                     rope_t=0.9, dmg_rope=10.0)),                     # the glove flicks the enemy into the ropes
        pp='HORN', register=(53, 77), idiom='horn_call'),

    # ---- #35 BALLFIGHT #9: cathedral vs angels (numbers tuned once with ballfight_organ_vs_harp/tune.py, frozen)
    'ORGAN': dict(
        color=(196, 52, 104), dark=(112, 24, 58), corner='red',      # wine (series colour table, IDEAS)
        view=(0, 22, 0),
        weapon=dict(kind='line', style='pipes',            # a row of organ pipes erupts from the floor toward the enemy
                    every=2, base=1.6, grow=0.06, max_len=5.6, speed=6.0, linger=0.55, width=0.3, dmg=4.0, knock=0.8),
        abilities=dict(
            unlock1=dict(name='CATHEDRAL', kind='shield', dur_beats=6, every_bars=6),
            unlock2=dict(name='PULL ALL STOPS', kind='haste', mult=2.0, dur_beats=8, every_bars=6),
            low=dict(name='TOCCATA', kind='slam', tele=2, radius=1.5, dmg=11.0, knock=1.6)),
        pp='ORGAN', register=(36, 84), idiom='organ'),
    'HARP': dict(
        color=(244, 228, 190), dark=(166, 146, 104), corner='blue',  # pearl, warm ivory (series colour table, IDEAS)
        view=(8, 26, 0),
        weapon=dict(kind='trap', style='strings',          # glowing strings stretched across the ring: tripwires
                    every=2, n0=1, grow=0.5, n_max=7, dmg=1.8, cool=0.4, at=0.5, knock=0.25),
        abilities=dict(
            unlock1=dict(name='PLUCK', kind='arc', n=7, sweep=70.0, beats=1, speed=6.0, dmg=1.2, every_bars=3),
            unlock2=dict(name='ANGEL WINGS', kind='shield', dur_beats=6, every_bars=6),
            low=dict(name='47 STRINGS', kind='wave', speed=4.5, radius=9.0, dmg=8.0, knock=1.5)),
        pp='HARP', register=(50, 84), idiom='harp'),

    # ---- #33 BALLFIGHT #7: little brother vs big brother. v2 (BALLFIGHT v2 redesign, user Oct 24): the way each is
    # played is the weapon (weapon kinds 'surf' / 'rake', ability kinds 'spout' / 'whip' / 'sweep' in fight.py);
    # numbers tuned once with ballfight_ukulele_vs_acoustic/tune.py, frozen. (v1's 'dash' / 'cone' stay in the engine.)
    #   'surf'  SURF'S UP: every `every` beats the strum raises a wave the ball rides at the enemy: a speed burst of
    #           p0 + p_h x height, a front w0 + w_h x height wide that crashes into the enemy once for `dmg` and pushes it
    #           along the wave (knock grows with the height); +`grow` height per landed wave up to `max_h`
    #   'rake'  SIX-STRING RAKE: every `every` beats six string lines (`gap` apart, `base` long, pointing at the enemy)
    #           sweep `sweep` to either side across the ring in front in `stroke` s, alternating down / up; one hit per
    #           stroke flicks the enemy along the stroke; +`grow` length per landed stroke up to `max_len`
    'UKULELE': dict(
        color=(255, 138, 178), dark=(160, 62, 104), corner='red',     # pink (series colour table, IDEAS)
        view=(10, 22, 28),
        weapon=dict(kind='surf', style='surf',              # SURF'S UP: each strum raises a rolling wave it rides in on
                    every=2, base=0.5, grow=0.03, max_h=1.6, p0=0.8, p_h=0.5, w0=0.35, w_h=0.35, life=1.1,
                    aim_k=0.55, dmg=5.0, sub_dmg=0.6, sub_power=0.8, knock=1.0),
        abilities=dict(
            unlock1=dict(name='CHUNK', kind='shockwave', speed=7.5, radius=3.0, dmg=3.0, knock=1.2, every_bars=4),
            unlock2=dict(name='SPEED STRUM', kind='haste', mult=2.0, dur_beats=8, every_bars=6),
            low=dict(name='UKE-NADO', kind='spout', speed=2.3, turn=2.6, life=3.2, radius=0.5, tick=0.28, dmg=2.4,
                     max_ticks=4, knock=0.7)),
        pp='UKULELE', register=(60, 81), idiom='uke_strum'),
    'ACOUSTIC': dict(
        color=(214, 160, 100), dark=(128, 84, 42), corner='blue',     # wood tan (series colour table, IDEAS)
        view=(14, 22, 330),
        weapon=dict(kind='rake', style='rake',              # SIX-STRING RAKE: a giant strum, alternating down / up
                    every=1, base=1.2, grow=0.03, max_len=3.0, sweep=0.6, gap=0.085, stroke=0.16, aim_k=0.47,
                    dmg=2.1, sub_dmg=0.5, knock=0.7),
        abilities=dict(
            unlock1=dict(name='CAPO', kind='haste', mult=2.0, dur_beats=8, every_bars=5),
            unlock2=dict(name='FINGERPICK', kind='whip', n=6, beats=1, len=2.6, dmg=1.3, knock=0.35, t_ext=0.07,
                         every_bars=3),
            low=dict(name='BIG STRUM', kind='sweep', beats=2, reach=9.0, dmg=11.0, knock=1.6)),
        pp='ACOUSTIC', register=(40, 76), idiom='open_strum'),

    # ---- #29 BALLFIGHT #3: the lead-horn argument. v3 (BALLFIGHT v2 redesign, user Oct 24): the instrument's body is the
    # weapon (VALVE COMBO, STRAP FLAIL); numbers tuned once with ballfight_trumpet_vs_sax/tune.py, frozen
    'TRUMPET': dict(
        color=(255, 214, 60), dark=(168, 124, 18), corner='red',     # yellow (series colour table, IDEAS)
        view=(0, 16, 0),                                   # side-on, bell right (#29's renderer turns the valves to the aim)
        weapon=dict(kind='combo', style='pistons',         # VALVE COMBO: three brass pistons punch out 1-2-3, steam
                    every=2, punches=3, gap=0.5, spread=0.15, aim_rate=8.0, head_r=0.1,
                    base=0.6, grow=0.05, max_len=1.7, t_out=0.05, t_hold=0.04, t_back=0.09,
                    dmg=1.2, dmg3=2.0, knock=0.35, knock3=1.0),
        abilities=dict(
            unlock1=dict(name='FANFARE', kind='shockwave', speed=7.5, radius=3.4, dmg=3.0, knock=1.2, every_bars=4),
            unlock2=dict(name='DOUBLE TONGUE', kind='haste', mult=2.0, dur_beats=8, every_bars=6),
            low=dict(name='HIGH C', kind='beam', charge_beats=1, lead=1.0, len=7.0, width=1.1, dmg=9.0, knock=1.6,
                     hold=0.3)),
        pp='TRUMPET', register=(60, 84), idiom='combo'),
    'SAX': dict(
        color=(36, 200, 176), dark=(14, 108, 94), corner='blue',     # teal (series colour table, IDEAS)
        view=(180, 20, 0),
        weapon=dict(kind='flail', style='strap',           # STRAP FLAIL: swings on its neck strap, wraps the enemy
                    base=1.2, grow=0.07, max_len=2.2, spin0=3.0, spin_acc=3.5, spin_max=9.0, head_r=0.2,
                    dmg=7.4, dmg_min=0.4, knock=0.8, cooldown=0.3, wrap_beats=2, reel=2.3, yank=1.0, haste_snap=True),
        abilities=dict(
            unlock1=dict(name='SMOKE SCREEN', kind='shield', dur_beats=6, every_bars=6),
            unlock2=dict(name='BEBOP RUN', kind='haste', mult=1.6, dur=4.0, every_bars=5),
            low=dict(name='HONK', kind='slam', tele=2, radius=1.35, dmg=11.0, knock=1.6)),
        pp='SAX', register=(49, 77), idiom='flail'),

    # ---- #31 BALLFIGHT #5 (v2 redesign, user Oct 24): THE RING IS THE RECORD vs PATCH CABLES (tuned once with
    # ballfight_turntables_vs_synth/tune.py, frozen)
    'TURNTABLES': dict(
        color=(190, 245, 60), dark=(96, 140, 20), corner='red',      # lime (series colour table, IDEAS)
        view=(20, 22, 0), pitch=45,                        # pitch: the deck tipped toward the camera (platter + label)
        weapon=dict(kind='scratch_floor', style='record',  # the ring is the DJ's record: every bar a scratch jerks the
                    every=4, dur_beats=1, base=1.3, grow=0.06, max=2.2,     # floor and flings the opponent into the ropes
                    out=0.4, window=0.8, dmg=1.6, cap=1.6),
        abilities=dict(
            unlock1=dict(name='SCRATCH', kind='haste', mult=2.0, dur_beats=8, every_bars=6),   # scratches twice as often
            unlock2=dict(name='REWIND', kind='rewind', window=2.0, zip_beats=1, min_undo=5.0, every_bars=6),
            low=dict(name='BASS DROP', kind='slam', tele=2, radius=1.35, dmg=11.0, knock=1.6)),
        pp='TURNTABLES', register=(50, 74), idiom='scratch'),
    'SYNTH': dict(
        color=(70, 228, 255), dark=(20, 120, 150), corner='blue',    # cyan (series colour table, IDEAS)
        view=(10, 20, 0), pitch=28,
        weapon=dict(kind='tether', style='cables',         # patch cables: plugs leash the opponent, a voltage pulse per
                    every=2, n0=1, grow=0.5, n_max=4,      # arpeggio note runs down every cable
                    plug_speed=6.0, plug_r=0.14, plug_dmg=1.0, life_beats=8,
                    volt16=2, volt_speed=9.0, volt_dmg=0.17, length=2.4, snap=0.15),
        abilities=dict(
            unlock1=dict(name='LFO', kind='haste', mult=2.0, dur=3.0, every_bars=6),           # pulses at double rate
            unlock2=dict(name='SAWTOOTH', kind='lash', beats=4, n_min=2, len=2.2, spread=24, sweep=70, period_beats=0.5,
                         lag=0.15, dmg=1.6, knock=0.5, cool=0.3, every_bars=5),
            low=dict(name='WHITE NOISE', kind='wave', speed=4.5, radius=9.0, dmg=8.0, knock=1.5)),
        pp='SYNTH', register=(57, 81), idiom='arp'),
}
