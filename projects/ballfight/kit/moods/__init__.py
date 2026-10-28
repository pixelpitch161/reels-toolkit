"""Reusable music beds (config MOOD = '<module>'): each a Bed class taking the episode's tempo and chords from the
timeline, with the sections kit.music calls in order (title, vs, title_rest, count_in, fight_start, fight, boom,
clash, ko, stinger, after_ko, victory, last_bars) and its own buses, reverb and compressor.

    big_top          #41 Flute vs Trombone: a sinister circus (calliope, bass drum, choked cymbal, snare, glock)
    baroque_storm    #37 Clarinet vs Oboe: harpsichord continuo, racing strings, timpani, thunder
    stadium_rock     #44 Electric vs Acoustic: minor arena rock in two arrangements that follow the leader (plugged:
                     fuzz rhythm guitar, pick bass, stadium kit; unplugged: strum shimmer, upright bass, hand percussion),
                     loudness-matched, crossfaded over a beat at each lead change; config PLUGGED names the plugged side
    prog_duel        #45 Piano vs Organ: a prog-rock keyboard-duel bed (drums + bass guitar, 3+3+2 pushes, no keyboards)
    balkan_fire      #28 Drums vs Violin: dark Balkan fire, drum-less (upright-bass oom-pah, musette accordion, pad,
                     cimbalom shimmer); the card sections are new (#28 had no card)
    orchestral_drill #42 Viola vs Violin: a UK drill beat (sliding 808, hat rolls, half-time clap) + cellos and basses
    brass_house      #36 Trombone vs Tuba: dark minor house, no bass synth and no brass (the fighters are the low end)
A new mood: copy one, keep its section methods, change the instruments and parts (or port an older episode's bed)."""
