//! CONTRABASS against the Python kit: every gate of Template F (studio/docs/porting-a-fighter.md).

bf_harness::fighter_suite!("CONTRABASS");

#[test]
#[ignore]
fn zz_first_bit() {
    use bf_core::event::Value as Ev;
    use bf_core::recording::Array;
    use bf_harness::fight::*;
    use bf_harness::refs::recording_of;
    let doc = doc(NAME);
    let which = std::env::var("ZZ_WHICH").unwrap_or("left".into());
    let left = &doc[which.as_str()];
    let py = recording_of(left, left["hash"].as_str());
    let like = setup_like(&py);
    let rs = traced(&bf_harness::sim!(), &like, true, "zz");
    for key in ["P", "V"] {
        let (Some(Array::F64 { data: a, .. }), Some(Array::F64 { data: b, .. })) =
            (rs.arrays.get(key), py.arrays.get(key))
        else {
            panic!("{key}")
        };
        if let Some(i) = a.iter().zip(b).position(|(x, y)| x.to_bits() != y.to_bits()) {
            println!(
                "{key}: first bit difference at flat {i} (frame {}): rs {:e} py {:e}",
                i / 4,
                a[i],
                b[i]
            );
        } else {
            println!("{key}: equal over {} values", a.len().min(b.len()));
        }
    }
    let fr: usize = std::env::var("ZZ_FRAME")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(43);
    let (t0, t1) = (
        (rs.f0 + fr - 2) as f64 / rs.fps as f64,
        (rs.f0 + fr) as f64 / rs.fps as f64,
    );
    println!("f0 {} fps {} window {t0} {t1}", rs.f0, rs.fps);
    for e in rs.events.iter().filter(|e| e.t >= t0 - 0.2 && e.t <= t1) {
        println!("  rs ev {e:?}");
    }
    if let (Some(Array::F64 { data: a, .. }), Some(Array::F64 { data: b, .. })) =
        (rs.arrays.get("V"), py.arrays.get("V"))
    {
        for i in fr - 2..=fr {
            println!(
                "  V frame {i}: rs {:?} py {:?}",
                &a[i * 4..i * 4 + 4],
                &b[i * 4..i * 4 + 4]
            );
        }
    }
    if let (Some(Array::F64 { data: a, .. }), Some(Array::F64 { data: b, .. })) =
        (rs.arrays.get("P"), py.arrays.get("P"))
    {
        for i in fr - 2..=fr {
            println!(
                "  P frame {i}: rs {:?} py {:?}",
                &a[i * 4..i * 4 + 4],
                &b[i * 4..i * 4 + 4]
            );
        }
    }
    for (j, (e, f)) in rs.events.iter().zip(&py.events).enumerate() {
        let same = e.kind == f.kind
            && e.t.to_bits() == f.t.to_bits()
            && e.fields.len() == f.fields.len()
            && e.fields.iter().zip(&f.fields).all(|(a, b)| match (a, b) {
                (Ev::Float(x), Ev::Float(y)) => x.to_bits() == y.to_bits(),
                _ => a == b,
            });
        if !same {
            println!("event {j}: rs {e:?}\n          py {f:?}");
            break;
        }
    }
}

#[test]
#[ignore]
fn zz_all_windows() {
    use bf_harness::fight::*;
    use bf_harness::refs::*;
    let doc = doc(NAME);
    let mut fights = Vec::new();
    let left = &doc["left"];
    fights.push(("left".to_string(), recording_of(left, left["hash"].as_str())));
    fights.push((
        "right".to_string(),
        cached_recording(doc["right"]["cache"].as_str().unwrap()),
    ));
    for s in doc["scenarios"].as_array().unwrap() {
        for file in s["cache"].as_array().unwrap() {
            let f = file.as_str().unwrap();
            fights.push((f.rsplit('/').next().unwrap().to_string(), cached_recording(f)));
        }
    }
    for (what, py) in &fights {
        let setup = setup_like(py);
        let rs = traced(&bf_harness::sim!(), &setup, true, what);
        let r = bf_sim::parity::prefix_window(&rs, py);
        let tf = match py.rules.get("t_fight") {
            Some(bf_core::event::Value::Float(x)) => *x,
            _ => 0.0,
        };
        println!(
            "ZZ {what}: {:.2} s, {:?}, {}",
            r.agree_until_s - tf,
            r.kind,
            r.first_break
                .clone()
                .unwrap_or_default()
                .chars()
                .take(90)
                .collect::<String>()
        );
    }
}
