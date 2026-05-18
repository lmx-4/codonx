# codonx Design Notes

These notes describe the design that exists in `codonx` 0.1.4. They are not a
promise of language completeness.

`codonx` is a Linux-only, Codon-first preprocessing tool:

```text
selected codonx source
    -> Python 3.12+ debug projection
    -> Codon release projection
    -> real codon compiler for run/build
```

The project is intentionally built around the official Codon compiler. Python is
the debug projection; Codon is the release source of truth.

## Core Philosophy

### Codon First

The input is assumed to be written for Codon first. It may contain Codon types,
parallel annotations, Python interop declarations, and release-only optimized
branches.

The Python target exists to make development faster. It should catch obvious
mismatches early, but it should not pretend to simulate every Codon semantic.

### Explicit Differences

When Python and Codon need different code, the difference should be visible in
the source:

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
branches can help. A runtime `if DEBUG:` cannot protect Python from Codon-only
syntax that appears elsewhere in the file.

### Conservative Automatic Lowering

Automatic lowering must be local, predictable, explainable, and testable.

Good automatic lowering examples:

- `from python import math as m` to `import math as m`.
- `@par` removal in Python debug output while preserving the loop body.
- `i32` annotation to Python `int` plus an `i32` range guard.
- `static.range(...)` to runtime `range(...)` with a warning.

Bad automatic lowering examples:

- simulating GPU kernel behavior in Python;
- resolving Codon overloads;
- rewriting arbitrary pointer or LLVM interop;
- translating whole Python libraries to Codon.

If a transformation needs global semantic knowledge, the correct 0.1.x answer is
usually an explicit `#%ifpy` / `#%ifcodon` branch.

## Runtime Model

### Python Debug Target

The Python target is for:

- Python 3.12+ execution;
- IDE debugging;
- `pdb`;
- `pytest`;
- runtime semantic guards;
- quick checks before compiling with Codon.

The Python target is not for:

- performance measurement;
- proving equivalence with Codon;
- simulating OpenMP races;
- simulating GPU kernels;
- simulating Codon Python interop conversion behavior.

### Codon Release Target

The Codon target should stay close to the selected release branch. `codonx`
should avoid injecting debug-only logic into Codon output.

`codonx run` and `codonx build` are thin wrappers:

```text
input.codon
    -> selected/preprocessed temporary .codon
    -> codon run/build
```

The official Codon compiler remains responsible for type checking, optimization,
execution, and build output.

## Implementation Shape in 0.1.4

The 0.1.4 implementation is a local AST/span rewrite MVP.

```text
raw source
    -> directive selection
    -> source lines and stable spans
    -> local AST/span or token-aware candidate handling
    -> non-overlapping patch application
    -> Python debug or Codon release output
```

Important properties:

- Rewrites are conservative and whitelist-based.
- Plain comments and quoted strings are protected from semantic rewrites.
- Syntax-sensitive transformations use local AST/span or token-aware logic.
- Regex is not the final authority for semantic rewrites.
- Unknown or ambiguous syntax should be preserved or reported, not guessed.

The local AST is not a full language AST. It is a rewrite IR for the constructs
that `codonx` can safely lower today.

## Numeric Semantics

0.1.4 follows Codon's Python-like mainline numeric style:

- `int` is the normal integer type for most user code.
- `float` is the normal floating-point type for most user code.
- `list[int]`, `dict[str, float]`, and similar annotations are the preferred
  high-level shape.

Fixed-width and low-level types remain meaningful:

- `i8`, `i16`, `i32`, `i64`;
- `u8`, `u16`, `u32`, `u64`;
- `Int[N]`, `UInt[N]`;
- `f32`, `f64`, `float32`.

Those types represent explicit low-level intent. Python debug guards may enforce
range or shape expectations, but Codon release behavior is still decided by
Codon's type system.

## Semantic Guards

Python runtime guards catch obvious mismatches early.

Examples:

- `i32` out of range;
- `u64` negative value;
- `bool` accidentally accepted as `int`;
- non-ASCII `str` where ASCII `str` is expected;
- shallow or full container element mismatches.

Guards are inserted for supported parameter types, annotated assignments, and
return values. They are mismatch detectors, not formal equivalence proofs.

They do not simulate:

- floating-point rounding differences;
- Codon overload resolution;
- dict/set order behavior;
- parallel races;
- GPU execution;
- Python interop conversion behavior.

## Directive Model

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

`#%ifdebug` is an alias for `#%ifpy` and should not be used in new code.

Supported define directives:

```text
#%define CODON_PYTHON <path>
#%define CODON_DEBUG <path>
```

`CODON_PYTHON` is passed to `codon run` and `codon build` for Python interop.
`CODON_DEBUG` configures Codon debug dump handling. Unknown define names are
errors because `#%define` is not a general macro system.

## Lowering Policy

A new automatic lowering belongs in `codonx` only if it meets all of these
conditions:

- It is local.
- It is deterministic.
- It can be explained in the README.
- It can be tested with small integration cases.
- It can fail closed without changing user semantics.

If a construct needs whole-program knowledge, overload resolution, pointer
analysis, control-flow analysis, or compiler-internal Codon AST access, it does
not belong in automatic 0.1.x lowering.

## Platform Policy

0.1.4 supports Linux only. This keeps the subprocess model, release packaging,
Codon compiler expectation, and Python 3.12+ debug path simple.

Non-Linux behavior is not intentionally supported in this series.

## Long-Term Direction

Near-term work should focus on:

- widening local AST coverage only where a concrete rewrite needs it;
- strengthening diagnostics for unsupported but recognized syntax;
- improving release/debug smoke tests;
- preserving the 0.1.x Codon-first workflow while preparing the 0.2.x frontend
  transition;
- keeping documentation aligned with actual behavior.

The next architecture step is documented in
[`roadmap-0.2.x.md`](roadmap-0.2.x.md). The short version is:

```text
Python 3.12 source
    -> Ruff parser AST and token stream
    -> CodonX IR
    -> macro/hint binding
    -> convertibility analysis
    -> guarded Codon candidate output or fallback diagnostics
```

0.2.x should use Ruff as the Python parser frontend, not fork it first. `#%`
macros should remain comments and be attached to AST nodes by token/source-range
metadata. The project-owned boundary is CodonX IR: that is where macro hints,
guard intent, fallback decisions, diagnostics, and future bidirectional
projection should live.

Import planning is compatibility-first. A Python import is assumed to be a
CPython fallback import unless the user places `#%codon` immediately before it.
That keeps third-party Python packages usable by default while making Codon
native standard-library intent explicit and diagnosable.

The first `py-codon` generator is intentionally compile-first. It uses Ruff to
locate imports, strips codonx control comments, keeps native-marked imports as
Codon imports, rewrites default imports to Codon's Python interop form, and
preserves the remaining Python/Codon common subset. `#%define CODON_PYTHON`
cannot be embedded as executable Codon code, so the generator surfaces it in the
candidate header as the environment value the caller should inject into the
Codon compile/run process. The `py-run` and `py-build` wrappers perform that
injection automatically while still delegating release semantics to the real
Codon compiler.

The 0.2.x line should not promise full Python-to-Codon conversion. Its job is to
make safe conversion, guarded conversion, fallback, and unsupported regions
explicit and testable.

Mid-term work may explore:

- multi-file project workflows;
- better source mapping;
- stricter guard categories;
- clearer warning taxonomies.

Codon native AST/log integration remains outside the 0.1.x implementation plan.
