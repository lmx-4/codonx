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

fn assert_success(out: &std::process::Output) {
    assert!(
        out.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn debug_and_codon_targets_select_opposite_branches() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "parallel.codonx",
        r#"def square_all(xs: list[i32]) -> list[i32]:
    out: list[i32] = [0 for _ in range(len(xs))]
    #%ifpy
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
    assert!(!codon_text.contains("#%ifpy"));
}

#[test]
fn ifcodon_can_put_codon_branch_first() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "codon_first.codon",
        r#"def work(n: i32) -> i32:
    total: i32 = 0
    #%ifcodon
    @par
    for i in range(n):
        total = total + i
    #%else
    for i in range(n):
        total = total + i
    #%endif
    return total
"#,
    );
    let py = dir.path().join("codon_first.py");
    let codon = dir.path().join("codon_first_pre.codon");

    let out = run(&[
        "--dbg",
        src.to_str().unwrap(),
        "--assert",
        "off",
        "-o",
        py.to_str().unwrap(),
    ]);
    assert_success(&out);
    let py_text = fs::read_to_string(py).unwrap();
    assert!(py_text.contains("for i in range(n):"));
    assert!(!py_text.contains("@par"));
    assert!(!py_text.contains("#%ifcodon"));

    let out = run(&[
        "codon",
        src.to_str().unwrap(),
        "-o",
        codon.to_str().unwrap(),
    ]);
    assert_success(&out);
    let codon_text = fs::read_to_string(codon).unwrap();
    assert!(codon_text.contains("@par"));
    assert!(!codon_text.contains("#%ifcodon"));
}

#[test]
fn ifdebug_remains_deprecated_alias_for_ifpy() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "legacy.codon",
        r#"#%ifdebug
print("py")
#%else
print("codon")
#%endif
"#,
    );
    let py = dir.path().join("legacy.py");
    let codon = dir.path().join("legacy_pre.codon");

    let out = run(&[
        "--dbg",
        src.to_str().unwrap(),
        "--assert",
        "off",
        "-o",
        py.to_str().unwrap(),
    ]);
    assert_success(&out);
    assert_eq!(fs::read_to_string(py).unwrap().trim(), "print(\"py\")");

    let out = run(&[
        "codon",
        src.to_str().unwrap(),
        "-o",
        codon.to_str().unwrap(),
    ]);
    assert_success(&out);
    assert_eq!(
        fs::read_to_string(codon).unwrap().trim(),
        "print(\"codon\")"
    );
}

#[test]
fn nested_ifpy_ifcodon_respects_inactive_parent() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "nested.codon",
        r#"#%ifpy
print("py outer")
    #%ifcodon
    print("leak")
    #%else
    print("py nested else")
    #%endif
#%else
print("codon outer")
#%endif
"#,
    );
    let py = dir.path().join("nested.py");
    let codon = dir.path().join("nested_pre.codon");

    let out = run(&[
        "--dbg",
        src.to_str().unwrap(),
        "--assert",
        "off",
        "-o",
        py.to_str().unwrap(),
    ]);
    assert_success(&out);
    let py_text = fs::read_to_string(py).unwrap();
    assert!(py_text.contains("print(\"py outer\")"));
    assert!(py_text.contains("print(\"py nested else\")"));
    assert!(!py_text.contains("leak"));
    assert!(!py_text.contains("codon outer"));

    let out = run(&[
        "codon",
        src.to_str().unwrap(),
        "-o",
        codon.to_str().unwrap(),
    ]);
    assert_success(&out);
    let codon_text = fs::read_to_string(codon).unwrap();
    assert!(codon_text.contains("print(\"codon outer\")"));
    assert!(!codon_text.contains("py outer"));
    assert!(!codon_text.contains("leak"));
}

#[cfg(unix)]
#[test]
fn codon_suffix_exercises_v003_features_and_boundaries() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "dialect.codon",
        r#"#%define CODON_PYTHON "/tmp/libpython-v003.so"
#%define CODON_DEBUG "target/codonx_v003_codon_suffix_debug"

DOC = """
#%define CODON_PYTHON should_not_be_collected
#%ifpy
not a real directive
#%endif
"""

from python import math as pymath
from python import numpy.array(pyobj) -> pyobj

@python
def py_bridge(x: i32) -> i32:
    return x

def branch_value(x: i32) -> i32:
    #%ifpy
    y: i32 = x + 1
    #%else
    y: i32 = x + 2
    #%endif
    return y

def sum_squares(xs: list[i32]) -> i32:
    total: i32 = 0
    @par(schedule="dynamic")
    for x in xs:
        total = total + x * x
    return total

def bad_full_guard(xs: list[i32]) -> i32:
    return xs[0]

@gpu.kernel
def gpu_only(x: i32):
    pass

print(branch_value(1))
print(sum_squares([1, 2, 3]))
"#,
    );
    let py = dir.path().join("dialect_dbg.py");
    let codon = dir.path().join("dialect_pre.codon");
    let report = dir.path().join("dialect_report.json");

    let out = run(&[
        "--dbg",
        src.to_str().unwrap(),
        "--assert",
        "full",
        "-o",
        py.to_str().unwrap(),
        "--report",
        report.to_str().unwrap(),
    ]);
    assert_success(&out);

    let py_text = fs::read_to_string(&py).unwrap();
    assert!(py_text.contains("DOC = \"\"\""));
    assert!(py_text.contains("should_not_be_collected"));
    assert!(py_text.contains("import math as pymath"));
    assert!(py_text.contains("unsupported typed Python interop"));
    assert!(py_text.contains("removed @python"));
    assert!(py_text.contains("removed @par"));
    assert!(py_text.contains("removed @gpu.kernel"));
    assert!(py_text.contains("y: int = x + 1"));
    assert!(!py_text.contains("y: int = x + 2"));
    assert!(!py_text.contains("#%define CODON_DEBUG"));
    assert!(!py_text.contains("\n    @par"));
    assert!(!py_text.contains("\n@gpu.kernel"));
    assert!(py_text.contains("__codonx_assert_value(x, \"i32\""));
    assert!(py_text.contains("__codonx_assert_value(xs, \"list[i32]\""));
    assert!(py_text.contains("full=True"));

    let report_text = fs::read_to_string(&report).unwrap();
    assert!(report_text.contains("typed-python-interop-unsupported"));
    assert!(report_text.contains("python-interop"));
    assert!(report_text.contains("parallel-fallback"));
    assert!(report_text.contains("gpu-fallback"));
    assert!(report_text.contains("\"rewritten_imports\": 2"));

    let py_run = Command::new("python3").arg(&py).output().unwrap();
    assert_success(&py_run);
    assert_eq!(String::from_utf8_lossy(&py_run.stdout).trim(), "2\n14");

    let guard_fail = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "ns={{}}; exec(open({:?}).read(), ns); ns['bad_full_guard']([True])",
            py.display().to_string()
        ))
        .output()
        .unwrap();
    assert!(!guard_fail.status.success());
    assert!(String::from_utf8_lossy(&guard_fail.stderr).contains("codonx guard failed"));

    let out = run(&[
        "codon",
        src.to_str().unwrap(),
        "-o",
        codon.to_str().unwrap(),
    ]);
    assert_success(&out);
    let codon_text = fs::read_to_string(&codon).unwrap();
    assert!(codon_text.contains("from python import math as pymath"));
    assert!(codon_text.contains("from python import numpy.array(pyobj) -> pyobj"));
    assert!(codon_text.contains("@python"));
    assert!(codon_text.contains("@par(schedule=\"dynamic\")"));
    assert!(codon_text.contains("@gpu.kernel"));
    assert!(codon_text.contains("y: i32 = x + 2"));
    assert!(!codon_text.contains("y: i32 = x + 1"));
    assert!(!codon_text.contains("#%define CODON_PYTHON \""));
    assert!(!codon_text.contains("#%define CODON_DEBUG \""));
    assert!(!codon_text.contains("\n    #%ifpy"));

    let spy = write_file(
        dir.path(),
        "spy.sh",
        r#"#!/usr/bin/env bash
printf 'BUILD_PWD=%s\n' "$PWD"
printf 'CODON_PYTHON=%s\n' "$CODON_PYTHON"
printf 'CODON_DEBUG=%s\n' "$CODON_DEBUG"
printf 'ARGS=%s\n' "$*"
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-o" ]; then
    shift
    out="$1"
    break
  fi
  shift
done
if [ -n "$out" ]; then
  cat > "$out" <<'EOF'
#!/usr/bin/env bash
printf 'RUN_PWD=%s\n' "$PWD"
printf 'RUN_CODON_DEBUG=%s\n' "$CODON_DEBUG"
printf 'RUN_ARGS=%s\n' "$*"
EOF
  chmod +x "$out"
fi
"#,
    );
    let mut perms = fs::metadata(&spy).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&spy, perms).unwrap();

    let out = run(&[
        "--codon-bin",
        spy.to_str().unwrap(),
        "--keep-pre",
        "run",
        src.to_str().unwrap(),
        "program-arg",
    ]);
    assert_success(&out);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let debug_dir = std::env::current_dir()
        .unwrap()
        .join("target/codonx_v003_codon_suffix_debug");
    assert!(stdout.contains("CODON_PYTHON=/tmp/libpython-v003.so"));
    assert!(stdout.contains(&format!("CODON_DEBUG={}", debug_dir.display())));
    assert!(stdout.contains(&format!("BUILD_PWD={}", debug_dir.display())));
    assert!(stdout.contains(&format!(
        "RUN_PWD={}",
        std::env::current_dir().unwrap().display()
    )));
    assert!(stdout.contains("-log l"));
    assert!(stdout.contains("build -log l"));
    assert!(stdout.contains("dialect_pre.codon"));
    assert!(stdout.contains("RUN_ARGS=program-arg"));
    assert!(dir.path().join("dialect_pre.codon").exists());
}

#[test]
fn unsupported_define_name_fails_fast() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "bad_define.codon",
        "#%define UNKNOWN value\nprint(1)\n",
    );

    let out = run(&["check", src.to_str().unwrap()]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unsupported #%define"));
    assert!(stderr.contains("only CODON_PYTHON and CODON_DEBUG are supported"));
}

#[test]
fn malformed_define_fails_fast() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "malformed_define.codon",
        "#%define CODON_DEBUG\nprint(1)\n",
    );

    let out = run(&["check", src.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("expected name and value"));
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
    assert!(report_text.contains("typed-python-interop-unsupported"));
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
fn extended_integer_guards_check_param_assignment_and_return() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "int_widths.codon",
        r#"def narrow(a: Int[8], b: UInt[8], c: byte) -> UInt[8]:
    x: Int[8] = a
    return b

print(narrow(127, 255, -128))
"#,
    );
    let py = dir.path().join("int_widths.py");

    let out = run(&[
        "--dbg",
        src.to_str().unwrap(),
        "--assert",
        "full",
        "-o",
        py.to_str().unwrap(),
    ]);
    assert_success(&out);
    let py_text = fs::read_to_string(&py).unwrap();
    assert!(py_text.contains("def narrow(a: int, b: int, c: int) -> int:"));
    assert!(py_text.contains("__codonx_assert_value(a, \"Int[8]\""));
    assert!(py_text.contains("__codonx_assert_value(b, \"UInt[8]\""));
    assert!(py_text.contains("__codonx_assert_value(c, \"i8\""));

    let ok = Command::new("python3").arg(&py).output().unwrap();
    assert_success(&ok);

    let overflow = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "ns={{}}; exec(open({:?}).read(), ns); ns['narrow'](128, 1, 0)",
            py.display().to_string()
        ))
        .output()
        .unwrap();
    assert!(!overflow.status.success());
    assert!(String::from_utf8_lossy(&overflow.stderr).contains("codonx guard failed"));

    let negative_uint = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "ns={{}}; exec(open({:?}).read(), ns); ns['narrow'](1, -1, 0)",
            py.display().to_string()
        ))
        .output()
        .unwrap();
    assert!(!negative_uint.status.success());
    assert!(String::from_utf8_lossy(&negative_uint.stderr).contains("codonx guard failed"));
}

#[test]
fn full_container_guards_recurse_through_aliases() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "containers.codon",
        r#"def use_containers(xs: List[Int[8]], ys: Dict[str, UInt[16]], zs: Set[byte], pair: Tuple[i32, str]) -> Tuple[i32, str]:
    return pair
"#,
    );
    let py = dir.path().join("containers.py");

    let out = run(&[
        "--dbg",
        src.to_str().unwrap(),
        "--assert",
        "full",
        "-o",
        py.to_str().unwrap(),
    ]);
    assert_success(&out);
    let py_text = fs::read_to_string(&py).unwrap();
    assert!(py_text.contains("\"list[Int[8]]\""));
    assert!(py_text.contains("\"dict[str, UInt[16]]\""));
    assert!(py_text.contains("\"set[i8]\""));
    assert!(py_text.contains("\"tuple[i32, str]\""));

    let ok = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "ns={{}}; exec(open({:?}).read(), ns); ns['use_containers']([1], {{'a': 65535}}, {{-1}}, (1, 'x'))",
            py.display().to_string()
        ))
        .output()
        .unwrap();
    assert_success(&ok);

    let bad_list = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "ns={{}}; exec(open({:?}).read(), ns); ns['use_containers']([128], {{'a': 1}}, {{1}}, (1, 'x'))",
            py.display().to_string()
        ))
        .output()
        .unwrap();
    assert!(!bad_list.status.success());

    let bad_dict = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "ns={{}}; exec(open({:?}).read(), ns); ns['use_containers']([1], {{'a': 65536}}, {{1}}, (1, 'x'))",
            py.display().to_string()
        ))
        .output()
        .unwrap();
    assert!(!bad_dict.status.success());

    let bad_set = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "ns={{}}; exec(open({:?}).read(), ns); ns['use_containers']([1], {{'a': 1}}, {{128}}, (1, 'x'))",
            py.display().to_string()
        ))
        .output()
        .unwrap();
    assert!(!bad_set.status.success());

    let bad_tuple = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "ns={{}}; exec(open({:?}).read(), ns); ns['use_containers']([1], {{'a': 1}}, {{1}}, (1,))",
            py.display().to_string()
        ))
        .output()
        .unwrap();
    assert!(!bad_tuple.status.success());
}

#[test]
fn guard_type_reports_include_new_warning_categories() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "guard_report.codon",
        r#"def risky(a: Mystery, b: pyobj, c: Any, d: object, e: f32, f: float32, g: Dict[str, i32], h: Set[byte], t: tuple[i32, ...]):
    pass
"#,
    );
    let py = dir.path().join("guard_report.py");
    let report = dir.path().join("guard_report.json");

    let out = run(&[
        "--dbg",
        src.to_str().unwrap(),
        "--assert",
        "shallow",
        "-o",
        py.to_str().unwrap(),
        "--report",
        report.to_str().unwrap(),
    ]);
    assert_success(&out);

    let report_text = fs::read_to_string(report).unwrap();
    assert!(report_text.contains("\"unknown_guard_types\": 1"));
    assert!(report_text.contains("\"unchecked_dynamic_types\": 3"));
    assert!(report_text.contains("\"semantic_warnings\": 5"));
    assert!(report_text.contains("unknown-guard-type"));
    assert!(report_text.contains("unchecked-dynamic-type"));
    assert!(report_text.contains("float32-precision"));
    assert!(report_text.contains("unordered-container"));
    assert!(report_text.contains("unsupported-tuple-ellipsis"));
}

#[test]
fn assert_off_skips_guard_prelude_and_guard_warnings() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "assert_off.codon",
        r#"def risky(a: Mystery, b: pyobj, c: f32, d: Dict[str, i32]):
    pass
"#,
    );
    let py = dir.path().join("assert_off.py");
    let report = dir.path().join("assert_off.json");

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
    assert_success(&out);

    let py_text = fs::read_to_string(py).unwrap();
    assert!(!py_text.contains("codonx semantic guard prelude"));
    assert!(!py_text.contains("__codonx_assert_value"));

    let report_text = fs::read_to_string(report).unwrap();
    assert!(report_text.contains("\"unknown_guard_types\": 0"));
    assert!(report_text.contains("\"unchecked_dynamic_types\": 0"));
    assert!(report_text.contains("\"semantic_warnings\": 0"));
    assert!(!report_text.contains("unknown-guard-type"));
    assert!(!report_text.contains("float32-precision"));
}

#[test]
fn scalar_casts_are_lowered_with_range_checks() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "casts.codon",
        r#"def ok() -> i32:
    a: i32 = i8(127)
    b: u8 = UInt[8](255)
    c: f32 = float32(1)
    d: i32 = i32(a + b)
    return d + int(c)

print(ok())
"#,
    );
    let py = dir.path().join("casts.py");
    let report = dir.path().join("casts.json");

    let out = run(&[
        "--dbg",
        src.to_str().unwrap(),
        "--assert",
        "full",
        "-o",
        py.to_str().unwrap(),
        "--report",
        report.to_str().unwrap(),
    ]);
    assert_success(&out);
    let py_text = fs::read_to_string(&py).unwrap();
    assert!(py_text.contains("__codonx_cast_int(127, \"__codonx_i8\")"));
    assert!(py_text.contains("__codonx_cast_int(255, \"__codonx_UInt[8]\")"));
    assert!(py_text.contains("float(1)"));

    let ok = Command::new("python3").arg(&py).output().unwrap();
    assert_success(&ok);
    assert_eq!(String::from_utf8_lossy(&ok.stdout).trim(), "383");

    let fail_src = write_file(dir.path(), "bad_cast.codon", "print(u8(-1))\n");
    let fail_py = dir.path().join("bad_cast.py");
    let out = run(&[
        "--dbg",
        fail_src.to_str().unwrap(),
        "-o",
        fail_py.to_str().unwrap(),
    ]);
    assert_success(&out);
    let fail = Command::new("python3").arg(fail_py).output().unwrap();
    assert!(!fail.status.success());
    assert!(String::from_utf8_lossy(&fail.stderr).contains("codonx guard failed"));

    let report_text = fs::read_to_string(report).unwrap();
    assert!(report_text.contains("\"lowered_casts\": 4"));
    assert!(report_text.contains("float32-precision"));
}

#[test]
fn py312_generics_are_preserved_while_type_params_are_erased() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "generics.codon",
        r#"def first[T](xs: list[T], T: type = int):
    return xs[0]

class Box[T]:
    value: T
    def __init__(self, value: T):
        self.value = value

print(first([3]))
print(Box("x").value)
"#,
    );
    let py = dir.path().join("generics.py");
    let report = dir.path().join("generics.json");

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
    assert_success(&out);
    let py_text = fs::read_to_string(&py).unwrap();
    assert!(py_text.contains("def first[T](xs: list[T]):"));
    assert!(py_text.contains("class Box[T]:"));
    assert!(!py_text.contains("T: type"));

    let ok = Command::new("python3").arg(&py).output().unwrap();
    assert_success(&ok);
    assert_eq!(String::from_utf8_lossy(&ok.stdout).trim(), "3\nx");

    let report_text = fs::read_to_string(report).unwrap();
    assert!(report_text.contains("\"erased_generics\": 1"));
    assert!(report_text.contains("generic-type-param-erased"));
}

#[test]
fn decorators_interop_and_static_boundaries_warn_clearly() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "boundaries.codon",
        r#"from C import foo(int) -> int

@export
def exported(n: i32) -> i32:
    return n

@tuple
class Pair:
    a: i32
    b: i32

@extend
class int:
    def twice(self):
        return self * 2

@llvm
def low(a: int) -> int:
    %res = add i64 %a, 1
    ret i64 %res

def uses_static(n: i32, p: Ptr[int], c: cobj):
    for i in static.range(n):
        print(i)

uses_static(2, None, None)
"#,
    );
    let py = dir.path().join("boundaries.py");
    let report = dir.path().join("boundaries.json");

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
    assert_success(&out);
    let py_text = fs::read_to_string(&py).unwrap();
    assert!(py_text.contains("unsupported C interop"));
    assert!(py_text.contains("removed @export"));
    assert!(py_text.contains("removed @tuple"));
    assert!(py_text.contains("removed @extend"));
    assert!(py_text.contains("omitted @llvm function"));
    assert!(py_text.contains("range(n)"));
    assert!(!py_text.contains("\n    %res"));

    let ok = Command::new("python3").arg(&py).output().unwrap();
    assert_success(&ok);

    let report_text = fs::read_to_string(report).unwrap();
    assert!(report_text.contains("c-interop-unsupported"));
    assert!(report_text.contains("export-ignored"));
    assert!(report_text.contains("tuple-class-semantics"));
    assert!(report_text.contains("extension-method-semantics"));
    assert!(report_text.contains("llvm-unsupported"));
    assert!(report_text.contains("pointer-interop-unsupported"));
    assert!(report_text.contains("static-range-lowered"));
    assert!(report_text.contains("\"interop_warnings\": 2"));
    assert!(report_text.contains("\"unsupported_regex_boundaries\": 1"));
}

#[test]
fn optional_union_none_and_literal_guards_are_checked() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "optional_union.codon",
        r#"def maybe(x: Optional[i32]) -> Optional[i32]:
    return x

def either(x: Union[i32, str]) -> Union[i32, str]:
    return x

def none_only(x: NoneType) -> NoneType:
    return x

def lit(x: Literal[int]) -> Literal[int]:
    return x

print(maybe(None))
print(either("ok"))
print(none_only(None))
print(lit(1))
"#,
    );
    let py = dir.path().join("optional_union.py");
    let report = dir.path().join("optional_union.json");

    let out = run(&[
        "--dbg",
        src.to_str().unwrap(),
        "--assert",
        "full",
        "-o",
        py.to_str().unwrap(),
        "--report",
        report.to_str().unwrap(),
    ]);
    assert_success(&out);
    let ok = Command::new("python3").arg(&py).output().unwrap();
    assert_success(&ok);

    let bad_optional = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "ns={{}}; exec(open({:?}).read(), ns); ns['maybe'](2**40)",
            py.display().to_string()
        ))
        .output()
        .unwrap();
    assert!(!bad_optional.status.success());

    let bad_union = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "ns={{}}; exec(open({:?}).read(), ns); ns['either'](1.5)",
            py.display().to_string()
        ))
        .output()
        .unwrap();
    assert!(!bad_union.status.success());

    let bad_none = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "ns={{}}; exec(open({:?}).read(), ns); ns['none_only'](0)",
            py.display().to_string()
        ))
        .output()
        .unwrap();
    assert!(!bad_none.status.success());

    let report_text = fs::read_to_string(report).unwrap();
    assert!(report_text.contains("literal-softened"));
}

#[test]
fn py312_target_edge_lowering_covers_overload_jit_static_float_and_ndarray() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "py312_edges.codon",
        r#"import codon

class Base:
    pass

class Child(Static[Base]):
    pass

@overload
def choose(x: i32) -> i32:
    return x

@codon.jit(pyvars=["choose"])
def jitted(x: float16, arr: ndarray[f32, 2]) -> complex:
    y: bfloat16 = bfloat16(1)
    z: float128 = float128(2)
    return complex(float16(x) + y + z)

print(Child().__class__.__name__)
"#,
    );
    let py = dir.path().join("py312_edges.py");
    let report = dir.path().join("py312_edges.json");

    let out = run(&[
        "--dbg",
        src.to_str().unwrap(),
        "--assert",
        "shallow",
        "-o",
        py.to_str().unwrap(),
        "--report",
        report.to_str().unwrap(),
    ]);
    assert_success(&out);
    let py_text = fs::read_to_string(&py).unwrap();
    assert!(py_text.contains("class Child(Base):"));
    assert!(py_text.contains("removed @overload"));
    assert!(py_text.contains("removed @codon.jit"));
    assert!(py_text.contains("x: float"));
    assert!(py_text.contains("arr: object"));
    assert!(py_text.contains("-> complex"));
    assert!(py_text.contains("float(1)"));
    assert!(py_text.contains("float(2)"));
    assert!(py_text.contains("float(x)"));

    let report_text = fs::read_to_string(report).unwrap();
    assert!(report_text.contains("codon-import-debug"));
    assert!(report_text.contains("static-inheritance-lowered"));
    assert!(report_text.contains("overload-ignored"));
    assert!(report_text.contains("codon-jit-ignored"));
    assert!(report_text.contains("ndarray-type-softened"));
    assert!(report_text.contains("float32-precision"));
}

#[test]
fn conservative_rewrites_do_not_touch_plain_strings_or_comments() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "conservative.codon",
        r#"# i32(1) static.range(3) List[i32]
print("i32(1) static.range(3) List[i32]")
value: i32 = i32(1)
print(value)
"#,
    );
    let py = dir.path().join("conservative.py");

    let out = run(&[
        "--dbg",
        src.to_str().unwrap(),
        "--assert",
        "off",
        "-o",
        py.to_str().unwrap(),
    ]);
    assert_success(&out);
    let py_text = fs::read_to_string(&py).unwrap();
    assert!(py_text.contains("# i32(1) static.range(3) List[i32]"));
    assert!(py_text.contains("print(\"i32(1) static.range(3) List[i32]\")"));
    assert!(py_text.contains("value: int = int(1)"));

    let ok = Command::new("python3").arg(py).output().unwrap();
    assert_success(&ok);
    assert_eq!(
        String::from_utf8_lossy(&ok.stdout).trim(),
        "i32(1) static.range(3) List[i32]\n1"
    );
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
    let src = write_file(dir.path(), "bad.codonx", "#%ifpy\nprint(1)\n");

    let out = run(&["check", src.to_str().unwrap()]);
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unclosed conditional directive started by #%ifpy"));
}

#[test]
fn unmatched_ifcodon_fails_check() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(dir.path(), "bad.codon", "#%ifcodon\nprint(1)\n");

    let out = run(&["check", src.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr)
        .contains("unclosed conditional directive started by #%ifcodon"));
}

#[test]
fn stray_else_and_endif_fail_check() {
    let dir = tempfile::tempdir().unwrap();
    let stray_else = write_file(dir.path(), "stray_else.codon", "#%else\nprint(1)\n");
    let out = run(&["check", stray_else.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("#%else without active conditional"));

    let stray_endif = write_file(dir.path(), "stray_endif.codon", "#%endif\nprint(1)\n");
    let out = run(&["check", stray_endif.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("#%endif without active conditional"));
}

#[test]
fn duplicate_else_fails_check() {
    let dir = tempfile::tempdir().unwrap();
    let src = write_file(
        dir.path(),
        "duplicate_else.codon",
        "#%ifpy\nprint(1)\n#%else\nprint(2)\n#%else\nprint(3)\n#%endif\n",
    );

    let out = run(&["check", src.to_str().unwrap()]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("duplicate #%else"));
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
printf 'BUILD_PWD=%s\n' "$PWD"
printf 'CODON_PYTHON=%s\n' "$CODON_PYTHON"
printf 'CODON_DEBUG=%s\n' "$CODON_DEBUG"
printf 'ARGS=%s\n' "$*"
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-o" ]; then
    shift
    out="$1"
    break
  fi
  shift
done
if [ -n "$out" ]; then
  cat > "$out" <<'EOF'
#!/usr/bin/env bash
printf 'RUN_PWD=%s\n' "$PWD"
printf 'RUN_CODON_PYTHON=%s\n' "$CODON_PYTHON"
printf 'RUN_CODON_DEBUG=%s\n' "$CODON_DEBUG"
EOF
  chmod +x "$out"
fi
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
    assert!(stdout.contains(&format!("BUILD_PWD={}", debug_dir.display())));
    assert!(stdout.contains(&format!(
        "RUN_PWD={}",
        std::env::current_dir().unwrap().display()
    )));
    assert!(stdout.contains("RUN_CODON_PYTHON=/tmp/libpython-test.so"));
    assert!(stdout.contains(&format!("RUN_CODON_DEBUG={}", debug_dir.display())));
    assert!(stdout.contains("-log l"));
    assert!(stdout.contains("build -log l"));
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
printf 'BUILD_PWD=%s\n' "$PWD"
printf 'ARGS=%s\n' "$*"
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "-o" ]; then
    shift
    out="$1"
    break
  fi
  shift
done
if [ -n "$out" ]; then
  cat > "$out" <<'EOF'
#!/usr/bin/env bash
printf 'RUN_PWD=%s\n' "$PWD"
printf 'RUN_ARGS=%s\n' "$*"
EOF
  chmod +x "$out"
fi
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
    assert!(stdout.contains(&format!("BUILD_PWD={}", debug_dir.display())));
    assert!(stdout.contains(&format!(
        "RUN_PWD={}",
        std::env::current_dir().unwrap().display()
    )));
    assert!(stdout.contains("-log l"));
    assert!(stdout.contains("RUN_ARGS=-release"));
}
