# codonx Examples

The examples are intentionally small and Linux-oriented. They assume:

- Python 3.12+ is available as `python3.12`;
- the Codon compiler is available as `codon` for release-path checks;
- `codonx` has been built or installed.

The example files use `.codonx` to make the dialect visible. New projects can
use `.codon`; the CLI does not depend on the extension.

## hello.codonx

Basic `int` annotation and Python debug guard example.

```bash
codonx --dbg examples/hello.codonx -o build/hello.py
python3.12 build/hello.py

codonx codon examples/hello.codonx -o build/hello.codon
codon run build/hello.codon
```

## parallel.codonx

Explicit Python/Codon target branches. The Python target keeps the serial loop;
the Codon release target keeps the `@par` branch.

```bash
codonx --dbg examples/parallel.codonx -o build/parallel.py
python3.12 build/parallel.py

codonx codon examples/parallel.codonx -o build/parallel.codon
codonx run -release examples/parallel.codonx
```

## python_interop.codonx

Simple `from python import ...` debug lowering.

```bash
codonx --dbg examples/python_interop.codonx -o build/python_interop.py
python3.12 build/python_interop.py
```

For real Codon Python interop, configure `CODON_PYTHON` through the shell or a
source-level define:

```python
#%define CODON_PYTHON /path/to/libpython3.12.so
```

## guard_failure.codonx

Shows Python runtime semantic guard failure for explicit fixed-width intent.

```bash
codonx --dbg examples/guard_failure.codonx -o build/guard_failure.py
python3.12 build/guard_failure.py
```

The Python run should fail because the value is outside `i32`.

## Recommended Smoke Pass

```bash
mkdir -p build
codonx --dbg examples/hello.codonx -o build/hello.py
python3.12 build/hello.py
codonx --dbg examples/parallel.codonx -o build/parallel.py
python3.12 build/parallel.py
codonx run -release examples/parallel.codonx
```
