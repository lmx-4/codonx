# Release Checklist

This checklist is for 0.x releases. The current public baseline is `0.1.3`.

## Release Contract

Before publishing, confirm that public docs still say:

- Linux only.
- Python 3.12+ is required for generated debug files.
- The official Codon compiler is required for `codonx run`, `codonx build`, and
  release validation.
- `--dbg` is the Python debug entry point in 0.1.x.
- `codonx` is not a full Codon parser, full Python parser, Python-to-Codon
  migrator, or semantic equivalence prover.

## Before Tagging

- [ ] Update `Cargo.toml` version.
- [ ] Update `CHANGELOG.md`.
- [ ] Update `README.md`.
- [ ] Update `docs/README.zh-CN.md` if public behavior changed.
- [ ] Update `docs/design.md` if implementation boundaries changed.
- [ ] Update `docs/roadmap-0.1.x.md` if the 0.1.x plan changed.
- [ ] Verify examples still match the current CLI.
- [ ] Verify generated files are not committed.

## Formatting and Checks

```bash
cargo fmt --all -- --check
cargo check --locked
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

## Example Smoke Tests

```bash
mkdir -p build

./target/release/codonx --dbg examples/hello.codonx -o build/hello.py
python3.12 build/hello.py

./target/release/codonx --dbg examples/parallel.codonx -o build/parallel.py
python3.12 build/parallel.py

./target/release/codonx --dbg examples/guard_failure.codonx -o build/guard_failure.py
! python3.12 build/guard_failure.py
```

## Codon Smoke Tests

Run these on a machine with the official Codon compiler installed:

```bash
./target/release/codonx codon examples/hello.codonx -o build/hello.codon
codon run build/hello.codon

./target/release/codonx run -release examples/parallel.codonx
```

If Codon is not on `PATH`:

```bash
./target/release/codonx --codon-bin /opt/codon/bin/codon run -release examples/parallel.codonx
```

## Tag

```bash
git tag v0.1.x
git push origin main
git push origin v0.1.x
```

## GitHub Release Package

Build the Linux x86_64 binary:

```bash
cargo build --release --locked
mkdir -p dist/codonx-v0.1.x-x86_64-linux
cp target/release/codonx dist/codonx-v0.1.x-x86_64-linux/codonx
cp README.md CHANGELOG.md LICENSE.md dist/codonx-v0.1.x-x86_64-linux/
tar -C dist -czf dist/codonx-v0.1.x-x86_64-linux.tar.gz codonx-v0.1.x-x86_64-linux
cd dist && sha256sum codonx-v0.1.x-x86_64-linux.tar.gz > codonx-v0.1.x-x86_64-linux.tar.gz.sha256
```

Release title:

```text
codonx 0.1.x
```

Release notes should include:

- major changes;
- behavior changes;
- known limitations;
- Linux/Python 3.12+/Codon compiler requirements;
- installation and smoke-test instructions.
