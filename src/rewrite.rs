use crate::cli::AssertArg;
use crate::emit::Target;
use crate::guard::{
    canonical_guard_type, guard_return_lines, guard_stmts_for_assignment, guard_stmts_for_params,
    guards_enabled, normalize_param_pairs, python_guard_prelude, record_guard_type_warnings,
};
use crate::report::Report;
use crate::source::SourceLine;
use crate::type_parse::{
    parse_ann_assign, parse_def_signature, split_top_level_commas, translate_annotations_in_line,
};
use regex::Regex;

#[derive(Debug, Clone)]
struct FunctionCtx {
    indent: usize,
    ret: Option<String>,
}

pub fn rewrite_lines(
    file: &str,
    lines: &[SourceLine],
    target: Target,
    assert_mode: AssertArg,
    report: &mut Report,
) -> String {
    match target {
        Target::Codon => join_raw_lines(lines),
        Target::Py => rewrite_py_lines(file, lines, assert_mode, report),
    }
}

fn join_raw_lines(lines: &[SourceLine]) -> String {
    let mut out = lines
        .iter()
        .map(|l| l.raw.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

fn rewrite_py_lines(
    file: &str,
    lines: &[SourceLine],
    assert_mode: AssertArg,
    report: &mut Report,
) -> String {
    let mut out = Vec::new();
    let mut funcs: Vec<FunctionCtx> = Vec::new();
    let mut skip_lowlevel_block: Option<(usize, bool)> = None;

    if guards_enabled(assert_mode) {
        out.push(python_guard_prelude().trim_end().to_string());
    }

    for line in lines {
        if let Some((indent, saw_def)) = skip_lowlevel_block {
            if line.trimmed.trim().is_empty() {
                out.push(line.raw.clone());
                continue;
            }
            if !saw_def && line.indent == indent && line.trimmed.trim_start().starts_with("def ") {
                out.push(format!(
                    "{}# codonx: omitted low-level Codon-only line: {}",
                    " ".repeat(line.indent),
                    line.trimmed.trim()
                ));
                skip_lowlevel_block = Some((indent, true));
                continue;
            }
            if saw_def && line.indent > indent {
                out.push(format!(
                    "{}# codonx: omitted low-level Codon-only line: {}",
                    " ".repeat(line.indent),
                    line.trimmed.trim()
                ));
                continue;
            }
            skip_lowlevel_block = None;
        }

        if !line.in_triple_string && line.trimmed.trim() == "@llvm" {
            report.warn_unsupported_regex_boundary(
                file,
                line.no,
                "llvm-unsupported",
                "LLVM functions require an explicit Python target branch",
            );
            out.push(format!(
                "{}# codonx: omitted @llvm function; use #%ifpy/#%ifcodon for a Python fallback",
                " ".repeat(line.indent)
            ));
            skip_lowlevel_block = Some((line.indent, false));
            continue;
        }

        if !line.trimmed.trim().is_empty() {
            while funcs.last().is_some_and(|ctx| line.indent <= ctx.indent) {
                funcs.pop();
            }
        }

        let Some(rewritten) = rewrite_py_line(file, line, report) else {
            continue;
        };

        if let Some(sig) = parse_def_signature(&line.raw, line.indent) {
            let ret = sig.ret.as_deref().map(canonical_guard_type);
            out.push(translate_annotations_in_line(&rewritten));
            let params = normalize_param_pairs(sig.params.into_iter().map(|p| (p.name, p.ty)));
            for (_, ty) in &params {
                record_guard_type_warnings(file, line.no, ty, assert_mode, report);
            }
            if let Some(ret_ty) = ret.as_deref() {
                record_guard_type_warnings(file, line.no, ret_ty, assert_mode, report);
            }
            let guards = guard_stmts_for_params(&params, assert_mode, line.indent + 4);
            report.inserted_guards += guards.len();
            out.extend(guards);
            funcs.push(FunctionCtx {
                indent: line.indent,
                ret,
            });
            continue;
        }

        if rewritten.trim_start().starts_with("return") {
            if let Some(ctx) = funcs.last() {
                let guarded =
                    guard_return_lines(&rewritten, ctx.ret.as_deref(), assert_mode, line.indent);
                report.inserted_guards += guarded.len().saturating_sub(1);
                out.extend(guarded);
                continue;
            }
        }

        out.push(rewritten.clone());

        if let Some((name, ty)) = parse_ann_assign(&line.raw) {
            let ty = canonical_guard_type(&ty);
            record_guard_type_warnings(file, line.no, &ty, assert_mode, report);
            let guards = guard_stmts_for_assignment(&name, &ty, assert_mode, line.indent);
            report.inserted_guards += guards.len();
            out.extend(guards);
        }
    }

    let mut text = out.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    text
}

fn rewrite_py_line(file: &str, line: &SourceLine, report: &mut Report) -> Option<String> {
    if line.in_triple_string {
        return Some(line.raw.clone());
    }
    let trimmed = line.trimmed.trim();

    if trimmed.starts_with("@par") {
        report.removed_parallel_annotations += 1;
        report.warn(
            file,
            line.no,
            "parallel-fallback",
            "removed Codon @par annotation in Python target; Python target is serial and cannot detect parallel races",
        );
        return Some(format!(
            "{}# codonx: removed @par; Python debug target runs this loop serially",
            " ".repeat(line.indent)
        ));
    }

    if trimmed.starts_with("@gpu.kernel") {
        report.warn(
            file,
            line.no,
            "gpu-fallback",
            "removed Codon @gpu.kernel annotation in Python target; GPU semantics are not simulated",
        );
        return Some(format!(
            "{}# codonx: removed @gpu.kernel; GPU semantics are not simulated",
            " ".repeat(line.indent)
        ));
    }

    if trimmed.starts_with("@python") {
        report.warn(
            file,
            line.no,
            "python-interop",
            "removed Codon @python annotation in Python target; Python target executes the function body directly",
        );
        return Some(format!(
            "{}# codonx: removed @python; Python debug target executes this function directly",
            " ".repeat(line.indent)
        ));
    }

    if let Some((kind, message)) = decorator_warning(trimmed) {
        if kind == "llvm-unsupported" {
            report.warn_unsupported_regex_boundary(file, line.no, kind, message);
        } else {
            report.warn_semantic(file, line.no, kind, message);
        }
        return Some(format!(
            "{}# codonx: removed {}; {}",
            " ".repeat(line.indent),
            trimmed,
            message
        ));
    }

    if is_c_interop_line(trimmed) {
        report.warn_interop(
            file,
            line.no,
            "c-interop-unsupported",
            "C interop is not simulated in Python debug target; use explicit target branches for a Python fallback",
        );
        return Some(format!(
            "{}# codonx: unsupported C interop; use #%ifpy/#%ifcodon for a Python fallback",
            " ".repeat(line.indent)
        ));
    }

    if contains_pointer_interop(trimmed) {
        report.warn_interop(
            file,
            line.no,
            "pointer-interop-unsupported",
            "pointer/cobj interop is not simulated in Python debug target",
        );
    }

    if trimmed.starts_with("from python import ")
        && (trimmed.contains("->") || trimmed.contains('(') || trimmed.contains(')'))
    {
        report.warn(
            file,
            line.no,
            "typed-python-interop-unsupported",
            "typed Python interop import is outside regex-level lowering; use #%ifpy/#%ifcodon to provide an explicit Python import or wrapper",
        );
    }

    let mut out = if let Some(import_line) = rewrite_from_python_import(&line.raw) {
        report.rewritten_imports += 1;
        import_line
    } else {
        line.raw.clone()
    };

    out = erase_generic_syntax(file, line.no, &out, report);
    out = lower_static_range(file, line.no, &out, report);
    out = lower_scalar_casts(file, line.no, &out, report);
    out = translate_annotations_in_line(&out);
    Some(out)
}

fn decorator_warning(trimmed: &str) -> Option<(&'static str, &'static str)> {
    match trimmed {
        "@export" => Some((
            "export-ignored",
            "Codon export visibility has no Python debug equivalent",
        )),
        "@tuple" => Some((
            "tuple-class-semantics",
            "Codon tuple class layout is not simulated in Python debug target",
        )),
        "@extend" => Some((
            "extension-method-semantics",
            "Codon extension method semantics are not simulated in Python debug target",
        )),
        "@llvm" | "@pure" | "@no_side_effect" | "@nocapture" | "@self_captures" | "@derives" => {
            Some((
                "llvm-unsupported",
                "LLVM/Codon's low-level annotations require an explicit Python target branch",
            ))
        }
        _ => None,
    }
}

fn is_c_interop_line(trimmed: &str) -> bool {
    trimmed.starts_with("from C import ") || trimmed == "import C"
}

fn contains_pointer_interop(trimmed: &str) -> bool {
    trimmed.contains("Ptr[")
        || trimmed.contains("cobj")
        || trimmed.contains("__ptr__")
        || trimmed.contains("CPtr[")
}

fn erase_generic_syntax(file: &str, line_no: usize, line: &str, report: &mut Report) -> String {
    let mut out = Regex::new(r"(\bdef\s+[A-Za-z_][A-Za-z0-9_]*)\[[^\]]+\]")
        .unwrap()
        .replace(line, "$1")
        .to_string();
    out = Regex::new(r"(\bclass\s+[A-Za-z_][A-Za-z0-9_]*)\[[^\]]+\]")
        .unwrap()
        .replace(&out, "$1")
        .to_string();
    if out != line {
        report.erased_generics += 1;
        report.warn_semantic(
            file,
            line_no,
            "generic-type-param-erased",
            "Codon generic type parameters are erased in Python debug target",
        );
    }
    erase_type_params_from_signature(file, line_no, &out, report)
}

fn erase_type_params_from_signature(
    file: &str,
    line_no: usize,
    line: &str,
    report: &mut Report,
) -> String {
    let Some(open) = line.find('(') else {
        return line.to_string();
    };
    let Some(close) = find_matching_paren(line, open) else {
        return line.to_string();
    };
    if !line[..open].trim_start().starts_with("def ") {
        return line.to_string();
    }

    let params = &line[open + 1..close];
    let mut erased = 0;
    let kept = split_top_level_commas(params)
        .into_iter()
        .filter(|param| {
            let is_type_param = Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*\s*:\s*type(?:\s*=.*)?$")
                .unwrap()
                .is_match(param.trim());
            if is_type_param {
                erased += 1;
            }
            !is_type_param
        })
        .collect::<Vec<_>>();
    if erased == 0 {
        return line.to_string();
    }
    report.erased_generics += erased;
    report.warn_semantic(
        file,
        line_no,
        "generic-type-param-erased",
        "Codon generic type parameters passed as `type` arguments are erased in Python debug target",
    );
    format!(
        "{}({}){}",
        &line[..open],
        kept.join(", "),
        &line[close + 1..]
    )
}

fn lower_static_range(file: &str, line_no: usize, line: &str, report: &mut Report) -> String {
    let out = Regex::new(r"\bstatic\.range\s*\(")
        .unwrap()
        .replace_all(line, "range(")
        .to_string();
    if out != line {
        report.warn_semantic(
            file,
            line_no,
            "static-range-lowered",
            "static.range(...) is lowered to runtime range(...) in Python debug target",
        );
    }
    out
}

fn lower_scalar_casts(file: &str, line_no: usize, line: &str, report: &mut Report) -> String {
    let mut out = line.to_string();
    for ty in ["i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64"] {
        let re = Regex::new(&format!(r"\b{}\s*\((?P<expr>[^()]*)\)", regex::escape(ty))).unwrap();
        let before = out.clone();
        out = re
            .replace_all(
                &out,
                format!("__codonx_cast_int($expr, \"__codonx_{}\")", ty),
            )
            .to_string();
        if out != before {
            report.lowered_casts += 1;
        }
    }
    for prefix in ["Int", "UInt"] {
        let re = Regex::new(&format!(
            r"\b{}\[(?P<bits>\s*\d+\s*)\]\s*\((?P<expr>[^()]*)\)",
            prefix
        ))
        .unwrap();
        let before = out.clone();
        out = re
            .replace_all(
                &out,
                format!("__codonx_cast_int($expr, \"__codonx_{}[$bits]\")", prefix),
            )
            .to_string();
        if out != before {
            report.lowered_casts += 1;
        }
    }
    for ty in ["f32", "float32", "f64"] {
        let re = Regex::new(&format!(r"\b{}\s*\((?P<expr>[^()]*)\)", regex::escape(ty))).unwrap();
        let before = out.clone();
        out = re.replace_all(&out, "float($expr)").to_string();
        if out != before {
            report.lowered_casts += 1;
            if ty != "f64" {
                report.warn_semantic(
                    file,
                    line_no,
                    "float32-precision",
                    "Python debug target lowers f32/float32 casts to Python float and does not simulate 32-bit rounding",
                );
            }
        }
    }
    out
}

fn find_matching_paren(line: &str, open: usize) -> Option<usize> {
    let mut depth = 0_i32;
    for (idx, ch) in line.char_indices().skip_while(|(idx, _)| *idx < open) {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn rewrite_from_python_import(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let indent_len = line.len() - trimmed.len();
    let rest = trimmed.strip_prefix("from python import ")?;
    if rest.contains("->") || rest.contains('(') || rest.contains(')') {
        return Some(format!(
            "{}# codonx: unsupported typed Python interop; use #%ifpy/#%ifcodon to provide a Python import/wrapper",
            &line[..indent_len]
        ));
    }
    Some(format!("{}import {}", &line[..indent_len], rest))
}
