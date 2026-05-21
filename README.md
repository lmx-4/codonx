# codonx

> A disciplined bridge between Python's inspectability and Codon's release path.

Python and Codon are close enough to share a development story, but not close
enough to make semantic differences disappear. In real projects, that gap often
turns into two source files: one Python file for debugging and one Codon file
for performance. The cost is drift. The same algorithm slowly becomes two
programs.

`codonx` is a Linux-only command-line tool for keeping that boundary explicit.
It generates Python-facing debug artifacts, prepares Codon-facing release
artifacts, and delegates final execution to the real Codon compiler. The design
principle is conservative: translate only what can be made local, mechanical,
and testable; expose everything else as a branch, guard, fallback, or diagnostic.

```text
Codon-first source
    -> Python 3.12+ debug projection with optional runtime guards
    -> Codon release projection compiled by the official codon binary

Python 3.12 source, experimental 0.2.x frontend
    -> Ruff parser frontend
    -> CodonX debug view / executable assert IR
    -> conservative Codon candidate with explicit Python interop fallback
```

Current release line: **0.2.3 experimental**.

The production-minded core remains the Codon-first workflow established in
0.1.x. The 0.2.x line adds the first Ruff-backed Python frontend and a
compile-first `py-codon` candidate generator. It is an architecture foundation,
not a claim of complete Python-to-Codon translation.

## The Contract

`codonx` is built around four rules.

- One source should describe the Python/Codon split explicitly.
- Python debug output should help catch mismatches early, not prove semantic
  equivalence.
- Codon release behavior belongs to the official Codon compiler.
- Automatic lowering must remain conservative, explainable, and test-covered.

When Python and Codon need different code, the difference is written into the
source:

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

The Python projection keeps the serial loop. The Codon projection keeps the
`@par` loop. The boundary is visible in code review and testable in both
directions.

## Requirements

- Linux.
- Python 3.12 or newer.
- Official `codon` compiler for `codonx run`, `codonx build`, `py-run`, and
  `py-build`.
- Rust only when building `codonx` from source.

`codonx` is not useful as a standalone compiler. It is a preprocessing and
projection layer around Python 3.12+ and Codon.

## Install

```bash
tar -xzf codonx-v0.2.3-x86_64-linux.tar.gz
install -m 0755 codonx-v0.2.3-x86_64-linux/codonx ~/.local/bin/codonx
codonx --version
```

Expected output:

```text
codonx 0.2.3
```

Verify the external toolchain:

```bash
python3.12 --version
codon --version
```

Use an explicit Codon binary when needed:

```bash
codonx --codon-bin /opt/codon/bin/codon run -release app.codon
```

or:

```bash
export CODONX_CODON_BIN=/opt/codon/bin/codon
```

## Codon-First Workflow

Start with a Codon-oriented source file:

```python
def square_sum(xs: list[int]) -> int:
    total: int = 0
    for x in xs:
        total += x * x
    return total

print(square_sum([1, 2, 3]))
```

Generate a Python debug projection:

```bash
codonx --dbg app.codon -o app_dbg.py
python3.12 app_dbg.py
```

Generate a guarded debug projection:

```bash
codonx --dbg app.codon --assert full -o app_dbg.py --report codonx-report.json
```

Generate a Codon projection without invoking the compiler:

```bash
codonx codon app.codon -o app_pre.codon
```

Run or build through the official Codon compiler:

```bash
codonx run -release app.codon
codonx build -release -o dist/app app.codon
```

Validate directive structure and generated Python syntax:

```bash
codonx check app.codon
codonx check --assert full app.codon
```

`check` is not a Codon type checker. It validates the `codonx` preprocessing
surface; release semantics remain Codon's responsibility.

## Python Frontend Workflow

The 0.2.x frontend begins the reverse direction: Python source enters through
Ruff's Python 3.12 parser and is represented through CodonX frontend artifacts.

```bash
codonx ir app.py -o app_ir.json
codonx assert-ir app.py -o app_assert_ir.py
codonx py-codon app.py -o app.codon
codonx py-run app.py
codonx py-build app.py
```

0.2.3 behavior is intentionally narrow.

- `ir` emits a JSON debug dump of the Ruff-backed CodonX view.
- `assert-ir` emits executable Python code with guards for supported basic
  Python annotations: `int`, `float`, `bool`, `str`, `list`, `dict`, `tuple`,
  and `set`.
- `py-codon` emits a conservative Codon candidate.
- `py-run` and `py-build` generate that candidate, inject supported `#%define`
  values into the Codon subprocess, invoke `codon run` or `codon build`, and
  delete the temporary candidate unless `--keep-pre` is set.

Import handling is compatibility-first. Unmarked Python imports are routed
through Codon's Python interop:

```python
import json as pyjson
```

generates:

```python
from python import json as pyjson
```

Native Codon import intent must be explicit:

```python
#%codon
import math
```

`#%define CODON_PYTHON /path/to/libpython3.12.so` is surfaced in generated
candidate headers and injected automatically by `py-run` / `py-build`.

The current generator preserves the remaining Python/Codon common subset as
source text. It does not yet lower arbitrary Python statements into native Codon
semantics.

## Directives

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

`#%ifdebug` remains accepted as an alias for `#%ifpy`; new code should use
`#%ifpy`.

Codon subprocess configuration:

```text
#%define CODON_PYTHON <path>
#%define CODON_DEBUG <path>
```

Python frontend import intent:

```text
#%codon
```

`#%codon` must appear immediately before the import it describes.

## Guard Boundary

Codon-first Python debug output supports guards for high-level Python-like types
and explicit low-level Codon intent, including fixed-width integers such as
`i32`/`u64`, `Int[N]`/`UInt[N]`, selected float aliases, `Optional`, `Union`,
softened `Literal`, and common container shapes.

The Ruff-backed `assert-ir` command currently guards only the basic Python
annotation families listed above. That narrower coverage is intentional in
0.2.3.

All guards are mismatch detectors. They do not simulate parallel races, GPU
execution, LLVM, C pointer behavior, Codon overload resolution, or Python
interop conversion semantics.

## Current Fit

Use `codonx` today for:

- Codon-first single-file workflows.
- Python 3.12+ debug projections for Codon-oriented code.
- Explicit Python/Codon target branches.
- Guarded debug runs that catch common type and shape mismatches.
- Conservative Python frontend experiments where CPython fallback imports are
  acceptable.

Do not treat 0.2.3 as:

- a general Python-to-Codon transpiler;
- a general Codon-to-Python transpiler;
- a full Codon parser;
- a whole-program type inferencer;
- an emulator for Codon parallelism, GPU kernels, LLVM, C interop, or JIT
  behavior.

## Documentation

- [Chinese overview](docs/README.zh-CN.md)
- [Design notes](docs/design.md)
- [0.1.x roadmap](docs/roadmap-0.1.x.md)
- [0.2.x roadmap](docs/roadmap-0.2.x.md)
- [Examples](examples/README.md)

## Build From Source

```bash
cargo fmt --all -- --check
cargo check --locked
cargo test --locked
cargo build --release --locked
```
