# codonx

> **One Codon-dialect source file. Two practical targets: Python for debugging, Codon for release.**

`codonx` is a small Linux-first preprocessor for developers who want to write **Codon-first** code without losing the comfort of the Python development loop.

Codon is fast and powerful, but real projects often start in Python because Python is easy to debug. Later, when performance matters, the code is ported to Codon. That usually creates two files, two mental models, and eventually two versions that drift.

`codonx` takes the opposite approach:

```text
write a Codon-first .codon file
        │
        ├── generate Python debug code
        │       for python3.12+ / pytest / pdb / IDE breakpoints
        │
        └── run or build Codon release code
                through codon run / codon build
```

The idea is not to pretend that Python and Codon are identical.  
The idea is to make their differences **explicit, local, and testable**.

Current status: **0.1.2 local AST/span rewrite MVP / experimental**.

---

## What codonx is

`codonx` treats its input as a **Codon dialect**.

Recommended suffix:

```text
.codon
```

A codonx-compatible `.codon` file may contain:

- Codon-style annotations such as `i32`, `u64`, `f64`;
- Codon constructs such as `@par`;
- Codon Python interop such as `from python import ...`;
- codonx target directives such as `#%ifpy` and `#%ifcodon`;
- codonx release/debug hooks such as `#%define CODON_PYTHON` and `#%define CODON_DEBUG`.

`codonx` is **not** a general Codon-to-Python transpiler.  
It is a lightweight preprocessing layer for a deliberately Codon-first workflow.

The Python debug target is explicitly **Python 3.12+ only**. codonx does not
try to generate Python 3.11-compatible output, and it may preserve Python 3.12
syntax such as PEP 695 generic function and class type parameter lists.

## Stability contract for 0.1.x

The 0.1.x line stabilizes codonx as a conservative local AST/span rewrite MVP.

Guaranteed:

- Python debug output targets Python 3.12 and newer only.
- Codon output keeps release-oriented Codon code as close to the selected source branch as possible.
- `#%ifpy` / `#%ifcodon` are the supported way to express real semantic differences.
- Existing lowering and guard behavior should stay backward-compatible unless it is fixing an unsafe rewrite.
- Regex is no longer used for semantic rewrites; syntax-sensitive rewrites should flow through local AST/span or token-aware mechanisms.

Conservative rewrite policy:

- codonx should prefer a warning or no-op over a risky automatic rewrite.
- Local AST/span lowering is limited to documented, conservative syntax patterns.
- Future compatibility expansion should use a stronger parser/AST layer or explicit directives instead of broadening ad-hoc token rules.

Explicitly unsupported:

- full Codon-to-Python transpilation;
- overload resolution or generic monomorphization;
- LLVM, C pointer, GPU, parallel race, and Codon JIT semantic simulation;
- reliable arbitrary multi-line expression rewriting.

---

## Quick start

Install the Linux x86_64 release binary:

```bash
tar -xzf codonx-v0.1.2-x86_64-linux.tar.gz
install -m 0755 codonx-v0.1.2-x86_64-linux/codonx ~/.local/bin/codonx
codonx --version
```

Expected version:

```text
codonx 0.1.2
```

Create `hello.codon`:

```python
def add_i32(a: i32, b: i32) -> i32:
    c: i32 = a + b
    return c

print(add_i32(1, 2))
```

Generate a Python debug file:

```bash
codonx --dbg hello.codon -o hello.py
python3.12 hello.py
```

Expected output:

```text
3
```

Run or build with Codon:

```bash
codonx run -release hello.codon
codonx build -release -o dist/hello hello.codon
```

Current binary version:

```bash
codonx --version
```

```text
codonx 0.1.2
```

---

## The core trick: explicit target branches

Use `#%ifpy` when Python and Codon should use different code.

```python
def square_all(xs: list[i32]) -> list[i32]:
    out: list[i32] = [0 for _ in range(len(xs))]

    #%ifpy
    for i in range(len(xs)):
        out[i] = xs[i] * xs[i]
    #%else
    @par(schedule="dynamic", chunk_size=64)
    for i in range(len(xs)):
        out[i] = xs[i] * xs[i]
    #%endif

    return out

print(square_all([1, 2, 3]))
```

Python debug output keeps the serial branch.  
Codon release output keeps the `@par` branch.

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

`#%ifdebug` still works as a deprecated alias for `#%ifpy`, but new code should use `#%ifpy`.

---

## What generated Python looks like

The Python target is not just “Codon syntax stripped down.”  
When possible, codonx inserts runtime guards so Python can catch obvious Codon semantic mismatches early.

Input:

```python
def add_i32(a: i32, b: i32) -> i32:
    c: i32 = a + b
    return c
```

Generated Python contains code like:

```python
def add_i32(a: int, b: int) -> int:
    __codonx_assert_value(a, "i32", "a", full=False)
    __codonx_assert_value(b, "i32", "b", full=False)

    c: int = a + b
    __codonx_assert_value(c, "i32", "c", full=False)

    __codonx_ret = c
    __codonx_assert_value(__codonx_ret, "i32", "<return>", full=False)
    return __codonx_ret
```

So this fails in Python debug mode:

```python
print(add_i32(2 ** 40, 1))
```

because `2 ** 40` is outside `i32`.

This is the intended philosophy:

```text
Python debug target catches obvious mistakes early.
Codon release target keeps the performance-oriented source.
```

---

## Python debug mode

In 0.1.0, Python output is generated with the top-level `--dbg` option.

```bash
codonx --dbg input.codon -o output.py
```

If `-o` is omitted, codonx writes a sibling file like:

```text
input_dbg.py
```

Assertion modes:

```bash
codonx --dbg input.codon --assert off -o output.py
codonx --dbg input.codon --assert shallow -o output.py
codonx --dbg input.codon --assert full -o output.py
```

Default:

```text
--assert shallow
```

With a warning report:

```bash
codonx --dbg input.codon -o output.py --report report.json
```

Recommended debug loop:

```bash
codonx --dbg input.codon --assert full -o input_dbg.py --report codonx-report.json
python3.12 input_dbg.py
```

Important: there is **no `codonx py` subcommand in 0.1.0**.  
The current release uses `--dbg`.

---

## Codon mode

Generate a Codon file and stop:

```bash
codonx codon input.codon -o input_pre.codon
```

If `-o` is omitted, codonx writes:

```text
input_pre.codon
```

Run through Codon:

```bash
codonx run input.codon
codonx run -release input.codon
codonx run -release input.codon arg1 arg2
```

Build through Codon:

```bash
codonx build -release -o dist/app input.codon
```

`run` and `build` are thin wrappers. codonx preprocesses the input first, then invokes `codon run` or `codon build`.

Keep the preprocessed file:

```bash
codonx --keep-pre run -release input.codon
```

Use a specific Codon compiler:

```bash
codonx --codon-bin /opt/codon/bin/codon run input.codon
```

---

## Codon subprocess hooks: `#%define`

0.1.0 also supports small source-level hooks for Codon subprocess setup.

```python
#%define CODON_PYTHON /path/to/libpython3.12.so
#%define CODON_DEBUG target/codon_debug
```

These directives are removed from generated Python and Codon output.

### `CODON_PYTHON`

Injected into the `codon run` / `codon build` subprocess environment.

This is useful for Codon Python interop:

```python
#%define CODON_PYTHON /usr/lib/libpython3.12.so
from python import sys
```

You can keep the required Python runtime path close to the source file instead of permanently editing your shell profile.

### `CODON_DEBUG`

Injected into the Codon subprocess environment.

When the Codon invocation is in debug mode, codonx creates the target directory and appends `-log l` if the user did not already pass a log option.

This makes Codon dump files easier to collect without changing how the program sees its runtime working directory.

`#%define` is intentionally **not** a general macro system.  
Unknown define names are errors.

---

## What codonx lowers today

Python debug output uses a small whitelist of lowering rules.

| Codon-dialect input | Python debug behavior |
|---|---|
| `from python import math as m` | becomes `import math as m` |
| `@par` | removed; loop runs serially |
| `@gpu.kernel` | removed with warning comment |
| `@python` | removed with warning comment |
| `i8/u8/i16/u16/i32/u32/i64/u64` | annotation becomes `int` |
| `Int[N]/UInt[N]/byte` | annotation becomes `int`; guard keeps width/range intent |
| `f32/f64` | annotation becomes `float` |
| `float32` | annotation becomes `float`; report warns about precision limits |
| `List/Dict/Set/Tuple` | annotation container name becomes lowercase |
| `i32(x)`, `u64(x)`, `Int[N](x)`, `UInt[N](x)` | simple casts become checked Python debug casts |
| `f32(x)`, `float32(x)`, `f64(x)` | simple casts become `float(x)` |
| `def f[T](...)`, `class C[T]:` | preserved as Python 3.12+ generic syntax |
| `T: type` function parameters | removed from Python debug signatures |
| `@export`, `@tuple`, `@extend` | removed with report warnings |
| `@overload` | removed with report warning |
| `@codon.jit`, `@codon.convert` | removed with interop warning |
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

Anything beyond this should use explicit target branches.

0.0.7 is intended as the final regex/string-level expansion release. Future
larger compatibility work should use a parser/AST layer or explicit directives
rather than adding increasingly broad regex rewrites.

### Real boundaries in 0.1.0

The Python target is intentionally conservative:

- Rewrites are line-oriented. General multi-line signatures, nested calls, and multi-line expressions are not mechanically rewritten.
- Plain comments and quoted strings are not rewritten.
- Simple annotations are rewritten only in function signatures, simple annotated assignments, and simple annotation declarations.
- Integer casts are checked only when `--assert` is enabled; with `--assert off`, simple integer casts lower to plain `int(...)`.
- `@extend` blocks are omitted, not translated, because preserving them can shadow Python builtins such as `int` or `str`.
- Codon release builds still use the real Codon type system. A Python debug run can pass while `codon build -release` fails on stricter Codon type constraints.

---

## What the guards cover

Currently guarded:

- `int`, `i64`, `u64`, `i32`, `u32`, `i16`, `u16`, `i8`, `u8`;
- `Int[N]`, `UInt[N]`, and `byte`;
- `float`, `f32`, `f64`, and `float32`;
- `complex`;
- `bool`;
- ASCII `str`;
- `Optional[T]`, `Union[...]`, `NoneType`, and softened `Literal[...]`;
- outer shapes for `list[T]`, `set[T]`, `dict[K, V]`, and `tuple[...]`;
- container elements with `--assert full`.

With `--report`, guard analysis also warns about unknown guard types, unchecked
dynamic types such as `pyobj`/`Any`/`object`, `f32` precision differences,
dict/set ordering risk, and `tuple[T, ...]` soft-check limitations.

Guards are mismatch detectors, not equivalence proofs.

They do **not** simulate:

- parallel races;
- GPU execution;
- Codon overload resolution;
- floating-point rounding differences;
- dict/set ordering differences;
- Python interop conversion behavior.

---

## Check

```bash
codonx check input.codon
codonx check --assert full input.codon
```

`check` validates directive structure and generated Python syntax.  
It is not a full Codon type checker.

---

## What codonx is not

codonx is not:

- a full Codon parser;
- a full Python parser;
- a general transpiler;
- a semantic equivalence prover;
- a GPU simulator;
- a race detector;
- a replacement for the Codon compiler;
- a replacement for Python debugging tools.

If a construct needs real semantic understanding, write explicit branches:

```python
#%ifpy
# simple Python debug version
#%else
# optimized Codon release version
#%endif
```

---

## Who this is for

codonx is useful if you:

- want to write Codon-first source code;
- still want generated Python files for debugging;
- understand that Python and Codon are not identical;
- are willing to make target-specific differences explicit;
- test release behavior under Codon.

It is not useful if you expect arbitrary Python or arbitrary Codon to become magically equivalent across targets.

---

## Build and test

```bash
cargo fmt
cargo check --locked
cargo test --locked
```

Full local check:

```bash
cargo fmt --all -- --check
cargo check --locked
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

---

## License

Licensed under either:

- Apache License, Version 2.0
- MIT License

at your option.
