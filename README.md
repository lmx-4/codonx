# codonx

> Status: 0.0.2 MVP / experimental.

codonx is a Codon-first, single-file preprocessor. It deliberately does not try
to parse all Codon or prove Python/Codon equivalence. The 0.0.2 boundary is:

- C-style text selection with `#%ifdebug`, `#%else`, and `#%endif`.
- A small regex-level whitelist of syntax lowering for Python debug output.
- Basic Python runtime guards that catch obvious type/range mismatches early.
- Thin `codon run` / `codon build` wrapping after preprocessing.

Anything outside that boundary should be isolated by an explicit
`#%ifdebug` / `#%else` split.

## CLI

Generate a Python debug file. Assertions are enabled by default with
`--assert shallow`.

```bash
codonx --dbg test.codonx
codonx --dbg test.codonx -o build/test.py
codonx --dbg test.codonx --assert off
codonx --dbg test.codonx --assert full --report build/codonx_report.json
```

Generate a pure Codon file and exit.

```bash
codonx codon test.codonx
codonx codon test.codonx -o build/test.codon
```

Run or build with the installed Codon compiler. codonx preprocesses the input to
`*_pre.codon`, swaps the source path passed to Codon, then deletes the
preprocessed file unless `--keep-pre` is used.

```bash
codonx run -release test.codonx arg1
codonx build -release -o dist/app test.codonx
codonx --keep-pre run test.codonx
```

Compiler selection:

```bash
CODONX_CODON_BIN=/opt/codon/bin/codon codonx run test.codonx
codonx --codon-bin /opt/codon/bin/codon build test.codonx
```

Check directive structure and Python syntax:

```bash
codonx check test.codonx
```

## Preprocessor Semantics

`#%ifdebug` selects the Python debug branch. `#%else` selects the Codon branch.
Directives are removed from output and can be nested.

```python
#%ifdebug
for i in range(n):
    work(i)
#%else
@par
for i in range(n):
    work(i)
#%endif
```

Python debug output keeps the first loop. Codon output keeps the `@par` loop.
Directives inside triple-quoted strings are ignored.

## Python Syntax Lowering

0.0.2 only lowers a small whitelist using line-level regex/string rules:

- `@par` / `@par(...)`: replace the decorator with a `codonx:` comment and keep
  the following loop serial.
- `@gpu.kernel`: replace the decorator with a warning comment. GPU semantics are
  not simulated.
- `@python`: replace the decorator with a warning comment. Python debug output
  executes the function body directly.
- `from python import module` and `from python import module as alias`: rewrite
  to ordinary Python `import`.
- Codon scalar annotations `i8/u8/i16/u16/i32/u32/i64/u64` become `int`;
  `f32/f64` become `float`.

Typed Python interop declarations such as
`from python import mod.fn(int) -> int` are outside the regex-level boundary.
The Python target emits a comment and report warning; the source should provide
an explicit debug branch.

Unsupported syntax is not made equivalent by codonx. Use explicit branches for
`@tuple`, `@extend`, overloads, nontrivial generics, Codon-only `match`
extensions, OpenMP constructs, GPU code, pointer/C/LLVM interop, and anything
else that needs real parsing or semantic analysis.

## Python Runtime Guards

`--assert shallow` is the default. It inserts a Python prelude plus guards for
regex-detectable function parameters, annotated assignments, and return values.

Currently guarded:

- `int/i64/u64/i32/u32/i16/u16/i8/u8` with fixed-width bounds.
- `float/f32/f64`, `bool`, and ASCII `str`.
- `list[T]`, `set[T]`, `dict[K, V]`, and `tuple[...]` outer shapes.
- Container elements only with `--assert full`.

Unknown types soft-pass. Guards are mismatch detectors, not equivalence proofs.
They do not simulate parallel races, GPU behavior, floating-point rounding,
Codon overload resolution, dict/set order, or Python interop conversion.

## Development Contract

For 0.0.x, every new automatic lowering must be:

- regex/string-level safe;
- documented as syntax lowering, semantic guard, or explicit-maintenance only;
- covered by tests for Python output, Codon output, report warnings, or CLI
  passthrough as appropriate.

If a feature requires AST parsing or Codon semantic knowledge, keep it out of
automatic lowering and require `#%ifdebug` management instead.

## Build And Test

```bash
cargo fmt
cargo check --locked
cargo test --locked
```

## License

Licensed under either Apache-2.0 or MIT, at your option.
