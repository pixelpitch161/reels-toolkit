use super::*;

fn h(strength: f32) -> Hit {
    Hit {
        t0: 10.0,
        seed: 3,
        at: [0.5, 0.2, 0.6],
        normal: [1.0, 0.0, 0.0],
        colour: [0.9, 0.4, 0.1],
        strength,
    }
}

/// Every preset at strength `s`.
pub(super) fn all(s: f32) -> Vec<(&'static str, Vec<FxRecord>)> {
    let x = h(s);
    vec![
        ("hit_light", hit_light(&x)),
        ("hit_heavy", hit_heavy(&x)),
        ("special_impact", special_impact(&x)),
        ("launch_trail", launch_trail(&x, 1)),
        ("land_dust", land_dust(&x)),
        ("slam_crater", slam_crater(&x)),
        ("clash_spark", clash_spark(&x)),
        ("block_flare", block_flare(&x)),
        ("ko_burst", ko_burst(&x, [0.2, 0.5, 1.0])),
        ("proj_spawn_flash", proj_spawn_flash(&x)),
        ("proj_expire_puff", proj_expire_puff(&x)),
    ]
}

#[test]
fn every_preset_has_an_exit_and_life() {
    for s in [0.1, 0.5, 1.0] {
        for (name, recs) in all(s) {
            assert!(!recs.is_empty(), "{name} makes records");
            for r in &recs {
                assert_eq!(r.check(), Ok(()), "{name}: {r:?}");
                assert!(r.life > 0.0 && r.life.is_finite(), "{name}: life {}", r.life);
                assert!(r.env.release >= MIN_FALL, "{name}: a fall of {}", r.env.release);
            }
        }
    }
}

#[test]
fn every_record_fades_to_nothing_by_its_end() {
    for s in [0.1, 0.5, 1.0] {
        for (name, recs) in all(s) {
            for r in &recs {
                let peak = (1..400)
                    .map(|i| brightness(r, r.t0 + f64::from(r.life) * f64::from(i) / 400.0))
                    .fold(0.0f32, f32::max);
                assert!(peak > 0.0, "{name}: {r:?} never shows");
                let last = brightness(r, r.t0 + f64::from(r.life) - 1.0 / 240.0);
                assert!(
                    last < 0.02 * peak,
                    "{name} at {s}: {:?} ends at {last} of {peak}",
                    r.kind
                );
            }
        }
    }
}

#[test]
fn light_and_heavy_differ() {
    for (ls, hs) in [(0.1, 0.4), (0.39, 0.6), (0.2, 0.84)] {
        let (light, heavy) = (hit_light(&h(ls)), hit_heavy(&h(hs)));
        assert!(
            particle_count(&heavy) >= 8 * particle_count(&light),
            "{} vs {}",
            particle_count(&heavy),
            particle_count(&light)
        );
        assert_eq!(screen_effects(&light), 0, "a light hit has no screen effects");
        assert!(screen_effects(&heavy) > 0);
    }
    let special = special_impact(&h(0.9));
    assert!(particle_count(&special) >= 4 * particle_count(&hit_heavy(&h(0.84))));
    assert!(screen_effects(&special) > screen_effects(&hit_heavy(&h(0.84))));
    assert!(particle_count(&ko_burst(&h(1.0), [1.0; 3])) >= 100_000);
    assert_eq!(Tier::of(0.39, false), Tier::Light);
    assert_eq!(Tier::of(0.4, false), Tier::Heavy);
    assert_eq!(Tier::of(0.86, false), Tier::Special);
    assert_eq!(Tier::of(0.1, true), Tier::Special);
}
