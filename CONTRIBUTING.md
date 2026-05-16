# Contributing to codonx

`codonx` is intentionally small and sharp. It is a Linux-only, Codon-first
preprocessing tool, not a general Codon-to-Python transpiler and not a
Python-to-Codon migration system.

Contributions are welcome when they preserve that boundary.

## Current Project Boundary

codonx 0.1.3 supports:

- target selection with `#%ifpy`, `#%ifcodon`, `#%else`, and `#%endif`;
- deprecated compatibility alias `#%ifdebug` for `#%ifpy`;
- Python 3.12+ debug output through the top-level `--dbg` option;
- local AST/span or token-aware Python debug lowering for documented constructs;
- Python runtime guards for obvious type, shape, and range mismatches;
- Codon release preprocessing through `codonx codon`;
- thin `codon run` / `codon build` wrapping after preprocessing;
- source-level `#%define CODON_PYTHON` and `#%define CODON_DEBUG` for Codon
  subprocess environment setup.

codonx 0.1.3 requires:

- Linux;
- Python 3.12+ for generated debug files;
- the official Codon compiler for release-path execution and validation.

codonx 0.1.3 does not aim to:

- parse all Codon;
- parse all Python;
- support non-Linux platforms;
- prove Python/Codon semantic equivalence;
- simulate parallel races;
- simulate GPU execution;
- lower arbitrary pointer/C/LLVM interop;
- replace the official Codon compiler, debugger, or IDE tooling.

## Contribution Rule

Every automatic lowering must fit one of these categories:

1. **Local syntax lowering**

   A small, AST/span or token-aware transformation that is safe, local, and easy
   to explain in the README.

2. **Semantic guard**

   A Python debug-time assertion that catches an obvious mismatch early without
   claiming full equivalence.

3. **Explicit-maintenance only**

   A construct that should be handled with `#%ifpy` / `#%ifcodon`, plus a
   warning or documentation note.

If a feature requires full AST parsing, Codon type checking, overload
resolution, control-flow analysis, race analysis, pointer analysis, or GPU
semantics, it should not be added as automatic lowering in 0.1.x.

## Numeric Style

New examples and tests should prefer Codon's normal high-level numeric style:

```python
int
float
list[int]
dict[str, float]
```

Use `i32`, `u64`, `f32`, `Int[N]`, and `UInt[N]` only when the fixed width is
part of the behavior being tested or documented.

## Local Development

```bash
cargo fmt --all -- --check
cargo check --locked
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

You can also run:

```bash
bash scripts/check.sh
```

Release-path smoke tests should be run on a machine with the Codon compiler
installed.

## Testing Requirements

A change to preprocessing behavior should include tests for at least one of:

- Python debug output;
- Codon output;
- warning report behavior;
- directive error behavior;
- CLI passthrough behavior;
- optional Codon compiler smoke behavior.

For tricky behavior, prefer integration tests in `tests/`.

## Documentation Requirements

If a feature changes user-visible behavior, update the relevant docs:

- `README.md` for public behavior;
- `docs/design.md` for implementation policy;
- `docs/roadmap-0.1.x.md` for planned 0.1.x direction;
- `CHANGELOG.md` for release-visible changes;
- `examples/` when behavior is easiest to explain by example.

## Good and Bad PR Shapes

Good PR examples:

- "Add warning for unsupported typed Python interop"
- "Harden multiline `static.range` lowering"
- "Document CODON_DEBUG run/build behavior"
- "Add Codon smoke fixture for int-mainline code"

Bad PR examples:

- "Implement full Codon parser"
- "Automatically translate all GPU kernels to Python"
- "Make codonx understand arbitrary Python objects"
- "Add non-Linux support without Codon subprocess validation"

## Release Notes

For each user-visible change, update `CHANGELOG.md`.
