# Contributing to codonx

Thank you for your interest in codonx.

codonx is intentionally small and sharp. It is a **Codon-first preprocessing tool**, not a general Codon-to-Python transpiler and not a Python-to-Codon migration system. Contributions are welcome, but they must preserve this boundary.

## Project Boundary

codonx 0.0.x supports:

- target selection with `#%ifpy`, `#%ifcodon`, `#%else`, and `#%endif`;
- deprecated compatibility alias `#%ifdebug` for `#%ifpy`;
- small whitelist-based Python syntax lowering;
- Python runtime guards for obvious type/range mismatches;
- thin `codon run` / `codon build` wrapping after preprocessing;
- source-level `#%define CODON_PYTHON` and `#%define CODON_DEBUG` for Codon subprocess environment setup.

codonx 0.0.x does **not** aim to:

- parse all Codon;
- parse all Python;
- prove Python/Codon semantic equivalence;
- simulate OpenMP races;
- simulate GPU execution;
- lower pointer/C/LLVM interop;
- replace the official Codon compiler, debugger, or IDE tooling.

## Contribution Rule

Every automatic lowering must be one of these three kinds:

1. **Syntax lowering**  
   A small, local, regex/string-level transformation that is safe and easy to explain.

2. **Semantic guard**  
   A Python debug-time assertion that catches an obvious mismatch early.

3. **Explicit-maintenance only**  
   A construct that must be managed with `#%ifpy` / `#%ifcodon`, with a warning or documentation note.

If a feature requires AST parsing, Codon type checking, overload resolution, control-flow analysis, race analysis, or GPU semantics, it should not be added as automatic lowering in 0.0.x.

## Local Development

```bash
cargo fmt
cargo check --locked
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

You can also run:

```bash
bash scripts/check.sh
```

if the script exists in your checkout.

## Testing Requirements

A PR that changes preprocessing behavior should include tests for at least one of:

- Python debug output;
- Codon output;
- warning report behavior;
- directive error behavior;
- CLI passthrough behavior.

For tricky behavior, prefer integration tests in `tests/`.

## Documentation Requirements

If a feature changes user-visible behavior, update at least one of:

- `README.md`
- `CHANGELOG.md`
- `docs/design.md`
- an example in `examples/`

## Commit and PR Style

Small, focused PRs are preferred.

Good PR examples:

- "Add warning for unsupported typed Python interop"
- "Support nested #%ifcodon inside inactive parent branches"
- "Document CODON_DEBUG run/build behavior"

Bad PR examples:

- "Implement full Codon parser"
- "Automatically translate all GPU kernels to Python"
- "Make codonx understand arbitrary Python objects"

## Release Notes

For each user-visible change, update `CHANGELOG.md`.
