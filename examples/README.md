# codonx Examples

These examples are intentionally small.

## hello.codonx

Basic type annotation and guard example.

```bash
codonx --dbg examples/hello.codonx -o build/hello.py
python3 build/hello.py

codonx codon examples/hello.codonx -o build/hello.codon
```

## parallel.codonx

Explicit Python/Codon target branches.

```bash
codonx --dbg examples/parallel.codonx -o build/parallel.py
python3 build/parallel.py

codonx codon examples/parallel.codonx -o build/parallel.codon
codonx run -release examples/parallel.codonx
```

## python_interop.codonx

Simple `from python import ...` debug lowering.

```bash
codonx --dbg examples/python_interop.codonx -o build/python_interop.py
python3 build/python_interop.py
```

## guard_failure.codonx

Shows Python runtime semantic guard failure.

```bash
codonx --dbg examples/guard_failure.codonx -o build/guard_failure.py
python3 build/guard_failure.py
```

The Python run should fail because the value is outside `i32`.
