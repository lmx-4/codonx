# codonx Examples

The examples are intentionally small. They are meant to show the current
0.2.3 semantics, not to imply broad Python/Codon compatibility.

Assumptions:

- Linux.
- Python 3.12+ is available as `python3.12`.
- The Codon compiler is available as `codon` for release-path checks.
- `codonx` has been built or installed.

Most example files use `.codonx` to make the dialect visible. New projects can
use `.codon`; the CLI does not depend on the extension.

## Codon-First Examples

### hello.codonx

Basic `int` annotation and Python debug guard flow.

```bash
mkdir -p build
codonx --dbg examples/hello.codonx -o build/hello.py
python3.12 build/hello.py

codonx codon examples/hello.codonx -o build/hello.codon
codon run build/hello.codon
```

### parallel.codonx

Explicit Python/Codon target branches. The Python target keeps the serial loop;
the Codon release target keeps the `@par` branch.

```bash
mkdir -p build
codonx --dbg examples/parallel.codonx -o build/parallel.py
python3.12 build/parallel.py

codonx codon examples/parallel.codonx -o build/parallel.codon
codonx run -release examples/parallel.codonx
```

### python_interop.codonx

Simple `from python import ...` debug lowering.

```bash
mkdir -p build
codonx --dbg examples/python_interop.codonx -o build/python_interop.py
python3.12 build/python_interop.py
```

For real Codon Python interop, configure `CODON_PYTHON` through the shell or a
source-level define:

```python
#%define CODON_PYTHON /path/to/libpython3.12.so
```

### guard_failure.codonx

Shows Python runtime semantic guard failure for explicit fixed-width intent.

```bash
mkdir -p build
codonx --dbg examples/guard_failure.codonx -o build/guard_failure.py
python3.12 build/guard_failure.py
```

The Python run should fail because the value is outside `i32`.

## Python Frontend Smoke Example

0.2.3 can also process a Python file through the Ruff-backed experimental path.
Create a temporary file:

```python
# /tmp/codonx_py_frontend_demo.py
#%define CODON_PYTHON /path/to/libpython3.12.so

#%codon
import math

import json as pyjson

def area(r: float) -> float:
    return math.pi * r * r

print(pyjson.dumps({"area": area(2.0)}))
```

Inspect the frontend view:

```bash
codonx ir /tmp/codonx_py_frontend_demo.py -o build/demo_ir.json
codonx assert-ir /tmp/codonx_py_frontend_demo.py -o build/demo_assert_ir.py
python3.12 build/demo_assert_ir.py
```

Generate a conservative Codon candidate:

```bash
codonx py-codon /tmp/codonx_py_frontend_demo.py -o build/demo.codon
```

The generated candidate keeps `math` native because of `#%codon` and rewrites
`json` to Codon's Python interop import. `py-run` and `py-build` can invoke the
real Codon compiler directly when `CODON_PYTHON` points to a usable Python 3.12
runtime library.

## Recommended Smoke Pass

```bash
mkdir -p build
codonx --dbg examples/hello.codonx -o build/hello.py
python3.12 build/hello.py
codonx --dbg examples/parallel.codonx -o build/parallel.py
python3.12 build/parallel.py
codonx run -release examples/parallel.codonx
```
