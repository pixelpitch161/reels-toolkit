//! Engine 2's fighters, as `bf2-sim` data (Engine 2 design section 7). Each fighter lives in its own folder and
//! builds its [`FighterDef`] in Rust. None is offered yet: the registry's [`crate::engines`] and
//! [`crate::available`] know nothing of these names until a fighter session moves a fighter here.

use std::sync::{Arc, OnceLock};

use bf2_sim::FighterDef;

mod test_dummy;

/// Every Engine 2 fighter, in order.
pub const NAMES: [&str; 1] = ["TEST DUMMY"];

/// The Engine 2 definition of the fighter called `name`, built once and shared.
pub fn def(name: &str) -> Option<Arc<FighterDef>> {
    static DUMMY: OnceLock<Arc<FighterDef>> = OnceLock::new();
    match name {
        "TEST DUMMY" => Some(DUMMY.get_or_init(|| Arc::new(test_dummy::def())).clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummy_checks() {
        let d = def("TEST DUMMY").expect("the dummy is defined");
        assert_eq!(d.check(), Ok(()));
        assert_eq!(d.name, "TEST DUMMY");
        assert!(NAMES.iter().all(|n| def(n).is_some()));
        assert!(def("FLUTE").is_none());
    }

    #[test]
    fn dummy_is_not_offered() {
        assert!(crate::engines("TEST DUMMY").is_empty());
        assert!(!crate::available().contains(&"TEST DUMMY"));
        assert!(!crate::PORTED.contains(&"TEST DUMMY"));
    }
}
