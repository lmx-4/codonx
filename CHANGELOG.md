# Changelog

## 0.1.0 - 2026-05-12

Regex-level MVP stabilization release.

- Stabilized codonx as a Python 3.12+ debug target and Codon release target preprocessor.
- Froze the regex/string-level lowering boundary except for conservative bug fixes.
- Documented the stability contract: ambiguous Codon/Python semantic differences require explicit `#%ifpy` / `#%ifcodon` branches.
- Kept the conservative rewrite policy: codonx should prefer warning or no-op behavior over risky automatic rewrites.
- Omitted `@extend` class blocks in Python debug output to avoid shadowing Python builtins.
- Documented Linux x86_64 binary usage and the real 0.1.0 rewrite/assert boundaries.

## 0.0.8 - 2026-05-12

Python 3.12+ target clarification release.

- Documented Python debug output as targeting Python 3.12 and newer only.
- Preserved Python 3.12 generic function/class syntax instead of erasing PEP 695 type parameter lists.
- Kept erasing Codon compile-time `T: type` parameters from Python debug call signatures.
- Added warning coverage for `@overload`, `@codon.jit`, `@codon.convert`, `Static[...]` inheritance, Codon ndarray types, and additional float widths.
- Added `complex` guard support and Python `float` lowering for `float16`, `bfloat16`, and `float128`.

## 0.0.7 - 2026-05-12

Regex exhaustion release.

- Added scalar cast lowering for fixed-width integer casts and documented simple-call boundaries.
- Added generic function/class type-parameter erasure for Python debug output.
- Added warning/removal coverage for `@export`, `@tuple`, `@extend`, LLVM-related annotations, C interop, pointer interop, and `static.range`.
- Added guard support for `Optional[T]`, `Union[...]`, `NoneType`, and `Literal[...]` softening.
- Added report JSON counters for lowered casts, erased generics, interop warnings, and unsupported regex boundaries.
- Documented 0.0.7 as the final regex/string-level expansion release before the 0.1.0 boundary.

## 0.0.6 - 2026-05-12

Python debug assert enhancement release.

- Added guard support for `Int[N]`, `UInt[N]`, `byte`, `float32`, and uppercase container aliases.
- Added generation-time warnings for unknown guard types, unchecked dynamic types, float32 precision risk, unordered dict/set behavior, and unsupported tuple ellipsis.
- Kept unknown and dynamic guard types as soft-pass behavior to avoid false positives in 0.0.x.
- Preserved existing `--assert off`, `--assert shallow`, and `--assert full` CLI behavior while expanding full container recursion coverage.
- Added report JSON counters for unknown guard types, unchecked dynamic types, and semantic warnings.

## 0.0.5 - 2026-05-11

Target directive model release.

- Added `#%ifpy` and `#%ifcodon` conditional directives, both with `#%else` and `#%endif` support.
- Kept `#%ifdebug` as a deprecated compatibility alias for `#%ifpy`.
- Updated docs, examples, and unsupported interop guidance to prefer explicit Python/Codon target branches.
- Generalized directive error messages for stray `#%else`, stray `#%endif`, duplicate `#%else`, and unclosed target branches.
- Added tests for `#%ifpy`, `#%ifcodon`, nested target branches, deprecated `#%ifdebug`, and directive error boundaries.

## 0.0.4 - 2026-05-11

Codon debug dump isolation release.

- Changed `codonx run` with `CODON_DEBUG` to keep the user program runtime cwd stable.
- Split debug `run` into a dump-directory `codon build` followed by executing the temporary binary from the original cwd.
- Kept `codonx build` dump redirection behavior by invoking Codon from the debug directory.
- Fixed automatic `-log l` insertion bookkeeping after inserting arguments before the preprocessed source path.
- Added `.codon` suffix coverage for 0.0.3/0.0.4 features and boundary cases.
- Added tests for unsupported and malformed `#%define` directives.

## 0.0.3 - 2026-05-11

Small Codon debug workflow release.

- Added source-level `#%define CODON_PYTHON <path>` and `#%define CODON_DEBUG <relative-or-absolute-dir>` directives.
- Stripped supported `#%define` directives from Python and Codon outputs.
- Injected `CODON_PYTHON` and `CODON_DEBUG` into `codon run` / `codon build` subprocess environments.
- When `CODON_DEBUG` is defined in debug mode, codonx creates the target directory and appends `-log l` to generate Codon dump files unless a log option is already present.
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
