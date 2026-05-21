# codonx Design Notes

These notes describe the implementation semantics of `codonx` 0.2.3. They are
not a language-completeness promise.

`codonx` currently has two related but different paths:

```text
Codon-first path
    raw .codon/.codonx-style source
    -> directive selection
    -> local AST/span and token-aware lowering
    -> Python 3.12+ debug output or Codon release output
    -> optional real codon run/build wrapper

Python frontend path, experimental
    Python 3.12 source
    -> Ruff parser frontend
    -> CodonX debug view / executable assert IR
    -> conservative Codon candidate
    -> optional real codon run/build wrapper
```

The first path is the practical workflow. The second path is the 0.2.x
foundation for later Python-to-Codon work.

## Core Commitments

### Real Codon Remains the Release Authority

`codonx` does not type-check Codon and does not replace the Codon compiler.
`codonx run`, `codonx build`, `py-run`, and `py-build` all delegate release
behavior to a real `codon` binary after preprocessing or candidate generation.

### Debug Output Is a Mismatch Detector

Python debug output exists for normal Python tooling: `pdb`, pytest, IDE
breakpoints, quick iteration, and runtime guard checks.

It is not a formal equivalence proof. It intentionally does not model Codon
overload resolution, parallel scheduling, GPU execution, LLVM behavior, C
pointer semantics, or Python interop conversion details.

### Explicit Differences Beat Silent Guessing

When Python and Codon need different source, the supported expression is a
target branch:

```python
#%ifpy
# Python debug implementation
#%else
# Codon release implementation
#%endif
```

or:

```python
#%ifcodon
# Codon release implementation
#%else
# Python debug implementation
#%endif
```

Comment directives are used because Python parses the whole file before runtime
branches can help. `if DEBUG:` cannot hide Codon-only syntax from Python's
parser.

## Codon-First Pipeline

The 0.1.x/0.2.3 Codon-first path is:

```text
source text
    -> directive validation and target selection
    -> local source spans
    -> whitelist-based local AST/span rewrites
    -> token-aware transformations that avoid strings/comments
    -> report diagnostics
    -> Python debug output or Codon output
```

Important properties:

- Regex is not the semantic authority for rewrites.
- Rewrites are whitelist-based.
- Comments and quoted strings are protected from semantic lowering.
- Transformations are local and non-overlapping.
- Ambiguous syntax should be preserved or reported, not guessed.

The local AST is intentionally not a full Codon AST. It is a rewrite IR for
constructs `codonx` can safely handle today.

## Python Frontend Pipeline

The 0.2.3 Python path is:

```text
Python 3.12 source
    -> Ruff parser frontend
    -> CodonX debug JSON view, via `codonx ir`
    -> executable Python assert IR, via `codonx assert-ir`
    -> conservative Codon candidate, via `codonx py-codon`
```

`ir` is a debug dump. It is useful for tests, snapshots, and inspection, but it
is not the primary user-facing semantic artifact.

`assert-ir` emits legal Python. It preserves program shape and adds runtime
guards around supported annotations, annotated assignments, and returns.

`py-codon` is compile-first:

- parse Python through Ruff;
- locate imports and macro lines;
- strip supported `#%` control lines from emitted source;
- keep `#%codon` imports as native Codon imports;
- rewrite default Python imports to Codon's `from python import ...` form;
- preserve the remaining Python/Codon common subset as source text.

This means 0.2.3 can generate useful candidates for simple compatible programs
and import-heavy fallback experiments. It does not yet perform broad
statement-level native Codon lowering.

## Import Policy

0.2.x is compatibility-first for Python imports.

Default import:

```python
import json as pyjson
```

generated candidate:

```python
from python import json as pyjson
```

Default from-import:

```python
from pathlib import Path
```

generated candidate shape:

```python
from python import pathlib as __codonx_py_pathlib
Path = __codonx_py_pathlib.Path
```

Native Codon import:

```python
#%codon
import math
```

generated candidate:

```python
import math
```

`#%codon` must bind to the immediately following import. Invalid placement is a
diagnostic condition in the frontend view.

Wildcard from-imports and relative imports that cannot be represented safely are
not expanded by the 0.2.3 generator; the candidate emits a conservative comment
instead of guessing.

## Directive and Define Model

Supported target directives:

```text
#%ifpy
#%ifcodon
#%else
#%endif
```

Compatibility directive:

```text
#%ifdebug
```

`#%ifdebug` is an alias for `#%ifpy`.

Supported define directives:

```text
#%define CODON_PYTHON <path>
#%define CODON_DEBUG <path>
```

Unknown define names are errors. `#%define` is not a general macro system.

Codon-first `run` and `build` inject supported define values into the Codon
subprocess environment.

Python-fronted `py-run` and `py-build` generate a temporary Codon candidate,
inject supported define values, invoke `codon run` or `codon build`, and delete
the candidate unless `--keep-pre` is set.

`#%define CODON_PYTHON` cannot be embedded as executable Codon code, so generated
`py-codon` candidates surface it in a header comment and wrappers inject it into
the process environment.

## Numeric and Type Policy

The mainline style follows Codon's Python-like syntax:

- prefer `int` for normal integer code;
- prefer `float` for normal floating-point code;
- prefer `list[int]`, `dict[str, float]`, and similar high-level containers.

Fixed-width types represent explicit low-level intent:

- `i8`, `i16`, `i32`, `i64`;
- `u8`, `u16`, `u32`, `u64`;
- `Int[N]`, `UInt[N]`;
- `f32`, `f64`, `float32`;
- `byte`.

Python guards can preserve range or shape intent for debug runs. Codon release
typing is still decided by Codon's compiler.

## Guard Semantics

Guard insertion covers supported function parameters, annotated assignments, and
return values.

Supported categories include:

- signed and unsigned fixed-width integer ranges;
- `int`, with `bool` rejected as an integer value;
- `float` and selected float aliases;
- `complex`, `bool`, ASCII `str`;
- `Optional`, `Union`, softened `Literal`, and `NoneType`;
- outer container shapes for `list`, `set`, `dict`, and `tuple`;
- deeper container element checks under `--assert full`.

Warning categories include unknown guard types, unchecked dynamic types such as
`pyobj`/`Any`/`object`, float precision risk, dict/set ordering risk, runtime
`Literal` behavior, and tuple ellipsis mode differences.

Guards should fail loudly for supported mismatch classes. They should not be
treated as proof that the Codon release path will behave identically.

## Lowering Policy

An automatic rewrite belongs in `codonx` only if it is:

- local;
- deterministic;
- explainable in public docs;
- testable with small fixtures;
- safe to fail closed without changing user semantics.

Examples of acceptable local rewrites:

- `from python import sys` to `import sys` in Python debug output;
- removing `@par` while preserving loop body in Python debug output;
- lowering `i32` annotation to Python `int` plus an `i32` guard;
- lowering `static.range(...)` to `range(...)` with a warning.

Examples that should not be automatic in the current system:

- simulating GPU kernels;
- resolving Codon overloads;
- translating arbitrary Python packages to native Codon;
- rewriting pointer, C interop, or LLVM behavior;
- whole-program type inference.

## Platform Policy

Only Linux is supported.

This keeps subprocess behavior, Codon compiler assumptions, release packaging,
and Python 3.12+ debug execution narrow and testable. Non-Linux behavior is not
intentionally supported.

## 0.2.x Direction

The intended direction is:

```text
Python source -> Ruff AST -> CodonX IR -> Codon target
Codon/CodonX source -> CodonX subset parser -> CodonX IR -> Python target
```

The central project-owned object should be CodonX IR, not Ruff AST and not
Codon native AST. Ruff is the Python frontend. Codon remains the release
compiler. CodonX IR is where macros, diagnostics, guard intent, fallback
planning, and future bidirectional projection should live.

Near-term work after 0.2.3 should focus on:

- source mapping and diagnostics for generated candidates;
- safe native lowering for a small, well-tested Python subset;
- explicit fallback islands instead of hidden performance cliffs;
- preserving the 0.1.x Codon-first workflow while the Ruff path matures.

Codon native AST/log integration remains out of scope for the current design.
