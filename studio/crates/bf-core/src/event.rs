//! Fight events and their values (Task 6).
//!
//! A [`Value`] is one JSON-like value of a recording (`docs/recording-format.md` section 2): the kit's Python values
//! as they come out of `meta.json` and the ragged per-frame arrays. An [`Event`] is one entry of the fight's event
//! list, `[t, kind, who, ...]`, indexed the way the Python code indexes it.
//!
//! Equality is bit-exact: two `Float`s are equal when their bits are, so `NaN == NaN`, `0.0 != -0.0`, and an `Int`
//! never equals a `Float` (the content hash tells them apart too).

/// One JSON-like value. `Int` and `Float` are different values even when they are numerically equal.
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Vec<Value>),
}

impl PartialEq for Value {
    /// Bit-exact: `Float`s compare by their bits.
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a.to_bits() == b.to_bits(),
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

/// One fight event: Python's `e[0]` is `t`, `e[1]` is `kind`, and `e[2..]` are `fields`.
#[derive(Debug, Clone)]
pub struct Event {
    pub t: f64,
    pub kind: String,
    pub fields: Vec<Value>,
}

impl PartialEq for Event {
    /// Bit-exact: `t` compares by its bits, the fields as [`Value`]s.
    fn eq(&self, other: &Self) -> bool {
        self.t.to_bits() == other.t.to_bits() && self.kind == other.kind && self.fields == other.fields
    }
}

impl Eq for Event {}

impl Event {
    /// Python's `e[i]` for `i >= 2` (`at(2)` is `who`).
    ///
    /// # Panics
    /// When `i < 2` (`t` and `kind` are not `Value`s: use `f(0)` and `s(1)`) or `i` is past the end, as Python
    /// raises `IndexError`.
    pub fn at(&self, i: usize) -> &Value {
        assert!(i >= 2, "event {:?}: at({i}): e[0] is .t and e[1] is .kind", self.kind);
        match self.fields.get(i - 2) {
            Some(v) => v,
            None => panic!(
                "event {:?}: no element {i} (it has {})",
                self.kind,
                2 + self.fields.len()
            ),
        }
    }

    /// `e[i]` as a float: `f(0)` is `t`; an `Int` element is widened, as Python arithmetic would.
    ///
    /// # Panics
    /// When the element is missing or is not a number.
    pub fn f(&self, i: usize) -> f64 {
        if i == 0 {
            return self.t;
        }
        match self.at(i) {
            Value::Float(x) => *x,
            Value::Int(n) => *n as f64,
            v => panic!("event {:?}: element {i} is {v:?}, not a number", self.kind),
        }
    }

    /// `e[i]` as an integer.
    ///
    /// # Panics
    /// When the element is missing or is not an `Int`.
    pub fn i(&self, i: usize) -> i64 {
        match self.at(i) {
            Value::Int(n) => *n,
            v => panic!("event {:?}: element {i} is {v:?}, not an integer", self.kind),
        }
    }

    /// `e[i]` as a string: `s(1)` is `kind`.
    ///
    /// # Panics
    /// When the element is missing or is not a `Str`.
    pub fn s(&self, i: usize) -> &str {
        if i == 1 {
            return &self.kind;
        }
        match self.at(i) {
            Value::Str(s) => s,
            v => panic!("event {:?}: element {i} is {v:?}, not a string", self.kind),
        }
    }

    /// `e[2]`, the fighter the event is about, when it is a fighter index: `None` when `e[2]` is absent or is not a
    /// non-negative integer (a `clash` carries -1).
    pub fn who(&self) -> Option<usize> {
        match self.fields.first() {
            Some(Value::Int(n)) => usize::try_from(*n).ok(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit() -> Event {
        Event {
            t: 4.48824,
            kind: "hit".into(),
            fields: vec![
                Value::Int(1),
                Value::Int(0),
                Value::Float(1.4000000000000001),
                Value::Float(-0.65),
                Value::Float(-0.73),
                Value::Str("blade".into()),
                Value::Int(1),
            ],
        }
    }

    #[test]
    fn accessors_index_like_python() {
        let e = hit();
        assert_eq!(e.f(0), 4.48824);
        assert_eq!(e.s(1), "hit");
        assert_eq!(e.at(2), &Value::Int(1));
        assert_eq!(e.who(), Some(1));
        assert_eq!(e.i(3), 0);
        assert_eq!(e.f(3), 0.0);
        assert_eq!(e.f(4), 1.4000000000000001);
        assert_eq!(e.s(7), "blade");
        assert_eq!(e.i(8), 1);
    }

    #[test]
    fn who_is_none_for_a_clash_or_a_short_event() {
        let clash = Event {
            t: 1.0,
            kind: "clash".into(),
            fields: vec![Value::Int(-1), Value::Float(0.5)],
        };
        assert_eq!(clash.who(), None);
        let bare = Event {
            t: 1.0,
            kind: "x".into(),
            fields: vec![],
        };
        assert_eq!(bare.who(), None);
        let odd = Event {
            t: 1.0,
            kind: "x".into(),
            fields: vec![Value::Float(1.0)],
        };
        assert_eq!(odd.who(), None);
    }

    #[test]
    #[should_panic(expected = "no element 9")]
    fn past_the_end_panics_like_python() {
        hit().at(9);
    }

    #[test]
    fn equality_is_bit_exact() {
        assert_eq!(Value::Float(f64::NAN), Value::Float(f64::NAN));
        assert_ne!(Value::Float(0.0), Value::Float(-0.0));
        assert_ne!(Value::Int(1), Value::Float(1.0));
        let mut e = hit();
        e.t = -0.0;
        let mut z = hit();
        z.t = 0.0;
        assert_ne!(e, z);
        assert_eq!(hit(), hit());
    }
}
