# codonx Design Notes

This document records the implementation semantics of `codonx` 0.2.3. It is a
technical description of the current system, not a language specification.

## System Shape

`codonx` has two active execution paths.

```text
Codon-first path
    source text
    -> source-line model
    -> #%define extraction
    -> target directive selection
    -> local AST/span and token-aware rewrites
    -> Python debug projection or Codon release projection
    -> optional codon run/build subprocess

Python frontend path
    Python 3.12 source text
    -> Ruff parser
    -> CodonX debug JSON view
    -> executable Python assert IR
    -> conservative Codon candidate
    -> optional codon run/build subprocess
```

The Codon-first path is the established workflow. The Python frontend path is
the 0.2.x architecture foundation.

## CLI Contract

Top-level `--dbg`:

```bash
codonx --dbg input.codon -o input_dbg.py
```

Generates Python debug output. `--assert` controls guard insertion and defaults
to `shallow`. `--report` writes an optional JSON warning report.

Codon projection:

```bash
codonx codon input.codon -o input_pre.codon
```

Generates a selected/preprocessed Codon file and exits.

Codon subprocess wrappers:

```bash
codonx run [codon run args...]
codonx build [codon build args...]
```

These commands locate the first source-file argument, generate a sibling
`*_pre.codon` file, replace the source argument with that generated file, invoke
the real `codon` binary, and remove the generated file unless `--keep-pre` is
set. The Codon binary is resolved from `--codon-bin`, then `CODONX_CODON_BIN`,
then `codon`.

Check:

```bash
codonx check input.codon
```

Checks directive structure and generated Python syntax. It is not a Codon type
checker.

Python frontend commands:

```bash
codonx ir input.py -o input_ir.json
codonx assert-ir input.py -o input_assert_ir.py
codonx py-codon input.py -o input_py.codon
codonx py-run [codon run args...]
codonx py-build [codon build args...]
```

`py-run` and `py-build` generate a sibling `*_py.codon` file, invoke `codon run`
or `codon build`, and remove the generated file unless `--keep-pre` is set.

## Directive Semantics

Target directives:

```text
#%ifpy
#%ifcodon
#%else
#%endif
```

`#%ifdebug` remains a compatibility alias for `#%ifpy`.

Supported defines:

```text
#%define CODON_PYTHON <path>
#%define CODON_DEBUG <path>
```

Unknown define names are hard errors. `#%define` is not a general macro
facility.

`CODON_PYTHON` is injected into wrapped Codon subprocesses. `CODON_DEBUG` is
used by the Codon-first `run`/`build` path; in debug-mode Codon `run`, codonx
builds in the debug directory and appends `-log l` unless the user already
supplied a log-dump option.

The Python frontend also recognizes `#%define CODON_PYTHON` while generating
Codon candidates. Because this value cannot be embedded as executable Codon, the
candidate header records it and `py-run` / `py-build` inject it into the Codon
subprocess environment.

## Codon-First Rewrite Model

The Codon-first path uses a source-line model plus local AST/span and
token-aware transformations. Regex is not the semantic authority for mechanical
rewrites.

Implemented rewrite properties:

- target directives are selected before semantic rewrites;
- supported `#%define` lines are stripped from generated targets;
- comments and strings are protected from token-level semantic rewrites;
- function signatures, class signatures, annotated assignments, assertions,
  selected casts, and selected multiline spans are handled through local
  span-aware logic;
- unsupported or ambiguous constructs should be preserved, warned about, or
  moved behind explicit target branches.

The local AST is not a complete Codon parser. It is a rewrite-oriented structure
for the subset currently supported by the debug projection.

## Codon-First Guard Coverage

The Codon-first Python debug path can guard supported function parameters,
annotated assignments, and return values.

Supported categories include:

- `int`, with `bool` rejected as an integer value;
- fixed-width integer intent: `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`,
  `u64`, `Int[N]`, `UInt[N]`, `byte`;
- `float`, `f32`, `f64`, `float32`, and related lowered float forms;
- `bool`, `complex`, ASCII `str`;
- `Optional`, `Union`, softened `Literal`, and `NoneType`;
- outer container shape checks for `list`, `set`, `dict`, and `tuple`;
- deeper container element checks under `--assert full`.

The guard system is diagnostic. It does not implement Codon type checking,
overload resolution, GPU semantics, parallel race behavior, C pointer semantics,
LLVM behavior, or Python interop conversion behavior.

## Ruff-Backed IR

`codonx ir` parses Python 3.12 source through `ruff_python_parser` and emits a
JSON debug view with schema version `1`.

Current top-level fields:

- `schema_version`;
- `frontend`;
- `python_target`;
- `source_path`;
- `source_bytes`;
- `macros`;
- `nodes`;
- `diagnostics`.

Current node fields:

- numeric node `id`;
- `kind`;
- optional `name`;
- byte and line `range`;
- attached macro IDs;
- optional `import_policy`;
- import module names;
- `conversion` status;
- diagnostic IDs.

Current conversion statuses:

- `codon_native`;
- `guarded`;
- `fallback`;
- `unsupported`.

The JSON output is a debug artifact for inspection and testing. The primary
user-facing semantic artifact for Python debugging is `assert-ir`.

## Macro Attachment

All `#%` lines in Python frontend input are collected as macros. The current
attachment algorithm binds any unbound macro that appears before a parsed node
to that node when the node is visited.

`#%codon` has defined meaning only for imports. If attached to a non-import
node, the node receives `invalid-codon-macro-target-import-required`.

This attachment rule is intentionally simple in 0.2.3. Future versions should
make placement stricter and source ranges clearer before expanding macro
categories.

## Python Frontend Import Policy

Default import policy is CPython fallback.

```python
import json as pyjson
```

generates:

```python
from python import json as pyjson
```

Default `from` import:

```python
from pathlib import Path
```

generates:

```python
from python import pathlib as __codonx_py_pathlib
Path = __codonx_py_pathlib.Path
```

Native Codon import intent:

```python
#%codon
import math
```

generates:

```python
import math
```

For `#%codon` imports, the IR marks `import_policy` as
`codon_native_required`. Modules outside the current known-native list receive
an unverified native-import diagnostic.

Wildcard fallback from-imports are not expanded. Relative imports without an
absolute module are skipped with a conservative generated comment.

## Python Assert IR

`codonx assert-ir` emits executable Python 3.12 code. The original program
shape is preserved where possible. The generator inserts `assert` statements
around supported:

- function parameters;
- annotated assignments;
- return values.

Current `assert-ir` type recognition is intentionally narrower than the
Codon-first guard system. It recognizes these annotation families:

- `int`;
- `float`;
- `bool`;
- `str`;
- `list`;
- `dict`;
- `tuple`;
- `set`.

Unsupported annotation families are preserved without an inserted assert. The
guard helper returns `True` for unknown type names, but the generator does not
currently emit guards for unknown families.

Import statements are preserved in assert IR, with comments documenting whether
the import policy is `codon_native_required` or `python_fallback_default`.

## Codon Candidate Generation

`codonx py-codon` is compile-first, not semantically complete.

Current generator behavior:

- parse source through Ruff;
- emit a generated-file header;
- surface `CODON_PYTHON` from `#%define CODON_PYTHON` in a header comment;
- strip lines whose trimmed form starts with `#%`;
- preserve `#%codon` imports as normal Codon imports;
- rewrite unmarked `import` statements to `from python import ...`;
- rewrite unmarked `from module import name` through a Python module binding;
- preserve non-import statements as source text.

The generator does not yet perform broad expression lowering, whole-function
native lowering, type inference, fallback-island generation, or source-map
reporting.

## Numeric Policy

The preferred mainline annotation style follows Codon's Python-like surface:

- `int`;
- `float`;
- `list[int]`;
- `dict[str, float]`;
- related high-level container forms.

Fixed-width forms such as `i32`, `u64`, `Int[32]`, `UInt[64]`, and `f32`
represent explicit low-level intent. The Python debug projection can preserve
some of that intent through guards, but release typing is still Codon's
responsibility.

## Non-Goals in 0.2.3

0.2.3 does not provide:

- complete Python-to-Codon translation;
- complete Codon-to-Python translation;
- full Codon parsing;
- whole-program type inference;
- Codon overload resolution;
- native AST integration with Codon's compiler internals;
- simulation of GPU, LLVM, C pointer, JIT, or parallel race semantics;
- guaranteed semantic equivalence between generated Python debug output and
  compiled Codon output.

## Implementation Direction

The intended 0.2.x architecture remains:

```text
Python source -> Ruff AST -> CodonX IR -> Codon target
Codon/CodonX source -> CodonX subset parser -> CodonX IR -> Python target
```

CodonX IR is the project-owned boundary. Ruff is the Python parser frontend.
Codon remains the release compiler. Future work should improve source mapping,
diagnostics, native-subset lowering, and explicit fallback islands without
weakening the existing Codon-first workflow.
