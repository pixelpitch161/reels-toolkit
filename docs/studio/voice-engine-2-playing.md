# Voice Engine 2: how a note can be played

For Aedan's composition system. A Voice Engine 2 instrument is asked for a note with a **`Play`**
(`bf-sound/src/voice/engine2/play.rs`): the pitch and length, plus everything a player does with one note. A part asks
with `MusicCtx::note_played(voice, &play, cap)`. Under Classic, or for a voice that is not on Voice Engine 2 yet, the
same call plays the voice's ordinary note and leaves the rest of the `Play` aside, so asking never breaks anything.

Every instrument can play every one of its techniques (Aedan, 2026-11-26): what is one note's business is asked for in
its `Play`; what is notes in a row (trills, runs, repeated notes, glissandos, clusters, a pedal changed between
chords) is the composition system's, made of single notes. An instrument takes what it can do and ignores the rest: a
piano has no vibrato, an organ no accent by touch. Since stage 1b the piano is the struck-string engine (Aedan's pick
P16) and the organ the pipe engine (O8); since stage 2 the acoustic guitar, electric guitar, bass guitar, ukulele and
harp are the plucked-string engine (A6, E2, B3, U6, H3); since stage 3 the violin, viola, cello, contrabass and
octobass are the bowed-string engine (Vn3, Va3, Vc1, Cb1, Oc2). Later stages add each family's own (a wind's slur and
vibrato, a brass's mute and fall).

The engines work one note at a time at any sample rate, so the same notes could later be played by a plugin.

## The fields

| Field | In plain words | Piano | Organ |
|---|---|---|---|
| `midi` | Which note (60 is middle C; a fraction is between two keys). | yes | yes (under G2 on the pedal board) |
| `dur` | How long the key is held, in seconds. The note rings on after it as the instrument does. | yes: the dampers land when the key comes up (the top keys from F6 have none, as on a real piano) | yes: the pipes stop quickly |
| `vel` | How hard, 0 to 1. | strength: the hammer's speed, harder is louder and brighter | how loud only (a pipe sounds the same however the key goes down) |
| `accent` | An accent on top of `vel`, 0 to 1. | a harder strike (up to 0.12 more strength) | the pipes speak quicker, with more of their start's chiff |
| `slur_from` | Join this note to the one before without a new start. | no (a piano always strikes) | no |
| `bend` | Slide the pitch: a bend, a fall, a scoop. | no | no |
| `swell` | A change over the note's start, from one value to another (0 to 1). | no | the crescendo: the stops come in, in the order a builder adds them, from the softest towards the full set |
| `vibrato` | A wobble, so many cents, so many times a second. | no | the tremulant: the wind shakes the level and a hair of the pitch (0.25 dB and 0.4 cents for every cent asked) |
| `pedals` | The piano's three pedals (below). | yes | no |
| `mute` | `Open`, `Muted`, `Choked`, `Clipped`. | `Muted`: damped by hand on the strings (short, dull); `Choked`, `Clipped`: the dampers land twice as hard | `Choked`, `Clipped`: the quickest stop |
| `together` | How many notes start at once, this one included. | no | a chord of 3 or more sags a hair in pitch, as the wind does under a big chord |
| `technique` | `Normal`, `Harmonic(n)`, `Plucked`. | `Harmonic(n)`: the string touched at 1/n as it is struck (2 sounds an octave up); `Plucked`: the string pulled by a finger inside the piano | no |
| `stops` | The organ's registration (below). | no | yes; none asked is the grand chorus Aedan picked |
| `expression` | A volume pedal over the note, 0 shut to 1 open, with no new start. | no | the swell box: shut is 24 dB quieter and much darker |
| `position` | Where a plucked string is plucked, as a part of the string from the bridge (0.5 the middle). | no | no |
| `string` | Which string a fretted note is played on, from 0 (a guitar's low E, a ukulele's high G). | no | no |

The plucked strings take `midi`, `dur`, `vel`, `accent`, `slur_from`, `bend`, `vibrato`, `mute`, `technique`,
`position` and `string` (below); the rest they leave aside. The bowed strings take those and `swell`, `expression`
and `tie` (below), `position` there being where the bow sits and `string` counted from the lowest string.

`Play::plain(midi, dur, vel)` is a note with none of the extras: open, on its own, no pedal, the organ's own stops.

## The piano's techniques

| Technique | Ask for it with | What it does |
|---|---|---|
| Strength | `vel` | a slow hammer is soft and round; a fast one squeezes the felt and is louder and brighter |
| Accent | `accent` | a harder strike on top of the strength |
| Staccato, legato | `dur` | the dampers land when the key comes up, over about 20 ms |
| Sustain pedal | `pedals.sustain = 1.0` | the dampers stay off: the note rings on and the strings around it ring along softly |
| Half pedal | `pedals.sustain` between 0 and 0.9 | the dampers only touch the strings: the note dies between damped and held |
| Pedal up at a moment | `pedals.lift = Some(s)` | the sustain and middle pedals come up `s` seconds into the note, and the dampers land then |
| Soft pedal (una corda) | `pedals.soft = true` | one string fewer struck, with softer felt: quieter and darker |
| Middle pedal (sostenuto) | `pedals.sostenuto = true` | this note's dampers stay up, the other strings stay quiet (no ring-along) |
| Damped by hand | `mute = Muted` | a hand on the strings: a short, dull note |
| Choked | `mute = Choked` or `Clipped` | the dampers land twice as hard |
| Harmonic | `technique = Harmonic(n)` | the string touched at 1/n of its length: only every n-th overtone rings on (2 an octave up, 3 an octave and a fifth) |
| Plucked inside | `technique = Plucked` | the string pulled aside and let go: no hammer's knock, a softer, rounder start |

Trills, runs, repeated notes, glissandos and clusters are the composition system's, as single notes (a key's repeats
already take turns among three slightly different strikes).

## The organ's techniques

| Technique | Ask for it with | What it does |
|---|---|---|
| Registration | `stops` | which stops sound; `Stops::of(manual, pedal)` names them, and there are ready sets (below) |
| Couplers | `stops.sub`, `stops.octave` | every manual note also an octave under, or over |
| Crescendo | `swell` | the stops come in over the note's start, in a builder's order |
| Swell box | `expression` | a volume pedal: quieter and darker shut |
| Tremulant | `vibrato` | the wind shakes |
| Accent | `accent` | the pipes speak quicker and with more chiff |
| Short stop | `mute = Clipped` | as short as the pipes go |
| Big chord | `together` | the wind sags a hair |

The stops: manual Praestant 16', Bourdon 16', Principal 8', Gedackt 8', Salicional 8', Voix celeste 8', Octave 4',
Rohrflute 4', Quint 2 2/3', Superoctave 2', Mixture IV, Cymbal III, Trumpet 8', Oboe 8'; pedal Subbass 16',
Octavebass 8', Choralbass 4', Posaune 16'. Ready sets: `soft` (a stopped flute), `flutes`, `strings` (the celeste
beating gently), `principal`, `chorus` (8' to the mixture), `grand` (the chorus with the 16' stops, O8 as picked),
`full` (with the cymbal and the reeds), `trumpet` (a trumpet solo over a flute), `oboe`.

## The plucked strings' techniques

ACOUSTIC, ELECTRIC, BASS GUITAR, UKULELE and HARP. A plain note is the instrument's own way: the electric and the bass
with a pick, the rest with the fingers.

| Technique | Ask for it with | What it does |
|---|---|---|
| Strength | `vel` | louder, and a touch brighter (the string leaves a harder pluck sooner) |
| Accent | `accent` | a harder pluck on top of the strength |
| Let ring, staccato | `dur` | the fretting finger lifts (or the hand damps) when the note is let go; a harp's strings ring on unless damped |
| Fingers, pick, nails, thumb | `technique = Fingers`, `Picked`, `Nails`, `Thumb` | the flesh is round, a pick sharp and bright with a click, nails crisp, the thumb deep and soft |
| Pluck point | `position` | near the bridge (0.05) thin and bright (sul ponticello, a harp's pres de la table); over the neck (0.4) round (sul tasto) |
| Which string | `string` | the same note higher up a lower string: rounder |
| Slap, pop | `technique = Slap`, `Pop` | the bass's thumb struck over the last frets, the finger snapping a string on the frets: a bright knock, a crack |
| Tapping | `technique = Tapped` | a fingertip hammered onto the fret, no pluck: softer, rounder |
| Hammer-on, pull-off | `technique = Hammer` with `slur_from` | no new pluck: the pitch changes at once, the string still ringing |
| Slide | `slur_from` | no new pluck: the string slides from that pitch to this one over 60 ms |
| Bend, scoop, fall | `bend` | the pitch pushed up (or down) by so many cents, starting at a moment, over a time; a bend let down is the bent pitch as both the note and `slur_from`, with a bend down from it |
| Vibrato | `vibrato` | the fretting finger rocks the pitch, growing in over a quarter second |
| Harmonic | `technique = Harmonic(n)` | the string touched at 1/n: only every n-th overtone rings (2 an octave up) |
| Pinch harmonic | `technique = PinchHarmonic` | the pick and the thumb's edge together: a high overtone squeals out (with the electric's amp) |
| Palm mute, etouffe | `mute = Muted` | the side of the hand on the strings by the bridge: short and dull (the chug); a harp's string damped by the palm |
| Dead note, chunk | `mute = Choked` | the string muted by the fretting hand: a short, pitchless thud |
| As short as it goes | `mute = Clipped` | let go at once |

Strums, rolled chords, tremolo picking, trills, glissandos and a harp's bisbigliando are the composition system's,
made of single notes (a strum is the chord's notes a few milliseconds apart, low to high going down). The electric's
and the bass's amps play on the fighter's whole part, so a chord's notes distort together, as on a real amp.

## The bowed strings' techniques

VIOLIN, VIOLA, CELLO, CONTRABASS and OCTOBASS. A plain note is bowed (détaché); on the contrabass and the octobass a
plain note under half a second is plucked, as an orchestra's bass plays a walking line, and `Arco` bows it whatever its
length.

| Technique | Ask for it with | What it does |
|---|---|---|
| Strength | `vel` | the bow's speed and weight together: louder, with the same overtone balance (no darker) |
| Accent | `accent` | a bitten start: more speed and weight at once, settling on the note within about 30 ms; a strong accent starts quicker |
| Détaché, legato | `dur` | the bow lifts after the note and the string rings on freely until the finger lifts (an open string until it dies away) |
| Slur | `slur_from` | the bow keeps moving: no new start, the finger moving to the new note in a few milliseconds |
| Tie, slur on | `tie` | this note hands over to the next instead of stopping (the next comes with `slur_from` set to this pitch) |
| Portamento | `slur_from` with a `bend` of 0 cents | the finger slides from the note before over the bend's `over` seconds |
| Bend, scoop, fall | `bend` | the finger slides the pitch so many cents, from a moment, over a time |
| Martelé | `technique = Martele` | a hammered stroke: a strong accent, and the bow stopped on the string at the note's end (its weight let off with its speed) |
| Spiccato | `technique = Spiccato` | the bow dropped onto the string and bouncing off: a short, light note that rings on free |
| Tremolo | `technique = Tremolo` | the bow back and forth 14 times a second all through the note |
| Col legno battuto | `technique = ColLegno` | the string struck with the bow's wood: a dry tick with a ghost of the note |
| Col legno tratto | `technique = ColLegnoTratto` | drawn with the wood: a thin, airy, whistling bowed note |
| Sul ponticello, sul tasto | `position` | where the bow sits, as a part of the string from the bridge (0.02 to 0.3): by the bridge glassy and thin (a lighter, slower bow, as a player does there), over the fingerboard soft and sweet; none is the instrument's own spot |
| Harmonic | `technique = Harmonic(n)` | the string touched lightly at 1/n: the n-th overtone sounds, the fundamental gone (2 an octave up, 3 an octave and a fifth) |
| Con sordino | `mute = Muted` | a mute on the bridge: darker and veiled, a little quieter |
| Pizzicato | `technique = Pizzicato` | plucked by the finger, dying away |
| Snap pizzicato | `technique = SnapPizz` | pulled up and let slap back onto the fingerboard: louder, with a crack |
| Left-hand pizzicato | `technique = LeftHandPizz` | plucked by the stopping hand: softer and thinner |
| Slap | `technique = Slap` | the basses' slap: the string pulled and let hit the fingerboard, a clack with the note |
| Vibrato | `vibrato` | the finger rocks the pitch, so many cents, so many times a second, coming in after `wait`; `None` is the player's own (the violin 24 cents at 5 Hz, the octobass 12 at 4.6), a vibrato of 0 cents none at all |
| Swell, expression | `swell`, `expression` | the bow's speed and weight change over the note: a crescendo, a diminuendo |
| Choked | `mute = Choked` | the hand damps the string as the bow leaves |
| As short as it goes | `mute = Clipped` | a stroke of 60 ms at the most, the bow stopped on the string |
| Which string | `string` | counted from the lowest (0 is the violin's G, the cello's C); the same note higher up a lower string is a heavier string, stopped: fuller and more covered; none, or a string tuned over the note, the highest string that has it |

Double stops and chords are several notes at once (one `Play` each). Trills, runs, glissandos of notes, sautillé and
ricochet runs are the composition system's, made of single notes; a note's repeats already take turns among three
slightly different bows.

## What today's parts pass by themselves

- Every fighter note that goes through `MusicCtx::note` (the attacks' riffs, the music's lines) is played as
  `Play::plain` with its pitch, length and strength, and its repeats take turns among three slightly different takes,
  as a real player never plays a note twice the same.
- The organ fighter's chords pass `together` (the chord's size) and, for CATHEDRAL, PULL ALL STOPS and the title blast,
  a `swell` from halfway (or 0.6) through the stops to the full set.
- Every note's loudness at its `vel` is the old voice's at that velocity (matched once per voice from 0.1 to 1, in
  the middle of its register), so the balance today's parts were written for stands; only the tone changes with
  `vel`. A real piano's soft notes are much quieter than that: the composition system can be given that range when
  it is built.

## How the composition system would ask for a phrase

A sketch, not code that exists yet:

```text
piano, bars 1-2, pedal down for the bar:
  C4 0.5 s  vel 0.55  sustain 1.0  lift 2.0
  E4 0.5 s  vel 0.50  sustain 1.0  lift 1.5
  G4 1.0 s  vel 0.65  sustain 1.0  lift 1.0  accent 0.5      (the top of the phrase leans in)
  C6 1.0 s  vel 0.6   technique Harmonic(2)                    (a bell-like harmonic over it)
organ, under it:
  chord C3 E3 G3  2.0 s  vel 0.8  together 3  stops flutes  swell 0.0 -> 1.0 over 1.5 s   (stops coming in)
  chord F3 A3 C4  1.0 s  vel 0.8  together 3  stops chorus  mute Clipped                  (a short, dry stab)
  chord G2 D3 B3  2.0 s  vel 0.8  together 3  stops strings  expression 0.3 -> 1.0 over 2 s  vibrato 6 cents
```

Each line is one `Play`; a chord is several `Play`s that share `together`. The composition system decides the
musical choices (where the pedal goes down, which note leans in, which stops); the instrument decides how that sounds.
