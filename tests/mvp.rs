use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_codonx"))
}

fn write_file(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, text).unwrap();
    path
}

fn run(args: &[&str]) -> std::process::Output {
    Command::new(bin()).args(args).output().unwrap()
}

#[test]
fn debug_and_codon_targets_select_opposite_branches() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "parallel.codonx",
        r#"def square_all(xs: list[i32]) -> list[i32]:
    out: list[i32] = [0 for _ in range(len(xs))]
    #%ifdebug
    for i in range(len(xs)):
        out[i] = xs[i] * xs[i]
    #%else
    @par(schedule="dynamic")
    for i in range(len(xs)):
        out[i] = xs[i] * xs[i]
    #%endif
    return out
"#,
    );
    let py = dir.path().join("parallel.py");
    let codon = dir.path().join("parallel.codon");

    let out = run(&[
        "--dbg",
        src.to_str().unwrap(),
        "--assert",
        "off",
        "-o",
        py.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let py_text = fs::read_to_string(py).unwrap();
    assert!(py_text.contains("for i in range(len(xs)):"));
    assert!(!py_text.contains("@par"));

    let out = run(&[
        "codon",
        src.to_str().unwrap(),
        "-o",
        codon.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let codon_text = fs::read_to_string(codon).unwrap();
    assert!(codon_text.contains("@par(schedule=\"dynamic\")"));
    assert!(!codon_text.contains("#%ifdebug"));
}

#[test]
fn regex_lowering_leaves_comments_and_report_warnings() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "lowering.codonx",
        r#"from python import math as m
from python import numpy.array(pyobj) -> pyobj

@python
def py_only(x: i32) -> i32:
    return x

def work(n: i32) -> i32:
    total: i32 = 0
    @par
    for i in range(n):
        total = total + i
    return total
"#,
    );
    let py = dir.path().join("lowering.py");
    let report = dir.path().join("report.json");

    let out = run(&[
        "--dbg",
        src.to_str().unwrap(),
        "--assert",
        "off",
        "-o",
        py.to_str().unwrap(),
        "--report",
        report.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let py_text = fs::read_to_string(py).unwrap();
    assert!(py_text.contains("import math as m"));
    assert!(py_text.contains("unsupported typed Python interop"));
    assert!(py_text.contains("removed @python"));
    assert!(py_text.contains("removed @par"));
    assert!(py_text.contains("def work(n: int) -> int:"));

    let report_text = fs::read_to_string(report).unwrap();
    assert!(report_text.contains("unsupported-syntax"));
    assert!(report_text.contains("python-interop"));
    assert!(report_text.contains("parallel-fallback"));
}

#[test]
fn shallow_guards_check_scalars_and_container_shape() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "guards.codonx",
        r#"def add_i32(a: i32, b: i32) -> i32:
    c: i32 = a + b
    return c

print(add_i32(1, 2))
"#,
    );
    let py = dir.path().join("guards.py");

    let out = run(&["--dbg", src.to_str().unwrap(), "-o", py.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let py_text = fs::read_to_string(&py).unwrap();
    assert!(py_text.contains("__codonx_assert_value(a, \"i32\""));
    assert!(py_text.contains("__codonx_assert_value(c, \"i32\""));
    assert!(py_text.contains("__codonx_assert_value(__codonx_ret, \"i32\""));

    let py_run = Command::new("python3").arg(py).output().unwrap();
    assert!(
        py_run.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&py_run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&py_run.stdout).trim(), "3");
}

#[test]
fn codon_run_swaps_input_path_and_deletes_preprocessed_file() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(dir.path(), "app.codonx", "print(1)\n");

    let out = run(&[
        "--codon-bin",
        "/bin/echo",
        "run",
        "-release",
        src.to_str().unwrap(),
        "arg1",
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("run -release"));
    assert!(stdout.contains("app_pre.codon"));
    assert!(stdout.contains("arg1"));
    assert!(!dir.path().join("app_pre.codon").exists());
}

#[test]
fn unmatched_directives_fail_check() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(dir.path(), "bad.codonx", "#%ifdebug\nprint(1)\n");

    let out = run(&["check", src.to_str().unwrap()]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unclosed #%ifdebug"));
}

#[cfg(unix)]
#[test]
fn defines_are_stripped_and_injected_into_codon_debug_process() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "defines.codonx",
        r#"#%define CODON_PYTHON /tmp/libpython-test.so
#%define CODON_DEBUG target/codonx_test_debug_dumps
print(1)
"#,
    );
    let spy = write_file(
        dir.path(),
        "spy.sh",
        r#"#!/usr/bin/env bash
printf 'PWD=%s\n' "$PWD"
printf 'CODON_PYTHON=%s\n' "$CODON_PYTHON"
printf 'CODON_DEBUG=%s\n' "$CODON_DEBUG"
printf 'ARGS=%s\n' "$*"
"#,
    );
    let mut perms = fs::metadata(&spy).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&spy, perms).unwrap();

    let codon = dir.path().join("defines.codon");
    let out = run(&[
        "codon",
        src.to_str().unwrap(),
        "-o",
        codon.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let codon_text = fs::read_to_string(codon).unwrap();
    assert!(!codon_text.contains("#%define"));
    assert!(codon_text.contains("print(1)"));

    let out = run(&[
        "--codon-bin",
        spy.to_str().unwrap(),
        "--keep-pre",
        "run",
        src.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let debug_dir = std::env::current_dir()
        .unwrap()
        .join("target/codonx_test_debug_dumps");
    assert!(stdout.contains("CODON_PYTHON=/tmp/libpython-test.so"));
    assert!(stdout.contains(&format!("CODON_DEBUG={}", debug_dir.display())));
    assert!(stdout.contains(&format!("PWD={}", debug_dir.display())));
    assert!(stdout.contains("-log l"));
    assert!(stdout.contains("defines_pre.codon"));
}

#[cfg(unix)]
#[test]
fn codon_debug_define_does_not_add_log_in_release_mode() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "release_defines.codonx",
        r#"#%define CODON_DEBUG target/codonx_test_release_debug_dumps
print(1)
"#,
    );
    let spy = write_file(
        dir.path(),
        "spy.sh",
        r#"#!/usr/bin/env bash
printf 'PWD=%s\n' "$PWD"
printf 'CODON_DEBUG=%s\n' "$CODON_DEBUG"
printf 'ARGS=%s\n' "$*"
"#,
    );
    let mut perms = fs::metadata(&spy).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&spy, perms).unwrap();

    let out = run(&[
        "--codon-bin",
        spy.to_str().unwrap(),
        "run",
        "-release",
        src.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let debug_dir = std::env::current_dir()
        .unwrap()
        .join("target/codonx_test_release_debug_dumps");
    assert!(stdout.contains(&format!("CODON_DEBUG={}", debug_dir.display())));
    assert!(!stdout.contains("-log l"));
    assert!(!stdout.contains(&format!("PWD={}", debug_dir.display())));
}

#[cfg(unix)]
#[test]
fn program_argument_named_release_does_not_disable_debug_dump() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "program_args.codonx",
        r#"#%define CODON_DEBUG target/codonx_test_program_arg_debug_dumps
print(1)
"#,
    );
    let spy = write_file(
        dir.path(),
        "spy.sh",
        r#"#!/usr/bin/env bash
printf 'PWD=%s\n' "$PWD"
printf 'ARGS=%s\n' "$*"
"#,
    );
    let mut perms = fs::metadata(&spy).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&spy, perms).unwrap();

    let out = run(&[
        "--codon-bin",
        spy.to_str().unwrap(),
        "run",
        src.to_str().unwrap(),
        "-release",
    ]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let debug_dir = std::env::current_dir()
        .unwrap()
        .join("target/codonx_test_program_arg_debug_dumps");
    assert!(stdout.contains(&format!("PWD={}", debug_dir.display())));
    assert!(stdout.contains("-log l"));
}
