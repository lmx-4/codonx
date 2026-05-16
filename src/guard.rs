//! Python semantic guard generation for codonx 0.1.x.
//!
//! This module emits Python runtime assertions that make the Python debug target
//! fail earlier when a value violates the intended Codon type semantics.
//!
//! Scope of 0.1.x:
//! - primitive scalar guards: int/i8/u8/.../i64/u64, float/f32/f64, bool, str
//! - shallow/full guards for list[T], set[T], dict[K, V], tuple[T1, T2, ...]
//! - helper prelude inserted into generated Python target
//! - function-parameter/local-variable/return-value guard snippets
//!
//! Non-goals of 0.1.x:
//! - full Codon type parser
//! - exact GPU/parallel simulation
//! - pointer/LLVM/C-interop semantics

use crate::cli::AssertArg;
use crate::report::Report;
use crate::type_parse::split_top_level_commas;

/// Convert the CLI assert mode to the boolean expected by the Python helper.
pub fn py_full_flag(mode: AssertArg) -> &'static str {
    match mode {
        AssertArg::Full => "True",
        AssertArg::Shallow | AssertArg::Off => "False",
    }
}

pub fn guards_enabled(mode: AssertArg) -> bool {
    !matches!(mode, AssertArg::Off)
}

/// Prelude inserted once at the top of generated Python debug targets.
///
/// Keep this helper intentionally conservative. Python's bool is a subclass of
/// int, so integer guards use `type(v) is int` rather than `isinstance(v, int)`.
pub fn python_guard_prelude() -> String {
    r#"# --- codonx semantic guard prelude ---
def _codonx_type_error(name, ty, value):
    raise AssertionError(
        f"codonx guard failed: {name} expected {ty}, got "
        f"{type(value).__name__}={value!r}"
    )


def _codonx_guard_ok(value, ty, full=False):
    try:
        _codonx_assert_value(value, ty, "<union>", full)
        return True
    except AssertionError:
        return False


def _codonx_cast_int(value, ty):
    value = int(value)
    return _codonx_assert_value(value, ty, "<cast>", full=False)


def _codonx_int_bounds(ty):
    aliases = {"byte": "i8"}
    ty = aliases.get(ty, ty)
    if ty.startswith("__codonx_"):
        ty = ty[len("__codonx_"):]
    bounds = {
        "int": (-(2 ** 63), 2 ** 63 - 1),
        "i64": (-(2 ** 63), 2 ** 63 - 1),
        "u64": (0, 2 ** 64 - 1),
        "i32": (-(2 ** 31), 2 ** 31 - 1),
        "u32": (0, 2 ** 32 - 1),
        "i16": (-(2 ** 15), 2 ** 15 - 1),
        "u16": (0, 2 ** 16 - 1),
        "i8": (-(2 ** 7), 2 ** 7 - 1),
        "u8": (0, 2 ** 8 - 1),
    }
    if ty in bounds:
        return bounds[ty]
    if ty.startswith("Int[") and ty.endswith("]"):
        bits = ty[4:-1].strip()
        if bits.isdigit() and int(bits) > 0:
            bits = int(bits)
            return (-(2 ** (bits - 1)), 2 ** (bits - 1) - 1)
    if ty.startswith("UInt[") and ty.endswith("]"):
        bits = ty[5:-1].strip()
        if bits.isdigit() and int(bits) > 0:
            bits = int(bits)
            return (0, 2 ** bits - 1)
    return None


def _codonx_split_top_level_commas(text):
    parts = []
    depth = 0
    start = 0
    pairs = {"[": "]", "(": ")", "{": "}"}
    opens = set(pairs)
    closes = set(pairs.values())
    for i, ch in enumerate(text):
        if ch in opens:
            depth += 1
        elif ch in closes:
            depth -= 1
        elif ch == "," and depth == 0:
            item = text[start:i].strip()
            if item:
                parts.append(item)
            start = i + 1
    tail = text[start:].strip()
    if tail:
        parts.append(tail)
    return parts


def _codonx_inner_type(ty, prefix):
    if not (ty.startswith(prefix + "[") and ty.endswith("]")):
        return None
    return ty[len(prefix) + 1:-1].strip()


def _codonx_assert_value(value, ty, name="<value>", full=False):
    ty = str(ty).strip()

    if ty in ("None", "NoneType"):
        if value is not None:
            _codonx_type_error(name, ty, value)
        return value

    # Common aliases produced by the Python debug target after type rewrite.
    if ty in ("Any", "object", "pyobj"):
        return value

    inner = _codonx_inner_type(ty, "Optional")
    if inner is not None:
        if value is None:
            return value
        return _codonx_assert_value(value, inner, name, full)

    inner = _codonx_inner_type(ty, "Union")
    if inner is not None:
        parts = _codonx_split_top_level_commas(inner)
        if any(_codonx_guard_ok(value, part, full) for part in parts):
            return value
        _codonx_type_error(name, ty, value)

    int_bounds = _codonx_int_bounds(ty)
    if int_bounds is not None:
        lo, hi = int_bounds
        if type(value) is not int or not (lo <= value <= hi):
            _codonx_type_error(name, ty, value)
        return value

    if ty in ("float", "f32", "f64", "float32"):
        if type(value) is not float:
            _codonx_type_error(name, ty, value)
        return value

    if ty in ("float16", "bfloat16", "float128"):
        if type(value) is not float:
            _codonx_type_error(name, ty, value)
        return value

    if ty == "complex":
        if type(value) is not complex:
            _codonx_type_error(name, ty, value)
        return value

    if ty == "bool":
        if type(value) is not bool:
            _codonx_type_error(name, ty, value)
        return value

    if ty == "str":
        if type(value) is not str:
            _codonx_type_error(name, ty, value)
        try:
            value.encode("ascii")
        except UnicodeEncodeError:
            raise AssertionError(
                f"codonx guard failed: {name} expected Codon ASCII str, got non-ASCII str={value!r}"
            )
        return value

    inner = _codonx_inner_type(ty, "list")
    if inner is not None:
        if type(value) is not list:
            _codonx_type_error(name, ty, value)
        if full:
            for i, item in enumerate(value):
                _codonx_assert_value(item, inner, f"{name}[{i}]", full)
        return value

    inner = _codonx_inner_type(ty, "set")
    if inner is not None:
        if type(value) is not set:
            _codonx_type_error(name, ty, value)
        if full:
            for item in value:
                _codonx_assert_value(item, inner, f"{name}{'{...}'}", full)
        return value

    inner = _codonx_inner_type(ty, "dict")
    if inner is not None:
        if type(value) is not dict:
            _codonx_type_error(name, ty, value)
        parts = _codonx_split_top_level_commas(inner)
        if len(parts) == 2 and full:
            kt, vt = parts
            for k, v in value.items():
                _codonx_assert_value(k, kt, f"key of {name}", full)
                _codonx_assert_value(v, vt, f"{name}[{k!r}]", full)
        return value

    inner = _codonx_inner_type(ty, "tuple")
    if inner is not None:
        if type(value) is not tuple:
            _codonx_type_error(name, ty, value)
        parts = _codonx_split_top_level_commas(inner)
        if parts and parts[-1] != "..." and len(value) != len(parts):
            raise AssertionError(
                f"codonx guard failed: {name} expected {ty}, got tuple length {len(value)}"
            )
        if full and (not parts or parts[-1] != "..."):
            for i, item_ty in enumerate(parts):
                _codonx_assert_value(value[i], item_ty, f"{name}[{i}]", full)
        return value

    # Unknown or unsupported types are not guessed. codonx keeps them as a soft
    # pass to avoid false positives; later versions may support strict mode.
    return value
# --- end codonx semantic guard prelude ---

"#
    .to_string()
}

/// Emit one Python statement that checks a value against a Codon type string.
pub fn guard_stmt(name: &str, ty: &str, mode: AssertArg, indent: usize) -> Option<String> {
    if !guards_enabled(mode) {
        return None;
    }
    let spaces = " ".repeat(indent);
    let ty_lit = py_string_lit(ty.trim());
    let name_lit = py_string_lit(name.trim());
    Some(format!(
        "{}_codonx_assert_value({}, {}, {}, full={})",
        spaces,
        name.trim(),
        ty_lit,
        name_lit,
        py_full_flag(mode)
    ))
}

/// Emit guard statements for function parameters.
pub fn guard_stmts_for_params(
    params: &[(String, String)],
    mode: AssertArg,
    indent: usize,
) -> Vec<String> {
    if !guards_enabled(mode) {
        return Vec::new();
    }
    params
        .iter()
        .filter_map(|(name, ty)| guard_stmt(name, ty, mode, indent))
        .collect()
}

/// Emit guard statements for one annotated assignment.
pub fn guard_stmts_for_assignment(
    name: &str,
    ty: &str,
    mode: AssertArg,
    indent: usize,
) -> Vec<String> {
    guard_stmt(name, ty, mode, indent).into_iter().collect()
}

/// Convert `return expr` into guarded return lines when the function return type
/// is known. If `ret_ty` is None or assert is off, returns the original line.
pub fn guard_return_lines(
    original_line: &str,
    ret_ty: Option<&str>,
    mode: AssertArg,
    indent: usize,
) -> Vec<String> {
    if !guards_enabled(mode) {
        return vec![original_line.to_string()];
    }
    let Some(ret_ty) = ret_ty else {
        return vec![original_line.to_string()];
    };
    let trimmed = original_line.trim_start();
    if !trimmed.starts_with("return") {
        return vec![original_line.to_string()];
    }

    let expr = trimmed.strip_prefix("return").unwrap_or("").trim();
    if expr.is_empty() {
        return vec![original_line.to_string()];
    }

    let spaces = " ".repeat(indent);
    let ty_lit = py_string_lit(ret_ty.trim());
    vec![
        format!("{}_codonx_ret = {}", spaces, expr),
        format!(
            "{}_codonx_assert_value(_codonx_ret, {}, '<return>', full={})",
            spaces,
            ty_lit,
            py_full_flag(mode)
        ),
        format!("{}return _codonx_ret", spaces),
    ]
}

/// Build parameter tuples from parsed `ParamAnn`-like pairs without coupling this
/// module to a specific signature struct.
pub fn normalize_param_pairs<I>(params: I) -> Vec<(String, String)>
where
    I: IntoIterator<Item = (String, String)>,
{
    params
        .into_iter()
        .map(|(name, ty)| (name.trim().to_string(), canonical_guard_type(&ty)))
        .collect()
}

/// Canonicalize a source Codon type for Python guard checking.
///
/// The generated Python annotations may rewrite `i32` to `int`, but guards must
/// keep the original Codon range semantics. Therefore guard type strings should
/// be derived from the original source annotation before Python annotation rewrite.
pub fn canonical_guard_type(ty: &str) -> String {
    let t = ty.trim();
    if t.is_empty() {
        return t.to_string();
    }

    let t = t.split('=').next().unwrap_or(t).trim();
    canonicalize_type(&normalize_type_spacing(t))
}

pub fn record_guard_type_warnings(
    file: &str,
    line: usize,
    ty: &str,
    mode: AssertArg,
    report: &mut Report,
) {
    if !guards_enabled(mode) {
        return;
    }
    record_guard_type_warnings_inner(file, line, ty, report);
}

fn record_guard_type_warnings_inner(file: &str, line: usize, ty: &str, report: &mut Report) {
    match classify_guard_type(ty) {
        GuardType::Float32 => report.warn_semantic(
            file,
            line,
            "float32-precision",
            "Python debug target checks f32/float32 as Python float and does not simulate 32-bit rounding",
        ),
        GuardType::Dict(key, value) => {
            report.warn_semantic(
                file,
                line,
                "unordered-container",
                "Codon dict ordering can differ from Python; Python debug code should not rely on dict iteration order",
            );
            record_guard_type_warnings_inner(file, line, &key, report);
            record_guard_type_warnings_inner(file, line, &value, report);
        }
        GuardType::Set(inner) => {
            report.warn_semantic(
                file,
                line,
                "unordered-container",
                "Codon set ordering can differ from Python; Python debug code should not rely on set iteration order",
            );
            record_guard_type_warnings_inner(file, line, &inner, report);
        }
        GuardType::List(inner) => record_guard_type_warnings_inner(file, line, &inner, report),
        GuardType::Tuple(items) => {
            if items.last().is_some_and(|item| item == "...") {
                report.warn_semantic(
                    file,
                    line,
                    "unsupported-tuple-ellipsis",
                    "tuple[T, ...] is soft-checked for tuple shape only in Python debug target",
                );
            } else {
                for item in items {
                    record_guard_type_warnings_inner(file, line, &item, report);
                }
            }
        }
        GuardType::Optional(inner) => record_guard_type_warnings_inner(file, line, &inner, report),
        GuardType::Union(items) => {
            for item in items {
                record_guard_type_warnings_inner(file, line, &item, report);
            }
        }
        GuardType::Literal(inner) => {
            report.warn_semantic(
                file,
                line,
                "literal-softened",
                "Codon Literal[...] is checked as its underlying Python debug type",
            );
            record_guard_type_warnings_inner(file, line, &inner, report);
        }
        GuardType::Unchecked => report.warn_unchecked_dynamic_type(file, line, ty),
        GuardType::Unknown => report.warn_unknown_guard_type(file, line, ty),
        GuardType::Known => {}
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum GuardType {
    Known,
    Float32,
    List(String),
    Set(String),
    Dict(String, String),
    Tuple(Vec<String>),
    Optional(String),
    Union(Vec<String>),
    Literal(String),
    Unchecked,
    Unknown,
}

fn classify_guard_type(ty: &str) -> GuardType {
    let ty = canonical_guard_type(ty);
    if ty.is_empty() {
        return GuardType::Unknown;
    }
    if matches!(
        ty.as_str(),
        "int"
            | "i8"
            | "u8"
            | "i16"
            | "u16"
            | "i32"
            | "u32"
            | "i64"
            | "u64"
            | "f64"
            | "float"
            | "complex"
            | "bool"
            | "str"
            | "None"
            | "NoneType"
    ) || parse_int_bits(&ty, "Int").is_some()
        || parse_int_bits(&ty, "UInt").is_some()
    {
        return GuardType::Known;
    }
    if matches!(ty.as_str(), "f32" | "float16" | "bfloat16" | "float128") {
        return GuardType::Float32;
    }
    if matches!(ty.as_str(), "Any" | "object" | "pyobj") {
        return GuardType::Unchecked;
    }
    if let Some(inner) = inner_type(&ty, "list") {
        return GuardType::List(inner.to_string());
    }
    if let Some(inner) = inner_type(&ty, "set") {
        return GuardType::Set(inner.to_string());
    }
    if let Some(inner) = inner_type(&ty, "dict") {
        let parts = split_top_level_commas(inner);
        if parts.len() == 2 {
            return GuardType::Dict(parts[0].clone(), parts[1].clone());
        }
        return GuardType::Unknown;
    }
    if let Some(inner) = inner_type(&ty, "tuple") {
        return GuardType::Tuple(split_top_level_commas(inner));
    }
    if let Some(inner) = inner_type(&ty, "Optional") {
        return GuardType::Optional(inner.to_string());
    }
    if let Some(inner) = inner_type(&ty, "Union") {
        return GuardType::Union(split_top_level_commas(inner));
    }
    if let Some(inner) = inner_type(&ty, "Literal") {
        return GuardType::Literal(inner.to_string());
    }
    GuardType::Unknown
}

fn canonicalize_type(ty: &str) -> String {
    match ty {
        "byte" => return "i8".to_string(),
        "float32" => return "f32".to_string(),
        _ => {}
    }

    for (from, to) in [
        ("List", "list"),
        ("Dict", "dict"),
        ("Set", "set"),
        ("Tuple", "tuple"),
    ] {
        if let Some(inner) = inner_type(ty, from) {
            let parts = split_top_level_commas(inner)
                .into_iter()
                .map(|part| {
                    if part == "..." {
                        part
                    } else {
                        canonicalize_type(&normalize_type_spacing(&part))
                    }
                })
                .collect::<Vec<_>>();
            return format!("{}[{}]", to, parts.join(", "));
        }
    }

    for name in ["list", "dict", "set", "tuple"] {
        if let Some(inner) = inner_type(ty, name) {
            let parts = split_top_level_commas(inner)
                .into_iter()
                .map(|part| {
                    if part == "..." {
                        part
                    } else {
                        canonicalize_type(&normalize_type_spacing(&part))
                    }
                })
                .collect::<Vec<_>>();
            return format!("{}[{}]", name, parts.join(", "));
        }
    }

    for name in ["Optional", "Union", "Literal"] {
        if let Some(inner) = inner_type(ty, name) {
            let parts = split_top_level_commas(inner)
                .into_iter()
                .map(|part| canonicalize_type(&normalize_type_spacing(&part)))
                .collect::<Vec<_>>();
            return format!("{}[{}]", name, parts.join(", "));
        }
    }

    ty.to_string()
}

fn inner_type<'a>(ty: &'a str, prefix: &str) -> Option<&'a str> {
    ty.strip_prefix(prefix)?
        .strip_prefix('[')?
        .strip_suffix(']')
        .map(str::trim)
}

fn parse_int_bits(ty: &str, prefix: &str) -> Option<u32> {
    let inner = inner_type(ty, prefix)?;
    let bits = inner.parse::<u32>().ok()?;
    (bits > 0).then_some(bits)
}

fn normalize_type_spacing(ty: &str) -> String {
    let mut out = String::new();
    let mut last_space = false;
    for ch in ty.chars() {
        if ch.is_whitespace() {
            last_space = true;
            continue;
        }
        if matches!(ch, '[' | ']' | ',') {
            if out.ends_with(' ') {
                out.pop();
            }
            out.push(ch);
            if ch == ',' {
                out.push(' ');
            }
            last_space = false;
        } else {
            if last_space && !out.ends_with('[') && !out.ends_with(' ') && !out.ends_with(',') {
                out.push(' ');
            }
            out.push(ch);
            last_space = false;
        }
    }
    out.trim().to_string()
}

fn py_string_lit(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    format!("\"{}\"", escaped)
}

/// Convenience parser for a raw parameter list. This is useful for tests and for
/// simple call sites that do not want to depend on `type_parse::parse_def_signature`.
#[allow(dead_code)]
pub fn parse_param_guards_from_text(params_src: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for p in split_top_level_commas(params_src) {
        let p = p.trim();
        if p.is_empty() || p == "self" || p == "cls" || p.starts_with('*') {
            continue;
        }
        let Some((name_part, ty_part)) = p.split_once(':') else {
            continue;
        };
        let name = name_part.trim();
        let ty = ty_part.split('=').next().unwrap_or(ty_part).trim();
        if !name.is_empty() && !ty.is_empty() {
            out.push((name.to_string(), canonical_guard_type(ty)));
        }
    }
    out
}
