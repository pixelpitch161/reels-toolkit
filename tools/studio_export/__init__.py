"""BALLFIGHT Studio reference export: reads the Python episode kit (projects/, reelkit/, series/) without editing it
and writes the committed reference data under studio/reference/ that the Rust parity tests compare against.

    python -m tools.studio_export list | <name>... | all | verify        (from the repo root; see README.md)

A module in this package registers its exports itself with a module-level `EXPORTS = {"name": function}`; the entry
point finds them by scanning the package, so a new export is a new module and nothing else changes."""
