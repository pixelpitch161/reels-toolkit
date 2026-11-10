//! The JSON sidecar beside every `.npz`: the list of cases, each naming its arrays by role.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

use crate::npz::{NpArray, Npz};
use crate::paths::reference;

/// One case: `id` (`c000`, ...), the Python `call` that produced it, its `args` as JSON, and `arrays`, a map from
/// role (`in`, `out`, ...) to the npz member name.
#[derive(Clone, Debug, PartialEq)]
pub struct Case {
    pub id: String,
    pub call: String,
    pub args: Value,
    pub arrays: BTreeMap<String, String>,
}

/// A sidecar: its `about` line, the export `tool` that wrote it, the cases, and the whole JSON (`raw`) for any
/// top-level field a test needs.
#[derive(Clone, Debug, PartialEq)]
pub struct Sidecar {
    pub about: String,
    pub tool: String,
    pub cases: Vec<Case>,
    pub raw: Value,
}

/// An `.npz` and its sidecar, opened together.
#[derive(Clone, Debug)]
pub struct Vectors {
    pub npz: Npz,
    pub index: Sidecar,
}

impl Vectors {
    /// Opens `reference(npz_rel)` and the `.json` with the same base name beside it; panics naming the file if
    /// either is missing or malformed.
    pub fn open(npz_rel: &str) -> Vectors {
        let npz_path = reference(npz_rel);
        assert!(
            npz_path.extension().is_some_and(|e| e == "npz"),
            "{} is not an .npz file",
            npz_path.display()
        );
        let json_path = npz_path.with_extension("json");
        assert!(
            json_path.is_file(),
            "{} has no sidecar: {} is missing",
            npz_path.display(),
            json_path.display()
        );
        let index = Sidecar::read(&json_path);
        let npz = Npz::open(&npz_path);
        Vectors { npz, index }
    }

    /// The array the case names for `role`; panics naming the case id and the role if the case has no such role
    /// or the archive has no such member.
    pub fn array(&self, case: &Case, role: &str) -> &NpArray {
        let Some(name) = case.arrays.get(role) else {
            panic!(
                "case {} has no array for role {role:?}; its roles: {}",
                case.id,
                case.arrays.keys().map(String::as_str).collect::<Vec<_>>().join(", ")
            );
        };
        assert!(
            self.npz.has(name),
            "case {}, role {role:?}: the archive has no member {name:?}; available: {}",
            case.id,
            self.npz.names().join(", ")
        );
        self.npz.get(name)
    }
}

impl Sidecar {
    /// Reads and parses the sidecar at `path`; panics naming the file if it is unreadable or malformed.
    fn read(path: &Path) -> Sidecar {
        let origin = path.display().to_string();
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {origin}: {e}"));
        let raw = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{origin} is not JSON: {e}"));
        Sidecar::from_value(raw, &origin).unwrap_or_else(|e| panic!("{e}"))
    }

    /// Reads the sidecar's JSON value; `origin` (the file) goes into every error.
    fn from_value(raw: Value, origin: &str) -> Result<Sidecar, String> {
        let top = raw
            .as_object()
            .ok_or_else(|| format!("{origin}: the sidecar is not a JSON object"))?;
        let text = |obj: &serde_json::Map<String, Value>, key: &str, place: &str| {
            obj.get(key)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| format!("{origin}: {place} has no string {key:?}"))
        };
        let about = text(top, "about", "the sidecar")?;
        let tool = text(top, "tool", "the sidecar")?;
        let list = top
            .get("cases")
            .and_then(Value::as_array)
            .ok_or_else(|| format!("{origin}: the sidecar has no list \"cases\""))?;
        let mut cases = Vec::with_capacity(list.len());
        let mut ids = BTreeSet::new();
        for (i, c) in list.iter().enumerate() {
            let place = format!("case {i}");
            let obj = c
                .as_object()
                .ok_or_else(|| format!("{origin}: {place} is not a JSON object"))?;
            let id = text(obj, "id", &place)?;
            let place = format!("case {i} ({id})");
            let call = text(obj, "call", &place)?;
            let args = obj
                .get("args")
                .cloned()
                .ok_or_else(|| format!("{origin}: {place} has no \"args\""))?;
            let map = obj
                .get("arrays")
                .and_then(Value::as_object)
                .ok_or_else(|| format!("{origin}: {place} has no object \"arrays\""))?;
            let mut arrays = BTreeMap::new();
            for (role, member) in map {
                let member = member
                    .as_str()
                    .ok_or_else(|| format!("{origin}: {place} names role {role:?} by {member}, not a string"))?;
                arrays.insert(role.clone(), member.to_string());
            }
            if !ids.insert(id.clone()) {
                return Err(format!("{origin}: case id {id:?} appears twice"));
            }
            cases.push(Case { id, call, args, arrays });
        }
        Ok(Sidecar {
            about,
            tool,
            cases,
            raw,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::npz::tests::PYTHON_NPZ;
    use serde_json::json;

    fn sample() -> Value {
        json!({
            "about": "three tiny arrays",
            "tool": "tools.studio_export.sample",
            "canvas": [360, 640],
            "cases": [
                {"id": "c000", "call": "f(a)", "args": {"x": "0x1.8000000000000p+1"},
                 "arrays": {"in": "c000__in", "out": "c000__out"}},
                {"id": "c001", "call": "g()", "args": [], "arrays": {"mask": "mask"}}
            ]
        })
    }

    fn vectors() -> Vectors {
        let bytes: Vec<u8> = (0..PYTHON_NPZ.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&PYTHON_NPZ[i..i + 2], 16).unwrap())
            .collect();
        Vectors {
            npz: Npz::from_bytes(&bytes, "python.npz").unwrap(),
            index: Sidecar::from_value(sample(), "python.json").unwrap(),
        }
    }

    #[test]
    fn reads_the_cases() {
        let s = Sidecar::from_value(sample(), "sample.json").unwrap();
        assert_eq!(
            (s.about.as_str(), s.tool.as_str()),
            ("three tiny arrays", "tools.studio_export.sample")
        );
        assert_eq!(s.cases.len(), 2);
        assert_eq!(s.cases[0].id, "c000");
        assert_eq!(s.cases[0].call, "f(a)");
        assert_eq!(s.cases[0].args["x"], "0x1.8000000000000p+1");
        assert_eq!(s.cases[0].arrays["out"], "c000__out");
        assert_eq!(s.raw["canvas"], json!([360, 640]));
    }

    #[test]
    fn malformed_sidecars_are_errors_naming_the_file() {
        let broken = |f: &dyn Fn(&mut Value)| {
            let mut v = sample();
            f(&mut v);
            Sidecar::from_value(v, "bad.json").unwrap_err()
        };
        for e in [
            broken(&|v| v["about"] = json!(3)),
            broken(&|v| drop(v.as_object_mut().unwrap().remove("tool"))),
            broken(&|v| v["cases"] = json!({})),
            broken(&|v| drop(v["cases"][1].as_object_mut().unwrap().remove("call"))),
            broken(&|v| v["cases"][0]["arrays"]["in"] = json!(1)),
            broken(&|v| v["cases"][1]["id"] = json!("c000")),
        ] {
            assert!(e.contains("bad.json"), "{e}");
        }
        assert!(
            Sidecar::from_value(json!([]), "list.json")
                .unwrap_err()
                .contains("list.json")
        );
        let e = broken(&|v| v["cases"][1]["id"] = json!("c000"));
        assert!(e.contains("c000") && e.contains("twice"), "{e}");
    }

    #[test]
    fn array_follows_the_role_to_the_member() {
        let v = vectors();
        let case = &v.index.cases[0];
        assert_eq!(v.array(case, "in").i32(), [1, 2, 3, 4, 5, 6]);
        assert_eq!(v.array(case, "out").f64(), [-0.5]);
        assert_eq!(v.array(&v.index.cases[1], "mask").bool(), [true, false, true]);
    }

    #[test]
    #[should_panic(expected = "case c001 has no array for role \"in\"")]
    fn array_names_case_and_role() {
        let v = vectors();
        v.array(&v.index.cases[1], "in");
    }

    #[test]
    #[should_panic(expected = "case c000, role \"out\"")]
    fn array_names_case_and_role_when_the_member_is_missing() {
        let mut v = vectors();
        v.index.cases[0].arrays.insert("out".into(), "c000__gone".into());
        v.array(&v.index.cases[0].clone(), "out");
    }
}
