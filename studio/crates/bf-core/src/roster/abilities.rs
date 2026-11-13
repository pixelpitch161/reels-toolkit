//! The abilities' roster settings: [`AbilityCfg`] and its kinds. The five kinds FLUTE and TROMBONE fight with and
//! `shield` are typed here; each further kind of the kit has its own module (`roster::abilities::<kind>`), owned by
//! its fighter's port, and loads as [`AbilityKind::Unsupported`] until that module's `PORTED` is true.

use std::collections::BTreeMap;

use serde_json::Value as Json;

use crate::roster::{Entry, RosterError, join};

pub mod altissimo;
pub mod arc;
pub mod barrage;
pub mod beam;
pub mod cannons;
pub mod clamp;
pub mod company_front;
pub mod copy;
pub mod detune;
pub mod double_stop;
pub mod drum_roll;
pub mod encore;
pub mod endless;
pub mod flash_spin;
pub mod flick;
pub mod fork;
pub mod heartbeat;
pub mod hook;
pub mod joke;
pub mod lash;
pub mod lights_out;
pub mod melody;
pub mod mountain;
pub mod octave;
pub mod offbeat;
pub mod prelude;
pub mod rewind;
pub mod rhapsody;
pub mod runoff;
pub mod runway;
pub mod slap;
pub mod smash;
pub mod snap;
pub mod snip;
pub mod spout;
pub mod stagedive;
pub mod stampede;
pub mod stun;
pub mod subtone;
pub mod sustain;
pub mod swan;
pub mod sweep;
pub mod tapping;
pub mod twin;
pub mod vibrato;
pub mod whip;

/// An ability.
#[derive(Clone, Debug, PartialEq)]
pub struct AbilityCfg {
    /// Its name, such as `FLUTTER TONGUE`.
    pub name: String,
    /// Its kind.
    pub kind: AbilityKind,
    /// Every number in the entry (ints as `f64`; all of the roster's are exact), by name: `every_bars`,
    /// `dur_beats`, `mult`, `speed`, `radius`, `dmg`, `knock`, `tele`, ...
    pub params: BTreeMap<String, f64>,
    /// A shockwave's or wave's `slow` (`ab.get('slow')`: the enemy it hits moves at `mult` x speed for `dur` s).
    /// `None` when the entry has none or one Python counts as false (an empty table).
    pub slow: Option<SlowCfg>,
    /// A wave's `detune` (OBOE's ILL WIND, `fight.py` 2845 to 2846, 3069 to 3071): `None` unless the option is ported
    /// ([`detune::PORTED`]) and the entry has one Python counts as true.
    pub detune: Option<detune::DetuneCfg>,
    /// What an ability of a kit kind holds besides its numbers ([`AbilitySettings::Plain`] for the kinds that hold
    /// only numbers, and for every kind not ported).
    pub settings: AbilitySettings,
}

impl AbilityCfg {
    /// `params[name]`, when the entry has it.
    pub fn get(&self, name: &str) -> Option<f64> {
        self.params.get(name).copied()
    }
}

/// An ability's `slow` option.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlowCfg {
    /// The speed multiplier while slowed.
    pub mult: f64,
    /// Seconds it lasts.
    pub dur: f64,
}

/// An ability kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AbilityKind {
    /// `haste`: the fighter's grid runs `mult` x faster for `dur` s or `dur_beats` beats.
    Haste,
    /// `dance`: the cobras chase the enemy (SNAKE DANCE).
    Dance,
    /// `wave`: a ring that spreads from the fighter.
    Wave,
    /// `shockwave`: a ring that spreads from the fighter (it may slow).
    Shockwave,
    /// `slam`: a telegraphed strike where the enemy is heading.
    Slam,
    /// `shield`: hits on the fighter are blocked for `dur` s or `dur_beats` beats (TUBA, HORN, ORGAN; `fight.py` 2847
    /// to 2848).
    Shield,
    /// `sustain`: PIANO's SUSTAIN (`fight.py` `trigger` 2941 to 2943).
    Sustain,
    /// `arc`: PIANO's 88 KEYS ARC (`fight.py` `trigger` 2852 to 2858).
    Arc,
    /// `runoff`: PIANO's 88 KEYS (`fight.py` `trigger` 2944 to 2945).
    Runoff,
    /// `beam`: TRUMPET's HIGH C (`fight.py` `trigger` 2870 to 2871).
    Beam,
    /// `stun`: CLARINET's SQUEAK (`fight.py` `trigger` 2889 to 2892).
    Stun,
    /// `rhapsody`: CLARINET's RHAPSODY (`fight.py` `trigger` 2930 to 2932 (the chain runs in `jump_tick`)).
    Rhapsody,
    /// `snip`: OBOE's DOUBLE REED (`fight.py` `trigger` 2882 to 2883).
    Snip,
    /// `endless`: OBOE's ENDLESS NOTE (`fight.py` `trigger` 2884 to 2888).
    Endless,
    /// `flash_spin`: SOUSAPHONE's HORN FLASH (`fight.py` `trigger` 2977 to 2979).
    FlashSpin,
    /// `company_front`: SOUSAPHONE's COMPANY FRONT (`fight.py` `trigger` 2980 to 2982).
    CompanyFront,
    /// `runway`: TENOR SAX's WALK THE BAR (`fight.py` `trigger` 2966 to 2968).
    Runway,
    /// `subtone`: TENOR SAX's SUBTONE (`fight.py` `trigger` 2969 to 2972).
    Subtone,
    /// `stagedive`: TENOR SAX's STAGE DIVE (`fight.py` `trigger` 2973 to 2976).
    Stagedive,
    /// `octave`: ALTO SAX's OCTAVE KEY (`fight.py` `trigger` 2958 to 2960).
    Octave,
    /// `altissimo`: ALTO SAX's ALTISSIMO (`fight.py` `trigger` 2961 to 2963).
    Altissimo,
    /// `encore`: ALTO SAX's ENCORE (`fight.py` `trigger` 2964 to 2965).
    Encore,
    /// `twin`: BASS CLARINET's MULTIPHONICS (`fight.py` `trigger` 2897 to 2899).
    Twin,
    /// `lights_out`: BASS CLARINET's LIGHTS OUT (`fight.py` `trigger` 2895 to 2896).
    LightsOut,
    /// `slap`: CONTRABASS's SLAP BASS (`fight.py` `trigger` 2946 to 2947).
    Slap,
    /// `stampede`: CONTRABASS's THE ELEPHANT (`fight.py` `trigger` 2953 to 2954).
    Stampede,
    /// `hook`: BASSOON's BOCAL HOOK (`fight.py` `trigger` 2915 to 2916).
    Hook,
    /// `mountain`: BASSOON's MOUNTAIN KING (`fight.py` `trigger` 2917 to 2918).
    Mountain,
    /// `flick`: HORN's FINAL FLICK (`fight.py` `trigger` 2880 to 2881).
    Flick,
    /// `whip`: ACOUSTIC's FINGERPICK (`fight.py` `trigger` 2872 to 2873).
    Whip,
    /// `sweep`: ACOUSTIC's BIG STRUM (`fight.py` `trigger` 2872 to 2873).
    Sweep,
    /// `fork`: ELECTRIC's DISTORTION (`fight.py` `trigger` 2933 to 2935).
    Fork,
    /// `tapping`: ELECTRIC's TAPPING (`fight.py` `trigger` 2936 to 2938).
    Tapping,
    /// `smash`: ELECTRIC's the smash (`fight.py` `trigger` 2939 to 2940).
    Smash,
    /// `vibrato`: CELLO's VIBRATO (`fight.py` `trigger` 2955 to 2957).
    Vibrato,
    /// `prelude`: CELLO's PRELUDE (`fight.py` `trigger` 2948 to 2950).
    Prelude,
    /// `swan`: CELLO's SWAN SONG (`fight.py` `trigger` 2951 to 2952).
    Swan,
    /// `heartbeat`: BASS DRUM's HEARTBEAT (`fight.py` `trigger` 2983 to 2984).
    Heartbeat,
    /// `drum_roll`: BASS DRUM's DRUM ROLL (`fight.py` `trigger` 2985 to 2986).
    DrumRoll,
    /// `cannons`: BASS DRUM's 1812 OVERTURE (`fight.py` `trigger` 2987 to 2988).
    Cannons,
    /// `copy`: EUPHONIUM's IDENTITY CRISIS (`fight.py` `trigger` 2893 to 2894).
    Copy,
    /// `barrage`: EUPHONIUM's NOT A TUBA (`fight.py` `trigger` 2900 to 2901).
    Barrage,
    /// `clamp`: ACCORDION's FULL BELLOWS (`fight.py` `trigger` 2876 to 2877; milestone 4).
    Clamp,
    /// `rewind`: TURNTABLES's REWIND (`fight.py` `trigger` 2866 to 2867; milestone 4).
    Rewind,
    /// `lash`: SYNTH's SAWTOOTH (`fight.py` `trigger` 2868 to 2869; milestone 4).
    Lash,
    /// `spout`: UKULELE's UKE-NADO (`fight.py` `trigger` 2872 to 2873; milestone 4).
    Spout,
    /// `offbeat`: BARI SAX's SKANK (`fight.py` `trigger` 2911 to 2914; milestone 4).
    Offbeat,
    /// `snap`: BARI SAX's CABLE SNAP (`fight.py` `trigger` 2919 to 2920; milestone 4).
    Snap,
    /// `double_stop`: VIOLA's DOUBLE STOP (`fight.py` `trigger` 2928 to 2929; milestone 4).
    DoubleStop,
    /// `melody`: VIOLA's MELODY AT LAST (`fight.py` `trigger` 2923 to 2927; milestone 4).
    Melody,
    /// `joke`: VIOLA's VIOLA JOKE (`fight.py` `trigger` 2921 to 2922; milestone 4).
    Joke,
    /// A kind the engine has not ported yet, or a ported kind with an option it has not ported, written
    /// `kind+option` (OBOE's `"wave+detune"`).
    Unsupported(String),
}

/// What an ability holds besides its numbers, by kind: each kit kind's own settings (`roster::abilities::<kind>`).
#[derive(Clone, Debug, PartialEq)]
pub enum AbilitySettings {
    /// Nothing besides the numbers: the kinds FLUTE and TROMBONE use, `shield`, and every kind not ported.
    Plain,
    /// Kind `sustain`.
    Sustain(sustain::SustainCfg),
    /// Kind `arc`.
    Arc(arc::ArcCfg),
    /// Kind `runoff`.
    Runoff(runoff::RunoffCfg),
    /// Kind `beam`.
    Beam(beam::BeamCfg),
    /// Kind `stun`.
    Stun(stun::StunCfg),
    /// Kind `rhapsody`.
    Rhapsody(rhapsody::RhapsodyCfg),
    /// Kind `snip`.
    Snip(snip::SnipCfg),
    /// Kind `endless`.
    Endless(endless::EndlessCfg),
    /// Kind `flash_spin`.
    FlashSpin(flash_spin::FlashSpinCfg),
    /// Kind `company_front`.
    CompanyFront(company_front::CompanyFrontCfg),
    /// Kind `runway`.
    Runway(runway::RunwayCfg),
    /// Kind `subtone`.
    Subtone(subtone::SubtoneCfg),
    /// Kind `stagedive`.
    Stagedive(stagedive::StagediveCfg),
    /// Kind `octave`.
    Octave(octave::OctaveCfg),
    /// Kind `altissimo`.
    Altissimo(altissimo::AltissimoCfg),
    /// Kind `encore`.
    Encore(encore::EncoreCfg),
    /// Kind `twin`.
    Twin(twin::TwinCfg),
    /// Kind `lights_out`.
    LightsOut(lights_out::LightsOutCfg),
    /// Kind `slap`.
    Slap(slap::SlapCfg),
    /// Kind `stampede`.
    Stampede(stampede::StampedeCfg),
    /// Kind `hook`.
    Hook(hook::HookCfg),
    /// Kind `mountain`.
    Mountain(mountain::MountainCfg),
    /// Kind `flick`.
    Flick(flick::FlickCfg),
    /// Kind `whip`.
    Whip(whip::WhipCfg),
    /// Kind `sweep`.
    Sweep(sweep::SweepCfg),
    /// Kind `fork`.
    Fork(fork::ForkCfg),
    /// Kind `tapping`.
    Tapping(tapping::TappingCfg),
    /// Kind `smash`.
    Smash(smash::SmashCfg),
    /// Kind `vibrato`.
    Vibrato(vibrato::VibratoCfg),
    /// Kind `prelude`.
    Prelude(prelude::PreludeCfg),
    /// Kind `swan`.
    Swan(swan::SwanCfg),
    /// Kind `heartbeat`.
    Heartbeat(heartbeat::HeartbeatCfg),
    /// Kind `drum_roll`.
    DrumRoll(drum_roll::DrumRollCfg),
    /// Kind `cannons`.
    Cannons(cannons::CannonsCfg),
    /// Kind `copy`.
    Copy(copy::CopyCfg),
    /// Kind `barrage`.
    Barrage(barrage::BarrageCfg),
    /// Kind `clamp`.
    Clamp(clamp::ClampCfg),
    /// Kind `rewind`.
    Rewind(rewind::RewindCfg),
    /// Kind `lash`.
    Lash(lash::LashCfg),
    /// Kind `spout`.
    Spout(spout::SpoutCfg),
    /// Kind `offbeat`.
    Offbeat(offbeat::OffbeatCfg),
    /// Kind `snap`.
    Snap(snap::SnapCfg),
    /// Kind `double_stop`.
    DoubleStop(double_stop::DoubleStopCfg),
    /// Kind `melody`.
    Melody(melody::MelodyCfg),
    /// Kind `joke`.
    Joke(joke::JokeCfg),
}

impl AbilityKind {
    /// The kind's name in `fighters.py` (`ab['kind']`), as an `ability` event carries it. For a kind the engine has
    /// not ported it is the kind alone, without the `+option` an unported option adds.
    pub fn name(&self) -> &str {
        match self {
            AbilityKind::Haste => "haste",
            AbilityKind::Dance => "dance",
            AbilityKind::Wave => "wave",
            AbilityKind::Shockwave => "shockwave",
            AbilityKind::Slam => "slam",
            AbilityKind::Shield => "shield",
            AbilityKind::Sustain => "sustain",
            AbilityKind::Arc => "arc",
            AbilityKind::Runoff => "runoff",
            AbilityKind::Beam => "beam",
            AbilityKind::Stun => "stun",
            AbilityKind::Rhapsody => "rhapsody",
            AbilityKind::Snip => "snip",
            AbilityKind::Endless => "endless",
            AbilityKind::FlashSpin => "flash_spin",
            AbilityKind::CompanyFront => "company_front",
            AbilityKind::Runway => "runway",
            AbilityKind::Subtone => "subtone",
            AbilityKind::Stagedive => "stagedive",
            AbilityKind::Octave => "octave",
            AbilityKind::Altissimo => "altissimo",
            AbilityKind::Encore => "encore",
            AbilityKind::Twin => "twin",
            AbilityKind::LightsOut => "lights_out",
            AbilityKind::Slap => "slap",
            AbilityKind::Stampede => "stampede",
            AbilityKind::Hook => "hook",
            AbilityKind::Mountain => "mountain",
            AbilityKind::Flick => "flick",
            AbilityKind::Whip => "whip",
            AbilityKind::Sweep => "sweep",
            AbilityKind::Fork => "fork",
            AbilityKind::Tapping => "tapping",
            AbilityKind::Smash => "smash",
            AbilityKind::Vibrato => "vibrato",
            AbilityKind::Prelude => "prelude",
            AbilityKind::Swan => "swan",
            AbilityKind::Heartbeat => "heartbeat",
            AbilityKind::DrumRoll => "drum_roll",
            AbilityKind::Cannons => "cannons",
            AbilityKind::Copy => "copy",
            AbilityKind::Barrage => "barrage",
            AbilityKind::Clamp => "clamp",
            AbilityKind::Rewind => "rewind",
            AbilityKind::Lash => "lash",
            AbilityKind::Spout => "spout",
            AbilityKind::Offbeat => "offbeat",
            AbilityKind::Snap => "snap",
            AbilityKind::DoubleStop => "double_stop",
            AbilityKind::Melody => "melody",
            AbilityKind::Joke => "joke",
            AbilityKind::Unsupported(kind) => kind.split('+').next().unwrap_or(kind),
        }
    }

    /// The kind of this name; a kit kind whose module is not ported yet is [`AbilityKind::Unsupported`].
    pub(crate) fn from_name(kind: &str) -> AbilityKind {
        match kind {
            "haste" => AbilityKind::Haste,
            "dance" => AbilityKind::Dance,
            "wave" => AbilityKind::Wave,
            "shockwave" => AbilityKind::Shockwave,
            "slam" => AbilityKind::Slam,
            "shield" => AbilityKind::Shield,
            "sustain" if sustain::PORTED => AbilityKind::Sustain,
            "arc" if arc::PORTED => AbilityKind::Arc,
            "runoff" if runoff::PORTED => AbilityKind::Runoff,
            "beam" if beam::PORTED => AbilityKind::Beam,
            "stun" if stun::PORTED => AbilityKind::Stun,
            "rhapsody" if rhapsody::PORTED => AbilityKind::Rhapsody,
            "snip" if snip::PORTED => AbilityKind::Snip,
            "endless" if endless::PORTED => AbilityKind::Endless,
            "flash_spin" if flash_spin::PORTED => AbilityKind::FlashSpin,
            "company_front" if company_front::PORTED => AbilityKind::CompanyFront,
            "runway" if runway::PORTED => AbilityKind::Runway,
            "subtone" if subtone::PORTED => AbilityKind::Subtone,
            "stagedive" if stagedive::PORTED => AbilityKind::Stagedive,
            "octave" if octave::PORTED => AbilityKind::Octave,
            "altissimo" if altissimo::PORTED => AbilityKind::Altissimo,
            "encore" if encore::PORTED => AbilityKind::Encore,
            "twin" if twin::PORTED => AbilityKind::Twin,
            "lights_out" if lights_out::PORTED => AbilityKind::LightsOut,
            "slap" if slap::PORTED => AbilityKind::Slap,
            "stampede" if stampede::PORTED => AbilityKind::Stampede,
            "hook" if hook::PORTED => AbilityKind::Hook,
            "mountain" if mountain::PORTED => AbilityKind::Mountain,
            "flick" if flick::PORTED => AbilityKind::Flick,
            "whip" if whip::PORTED => AbilityKind::Whip,
            "sweep" if sweep::PORTED => AbilityKind::Sweep,
            "fork" if fork::PORTED => AbilityKind::Fork,
            "tapping" if tapping::PORTED => AbilityKind::Tapping,
            "smash" if smash::PORTED => AbilityKind::Smash,
            "vibrato" if vibrato::PORTED => AbilityKind::Vibrato,
            "prelude" if prelude::PORTED => AbilityKind::Prelude,
            "swan" if swan::PORTED => AbilityKind::Swan,
            "heartbeat" if heartbeat::PORTED => AbilityKind::Heartbeat,
            "drum_roll" if drum_roll::PORTED => AbilityKind::DrumRoll,
            "cannons" if cannons::PORTED => AbilityKind::Cannons,
            "copy" if copy::PORTED => AbilityKind::Copy,
            "barrage" if barrage::PORTED => AbilityKind::Barrage,
            "clamp" if clamp::PORTED => AbilityKind::Clamp,
            "rewind" if rewind::PORTED => AbilityKind::Rewind,
            "lash" if lash::PORTED => AbilityKind::Lash,
            "spout" if spout::PORTED => AbilityKind::Spout,
            "offbeat" if offbeat::PORTED => AbilityKind::Offbeat,
            "snap" if snap::PORTED => AbilityKind::Snap,
            "double_stop" if double_stop::PORTED => AbilityKind::DoubleStop,
            "melody" if melody::PORTED => AbilityKind::Melody,
            "joke" if joke::PORTED => AbilityKind::Joke,
            other => AbilityKind::Unsupported(other.to_string()),
        }
    }

    /// The numbers `fight.py` reads as `ab[name]` for this kind (`every_bars` for an unlock is checked separately);
    /// a group of names means one of them (`dur` or `dur_beats`).
    pub(crate) fn required(&self) -> &'static [&'static [&'static str]] {
        match self {
            AbilityKind::Haste => &[&["mult"], &["dur", "dur_beats"]],
            AbilityKind::Dance | AbilityKind::Shield => &[&["dur", "dur_beats"]],
            AbilityKind::Wave | AbilityKind::Shockwave => &[&["speed"], &["radius"], &["dmg"], &["knock"]],
            AbilityKind::Slam => &[&["tele"], &["radius"], &["dmg"], &["knock"]],
            AbilityKind::Sustain => sustain::REQUIRED,
            AbilityKind::Arc => arc::REQUIRED,
            AbilityKind::Runoff => runoff::REQUIRED,
            AbilityKind::Beam => beam::REQUIRED,
            AbilityKind::Stun => stun::REQUIRED,
            AbilityKind::Rhapsody => rhapsody::REQUIRED,
            AbilityKind::Snip => snip::REQUIRED,
            AbilityKind::Endless => endless::REQUIRED,
            AbilityKind::FlashSpin => flash_spin::REQUIRED,
            AbilityKind::CompanyFront => company_front::REQUIRED,
            AbilityKind::Runway => runway::REQUIRED,
            AbilityKind::Subtone => subtone::REQUIRED,
            AbilityKind::Stagedive => stagedive::REQUIRED,
            AbilityKind::Octave => octave::REQUIRED,
            AbilityKind::Altissimo => altissimo::REQUIRED,
            AbilityKind::Encore => encore::REQUIRED,
            AbilityKind::Twin => twin::REQUIRED,
            AbilityKind::LightsOut => lights_out::REQUIRED,
            AbilityKind::Slap => slap::REQUIRED,
            AbilityKind::Stampede => stampede::REQUIRED,
            AbilityKind::Hook => hook::REQUIRED,
            AbilityKind::Mountain => mountain::REQUIRED,
            AbilityKind::Flick => flick::REQUIRED,
            AbilityKind::Whip => whip::REQUIRED,
            AbilityKind::Sweep => sweep::REQUIRED,
            AbilityKind::Fork => fork::REQUIRED,
            AbilityKind::Tapping => tapping::REQUIRED,
            AbilityKind::Smash => smash::REQUIRED,
            AbilityKind::Vibrato => vibrato::REQUIRED,
            AbilityKind::Prelude => prelude::REQUIRED,
            AbilityKind::Swan => swan::REQUIRED,
            AbilityKind::Heartbeat => heartbeat::REQUIRED,
            AbilityKind::DrumRoll => drum_roll::REQUIRED,
            AbilityKind::Cannons => cannons::REQUIRED,
            AbilityKind::Copy => copy::REQUIRED,
            AbilityKind::Barrage => barrage::REQUIRED,
            AbilityKind::Clamp => clamp::REQUIRED,
            AbilityKind::Rewind => rewind::REQUIRED,
            AbilityKind::Lash => lash::REQUIRED,
            AbilityKind::Spout => spout::REQUIRED,
            AbilityKind::Offbeat => offbeat::REQUIRED,
            AbilityKind::Snap => snap::REQUIRED,
            AbilityKind::DoubleStop => double_stop::REQUIRED,
            AbilityKind::Melody => melody::REQUIRED,
            AbilityKind::Joke => joke::REQUIRED,
            AbilityKind::Unsupported(_) => &[],
        }
    }

    /// Every key `fight.py` reads from an ability of this kind: `name`, `kind`, `every_bars` (the timer of an
    /// unlock), then the kind's own. An entry holding anything else is not this kind yet (`Unsupported`).
    pub(crate) fn allowed(&self) -> &'static [&'static str] {
        match self {
            AbilityKind::Haste => &["name", "kind", "every_bars", "mult", "dur", "dur_beats"],
            AbilityKind::Dance | AbilityKind::Shield => &["name", "kind", "every_bars", "dur", "dur_beats"],
            AbilityKind::Wave | AbilityKind::Shockwave => {
                &["name", "kind", "every_bars", "speed", "radius", "dmg", "knock", "slow"]
            }
            AbilityKind::Slam => &["name", "kind", "every_bars", "tele", "radius", "dmg", "knock"],
            AbilityKind::Sustain => sustain::ALLOWED,
            AbilityKind::Arc => arc::ALLOWED,
            AbilityKind::Runoff => runoff::ALLOWED,
            AbilityKind::Beam => beam::ALLOWED,
            AbilityKind::Stun => stun::ALLOWED,
            AbilityKind::Rhapsody => rhapsody::ALLOWED,
            AbilityKind::Snip => snip::ALLOWED,
            AbilityKind::Endless => endless::ALLOWED,
            AbilityKind::FlashSpin => flash_spin::ALLOWED,
            AbilityKind::CompanyFront => company_front::ALLOWED,
            AbilityKind::Runway => runway::ALLOWED,
            AbilityKind::Subtone => subtone::ALLOWED,
            AbilityKind::Stagedive => stagedive::ALLOWED,
            AbilityKind::Octave => octave::ALLOWED,
            AbilityKind::Altissimo => altissimo::ALLOWED,
            AbilityKind::Encore => encore::ALLOWED,
            AbilityKind::Twin => twin::ALLOWED,
            AbilityKind::LightsOut => lights_out::ALLOWED,
            AbilityKind::Slap => slap::ALLOWED,
            AbilityKind::Stampede => stampede::ALLOWED,
            AbilityKind::Hook => hook::ALLOWED,
            AbilityKind::Mountain => mountain::ALLOWED,
            AbilityKind::Flick => flick::ALLOWED,
            AbilityKind::Whip => whip::ALLOWED,
            AbilityKind::Sweep => sweep::ALLOWED,
            AbilityKind::Fork => fork::ALLOWED,
            AbilityKind::Tapping => tapping::ALLOWED,
            AbilityKind::Smash => smash::ALLOWED,
            AbilityKind::Vibrato => vibrato::ALLOWED,
            AbilityKind::Prelude => prelude::ALLOWED,
            AbilityKind::Swan => swan::ALLOWED,
            AbilityKind::Heartbeat => heartbeat::ALLOWED,
            AbilityKind::DrumRoll => drum_roll::ALLOWED,
            AbilityKind::Cannons => cannons::ALLOWED,
            AbilityKind::Copy => copy::ALLOWED,
            AbilityKind::Barrage => barrage::ALLOWED,
            AbilityKind::Clamp => clamp::ALLOWED,
            AbilityKind::Rewind => rewind::ALLOWED,
            AbilityKind::Lash => lash::ALLOWED,
            AbilityKind::Spout => spout::ALLOWED,
            AbilityKind::Offbeat => offbeat::ALLOWED,
            AbilityKind::Snap => snap::ALLOWED,
            AbilityKind::DoubleStop => double_stop::ALLOWED,
            AbilityKind::Melody => melody::ALLOWED,
            AbilityKind::Joke => joke::ALLOWED,
            AbilityKind::Unsupported(_) => &[],
        }
    }

    /// Whether this is a kind the engine runs (not [`AbilityKind::Unsupported`]).
    pub fn is_ported(&self) -> bool {
        !matches!(self, AbilityKind::Unsupported(_))
    }
}

/// A kit kind's own settings, read by its module once the kind's numbers and keys are checked.
fn settings(e: &Entry, prefix: &str, kind: &AbilityKind, ab: &Json) -> Result<AbilitySettings, RosterError> {
    Ok(match kind {
        AbilityKind::Haste
        | AbilityKind::Dance
        | AbilityKind::Wave
        | AbilityKind::Shockwave
        | AbilityKind::Slam
        | AbilityKind::Shield
        | AbilityKind::Unsupported(_) => AbilitySettings::Plain,
        AbilityKind::Sustain => AbilitySettings::Sustain(sustain::parse(e, prefix, ab)?),
        AbilityKind::Arc => AbilitySettings::Arc(arc::parse(e, prefix, ab)?),
        AbilityKind::Runoff => AbilitySettings::Runoff(runoff::parse(e, prefix, ab)?),
        AbilityKind::Beam => AbilitySettings::Beam(beam::parse(e, prefix, ab)?),
        AbilityKind::Stun => AbilitySettings::Stun(stun::parse(e, prefix, ab)?),
        AbilityKind::Rhapsody => AbilitySettings::Rhapsody(rhapsody::parse(e, prefix, ab)?),
        AbilityKind::Snip => AbilitySettings::Snip(snip::parse(e, prefix, ab)?),
        AbilityKind::Endless => AbilitySettings::Endless(endless::parse(e, prefix, ab)?),
        AbilityKind::FlashSpin => AbilitySettings::FlashSpin(flash_spin::parse(e, prefix, ab)?),
        AbilityKind::CompanyFront => AbilitySettings::CompanyFront(company_front::parse(e, prefix, ab)?),
        AbilityKind::Runway => AbilitySettings::Runway(runway::parse(e, prefix, ab)?),
        AbilityKind::Subtone => AbilitySettings::Subtone(subtone::parse(e, prefix, ab)?),
        AbilityKind::Stagedive => AbilitySettings::Stagedive(stagedive::parse(e, prefix, ab)?),
        AbilityKind::Octave => AbilitySettings::Octave(octave::parse(e, prefix, ab)?),
        AbilityKind::Altissimo => AbilitySettings::Altissimo(altissimo::parse(e, prefix, ab)?),
        AbilityKind::Encore => AbilitySettings::Encore(encore::parse(e, prefix, ab)?),
        AbilityKind::Twin => AbilitySettings::Twin(twin::parse(e, prefix, ab)?),
        AbilityKind::LightsOut => AbilitySettings::LightsOut(lights_out::parse(e, prefix, ab)?),
        AbilityKind::Slap => AbilitySettings::Slap(slap::parse(e, prefix, ab)?),
        AbilityKind::Stampede => AbilitySettings::Stampede(stampede::parse(e, prefix, ab)?),
        AbilityKind::Hook => AbilitySettings::Hook(hook::parse(e, prefix, ab)?),
        AbilityKind::Mountain => AbilitySettings::Mountain(mountain::parse(e, prefix, ab)?),
        AbilityKind::Flick => AbilitySettings::Flick(flick::parse(e, prefix, ab)?),
        AbilityKind::Whip => AbilitySettings::Whip(whip::parse(e, prefix, ab)?),
        AbilityKind::Sweep => AbilitySettings::Sweep(sweep::parse(e, prefix, ab)?),
        AbilityKind::Fork => AbilitySettings::Fork(fork::parse(e, prefix, ab)?),
        AbilityKind::Tapping => AbilitySettings::Tapping(tapping::parse(e, prefix, ab)?),
        AbilityKind::Smash => AbilitySettings::Smash(smash::parse(e, prefix, ab)?),
        AbilityKind::Vibrato => AbilitySettings::Vibrato(vibrato::parse(e, prefix, ab)?),
        AbilityKind::Prelude => AbilitySettings::Prelude(prelude::parse(e, prefix, ab)?),
        AbilityKind::Swan => AbilitySettings::Swan(swan::parse(e, prefix, ab)?),
        AbilityKind::Heartbeat => AbilitySettings::Heartbeat(heartbeat::parse(e, prefix, ab)?),
        AbilityKind::DrumRoll => AbilitySettings::DrumRoll(drum_roll::parse(e, prefix, ab)?),
        AbilityKind::Cannons => AbilitySettings::Cannons(cannons::parse(e, prefix, ab)?),
        AbilityKind::Copy => AbilitySettings::Copy(copy::parse(e, prefix, ab)?),
        AbilityKind::Barrage => AbilitySettings::Barrage(barrage::parse(e, prefix, ab)?),
        AbilityKind::Clamp => AbilitySettings::Clamp(clamp::parse(e, prefix, ab)?),
        AbilityKind::Rewind => AbilitySettings::Rewind(rewind::parse(e, prefix, ab)?),
        AbilityKind::Lash => AbilitySettings::Lash(lash::parse(e, prefix, ab)?),
        AbilityKind::Spout => AbilitySettings::Spout(spout::parse(e, prefix, ab)?),
        AbilityKind::Offbeat => AbilitySettings::Offbeat(offbeat::parse(e, prefix, ab)?),
        AbilityKind::Snap => AbilitySettings::Snap(snap::parse(e, prefix, ab)?),
        AbilityKind::DoubleStop => AbilitySettings::DoubleStop(double_stop::parse(e, prefix, ab)?),
        AbilityKind::Melody => AbilitySettings::Melody(melody::parse(e, prefix, ab)?),
        AbilityKind::Joke => AbilitySettings::Joke(joke::parse(e, prefix, ab)?),
    })
}

/// A kit kind's own checks on an entry (at load time and in `FighterCfg::check`).
pub(crate) fn kind_check(e: &Entry, prefix: &str, ab: &AbilityCfg) -> Result<(), RosterError> {
    match &ab.kind {
        AbilityKind::Haste
        | AbilityKind::Dance
        | AbilityKind::Wave
        | AbilityKind::Shockwave
        | AbilityKind::Slam
        | AbilityKind::Shield
        | AbilityKind::Unsupported(_) => Ok(()),
        AbilityKind::Sustain => sustain::check(e, prefix, ab),
        AbilityKind::Arc => arc::check(e, prefix, ab),
        AbilityKind::Runoff => runoff::check(e, prefix, ab),
        AbilityKind::Beam => beam::check(e, prefix, ab),
        AbilityKind::Stun => stun::check(e, prefix, ab),
        AbilityKind::Rhapsody => rhapsody::check(e, prefix, ab),
        AbilityKind::Snip => snip::check(e, prefix, ab),
        AbilityKind::Endless => endless::check(e, prefix, ab),
        AbilityKind::FlashSpin => flash_spin::check(e, prefix, ab),
        AbilityKind::CompanyFront => company_front::check(e, prefix, ab),
        AbilityKind::Runway => runway::check(e, prefix, ab),
        AbilityKind::Subtone => subtone::check(e, prefix, ab),
        AbilityKind::Stagedive => stagedive::check(e, prefix, ab),
        AbilityKind::Octave => octave::check(e, prefix, ab),
        AbilityKind::Altissimo => altissimo::check(e, prefix, ab),
        AbilityKind::Encore => encore::check(e, prefix, ab),
        AbilityKind::Twin => twin::check(e, prefix, ab),
        AbilityKind::LightsOut => lights_out::check(e, prefix, ab),
        AbilityKind::Slap => slap::check(e, prefix, ab),
        AbilityKind::Stampede => stampede::check(e, prefix, ab),
        AbilityKind::Hook => hook::check(e, prefix, ab),
        AbilityKind::Mountain => mountain::check(e, prefix, ab),
        AbilityKind::Flick => flick::check(e, prefix, ab),
        AbilityKind::Whip => whip::check(e, prefix, ab),
        AbilityKind::Sweep => sweep::check(e, prefix, ab),
        AbilityKind::Fork => fork::check(e, prefix, ab),
        AbilityKind::Tapping => tapping::check(e, prefix, ab),
        AbilityKind::Smash => smash::check(e, prefix, ab),
        AbilityKind::Vibrato => vibrato::check(e, prefix, ab),
        AbilityKind::Prelude => prelude::check(e, prefix, ab),
        AbilityKind::Swan => swan::check(e, prefix, ab),
        AbilityKind::Heartbeat => heartbeat::check(e, prefix, ab),
        AbilityKind::DrumRoll => drum_roll::check(e, prefix, ab),
        AbilityKind::Cannons => cannons::check(e, prefix, ab),
        AbilityKind::Copy => copy::check(e, prefix, ab),
        AbilityKind::Barrage => barrage::check(e, prefix, ab),
        AbilityKind::Clamp => clamp::check(e, prefix, ab),
        AbilityKind::Rewind => rewind::check(e, prefix, ab),
        AbilityKind::Lash => lash::check(e, prefix, ab),
        AbilityKind::Spout => spout::check(e, prefix, ab),
        AbilityKind::Offbeat => offbeat::check(e, prefix, ab),
        AbilityKind::Snap => snap::check(e, prefix, ab),
        AbilityKind::DoubleStop => double_stop::check(e, prefix, ab),
        AbilityKind::Melody => melody::check(e, prefix, ab),
        AbilityKind::Joke => joke::check(e, prefix, ab),
    }
}

impl Entry<'_> {
    /// The numbers a ported ability kind cannot do without (`fight.py` reads them as `ab[name]`), the `every_bars` of
    /// an unlock, and a haste's factor, which the weapon's grid, a thrust and a lunge are divided by.
    pub(crate) fn ability_numbers(
        &self,
        prefix: &str,
        slot: &str,
        kind: &AbilityKind,
        params: &BTreeMap<String, f64>,
    ) -> Result<(), RosterError> {
        for group in kind.required() {
            if !group.iter().any(|k| params.contains_key(*k)) {
                return Err(self.err(&join(prefix, &group.join(" or ")), "missing"));
            }
        }
        if slot != "low" && !params.contains_key("every_bars") {
            return Err(self.err(&join(prefix, "every_bars"), "missing"));
        }
        // A haste divides its fighter's weapon grid, the time of a thrust and the time of a lunge by `mult`
        // (`ab_mult`): with 0 the Python fight raises (a division by zero) on the first 16th note of the haste,
        // where the engine would start a thrust that never ends; below 0 a thrust runs backwards for ever.
        if let (AbilityKind::Haste, Some(&mult)) = (kind, params.get("mult")) {
            if mult <= 0.0 {
                return Err(self.err(
                    &join(prefix, "mult"),
                    format!("{mult} is not above 0 (a haste divides the weapon's grid by it)"),
                ));
            }
            // So small that its inverse (a thrust's `thr_k = 1 / mult`) is infinite: Python's
            // `int(round(main16 / mult))` then raises `OverflowError` for any grid but 0, where the engine would
            // saturate and run on. The weapon's own grid is checked against it in `FighterCfg::check`.
            if !(1.0 / mult).is_finite() {
                return Err(self.err(
                    &join(prefix, "mult"),
                    format!(
                        "{mult:e} is too small (a haste divides the weapon's grid by it, and 1 / mult must be a \
                         finite number)"
                    ),
                ));
            }
        }
        Ok(())
    }

    /// One ability entry, `abilities.<slot>`.
    pub(crate) fn ability(&self, abilities: &Json, slot: &str) -> Result<AbilityCfg, RosterError> {
        let prefix = format!("abilities.{slot}");
        let ab = self.get(abilities, "abilities", slot)?;
        if !ab.is_object() {
            return Err(self.err(&prefix, "must be a table"));
        }
        let name = self.string(ab, &prefix, "name")?;
        let kind_name = self.string(ab, &prefix, "kind")?;
        let mut kind = AbilityKind::from_name(&kind_name);
        // A ported kind carrying an option the engine has not ported (OBOE's wave has `detune`) is not that kind:
        // it loads as `Unsupported("wave+detune")`, so it is never typed with the option silently dropped. The
        // option `detune` of a wave is let in once its module is ported.
        let mut allowed = kind.allowed().to_vec();
        if kind == AbilityKind::Wave && detune::PORTED {
            allowed.push("detune");
        }
        if let Some(extra) = unported_option(ab, &allowed) {
            kind = AbilityKind::Unsupported(format!("{kind_name}+{extra}"));
        }
        let mut params = BTreeMap::new();
        if let Some(obj) = ab.as_object() {
            for (k, v) in obj {
                if let Some(x) = v.as_f64() {
                    params.insert(k.clone(), x);
                }
            }
        }
        let supported = kind.is_ported();
        // `fight.py` asks `if ab.get('slow')` (and `if W.get('slow')` when the ring lands): Python's truth test, by
        // which an empty table, like no table at all, is no slow.
        let slow = match ab.get("slow") {
            Some(s) if !py_truthy(s) => None,
            None => None,
            Some(s) => {
                let mult = s.get("mult").and_then(Json::as_f64);
                let dur = s.get("dur").and_then(Json::as_f64);
                match (mult, dur) {
                    (Some(mult), Some(dur)) => Some(SlowCfg { mult, dur }),
                    _ if supported => {
                        return Err(self.err(&join(&prefix, "slow"), "must be a table with mult and dur"));
                    }
                    _ => None,
                }
            }
        };
        // `if ab.get('detune')` (2845): Python's truth test again
        let detune = match ab.get("detune") {
            Some(d) if kind == AbilityKind::Wave && py_truthy(d) => Some(detune::parse(self, &prefix, d)?),
            _ => None,
        };
        if supported {
            self.ability_numbers(&prefix, slot, &kind, &params)?;
        }
        let settings = settings(self, &prefix, &kind, ab)?;
        let cfg = AbilityCfg {
            name,
            kind,
            params,
            slow,
            detune,
            settings,
        };
        kind_check(self, &prefix, &cfg)?;
        Ok(cfg)
    }
}

/// Python's truth value of a JSON value (`if x:`): `None`, `False`, zero, `""`, `[]` and `{}` are false.
pub(crate) fn py_truthy(v: &Json) -> bool {
    match v {
        Json::Null => false,
        Json::Bool(b) => *b,
        Json::Number(n) => n.as_f64() != Some(0.0),
        Json::String(s) => !s.is_empty(),
        Json::Array(a) => !a.is_empty(),
        Json::Object(o) => !o.is_empty(),
    }
}

/// The first key of a ported ability kind's entry outside `allowed` (or inside its `slow`, outside `mult` and
/// `dur`), as `key` or `slow.key`. `None` for an unported kind (its `allowed` is empty and nothing is checked).
fn unported_option(ab: &Json, allowed: &[&str]) -> Option<String> {
    if allowed.is_empty() {
        return None;
    }
    let obj = ab.as_object()?;
    if let Some(k) = obj.keys().find(|k| !allowed.contains(&k.as_str())) {
        return Some(k.clone());
    }
    let slow = obj.get("slow").and_then(Json::as_object)?;
    slow.keys()
        .find(|k| !["mult", "dur"].contains(&k.as_str()))
        .map(|k| format!("slow.{k}"))
}
