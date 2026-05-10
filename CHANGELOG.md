# Changelog

## 0.0.2 - 2026-05-10

MVP boundary tightening release.

- Clarified that 0.0.x supports only regex/string-level syntax lowering plus explicit `#%ifdebug` branching for anything more complex.
- Added Python output comments and report warnings for lowered `@par`, `@gpu.kernel`, `@python`, and unsupported typed Python interop declarations.
- Kept semantic guards intentionally basic: scalar bounds, selected containers, function parameters, annotated assignments, and return values.
- Added integration tests for branch selection, regex lowering/report warnings, guard insertion/runtime execution, Codon CLI passthrough, preprocessed-file deletion, and directive errors.
- Updated documentation to match implementation boundaries.

## 0.0.1 - 2026-05-10

Initial MVP release.

- Added Codon-first preprocessing for Python debug and Codon release targets.
- Added `#%ifdebug`, `#%else`, and `#%endif` conditional selection.
- Added Python debug rewrites for selected Codon constructs, including `from python import`, `@par`, `@gpu.kernel`, and `@python`.
- Added runtime semantic guards for primitive types, selected containers, function parameters, annotated assignments, and return values.
- Added `codon`, `run`, `build`, `check`, and `--dbg` CLI flows.
