# codonx

> Status: 0.0.1 MVP / experimental. codonx is useful for early Codon-first
> workflows, but it is not a complete Codon parser, Python parser, or semantic
> equivalence tool.

Codon-first dual-target preprocessor. A `.codonx`/Codon-like source file can be projected to:

- a Python debug file with runtime assertions for type/range consistency;
- a pure Codon file that is immediately handed to the installed `codon` compiler for `run` or `build`.

codonx is intentionally a thin wrapper around Codon for release workflows. It preprocesses the input source, swaps the source path for the generated pure Codon file, and then invokes the system compiler.

## Requirements

- Rust toolchain for building codonx.
- `python3` for `codonx check` Python syntax compilation.
- The Codon compiler on `PATH` for `codonx run` and `codonx build`, unless `CODONX_CODON_BIN` or `--codon-bin` is used.

## Install From Source

```bash
cargo build --release --locked
```

The binary is written to:

```text
target/release/codonx
```

For a local release copy:

```bash
bash scripts/build_release.sh
```

## Python Debug Output

Generate a Python debug file with assertions enabled by default:

```bash
codonx --dbg test.codonx
```

Default output is written next to the input as:

```text
test_dbg.py
```

You can override the output path and assertion mode:

```bash
codonx --dbg test.codonx -o build/test.py
codonx --dbg test.codonx --assert off
codonx --dbg test.codonx --assert shallow
codonx --dbg test.codonx --assert full
codonx --dbg test.codonx --report build/codonx_report.json
```

`shallow` is the default. It checks scalar values and container shapes without recursively walking large containers. `full` recursively checks supported containers.

## Pure Codon Generation

Generate the preprocessed Codon file and exit:

```bash
codonx codon test.codonx
```

Default output is:

```text
test_pre.codon
```

Override it with:

```bash
codonx codon test.codonx -o build/test.codon
```

## Running With Codon

`codonx run` follows the shape of `codon run`. It preprocesses the source, invokes `codon run` on the generated pure Codon file, then deletes the generated file unless `--keep-pre` is set.

```bash
codonx run test.codon
```

Equivalent flow:

```bash
codonx codon test.codon -o test_pre.codon
codon run test_pre.codon
rm test_pre.codon
```

Codon arguments are passed through:

```bash
codonx run -debug test.codon
codonx run -release test.codon
codonx run -release -DN=16 test.codon arg1 arg2
```

Preserve the generated pure Codon file:

```bash
codonx --keep-pre run -release test.codon
```

## Building With Codon

`codonx build` follows the shape of `codon build`. Codon build options are passed through, including `-release`, `-debug`, `-o`, `-exe`, `-lib`, `-obj`, `-asm`, `-llvm`, and `-pyext`.

```bash
codonx build -release -o dist/app test.codon
codonx build -obj test.codon
codonx build -llvm test.codon
```

When `-o` is omitted, codonx supplies the output path that Codon would have derived from the original input file, so `codonx build -obj test.codon` writes `test.o` rather than `test_pre.o`.

## Codon Compiler Selection

By default codonx invokes `codon` directly through the shell environment path:

```bash
codon run ...
codon build ...
```

Use `CODONX_CODON_BIN` to choose another compiler:

```bash
CODONX_CODON_BIN=/opt/codon/bin/codon codonx run test.codon
```

Or pass it explicitly:

```bash
codonx --codon-bin /opt/codon/bin/codon run test.codon
```

## Check

Check directive structure, generate both targets, and run Python syntax compilation:

```bash
codonx check test.codonx
```

## Codon CLI Compatibility Notes

Codon documentation defines `codon run file` as compile-and-run in debug mode by default, with `-debug` and `-release` selecting debug or optimized builds. Program arguments appear after the source file:

```bash
codon run -release file.py arg1 arg2
```

Codon documentation defines `codon build` as compilation to executable, shared library, object file, assembly, LLVM IR, or Python extension. `-o <file>` controls the output path; if omitted, Codon derives the output path from the input file and selected output type.

codonx preserves those conventions by changing only the source path passed to Codon.

## Current Limits

- Single-file preprocessing is the supported 0.0.1 workflow.
- Python debug output is a debugging projection, not a guarantee of full equivalence with Codon release behavior.
- Parallel and GPU constructs are not simulated in Python; codonx may lower or report them, but release correctness still needs Codon-side testing.
- Unsupported Codon-only syntax should be isolated with `#%ifdebug`, `#%else`, and `#%endif`.
- Runtime guards are intentionally conservative and do not cover every Codon/Python semantic difference.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
