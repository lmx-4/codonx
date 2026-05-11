# Changelog

## 0.0.3 - 2026-05-11

Small Codon debug workflow release.

- Added source-level `#%define CODON_PYTHON <path>` and `#%define CODON_DEBUG <relative-or-absolute-dir>` directives.
- Stripped supported `#%define` directives from Python and Codon outputs.
- Injected `CODON_PYTHON` and `CODON_DEBUG` into `codon run` / `codon build` subprocess environments.
- When `CODON_DEBUG` is defined in debug mode, codonx creates the target directory, runs Codon from that directory, and appends `-log l` to generate Codon dump files unless a log option is already present.
- Added tests for define stripping, environment injection, debug dump argument insertion, and release-mode behavior.

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
