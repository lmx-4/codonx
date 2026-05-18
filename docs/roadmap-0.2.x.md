# codonx 0.2.x Evolution Roadmap

This roadmap describes the intended 0.2.x line after the 0.1.4 release.

0.1.x proved the current Codon-first preprocessing model: directive selection,
local AST/span rewrites, guarded Python debug output, and thin Codon compiler
wrapping. 0.2.x starts the next architecture step: a Python 3.12 frontend backed
by Ruff's parser, a stable CodonX IR, and conservative "translate what is safe"
planning.

0.2.x must not become a full Python-to-Codon promise. Its job is to build the
front-end and IR foundation that can later support Python -> Codon, Codon ->
Python, and CPython fallback islands.

## Strategic Direction

The 0.2.x architecture target is:

```text
Python 3.12 source
    -> Ruff parser AST and token stream
    -> CodonX IR
    -> macro/hint binding
    -> convertibility analysis
    -> guarded Codon candidate output or fallback diagnostics
```

The long-term bidirectional direction is:

```text
Python source -> Ruff AST -> CodonX IR -> Codon target
Codon/CodonX source -> CodonX subset parser -> CodonX IR -> Python target
```

The central object is CodonX IR, not Ruff AST and not Codon native AST. Ruff is
the Python parser frontend. Codon remains the release compiler. CodonX IR is the
project-owned semantic boundary where macros, diagnostics, guards, fallback
planning, and later bidirectional projection are represented.

## Ruff Integration Policy

0.2.x should use Ruff parser crates as dependencies or as a source-compatible
frontend layer. It should not fork Ruff at the start.

Rationale:

- Ruff already parses Python 3.12 syntax.
- Ruff AST nodes preserve text ranges.
- Ruff token streams preserve comments.
- `#%` macros can be represented as comment-token metadata and bound to nearby
  AST nodes.
- Forking Ruff would immediately inherit parser maintenance, Python-version
  drift, AST schema drift, and upstream bug tracking.

Forking or patching Ruff is only justified if comment-token macro binding cannot
represent a required language feature after a working IR prototype exists.

## Macro Binding Rule

0.2.x treats `#%` syntax as source comments, not custom Python syntax.

Binding policy:

- A contiguous leading block of `#%` comments binds to the next relevant AST
  node at the same or deeper logical position.
- Inline `#%` comments may bind to the statement on the same physical line only
  when this is unambiguous.
- Ambiguous macro placement must produce a diagnostic instead of guessing.
- Macro binding must preserve byte ranges and line numbers for reports.
- Macro binding must be represented in CodonX IR, not hidden in a rewrite pass.

Initial macro categories:

- `#%codon`: bind to the next import and require Codon native import semantics;
- `#%parallel`: bind to a loop or function and request Codon parallel planning;
- `#%type`: add type intent that annotations cannot express clearly;
- `#%fallback`: make a CPython fallback boundary explicit;
- unsupported or unknown macro diagnostics.

0.2.x intentionally does not start with broad low-level macro categories such as
`#%ffi`, `#%unsafe`, or a separate `#%generic`. Codon already exposes its own
compiler flags, decorators, C interop, and Python interop. CodonX macros should
first model Python-to-Codon translation intent, not invent another low-level
optimization interface.

## CodonX IR Requirements

0.2.x uses two related artifacts:

- the primary semantic IR is executable Python assert IR;
- JSON output is a debug dump for tests, snapshots, and inspection.

Required module-level fields:

- schema version;
- source path;
- Python target version;
- functions;
- classes;
- imports;
- top-level statements;
- macro attachments;
- diagnostics.

Required node-level fields:

- stable node kind;
- source byte range;
- start and end line;
- optional name;
- annotations where available;
- attached macros;
- import policy for import nodes;
- conversion status;
- diagnostic IDs.

Initial conversion statuses:

- `codon_native`: safe candidate for direct Codon generation;
- `guarded`: can be generated with runtime assertions or explicit comments;
- `fallback`: should remain Python/CPython-backed;
- `unsupported`: cannot be translated yet and must explain why.

## 0.2.x Milestones

### 0.2.0: Ruff Frontend and Assert IR

- Add a `codonx ir <input.py>` style command.
- Add a `codonx assert-ir <input.py>` command that emits executable Python 3.12
  semantic IR.
- Parse Python 3.12 source through Ruff parser.
- Keep JSON output as a debug dump, not as the primary semantic IR.
- Emit assert IR that preserves source shape and inserts Codon-facing runtime
  guards around supported annotations and returns.
- Represent functions, classes, imports, assignments, annotations, returns,
  calls, binary expressions, subscripts, `if`, `for`, and `while` at a useful
  structural level.
- Preserve source ranges and line numbers.
- Keep existing 0.1.x Codon-first CLI behavior unchanged.

The first implementation may start with statement-level IR plus macro and
convertibility skeletons. Expression-level IR is still part of the 0.2.0 target
before the line is considered complete.

### 0.2.1: `#%` Macro Attachment

- Extract `#%` directives from Ruff tokens or source ranges.
- Attach leading macro blocks to AST nodes.
- Emit macro attachments in IR.
- Reject ambiguous macro placement with diagnostics.
- Add tests for function, class, loop, assignment, and inline macro placement.

### 0.2.2: Convertibility Analysis and First Codon Candidate

- Analyze function-level and statement-level convertibility.
- Classify nodes as `codon_native`, `guarded`, `fallback`, or `unsupported`.
- Explain every fallback or unsupported decision in diagnostics.
- Keep the rule conservative: no silent semantic guessing.
- Add `codonx py-codon <input.py>` as a compile-first candidate generator.
- Route default Python imports through Codon's `from python import ...`
  fallback, while `#%codon` imports keep native Codon import semantics.
- Surface `#%define CODON_PYTHON` in the generated candidate so the caller knows
  which `CODON_PYTHON` value to inject into the Codon process.

### 0.2.3: Safer Native Python -> Codon Generation

- Expand generation beyond import policy and common-subset source preservation.
- Generate Codon rewrites only for functions and statements classified as safe.
- Preserve unsupported regions through diagnostics or explicit fallback stubs.
- Do not claim whole-file conversion unless every required region is safe.
- Reuse 0.1.x numeric and type policy: prefer `int` and `float`, treat fixed
  width types as explicit low-level intent.

### 0.2.4: Guarded Assert IR Integration

- Route supported annotations through the existing guard/assert machinery.
- Generate Python-debug-friendly assert IR from CodonX IR.
- Keep guard insertion explainable in reports.
- Ensure guard behavior remains a mismatch detector, not an equivalence proof.

### 0.2.5: CPython Fallback Island Prototype

- Represent fallback regions explicitly in IR.
- Prototype calling Python-backed code for unsupported functions.
- Require diagnostics that make the boundary visible.
- Avoid hidden performance cliffs: fallback must be reported.

### 0.2.6: Practical Validation Gate

- Add a 300+ line Python fixture with `#%` hints.
- Compare Python output with generated Codon/fallback behavior where feasible.
- Measure frontend parse time, IR generation time, and generated diagnostics.
- Document which constructs are safe, guarded, fallback, or unsupported.

## Non-Goals for 0.2.x

- No direct Ruff fork unless proven necessary.
- No Codon native AST dependency.
- No complete Python -> Codon conversion promise.
- No complete Codon -> Python conversion promise.
- No whole-program type inference.
- No overload resolution equivalent to Codon.
- No simulation of parallel races, GPU execution, LLVM, C pointer behavior, or
  Python interop conversion behavior.
- No claim of 100% compatibility.

## Acceptance Criteria for 0.2.x

0.2.x is successful when:

- A Python 3.12 file can be parsed into stable CodonX IR.
- `#%` comments can be attached to AST nodes without custom Python syntax.
- Each relevant function receives a clear conversion status.
- Reports explain every unsupported or fallback decision.
- Existing 0.1.x Codon-first behavior remains intact.
- Tests prove ranges, macro binding, diagnostics, and conversion classification.
- The implementation makes later Python -> Codon and Codon -> Python projection
  possible without committing to full conversion in 0.2.x.

## Implementation Policy

- Build IR first, generation second.
- Make every unsafe decision visible in diagnostics.
- Prefer fallback or unsupported status over speculative rewrites.
- Keep Ruff integration isolated behind a frontend module.
- Keep CodonX IR independent from Ruff's concrete AST shape.
- Do not expand 0.1.x local AST as the main path once Ruff IR work begins.
