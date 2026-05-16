# codonx Design Notes

codonx is a Codon-first preprocessing tool.

It exists to support a practical workflow:

```text
one .codonx source
    -> Python debug target
    -> Codon release target
```

It is not a general transpiler.

## Design Philosophy

### Codon-first

The source file is assumed to be written for Codon first.

Codon carries information that Python does not naturally express, such as:

- fixed-width integer intent;
- Codon interop declarations;
- `@par` parallel loops;
- GPU annotations;
- release-only optimized branches.

The Python target is a debug projection, not the source of truth.

### Explicit Differences

Python/Codon differences should be explicit.

The main mechanism is:

```python
#%ifpy
# Python debug behavior
#%else
# Codon release behavior
#%endif
```

or:

```python
#%ifcodon
# Codon release behavior
#%else
# Python debug behavior
#%endif
```

The deprecated `#%ifdebug` alias is kept only for compatibility.

### Whitelist, Not Magic

codonx only performs small, documented, whitelist-based lowering.

Examples:

- `from python import math as m` -> `import math as m`
- `@par` -> a comment warning in Python output, with the loop kept serial
- `i32` annotation -> `int` annotation in Python output
- explicit runtime guard for `i32` range

Normal Codon code should prefer standard `int` and `float`. Fixed-width aliases
such as `i32` and `u64` are treated as explicit low-level intent, not as the
default numeric model.

Anything requiring real semantic analysis should be handled with explicit target branches.

## Why Not `if DEBUG:`?

Python parses the whole file before executing it. If the file contains Codon-only syntax, Python can fail before runtime branching matters.

Therefore codonx uses comment directives:

```python
#%ifpy
#%else
#%endif
```

These are preprocessing directives, not runtime branches.

## Python Target

The Python target is for:

- IDE debugging;
- `pytest`;
- `pdb`;
- quick correctness checks;
- runtime semantic guards.

The Python target is not for:

- performance;
- simulating OpenMP races;
- simulating GPU kernels;
- proving equivalence with Codon.

## Codon Target

The Codon target should stay close to the release intent.

codonx should avoid injecting debug-only logic into Codon output.

`codonx run` and `codonx build` are thin wrappers:

```text
.codonx
  -> temporary/preprocessed .codon
  -> codon run/build
```

## Semantic Guards

Python runtime guards catch obvious mismatches early.

Examples:

- `i32` out of range;
- `u64` negative value;
- `bool` accidentally accepted as `int`;
- ASCII string expectation;
- shallow or full container element checks.

Guards are mismatch detectors, not equivalence proofs.

They do not simulate:

- floating-point rounding differences;
- Codon overload resolution;
- dict/set order behavior;
- OpenMP races;
- GPU execution;
- Python interop conversion behavior.

## Directive Model

Current directives:

```text
#%ifpy
#%ifcodon
#%else
#%endif
#%define CODON_PYTHON <path>
#%define CODON_DEBUG <path>
```

`#%define` is intentionally not a macro system.

Supported names:

- `CODON_PYTHON`: injected into Codon subprocess environment.
- `CODON_DEBUG`: debug dump directory for Codon runs/builds.

Unknown names are errors.

## What Belongs in Automatic Lowering?

A new automatic lowering belongs in codonx only if it is:

- local;
- predictable;
- explainable in README;
- testable with small integration cases;
- safe without full semantic analysis.

If not, prefer explicit `#%ifpy` / `#%ifcodon`.

## Long-Term Direction

Near-term:

- stronger tests;
- cleaner CLI;
- better examples;
- better report output.

Mid-term:

- multi-file project mode;
- source mapping comments;
- stricter guard mode;
- warning categories.

Long-term:

- possible Codon self-hosting experiment;
- Rust version remains bootstrap/reference implementation.
