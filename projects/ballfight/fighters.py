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

    # ---- #30 BALLFIGHT #4: the "most hated instrument" final (numbers tuned once with
    # ballfight_bagpipes_vs_accordion/tune.py, frozen). Two new weapon kinds, both on the 16th grid (fight.py):
    #   'aura'     a drone ring of radius R + reach around the ball: hurts the opponent for `dmg` on every tick (every
    #              `every` beats) while it is inside; +`grow` reach on the first landed tick of each contact, up to
    #              `max_reach`; no knock. Haste = more ticks (damage x mult)
    #   'bellows'  strokes every `every` beats alternating push / pull: a push blasts a ring out to the reach (knock
    #              away), a pull collapses a ring in from `pull_k` x the reach (`pull_speed` x speed) and yanks the
    #              opponent toward the ball; +`grow` reach per landed main stroke, knock x (1 + `force_grow` x reach
    #              gained); haste strokes hit for `sub_dmg` x and don't grow it
    'BAGPIPES': dict(
        color=(64, 196, 88), dark=(26, 104, 42), corner='red',       # tartan green (series colour table, IDEAS)
        view=(42, 22, 0),
        weapon=dict(kind='aura', style='drone',            # the drone: a ring around the ball that hurts while you're in it
                    every=0.5, base=0.9, grow=0.12, max_reach=1.9, dmg=0.85),
        abilities=dict(
            unlock1=dict(name='WAR CRY', kind='shockwave', speed=7.5, radius=3.4, dmg=3.0, knock=1.2, every_bars=4),
            unlock2=dict(name='DOUBLE DRONE', kind='haste', mult=2.0, dur_beats=8, every_bars=6),
            low=dict(name='BAG BURST', kind='wave', speed=4.5, radius=9.0, dmg=8.0, knock=1.5)),
        pp='BAGPIPES', register=(67, 81), idiom='drone'),
    'ACCORDION': dict(
        color=(236, 82, 214), dark=(140, 34, 124), corner='blue',    # magenta (series colour table, IDEAS)
        view=(72, 22, 0),
        weapon=dict(kind='bellows', style='bellows',       # push on one bar, suck on the next: wheezy chord stabs
                    every=2, base=0.9, grow=0.09, max_reach=2.2, speed=9.0, pull_k=1.7, pull_speed=0.45, dmg=3.0,
                    sub_dmg=0.5, knock=1.0, force_grow=0.5),
        abilities=dict(
            unlock1=dict(name='POLKA', kind='haste', mult=2.0, dur_beats=8, every_bars=5),
            unlock2=dict(name='SQUEEZEBOX', kind='shield', dur_beats=6, every_bars=6),
            low=dict(name='FULL BELLOWS', kind='slam', tele=2, radius=1.35, dmg=11.0, knock=1.6)),
        pp='ACCORDION', register=(55, 79), idiom='reeds'),
    # ---- #32 BALLFIGHT #6: biggest vs smallest, lowest vs highest (numbers tuned once with ballfight_tuba_vs_flute/tune.py)
    'TUBA': dict(
        color=(104, 92, 236), dark=(52, 44, 140), corner='red',      # indigo (series colour table, IDEAS)
        view=(0, 32, 0),
        weapon=dict(kind='pulse', style='bell_wave',       # a slow, huge brass shockwave every bar + a constant gravity pull
                    every=4, base=1.8, grow=0.08, max_reach=3.0, speed=4.0, dmg=6.0, sub_dmg=0.45, knock=1.0,
                    clears=True, pull=1.2, pull_grow=0.08, pull_max=2.4),
        abilities=dict(
            unlock1=dict(name='OOMPAH', kind='haste', mult=2.0, dur_beats=8, every_bars=4),
            unlock2=dict(name='BRASS WALL', kind='shield', dur_beats=6, every_bars=6),
            low=dict(name='SUBSONIC', kind='slam', tele=2, radius=1.35, dmg=11.0, knock=1.6)),
        pp='TUBA', register=(33, 55), idiom='oompah'),
    'FLUTE': dict(
        color=(222, 232, 246), dark=(120, 134, 162), corner='blue',  # silver (series colour table, IDEAS)
        view=(0, 18, 32),
        weapon=dict(kind='volley', style='darts', on16=True,   # rapid tiny fast darts on every beat (16th grid: haste)
                    n0=1, grow=0.5, n_max=9, every=1, speed=7.5, dmg=0.32, spread=9.0, max_fan=70.0, size=(0.22, 0.06)),
        abilities=dict(
            unlock1=dict(name='FLUTTER TONGUE', kind='haste', mult=2.0, dur_beats=8, every_bars=4),
            unlock2=dict(name='TRILL', kind='arc', n=12, sweep=120.0, beats=1, speed=7.0, dmg=0.5, every_bars=4),
            low=dict(name='PICCOLO', kind='wave', speed=6.0, radius=9.0, dmg=8.0, knock=1.5)),
        pp='FLUTE', register=(72, 93), idiom='flutter'),

    # ---- #34 BALLFIGHT #8: the brass section's pecking order (tuned once with ballfight_trombone_vs_horn/tune.py, frozen)
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
        weapon=dict(kind='volley', style='arrow',          # hunting-call arrows curving in on the enemy (#29's `curve`)
                    n0=1, grow=0.5, n_max=6, every=2, speed=3.0, dmg=1.4, spread=16.0, max_fan=80.0,
                    curve=52.0, curve_grow=4.3, curve_max=137.0, life=4.0),
        abilities=dict(
            unlock1=dict(name='STOPPED HORN', kind='shield', dur_beats=6, every_bars=6),
            unlock2=dict(name='HORN RIP', kind='arc', n=7, sweep=120.0, beats=1, speed=3.6, dmg=1.6, curve=True,
                         every_bars=4),
            low=dict(name='TALLY-HO', kind='wave', speed=4.5, radius=9.0, dmg=20.0, knock=1.5)),
        pp='HORN', register=(53, 77), idiom='horn_call'),
}
