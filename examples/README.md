# codonx Examples

The examples are intentionally small and Linux-oriented. They demonstrate the
documented 0.2.3 behavior; they are not compatibility claims for arbitrary
Python or arbitrary Codon.

Assumptions:

- Python 3.12+ is available as `python3.12`.
- The Codon compiler is available as `codon` for release-path checks.
- `codonx` has been built or installed.

Most example files use `.codonx` to make the dialect visible. New projects can
use `.codon`; the CLI does not depend on file extension.

## Codon-First Examples

### `hello.codonx`

Basic Codon-first source, Python debug generation, and Codon projection.

```bash
mkdir -p build
codonx --dbg examples/hello.codonx -o build/hello.py
python3.12 build/hello.py

codonx codon examples/hello.codonx -o build/hello.codon
codon run build/hello.codon
```

### `parallel.codonx`

Explicit target branching. The Python projection uses the serial branch; the
Codon projection keeps the `@par` branch.

```bash
mkdir -p build
codonx --dbg examples/parallel.codonx -o build/parallel.py
python3.12 build/parallel.py

codonx codon examples/parallel.codonx -o build/parallel.codon
codonx run -release examples/parallel.codonx
```

### `python_interop.codonx`

Codon Python interop syntax in a Codon-first source file. The Python debug
projection lowers supported `from python import ...` forms to normal Python
imports.

```bash
mkdir -p build
codonx --dbg examples/python_interop.codonx -o build/python_interop.py
python3.12 build/python_interop.py
```

For real Codon Python interop, configure `CODON_PYTHON` through the environment
or through a supported source-level define:

```python
#%define CODON_PYTHON /path/to/libpython3.12.so
```

### `guard_failure.codonx`

Runtime guard failure for explicit fixed-width intent.

```bash
mkdir -p build
codonx --dbg examples/guard_failure.codonx -o build/guard_failure.py
python3.12 build/guard_failure.py
```

The generated Python program should fail because the value violates the `i32`
range contract.

## Python Frontend Smoke Example

0.2.3 can process Python source through the Ruff-backed experimental frontend.
The generated Codon candidate is conservative: native-marked imports remain
native, unmarked imports become Codon Python interop imports, and non-import
source is preserved where possible.

Create a temporary Python file:

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

Inspect frontend artifacts:

```bash
mkdir -p build
codonx ir /tmp/codonx_py_frontend_demo.py -o build/demo_ir.json
codonx assert-ir /tmp/codonx_py_frontend_demo.py -o build/demo_assert_ir.py
python3.12 build/demo_assert_ir.py
```

Generate the Codon candidate:

```bash
codonx py-codon /tmp/codonx_py_frontend_demo.py -o build/demo.codon
```

`math` remains a native import because of `#%codon`. `json` is rewritten to a
Codon Python interop import. `py-run` and `py-build` can invoke the real Codon
compiler when `CODON_PYTHON` points to a usable Python 3.12 runtime library.

## Recommended Smoke Pass

```bash
mkdir -p build
codonx --dbg examples/hello.codonx -o build/hello.py
python3.12 build/hello.py
codonx --dbg examples/parallel.codonx -o build/parallel.py
python3.12 build/parallel.py
codonx run -release examples/parallel.codonx
```
