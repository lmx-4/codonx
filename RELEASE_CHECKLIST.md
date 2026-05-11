# Release Checklist

This checklist is for 0.x releases.

## Before Tagging

- [ ] Update `Cargo.toml` version.
- [ ] Update `CHANGELOG.md`.
- [ ] Run formatting.

```bash
cargo fmt --all -- --check
```

- [ ] Run checks.

```bash
cargo check --locked
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
```

- [ ] Run example smoke tests.

```bash
mkdir -p build
./target/release/codonx --dbg examples/hello.codonx -o build/hello.py
python3 build/hello.py

./target/release/codonx --dbg examples/parallel.codonx -o build/parallel.py
python3 build/parallel.py

./target/release/codonx --dbg examples/guard_failure.codonx -o build/guard_failure.py
! python3 build/guard_failure.py
```

- [ ] Verify README examples still match current CLI.
- [ ] Verify docs mention experimental status.
- [ ] Verify generated files are not committed.

## Tag

```bash
git tag v0.0.x
git push origin v0.0.x
```

## GitHub Release

Attach Linux binary if desired:

```bash
cargo build --release --locked
cp target/release/codonx codonx-linux-x86_64
```

Release title:

```text
codonx 0.0.x
```

Release notes should include:

- major changes;
- breaking changes;
- known limitations;
- installation/build instructions.
