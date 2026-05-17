# codonx 0.1.x Evolution Roadmap

This roadmap describes the 0.1.x line after the 0.1.4 release.

The purpose of 0.1.x is to move `codonx` from the old 0.0.x string/regex MVP to
a conservative local AST/span rewrite engine. The design target is not a full
Codon parser. The target is a practical rewrite IR that can safely support the
documented Python debug projection while keeping Codon release output close to
the selected source branch.

Codon compiler log AST integration is intentionally out of scope for 0.1.x.

## Current 0.1.4 State

0.1.4 has established the current public contract:

- Linux-only support.
- Python debug output requires Python 3.12 or newer.
- `codonx run` and `codonx build` require the official Codon compiler.
- Mainline numeric documentation and examples prefer Codon-style `int` and
  `float`.
- Fixed-width types such as `i32`, `u64`, and `f32` are explicit low-level
  intent, not the default inference target.
- Semantic rewrites are routed through local AST/span or token-aware mechanisms.
- Regex is no longer the final authority for mechanical semantic rewrites.
- Runtime guards cover supported parameters, annotated assignments, return
  values, scalar casts, and container shapes.
- `Literal[...]`, `tuple[T, ...]`, supported attribute/index annotated
  assignments, and multiline returns have stronger Python debug guard coverage.
- Multiline support exists for conservative expression-block cases, but
  arbitrary multiline rewriting remains out of scope.

## Goals

- Preserve the Codon-first workflow: one source file, Python debug target, Codon
  release target.
- Keep automatic lowering conservative, local, explainable, and testable.
- Prefer diagnostics or no-op behavior over unsafe guessing.
- Expand local AST coverage only when a concrete supported rewrite needs it.
- Keep the implementation smaller than a full Codon parser.
- Keep Linux/Python 3.12+/Codon compiler requirements explicit.

## Non-Goals

- No full Codon parser.
- No full Python parser.
- No Python-to-Codon migration engine.
- No complete Codon-to-Python transpiler.
- No Codon compiler `_dump_typecheck.sexp` or native AST dependency in 0.1.x.
- No overload resolution, generic monomorphization, ownership analysis, pointer
  analysis, GPU semantic simulation, or parallel race simulation.
- No support promise for non-Linux platforms in 0.1.x.

## Architecture Direction

The intended rewrite architecture is:

```text
selected source after codonx directives
    -> lexical/source model with spans
    -> local block and statement candidates
    -> local AST or token-aware validation
    -> rewrite plan
    -> non-overlapping span patch application
```

The local AST is a codonx rewrite IR, not a language-complete AST. It should
only model constructs that `codonx` can safely lower for Python debug output.
Unknown nodes are a feature, not a failure: they keep the parser from expanding
into a fragile half-language implementation.

## Source Model and Spans

Required behavior:

- Preserve raw source text, comments, whitespace, and newlines.
- Track line number, byte offsets, indentation, and triple-string state.
- Provide spans for whole lines, block headers, block bodies, and local tokens.
- Support patch application by non-overlapping spans.
- Reject overlapping patches as an internal error.

The old `SourceLine` compatibility layer can remain where it is useful, but new
rewrite code should treat spans as the source of truth.

## Local AST Scope

Statement nodes should remain limited to supported rewrite needs:

- function signatures;
- class signatures;
- annotated assignments;
- simple assignments where guard insertion needs them;
- `return`;
- `for` loops relevant to `range` or `static.range`;
- imports and `from python import`;
- decorators;
- unknown statements.

Expression nodes should be added only as needed:

- names;
- literals;
- attributes;
- calls;
- subscripts;
- tuple/list/dict/set literals when syntactically shallow;
- unary/binary expressions needed by existing rewrites;
- unknown expressions.

Type expression nodes should cover:

- name types such as `int`, `float`, `i32`, `u64`, `f64`;
- generic types such as `list[int]` and `Dict[str, i32]`;
- existing `Optional`, `Union`, `Literal`, and container guard forms;
- pointer/C/GPU-related types as unsupported typed nodes;
- unknown type expressions.

A rewrite may only modify a node when the required shape is known and
validated. Unknown text may be preserved, but it should not be silently
reinterpreted.

## Rewrite Plan Rules

Automatic lowering should produce a rewrite plan rather than directly mutating
lines.

Patch operations:

- replace span;
- insert before span;
- insert after span;
- delete span;
- add warning without patch.

Patch rules:

- Patches must be non-overlapping.
- Patches must reference original selected-source spans.
- A rewrite pass should be idempotent where practical.
- If validation fails, no partial patch from that rewrite may be applied.

## Handling False Positives and False Negatives

Candidate scanning is allowed only for discovery.

False positives are controlled by validation:

- A candidate block must parse into the required local AST shape.
- If parsing fails, `codonx` keeps the original source and emits a warning when
  the construct looks relevant.
- Rewriters must not operate on `Unknown` nodes unless the rewrite explicitly
  supports preserving the unknown text unchanged.

False negatives are controlled by sentinel diagnostics:

- Known trigger tokens such as `static.range`, fixed-width scalar casts,
  `from python import`, `@par`, `@llvm`, `@extend`, and typed assignments should
  be detected lexically.
- If a sentinel appears outside a successfully parsed rewrite region, `codonx`
  should emit a diagnostic instead of silently ignoring it.
- Tests should cover both "similar but should not rewrite" and "should warn but
  not rewrite" cases.

## Completed Milestones

### 0.1.1: Local AST/Span Foundation

- Added local spans and patch application for supported rewrite regions.
- Moved function signatures, annotated assignments, class signatures, generic
  parameter erasure, `Static[...]` inheritance, `from python import`, scalar
  casts, and `static.range` onto AST/span or token/span handling.
- Added tests for multiline headers, nested scalar casts, comments, strings, and
  Python 3.12 generic class edge cases.

### 0.1.2: Regex Residual Closure and Assert Robustness

- Removed the legacy `regex` dependency and regex-based annotation fallback.
- Kept regex-like searching limited to candidate discovery rather than final
  semantic authority.
- Improved assert lowering for Codon type tokens while preserving Python 3.12
  assert semantics.
- Expanded tests around regex-free rewrite paths and assert conversion edge
  cases.

### 0.1.3: Codon-Standard Numeric Semantics and Smoke Coverage

- Reoriented examples and documentation around `int` and `float` as the
  recommended Codon-style mainline types.
- Treated fixed-width aliases as explicit low-level range/precision intent.
- Added conservative multiline expression-block lowering.
- Fixed generated Python helper names to avoid class-scope name mangling.
- Made scalar cast lowering iterate to a stable result for nested supported
  casts.
- Added optional local Codon compile/run smoke coverage when a Codon compiler is
  available.

### 0.1.4: Local AST Guard Consolidation

- Extended local AST assignment target support for guarded annotated
  assignments.
- Added stronger `Literal[...]`, `tuple[T, ...]`, and multiline return guard
  behavior.
- Hardened call-form `@llvm(...)` and `@extend(...)` block omission.
- Validated the supported subset with a 300+ line practical Python/Codon smoke
  fixture.

## Next Milestones

### 0.1.5: Block-Aware Diagnostics and Guard Source Unification

- Move remaining guard insertion decisions onto AST/span-derived nodes.
- Unify parameter, return, annotated-assignment, and scalar-cast type sources.
- Improve diagnostics for unknown or partially supported type expressions.
- Add more tests for class methods, multiline annotations, and nested containers.

### 0.1.6: Loop and Codon-Only Construct Hardening

- Harden `static.range`, `@par`, `@llvm`, `@extend`, and related Codon-only
  handling with block-aware validation.
- Ensure unsupported constructs produce explicit diagnostics and no unsafe
  rewrites.
- Expand practical fixtures that compile and run under both Python debug and
  Codon release paths.

### 0.1.7: Stability Gate

- Prove every previous 0.0.x rewrite is covered by AST/span or token-aware
  validation.
- Add regression tests for comments, strings, triple-quoted strings, multiline
  expressions, idempotence, and patch conflicts.
- Remove stale documentation or diagnostics that imply regex-final rewriting.

### 0.1.8: 0.1.x Stabilization

- Treat local AST/span rewrite behavior as the stable 0.1.x implementation.
- Freeze the documented 0.1.x CLI unless a behavior is unsafe.
- Update README, design docs, and release checklist to reflect the stabilized
  boundary.

## Acceptance Criteria for 0.1.x

0.1.x is complete when:

- All automatic rewrites use local AST/span or token-aware validation.
- Regex is not used as the final authority for any mechanical semantic rewrite.
- Existing 0.0.x behavior is preserved, intentionally tightened, or replaced by
  a documented warning.
- Ambiguous code is preserved unchanged.
- Reports clearly explain skipped rewrites.
- Codon release output remains close to the selected Codon source branch.
- Python debug output remains Python 3.12+.
- Documentation consistently states Linux-only and Codon-compiler-required
  operation.

## Testing Strategy

Each migrated rewrite needs tests for:

- normal rewrite;
- no rewrite inside comments;
- no rewrite inside strings or triple-quoted strings;
- no rewrite for lookalike syntax;
- nested expression behavior;
- multiline boundary behavior;
- skipped rewrite diagnostic;
- idempotence where practical;
- patch conflict rejection;
- optional Codon compiler smoke behavior when the local machine has Codon.

Regression fixtures should include small unit tests, integration tests, and at
least one practical 200+ line source file that exercises real optimization
branches.

## Implementation Policy

- Add AST support only when a concrete rewrite needs it.
- Prefer `Unknown` nodes and diagnostics over expanding grammar scope.
- Keep Codon native AST/log integration out of 0.1.x.
- Do not optimize for performance until the AST/span behavior is correct.
- Keep public CLI behavior stable unless a previous rewrite was unsafe.
