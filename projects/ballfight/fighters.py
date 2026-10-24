"""BALLFIGHT roster: every fighter's look, weapon, scaling, abilities and sound in ONE table, so a new episode is mostly a
new entry here (or a new ability pair) on the same engine (fight.py) and renderer.

A fighter is a glass ball with a low-poly Pixel Pitch instrument inside. Every fighter has:
  weapon      one of the engine's weapon kinds (fight.py):
                'blade'   a rigid arm spinning around the ball (sword); grows by `grow` per landed hit up to `max_len`
                'volley'  projectiles fired at the opponent every `every` beats; +`grow` keys per volley that lands (a
                          fraction adds up: 0.5 = one more key every second landed volley)
  abilities   unlock1 (at LV2), unlock2 (at LV3), low (once, when HP drops below Rules.LOW_HP). Kinds:
                'shockwave'  expanding ring: knocks back + damages what it passes, clears enemy projectiles
                'shield'     blocks all damage for `dur` s (a blade hitting it bounces and reverses)
                'haste'      weapon speed x `mult` for `dur` s (blade spin / volley rate)
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
}
