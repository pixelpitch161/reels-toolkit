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
}
