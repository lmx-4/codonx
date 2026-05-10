//! Python semantic guard generation for codonx 0.0.1.
//!
//! This module emits Python runtime assertions that make the Python debug target
//! fail earlier when a value violates the intended Codon type semantics.
//!
//! Scope of 0.0.1:
//! - primitive scalar guards: int/i8/u8/.../i64/u64, float/f32/f64, bool, str
//! - shallow/full guards for list[T], set[T], dict[K, V], tuple[T1, T2, ...]
//! - helper prelude inserted into generated Python target
//! - function-parameter/local-variable/return-value guard snippets
//!
//! Non-goals of 0.0.1:
//! - full Codon type parser
//! - exact GPU/parallel simulation
//! - pointer/LLVM/C-interop semantics

use crate::cli::AssertArg;
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
    r#"# --- codonx 0.0.1 semantic guard prelude ---
def __codonx_type_error(name, ty, value):
    raise AssertionError(
        f"codonx guard failed: {name} expected {ty}, got "
        f"{type(value).__name__}={value!r}"
    )


def __codonx_int_bounds(ty):
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
    return bounds.get(ty)


def __codonx_split_top_level_commas(text):
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


def __codonx_inner_type(ty, prefix):
    if not (ty.startswith(prefix + "[") and ty.endswith("]")):
        return None
    return ty[len(prefix) + 1:-1].strip()


def __codonx_assert_value(value, ty, name="<value>", full=False):
    ty = str(ty).strip()

    # Common aliases produced by the Python debug target after type rewrite.
    if ty in ("Any", "object", "pyobj"):
        return value

    int_bounds = __codonx_int_bounds(ty)
    if int_bounds is not None:
        lo, hi = int_bounds
        if type(value) is not int or not (lo <= value <= hi):
            __codonx_type_error(name, ty, value)
        return value

    if ty in ("float", "f32", "f64"):
        if type(value) is not float:
            __codonx_type_error(name, ty, value)
        return value

    if ty == "bool":
        if type(value) is not bool:
            __codonx_type_error(name, ty, value)
        return value

    if ty == "str":
        if type(value) is not str:
            __codonx_type_error(name, ty, value)
        try:
            value.encode("ascii")
        except UnicodeEncodeError:
            raise AssertionError(
                f"codonx guard failed: {name} expected Codon ASCII str, got non-ASCII str={value!r}"
            )
        return value

    inner = __codonx_inner_type(ty, "list")
    if inner is not None:
        if type(value) is not list:
            __codonx_type_error(name, ty, value)
        if full:
            for i, item in enumerate(value):
                __codonx_assert_value(item, inner, f"{name}[{i}]", full)
        return value

    inner = __codonx_inner_type(ty, "set")
    if inner is not None:
        if type(value) is not set:
            __codonx_type_error(name, ty, value)
        if full:
            for item in value:
                __codonx_assert_value(item, inner, f"{name}{'{...}'}", full)
        return value

    inner = __codonx_inner_type(ty, "dict")
    if inner is not None:
        if type(value) is not dict:
            __codonx_type_error(name, ty, value)
        parts = __codonx_split_top_level_commas(inner)
        if len(parts) == 2 and full:
            kt, vt = parts
            for k, v in value.items():
                __codonx_assert_value(k, kt, f"key of {name}", full)
                __codonx_assert_value(v, vt, f"{name}[{k!r}]", full)
        return value

    inner = __codonx_inner_type(ty, "tuple")
    if inner is not None:
        if type(value) is not tuple:
            __codonx_type_error(name, ty, value)
        parts = __codonx_split_top_level_commas(inner)
        if parts and len(value) != len(parts):
            raise AssertionError(
                f"codonx guard failed: {name} expected {ty}, got tuple length {len(value)}"
            )
        if full:
            for i, item_ty in enumerate(parts):
                __codonx_assert_value(value[i], item_ty, f"{name}[{i}]", full)
        return value

    # Unknown or unsupported types are not guessed. 0.0.1 keeps them as a soft
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
        "{}__codonx_assert_value({}, {}, {}, full={})",
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
        format!("{}__codonx_ret = {}", spaces, expr),
        format!(
            "{}__codonx_assert_value(__codonx_ret, {}, '<return>', full={})",
            spaces,
            ty_lit,
            py_full_flag(mode)
        ),
        format!("{}return __codonx_ret", spaces),
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
    normalize_type_spacing(t)
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
