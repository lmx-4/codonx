# codonx 0.1.x Evolution Roadmap

This document describes the intended 0.1.x evolution path.

0.1.x is the transition line from the 0.0.x regex/string MVP to a conservative
local AST and span-based rewrite engine. By the end of 0.1.x, all automatic
rewrites inherited from the 0.0.x regex layer should be implemented through the
local AST pipeline, or removed and replaced by explicit diagnostics.

Codon compiler log AST integration is intentionally out of scope for 0.1.x.

## Goals

- Replace regex-based mechanical rewriting with a local AST and span patch
  pipeline.
- Preserve the Codon-first workflow: one source file, Python debug target, Codon
  release target.
- Keep automatic lowering conservative, local, explainable, and testable.
- Ensure uncertain cases fail closed: keep source unchanged and emit a warning
  instead of guessing.
- Keep the implementation smaller than a full Codon parser.

## Non-Goals

- No full Codon parser.
- No full Python parser.
- No Codon compiler `_dump_typecheck.sexp` or native AST dependency in 0.1.x.
- No overload resolution, generic monomorphization, ownership analysis, pointer
  analysis, GPU semantic simulation, or parallel race simulation.
- No direct Codon-to-Python transpiler ambition.

## Architecture Direction

0.1.x should introduce a three-stage rewrite architecture:

```text
selected source after codonx directives
    -> lexical/source model with spans
    -> block index and local AST
    -> validated rewrite plan
    -> span patch application
```

The local AST is a codonx rewrite IR, not a language-complete AST. It should
only model constructs that codonx can safely lower for Python debug output.

## Stage 1: Source Model and Spans

Introduce a source model that can represent stable byte spans and line/column
positions after directive selection.

Required behavior:

- Preserve raw source text, comments, whitespace, and newlines.
- Track line number, byte offsets, indentation, and triple-string state.
- Provide spans for whole lines, block headers, block bodies, and local tokens.
- Support patch application by non-overlapping spans.
- Reject overlapping patches as an internal error.

The existing `SourceLine` model can remain as a compatibility layer during the
transition, but new rewrite code should use spans as the source of truth.

## Stage 2: Conservative Block Index

Replace broad rewrite scans with a block indexer.

The block indexer should detect candidate regions, not perform final rewrites.
It should prefer safe over clever behavior.

Initial block kinds:

- module;
- decorated function;
- class;
- `if` / `elif` / `else`;
- `for`;
- `while`;
- `with`;
- unknown indented block.

Rules:

- Determine block boundaries by indentation and lexical state, not by multiline
  regex.
- Treat comments and blank lines as part of the surrounding block when safe.
- Keep decorators attached to the following function or class.
- Never enter triple-quoted strings.
- Produce warnings for malformed or ambiguous indentation instead of rewriting.

## Stage 3: Local AST

Implement only the nodes needed for existing codonx rewrites.

Initial statement nodes:

- function signature;
- annotated assignment;
- simple assignment;
- `return`;
- `for ... in range(...)`;
- `for ... in static_range(...)`;
- import and `from python import ...`;
- decorator;
- unknown statement.

Initial expression nodes:

- name;
- literal;
- attribute;
- call;
- subscript;
- tuple/list/dict/set literal when syntactically shallow;
- unary and binary expressions required by existing rewrites;
- unknown expression.

Type expression nodes:

- name type, such as `i32`, `u64`, `f64`;
- generic type, such as `List[i32]` or `Dict[str, i32]`;
- union-like type forms already recognized by codonx;
- pointer/C interop/GPU-related types as unsupported typed nodes;
- unknown type.

Unknown nodes are mandatory. They prevent the local AST from expanding into a
full parser. A rewrite may only modify a node when all required child nodes are
known and validated.

## Stage 4: Rewrite Plan

Automatic lowering should produce a rewrite plan rather than directly emitting
rewritten lines.

Patch operations:

- replace span;
- insert before span;
- insert after span;
- delete span;
- add warning without patch.

Patch rules:

- Patches must be non-overlapping.
- Patches must reference original selected-source spans.
- A rewrite pass must be idempotent where practical.
- If validation fails, no partial patch from that rewrite may be applied.

## Stage 5: Migrate 0.0.x Regex Rewrites

Every 0.0.x rewrite must be migrated to AST/span form before 0.1.x is complete.

Migration order:

1. Function signatures and type annotation translation.
2. Runtime guard insertion for parameters, assignments, and returns.
3. Scalar cast lowering, such as `i32(x)` to Python-compatible behavior.
4. `from python import ...` lowering.
5. Generic function/class type parameter erasure for Python 3.12 debug output.
6. `@par`, `@llvm`, `@extend`, and other Codon-only annotation handling.
7. `static_range` and range-like loop lowering.
8. Unsupported boundary reporting currently tied to regex matching.

Completion rule:

- The old regex rewrite implementation must either be deleted or reduced to
  lexical candidate detection that is followed by AST validation.
- No production rewrite should rely on regex capture groups as its final source
  of truth.

## Handling False Positives and False Negatives

Regex-like scanning is allowed only for candidate discovery.

False positives are controlled by validation:

- A candidate block must parse into the required local AST shape.
- If it does not parse, codonx keeps the original source and emits a warning
  when the construct looks relevant.
- Rewriters must not operate on `Unknown` nodes unless the rewrite explicitly
  supports preserving the unknown text unchanged.

False negatives are controlled by sentinel detection:

- Known rewrite-trigger tokens such as `static_range`, fixed-width scalar casts,
  `from python import`, `@par`, `@llvm`, `@extend`, and typed assignments should
  be detected lexically.
- If a sentinel appears outside a successfully parsed rewrite region, codonx
  should emit a diagnostic instead of silently ignoring it.
- Tests must cover both "similar but should not rewrite" and "should warn but
  not rewrite" cases.

## Diagnostics and Reports

0.1.x diagnostics should become more precise than the 0.0.x regex boundary
warnings.

Suggested warning kinds:

- `unsupported-local-parse`;
- `ambiguous-block-boundary`;
- `unsupported-type-expression`;
- `unsupported-expression-shape`;
- `unsupported-statement-shape`;
- `rewrite-skipped`;
- `patch-conflict`;
- `legacy-regex-fallback`.

The final 0.1.x release should not emit `legacy-regex-fallback` during normal
operation.

## Version Milestones

### 0.1.1: Local AST/Span Foundation

- Add local spans and patch application for supported rewrite regions.
- Move function signatures, annotated assignments, class signatures, generic
  parameter erasure, `Static[...]` inheritance, `from python import`, scalar
  casts, and `static.range` onto AST/span or token/span handling.
- Add tests for multiline headers, nested scalar casts, comments, strings, and
  Python 3.12 generic class edge cases.

### 0.1.2: Diagnostics and Sentinel Warnings

- Detect known rewrite-trigger tokens outside successfully parsed rewrite
  regions.
- Emit explicit diagnostics for parse failures, skipped rewrites, and legacy
  fallback paths instead of silently preserving suspicious code.
- Add tests for malformed candidates and unsupported-but-recognized syntax.

### 0.1.3: Block Index

- Add indentation-based block indexing.
- Attach decorators to function/class blocks.
- Detect candidate regions for current rewrites.
- Keep old rewrite output as the behavior baseline.

### 0.1.4: Assert Migration

- Move runtime guard insertion fully onto AST/span-derived nodes.
- Unify parameter, return, and annotated-assignment type sources.
- Keep assert behavior conservative when a type expression is unknown.

### 0.1.5: Type and Expression Coverage

- Expand local type-expression handling only for constructs required by existing
  rewrites and guards.
- Add unsupported expression diagnostics for ambiguous forms.
- Expand tests around default values, nested annotations, multiline statements,
  comments, strings, and idempotence.

### 0.1.6: Loop and Codon-Only Construct Handling

- Move `static_range`, `@par`, `@llvm`, `@extend`, and related Codon-only
  handling onto block-aware AST/span logic.
- Ensure unsupported constructs produce explicit diagnostics and no unsafe
  rewrites.

### 0.1.7: Regex Removal Gate

- Delete or quarantine production regex rewrite paths.
- Keep only lexical sentinel detection and small token-level helpers where they
  are followed by parser validation.
- Add regression tests proving every previous 0.0.x rewrite is covered by the
  AST/span path.

### 0.1.8: Stabilization

- Treat AST/span rewrite behavior as the 0.1.x stable implementation.
- Remove `legacy-regex-fallback` from normal reports.
- Update README stability text to reflect that 0.1.x has completed the migration
  away from regex-level rewriting.

## Acceptance Criteria

0.1.x is complete when:

- All automatic rewrites use local AST/span validation.
- Regex is not used as the final authority for any mechanical rewrite.
- Existing 0.0.x behavior is either preserved, intentionally tightened, or
  replaced by a documented warning.
- Ambiguous code is preserved unchanged.
- Reports clearly explain every skipped rewrite.
- Codon release output remains close to the selected Codon source branch.
- Python debug output remains Python 3.12+.

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
- patch conflict rejection.

Regression fixtures should include the current MVP integration tests and new
cases that previously motivated regex boundary warnings.

## Implementation Policy

- Add AST support only when a concrete rewrite needs it.
- Prefer `Unknown` nodes and diagnostics over expanding grammar scope.
- Keep Codon native AST/log integration out of 0.1.x.
- Do not optimize for performance until the AST/span behavior is correct.
- Keep public CLI behavior stable unless a regex rewrite was unsafe.
