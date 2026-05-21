# codonx

> One source of truth between Python's debugging comfort and Codon's release path.

`codonx` exists because the Python-to-Codon story usually starts well and then
splits in two.

You keep a Python file because it is easy to debug with `pdb`, pytest, IDE
breakpoints, print probes, and the rest of the Python ecosystem. You keep a
Codon file because the release path needs Codon's compiler. After a few edits,
the two files stop being the same program.

`codonx` is a small Linux CLI that tries to keep that split explicit,
mechanical, and testable.

```text
Codon-first source
    -> Python 3.12+ debug file with runtime guards
    -> Codon release file passed to the real codon compiler

Python 3.12 source, experimental 0.2.x path
    -> Ruff parser frontend
    -> CodonX debug/semantic IR
    -> conservative Codon candidate or CPython fallback imports
```

Current status: **0.2.3 experimental**.

The stable practical core is still the Codon-first workflow from 0.1.x. The new
0.2.x line adds a Ruff-backed Python frontend and the first conservative
`py-codon` bridge. It is not yet a general Python-to-Codon transpiler.

## What It Feels Like

Write code that is meant to run under Codon:

```python
def square_sum(xs: list[int]) -> int:
    total: int = 0
    for x in xs:
        total += x * x
    return total

print(square_sum([1, 2, 3]))
```

Debug it as Python:

```bash
codonx --dbg app.codon -o app_dbg.py
python3.12 app_dbg.py
```

Run it through Codon:

```bash
codonx run -release app.codon
```

When the two targets need different code, say so in the source:

```python
def fill(out: list[int], n: int):
    #%ifpy
    for i in range(n):
        out[i] = i * i
    #%else
    @par(schedule="dynamic", chunk_size=64)
    for i in range(n):
        out[i] = i * i
    #%endif
```

The Python debug file keeps the serial loop. The Codon release file keeps the
`@par` loop. There is no hidden semantic guess.

## Requirements

- Linux only.
- Python 3.12 or newer.
- The official `codon` compiler for `codonx run`, `codonx build`, `py-run`, and
  `py-build`.
- Rust only when building from source.

`codonx` is not a replacement for Codon. It preprocesses, checks, and generates
files around the real compiler.

## Install

Install the Linux x86_64 release binary:

```bash
tar -xzf codonx-v0.2.3-x86_64-linux.tar.gz
install -m 0755 codonx-v0.2.3-x86_64-linux/codonx ~/.local/bin/codonx
codonx --version
```

Expected output:

```text
codonx 0.2.3
```

Check the external tools:

```bash
python3.12 --version
codon --version
```

If Codon is not on `PATH`:

```bash
codonx --codon-bin /opt/codon/bin/codon run -release app.codon
```

or:

```bash
export CODONX_CODON_BIN=/opt/codon/bin/codon
```

## Main Workflow: Codon First

Generate Python debug output:

```bash
codonx --dbg input.codon -o input_dbg.py
python3.12 input_dbg.py
```

Generate Python debug output with stronger runtime guards:

```bash
codonx --dbg input.codon --assert full -o input_dbg.py --report codonx-report.json
```

Generate a preprocessed Codon file:

```bash
codonx codon input.codon -o input_pre.codon
```

Run or build through the real Codon compiler:

```bash
codonx run -release input.codon
codonx build -release -o dist/app input.codon
```

Check directive structure and generated Python syntax:

```bash
codonx check input.codon
codonx check --assert full input.codon
```

`check` is not a Codon type checker. Release behavior still belongs to `codon`.

## Experimental Workflow: Python Frontend

0.2.x starts the other direction: Python source enters through Ruff's Python
3.12 parser and becomes CodonX frontend data.

```bash
codonx ir app.py -o app_ir.json
codonx assert-ir app.py -o app_assert_ir.py
codonx py-codon app.py -o app.codon
codonx py-run app.py
codonx py-build app.py
```

Current 0.2.3 behavior is deliberately conservative:

- `ir` emits a JSON debug dump of the Ruff-backed CodonX view.
- `assert-ir` emits legal Python code with Codon-facing runtime guards around
  supported annotations, assignments, and returns.
- `py-codon` emits a compile-first Codon candidate.
- `py-run` and `py-build` generate that candidate, inject supported `#%define`
  values into the Codon subprocess, invoke `codon run` or `codon build`, and
  delete the temporary candidate unless `--keep-pre` is set.

Imports are compatibility-first. A normal Python import becomes a Codon Python
interop import:

```python
import json as pyjson
```

becomes:

```python
from python import json as pyjson
```

If an import must stay native Codon, mark it:

```python
#%codon
import math
```

`#%define CODON_PYTHON /path/to/libpython3.12.so` is surfaced in generated
candidate headers and automatically injected by `py-run` / `py-build`.

This path preserves the remaining Python/Codon common subset. It does not yet
lower arbitrary Python semantics into native Codon.

## Source Directives

Target selection:

```text
#%ifpy
#%ifcodon
#%else
#%endif
```

Compatibility alias:

```text
#%ifdebug
```

`#%ifdebug` still works as an alias for `#%ifpy`, but new code should use
`#%ifpy`.

Codon subprocess hooks:

```text
#%define CODON_PYTHON <path>
#%define CODON_DEBUG <path>
```

`CODON_PYTHON` is used for Codon Python interop. `CODON_DEBUG` controls Codon
debug dump handling for wrapped Codon invocations.

0.2.x Python import intent:

```text
#%codon
```

`#%codon` must appear immediately before the import it describes.

## Runtime Guards

The Python debug target can insert guard checks for supported annotations.

Supported intent includes:

- `int`, `float`, `bool`, `complex`, ASCII `str`.
- Fixed-width integer intent such as `i32`, `u64`, `Int[32]`, `UInt[64]`, and
  `byte`.
- Float aliases such as `f32`, `f64`, and `float32`.
- `Optional`, `Union`, softened `Literal`, `NoneType`.
- Outer container shapes for `list`, `set`, `dict`, and `tuple`, with deeper
  element checks under `--assert full`.

These guards are mismatch detectors. They do not prove Python/Codon
equivalence, simulate parallel races, model GPU execution, resolve Codon
overloads, or reproduce Python interop conversion details.

## What Is Safe to Expect

Good current use cases:

- Codon-first single-file programs.
- Explicit Python/Codon target branches.
- Python 3.12 debug files with guard checks.
- Thin wrapping around the real Codon compiler.
- Early experiments with Python -> Codon candidate generation where fallback is
  acceptable.

Bad current use cases:

- Arbitrary Python-to-Codon conversion.
- Arbitrary Codon-to-Python conversion.
- Full Codon parsing.
- Whole-program type inference.
- Simulation of LLVM, C pointer, GPU, JIT, or parallel race semantics.
- Treating generated debug output as proof of release equivalence.

The project rule is simple: if `codonx` cannot make a local, explainable,
testable rewrite, it should preserve, warn, or require an explicit branch.

## More Documentation

- [Chinese README](docs/README.zh-CN.md)
- [Design notes](docs/design.md)
- [0.1.x roadmap](docs/roadmap-0.1.x.md)
- [0.2.x roadmap](docs/roadmap-0.2.x.md)
- [Examples](examples/README.md)

## Build and Test From Source

```bash
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo build --release --locked
```
