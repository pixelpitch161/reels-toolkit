//! Every `.npz` under `studio/reference` opens, has a sidecar that parses, and holds every array its cases name.
//! Also: an `.npz` laid out the way `tools/studio_export/common.py` writes one reads back through `Npz::open`.

use std::fs;
use std::io::{Cursor, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};

use bf_testdata::{Npz, Vectors, reference_dir};

/// The `.npz` files under `dir`, in path order, skipping the git-ignored `.cache` folder (episode copies and the
/// kit's own caches, not reference files). A file another export is writing at this moment is a `.tmp` until it
/// is complete, so it is not picked up.
fn npz_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    let mut entries: Vec<PathBuf> = entries.map(|e| e.unwrap().path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n != ".cache") {
                npz_files(&path, out);
            }
        } else if path.extension().is_some_and(|e| e == "npz") {
            out.push(path);
        }
    }
}

fn check(rel: &str) -> String {
    let v = Vectors::open(rel);
    let mut roles = 0;
    for case in &v.index.cases {
        for role in case.arrays.keys() {
            v.array(case, role);
            roles += 1;
        }
    }
    format!(
        "{} arrays, {} cases, {roles} case arrays, tool {}",
        v.npz.names().len(),
        v.index.cases.len(),
        v.index.tool
    )
}

fn panic_text(e: &(dyn std::any::Any + Send)) -> String {
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string()))
        .unwrap_or_else(|| "a panic without a message".into())
}

#[test]
fn every_reference_npz_reads() {
    let root = reference_dir();
    let mut files = Vec::new();
    npz_files(&root, &mut files);
    println!("{} .npz file(s) under {}", files.len(), root.display());
    let mut failed = Vec::new();
    for path in &files {
        let rel = path.strip_prefix(&root).unwrap().to_string_lossy().replace('\\', "/");
        match catch_unwind(AssertUnwindSafe(|| check(&rel))) {
            Ok(summary) => println!("ok   studio/reference/{rel}: {summary}"),
            Err(e) => {
                let why = panic_text(&*e);
                println!("FAIL studio/reference/{rel}: {why}");
                failed.push(format!("studio/reference/{rel}: {why}"));
            }
        }
    }
    assert!(
        failed.is_empty(),
        "{} reference file(s) failed:\n{}",
        failed.len(),
        failed.join("\n")
    );
}

/// A `.npy` file as numpy 2.5.3 writes version 1.0: the dict text padded with spaces and a newline so the data
/// starts on a 64-byte boundary.
fn npy(descr: &str, shape: &str, data: &[u8]) -> Vec<u8> {
    let mut header = format!("{{'descr': '{descr}', 'fortran_order': False, 'shape': {shape}, }}");
    while (10 + header.len() + 1) % 64 != 0 {
        header.push(' ');
    }
    header.push('\n');
    let mut out = b"\x93NUMPY\x01\x00".to_vec();
    out.extend_from_slice(&(header.len() as u16).to_le_bytes());
    out.extend_from_slice(header.as_bytes());
    out.extend_from_slice(data);
    out
}

#[test]
fn an_npz_in_the_export_layout_reads_back() {
    let pixels: Vec<u8> = (0..2 * 3 * 3).map(|i| i * 13).collect();
    let samples: Vec<u8> = [0.25f32, -1.0].iter().flat_map(|x| x.to_le_bytes()).collect();
    let mut members = vec![
        ("c000__out.npy", npy("|u1", "(2, 3, 3)", &pixels)),
        ("c000__in.npy", npy("<f4", "(2,)", &samples)),
        ("flags.npy", npy("|b1", "()", &[1])),
    ];
    // common.zip_bytes: members in ascending name order, deflate level 9, the 1980 timestamp, DOS system.
    members.sort();
    let options = zip::write::SimpleFileOptions::DEFAULT
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(9))
        .last_modified_time(zip::DateTime::DEFAULT)
        .system(zip::System::Dos);
    let mut z = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in &members {
        z.start_file(*name, options).unwrap();
        z.write_all(bytes).unwrap();
    }
    let bytes = z.finish().unwrap().into_inner();
    let path = Path::new(env!("CARGO_TARGET_TMPDIR")).join("export_layout.npz");
    fs::write(&path, bytes).unwrap();

    let npz = Npz::open(&path);
    assert_eq!(npz.names(), ["c000__in", "c000__out", "flags"]);
    let out = npz.get("c000__out");
    assert_eq!((out.shape.as_slice(), out.u8()), (&[2, 3, 3][..], &pixels[..]));
    assert_eq!(npz.get("c000__in").f32(), [0.25, -1.0]);
    let flags = npz.get("flags");
    assert_eq!((flags.shape.len(), flags.bool()), (0, &[true][..]));
}
