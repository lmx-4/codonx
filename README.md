# codonx

> **Write one Codon-first source file. Debug it as Python. Run it as Codon.**

`codonx` is a Linux-only preprocessing CLI for a very specific workflow:

```text
one Codon-first source file
        |
        +-- Python 3.12+ debug target
        |       for pdb, pytest, IDE breakpoints, and runtime guards
        |
        +-- Codon release target
                through the real Codon compiler
```

The project starts from a practical pain point. Codon can make Python-like code
fast, but development often splits into two files: a Python file that is easy to
debug, and a Codon file that is fast enough to ship. Those files drift. `codonx`
tries to keep one source of truth by making the Python/Codon split explicit,
local, and testable.

Current status: **0.1.4 local AST/span rewrite MVP / experimental**.

The next planned line is **0.2.x Ruff frontend + CodonX IR**. 0.2.x is intended
to parse Python 3.12 through Ruff, bind `#%` comment macros to AST nodes, and
classify code as `codon_native`, `guarded`, `fallback`, or `unsupported` before
any Codon generation. See [docs/roadmap-0.2.x.md](docs/roadmap-0.2.x.md).

## Hard Requirements

`codonx` 0.1.4 is intentionally narrow.

- **Operating system:** Linux only.
- **Python:** Python 3.12 or newer must be installed for debug output.
- **Codon:** the official `codon` compiler must be installed for `codonx run`,
  `codonx build`, and real release validation.
- **Rust:** required only when building `codonx` from source.

`codonx` is not useful without Codon. The tool is a thin, Codon-first layer over
the real compiler, not a replacement for it.

## What codonx Is

`codonx` treats the input as a Codon-first dialect with optional target
directives.

Recommended source suffix for new projects:

```text
.codon
```

The existing examples still use `.codonx` to make the dialect visible, but the
CLI accepts normal file paths and does not depend on the extension.

A codonx-compatible file may contain:

- Codon-style types such as `int`, `float`, `list[int]`, and `dict[str, int]`.
- Explicit low-level Codon types such as `i32`, `u64`, `f32`, and `Int[32]`.
- Codon-only constructs such as `@par`, `@llvm`, `@extend`, and Python interop.
- codonx directives such as `#%ifpy`, `#%ifcodon`, `#%else`, and `#%endif`.
- Codon subprocess hooks such as `#%define CODON_PYTHON` and
  `#%define CODON_DEBUG`.

`codonx` is not a general Python-to-Codon converter. It is also not a complete
Codon-to-Python transpiler. It is a conservative preprocessing layer for people
who already intend to run the release path under Codon.

The planned 0.2.x work starts the foundation for that larger direction, but it
does so by adding a CodonX IR and explicit fallback diagnostics first, not by
claiming full automatic conversion.

## Install

Install the Linux x86_64 release binary:

```bash
tar -xzf codonx-v0.1.4-x86_64-linux.tar.gz
install -m 0755 codonx-v0.1.4-x86_64-linux/codonx ~/.local/bin/codonx
codonx --version
```

Expected output:

```text
codonx 0.1.4
```

Check the external toolchain:

```bash
python3.12 --version
codon --version
```

If Codon is not on `PATH`, pass it explicitly:

```bash
codonx --codon-bin /opt/codon/bin/codon run -release app.codon
```

You can also set:

```bash
export CODONX_CODON_BIN=/opt/codon/bin/codon
```

## Quick Start

Create `hello.codon`:

```python
def add(a: int, b: int) -> int:
    c: int = a + b
    return c

print(add(1, 2))
```

Generate and run the Python debug target:

```bash
codonx --dbg hello.codon -o hello.py
python3.12 hello.py
```

Run or build through Codon:

```bash
codonx run -release hello.codon
codonx build -release -o dist/hello hello.codon
```

Generate only the preprocessed Codon file:

```bash
codonx codon hello.codon -o hello_pre.codon
```

Check directive structure and generated Python syntax:

```bash
codonx check hello.codon
codonx check --assert full hello.codon
```

`check` is not a Codon type checker. The Codon compiler remains the source of
truth for release behavior.

Experimental 0.2.x frontend work is available behind explicit commands:

```bash
codonx ir app.py -o app_ir.json
codonx assert-ir app.py -o app_assert_ir.py
python3.12 app_assert_ir.py
```

`ir` parses Python 3.12 source through Ruff and emits a debug JSON dump of the
current CodonX AST view. `assert-ir` emits executable Python semantic IR: it
keeps the program shape and inserts Codon-facing runtime guards around supported
annotations and returns. These commands do not yet generate Codon.

For Python imports, 0.2.x is compatibility-first: imports are treated as CPython
fallback candidates by default. Add `#%codon` immediately before an import to
require Codon native import semantics:

```python
#%codon
import math

import numpy as np  # planned as CPython fallback
```

## The Central Idea: Explicit Target Branches

Python and Codon are close enough to share a lot of code, but not close enough
to pretend every construct means the same thing. `codonx` uses comment
directives because Python parses the whole file before runtime `if` statements
can help.

```python
def square_all(xs: list[int]) -> list[int]:
    out: list[int] = [0 for _ in range(len(xs))]

    #%ifpy
    for i in range(len(xs)):
        out[i] = xs[i] * xs[i]
    #%else
    @par(schedule="dynamic", chunk_size=64)
    for i in range(len(xs)):
        out[i] = xs[i] * xs[i]
    #%endif

    return out
```

The Python debug target keeps the serial loop. The Codon release target keeps
the `@par` loop.

You can also put the Codon branch first:

```python
#%ifcodon
@par
for i in range(n):
    work(i)
#%else
for i in range(n):
    work(i)
#%endif
```

`#%ifdebug` is still accepted as a deprecated alias for `#%ifpy`, but new code
should use `#%ifpy`.

## Numeric Semantics in 0.1.4

Codon's normal numeric style is intentionally close to Python. In 0.1.4, the
recommended mainline types are:

```python
int
float
list[int]
dict[str, float]
```

Fixed-width types are still supported, but they mean explicit low-level intent:

```python
i32
u64
f32
Int[32]
UInt[64]
```

Use fixed-width types only when the width is part of the program contract. The
Python debug target can guard those ranges, but the Codon release target still
uses the real Codon type system. If Codon rejects mixed `int` and `i32`
arithmetic, that is a release-code issue that should be fixed in the source.

## What Generated Python Looks Like

Python debug output is not just "Codon syntax with a few tokens removed." When
assertions are enabled, `codonx` inserts runtime guards for supported
annotations.

Input:

```python
def add(a: int, b: int) -> int:
    c: int = a + b
    return c
```

Debug output contains guard logic like:

```python
def add(a: int, b: int) -> int:
    _codonx_assert_value(a, "int", "a", full=False)
    _codonx_assert_value(b, "int", "b", full=False)

    c: int = a + b
    _codonx_assert_value(c, "int", "c", full=False)

    _codonx_ret = c
    _codonx_assert_value(_codonx_ret, "int", "<return>", full=False)
    return _codonx_ret
```

Assertion modes:

```bash
codonx --dbg input.codon --assert off -o input_dbg.py
codonx --dbg input.codon --assert shallow -o input_dbg.py
codonx --dbg input.codon --assert full -o input_dbg.py
```

Default:

```text
--assert shallow
```

Recommended debug loop:

```bash
codonx --dbg input.codon --assert full -o input_dbg.py --report codonx-report.json
python3.12 input_dbg.py
```

There is no `codonx py` subcommand in 0.1.4. Python debug generation uses the
top-level `--dbg` option.

## Codon Mode

`run` and `build` are thin wrappers. `codonx` selects directives, writes a
temporary/preprocessed Codon file, then invokes the real Codon compiler.

```bash
codonx run input.codon
codonx run -release input.codon
codonx run -release input.codon arg1 arg2
codonx build -release -o dist/app input.codon
```

Keep the generated Codon file:

```bash
codonx --keep-pre run -release input.codon
```

## Codon Subprocess Hooks

`#%define` is intentionally not a macro system. Only the documented names are
accepted.

```python
#%define CODON_PYTHON /path/to/libpython3.12.so
#%define CODON_DEBUG target/codon_debug
```

`CODON_PYTHON` is injected into `codon run` and `codon build`. It is useful for
Codon Python interop:

```python
#%define CODON_PYTHON /usr/lib/libpython3.12.so
from python import sys
```

`CODON_DEBUG` is injected into the Codon subprocess environment. In debug Codon
invocations, `codonx` creates the target directory and appends `-log l` if the
user did not already provide a log option.

These directives are removed from generated Python and Codon output.

## What codonx Lowers Today

Python debug output uses local AST/span parsing plus conservative token-aware
lowering. Regex is not the final authority for semantic rewrites in 0.1.4.

| Codon-dialect input | Python debug behavior |
|---|---|
| `from python import math as m` | becomes `import math as m` |
| `@par` | removed; loop body runs serially |
| `@gpu.kernel` | removed with warning comment |
| `@python` | removed with warning comment |
| `i8/u8/i16/u16/i32/u32/i64/u64` | annotation becomes `int`; guard keeps range intent |
| `Int[N]/UInt[N]/byte` | annotation becomes `int`; guard keeps width/range intent |
| `f32/f64/float32` | annotation becomes `float`; report may warn about precision |
| `List/Dict/Set/Tuple` | container name becomes lowercase in Python annotations |
| `i32(x)`, `u64(x)`, `Int[N](x)`, `UInt[N](x)` | supported scalar casts become checked Python casts when assertions are enabled |
| `f32(x)`, `float32(x)`, `f64(x)` | supported scalar casts become `float(x)` |
| `def f[T](...)`, `class C[T]:` | preserved as Python 3.12+ generic syntax |
| `T: type` function parameters | removed from Python debug signatures |
| `@export`, `@tuple`, `@overload` | removed with report warnings |
| `@codon.jit`, `@codon.convert` | removed with interop warnings |
| `@extend` class blocks | omitted with warning to avoid shadowing Python types |
| `@llvm` functions | omitted with warning; use explicit target branches |
| `static.range(...)` | lowered to runtime `range(...)` with warning |
| `class Child(Static[Base]):` | lowered to `class Child(Base):` with warning |
| `float16`, `bfloat16`, `float128` | annotation/cast becomes Python `float` with precision warning |

Warning-only boundaries:

| Codon-dialect input | Python debug behavior |
|---|---|
| typed `from python import ...(...) -> ...` | warning/comment; use explicit branch for wrappers |
| `from C import ...`, `import C` | warning/comment; C interop is not simulated |
| `Ptr[...]`, `cobj`, `__ptr__` | warning; pointer semantics are not simulated |
| `ndarray[dtype, ndim]` | warning; dtype/ndim semantics are not simulated |

Anything outside this whitelist should use explicit `#%ifpy` / `#%ifcodon`
branches.

## Guard Coverage

Currently guarded:

- `int`, `i64`, `u64`, `i32`, `u32`, `i16`, `u16`, `i8`, `u8`.
- `Int[N]`, `UInt[N]`, and `byte`.
- `float`, `f32`, `f64`, and `float32`.
- `complex`, `bool`, ASCII `str`.
- `Optional[T]`, `Union[...]`, `NoneType`, and softened `Literal[...]`.
- Outer shapes for `list[T]`, `set[T]`, `dict[K, V]`, and `tuple[...]`.
- Container elements with `--assert full`.

With `--report`, guard analysis also warns about unknown guard types, unchecked
dynamic types such as `pyobj`/`Any`/`object`, `f32` precision differences,
dict/set ordering risk, runtime `Literal[...]` checks, and `tuple[T, ...]`
mode differences.

Guards are mismatch detectors, not equivalence proofs. They do not simulate
parallel races, GPU execution, Codon overload resolution, floating-point
rounding differences, dict/set ordering differences, or Python interop
conversion behavior.

## Stability Contract for 0.1.x

Guaranteed for the 0.1.x line:

- Python debug output targets Python 3.12 and newer only.
- Linux is the only supported platform.
- Codon output stays close to the selected Codon source branch.
- `#%ifpy` / `#%ifcodon` are the supported way to express real target
  differences.
- Existing lowering and guard behavior should stay backward-compatible unless
  tightening is required to avoid unsafe rewrites.
- Uncertain syntax should be preserved or warned about rather than guessed.

Explicitly unsupported:

- Full Codon parsing.
- Full Python parsing.
- Full Python-to-Codon migration.
- Full Codon-to-Python transpilation.
- Overload resolution or generic monomorphization.
- LLVM, C pointer, GPU, parallel race, and JIT semantic simulation.
- Reliable arbitrary multi-line expression rewriting beyond documented
  conservative cases.

## Who This Is For

Use `codonx` if you:

- want to write Codon-first source code;
- need Python 3.12+ debug files for normal development tools;
- are willing to mark true Python/Codon differences explicitly;
- test release behavior with the real Codon compiler.

Do not use `codonx` if you expect arbitrary Python or arbitrary Codon to become
magically equivalent across targets.

## Build and Test From Source

```bash
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo build --release --locked
```

Full local release checks may also run Codon smoke tests when a Codon compiler
is available on the machine.

## License

Licensed under either:

- Apache License, Version 2.0
- MIT License

at your option.
