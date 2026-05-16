use crate::cli::AssertArg;
use crate::emit::Target;
use crate::guard::{
    canonical_guard_type, guard_return_lines, guard_stmts_for_assignment, guard_stmts_for_params,
    guards_enabled, normalize_param_pairs, python_guard_prelude, record_guard_type_warnings,
};
use crate::report::Report;
use crate::source::SourceLine;

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
    let mut skip_extend_block: Option<(usize, bool)> = None;

    if guards_enabled(assert_mode) {
        out.push(python_guard_prelude().trim_end().to_string());
    }

    let mut idx = 0;
    while idx < lines.len() {
        let line = &lines[idx];
        if let Some((indent, saw_class)) = skip_extend_block {
            if line.trimmed.trim().is_empty() {
                out.push(line.raw.clone());
                idx += 1;
                continue;
            }
            if !saw_class
                && line.indent == indent
                && line.trimmed.trim_start().starts_with("class ")
            {
                out.push(format!(
                    "{}# codonx: omitted Codon extension class: {}",
                    " ".repeat(line.indent),
                    line.trimmed.trim()
                ));
                skip_extend_block = Some((indent, true));
                idx += 1;
                continue;
            }
            if saw_class && line.indent > indent {
                out.push(format!(
                    "{}# codonx: omitted Codon extension-only line: {}",
                    " ".repeat(line.indent),
                    line.trimmed.trim()
                ));
                idx += 1;
                continue;
            }
            skip_extend_block = None;
        }

        if let Some((indent, saw_def)) = skip_lowlevel_block {
            if line.trimmed.trim().is_empty() {
                out.push(line.raw.clone());
                idx += 1;
                continue;
            }
            if !saw_def && line.indent == indent && line.trimmed.trim_start().starts_with("def ") {
                out.push(format!(
                    "{}# codonx: omitted low-level Codon-only line: {}",
                    " ".repeat(line.indent),
                    line.trimmed.trim()
                ));
                skip_lowlevel_block = Some((indent, true));
                idx += 1;
                continue;
            }
            if saw_def && line.indent > indent {
                out.push(format!(
                    "{}# codonx: omitted low-level Codon-only line: {}",
                    " ".repeat(line.indent),
                    line.trimmed.trim()
                ));
                idx += 1;
                continue;
            }
            skip_lowlevel_block = None;
        }

        if !line.in_triple_string && line.trimmed.trim() == "@llvm" {
            report.warn_unsupported_rewrite_boundary(
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
            idx += 1;
            continue;
        }

        if !line.in_triple_string && line.trimmed.trim() == "@extend" {
            report.warn_semantic(
                file,
                line.no,
                "extension-method-semantics",
                "Codon extension class blocks are omitted in Python debug target to avoid shadowing Python types",
            );
            out.push(format!(
                "{}# codonx: omitted @extend block; use #%ifpy/#%ifcodon for a Python fallback",
                " ".repeat(line.indent)
            ));
            skip_extend_block = Some((line.indent, false));
            idx += 1;
            continue;
        }

        if !line.trimmed.trim().is_empty() {
            while funcs.last().is_some_and(|ctx| line.indent <= ctx.indent) {
                funcs.pop();
            }
        }

        if let Some(consumed) =
            rewrite_multiline_header(file, lines, idx, assert_mode, report, &mut out, &mut funcs)
        {
            idx += consumed;
            continue;
        }

        let Some(rewritten) = rewrite_py_line(file, line, assert_mode, report) else {
            idx += 1;
            continue;
        };

        if let Some(sig) = crate::ast::parse_def_signature_line(&line.raw, line.indent) {
            let (params, ret) = crate::ast::def_guard_signature(&sig);
            let ret = ret.as_deref().map(canonical_guard_type);
            out.push(
                crate::ast::rewrite_def_signature_for_python(&rewritten)
                    .unwrap_or_else(|| rewritten.clone()),
            );
            let params = normalize_param_pairs(params.into_iter().map(|p| (p.name, p.ty)));
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
            idx += 1;
            continue;
        }

        if let Some(ann) = crate::ast::parse_ann_assign_line(&line.raw) {
            out.push(
                crate::ast::rewrite_ann_assign_for_python(&rewritten)
                    .unwrap_or_else(|| rewritten.clone()),
            );

            let ty = canonical_guard_type(&ann.ty.text);
            record_guard_type_warnings(file, line.no, &ty, assert_mode, report);
            if ann.has_value {
                let guards = guard_stmts_for_assignment(&ann.name, &ty, assert_mode, line.indent);
                report.inserted_guards += guards.len();
                out.extend(guards);
            }
            idx += 1;
            continue;
        }

        if rewritten.trim_start().starts_with("return") {
            if let Some(ctx) = funcs.last() {
                let guarded =
                    guard_return_lines(&rewritten, ctx.ret.as_deref(), assert_mode, line.indent);
                report.inserted_guards += guarded.len().saturating_sub(1);
                out.extend(guarded);
                idx += 1;
                continue;
            }
        }

        out.push(rewritten.clone());
        idx += 1;
    }

    let mut text = out.join("\n");
    if !text.is_empty() {
        text.push('\n');
    }
    text
}

fn rewrite_multiline_header(
    file: &str,
    lines: &[SourceLine],
    start: usize,
    assert_mode: AssertArg,
    report: &mut Report,
    out: &mut Vec<String>,
    funcs: &mut Vec<FunctionCtx>,
) -> Option<usize> {
    let first = &lines[start];
    if first.in_triple_string || first.trimmed.trim_start().starts_with('#') {
        return None;
    }
    let trimmed = first.trimmed.trim_start();
    if !(trimmed.starts_with("def ") || trimmed.starts_with("class ")) {
        return None;
    }

    let end = collect_header_end(lines, start)?;
    if end == start {
        return None;
    }

    let raw = lines[start..=end]
        .iter()
        .map(|line| line.raw.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    if trimmed.starts_with("def ") {
        let sig = crate::ast::parse_def_signature_line(&raw, first.indent)?;
        let (params, ret) = crate::ast::def_guard_signature(&sig);
        let ret = ret.as_deref().map(canonical_guard_type);
        let rewritten = crate::ast::rewrite_def_signature_for_python(&raw)?;
        out.extend(rewritten.lines().map(str::to_string));

        let params = normalize_param_pairs(params.into_iter().map(|p| (p.name, p.ty)));
        for (_, ty) in &params {
            record_guard_type_warnings(file, first.no, ty, assert_mode, report);
        }
        if let Some(ret_ty) = ret.as_deref() {
            record_guard_type_warnings(file, first.no, ret_ty, assert_mode, report);
        }
        let guards = guard_stmts_for_params(&params, assert_mode, first.indent + 4);
        report.inserted_guards += guards.len();
        out.extend(guards);
        funcs.push(FunctionCtx {
            indent: first.indent,
            ret,
        });
        return Some(end - start + 1);
    }

    let rewritten = lower_class_signature(file, first.no, &raw, report);
    out.extend(rewritten.lines().map(str::to_string));
    Some(end - start + 1)
}

fn collect_header_end(lines: &[SourceLine], start: usize) -> Option<usize> {
    let mut depth = 0_i32;
    let mut saw_open = false;
    for (idx, line) in lines.iter().enumerate().skip(start) {
        if line.in_triple_string {
            return None;
        }
        for ch in line.raw.chars() {
            match ch {
                '(' | '[' | '{' => {
                    depth += 1;
                    saw_open = true;
                }
                ')' | ']' | '}' => depth -= 1,
                ':' if depth == 0 => return Some(idx),
                _ => {}
            }
        }
        if idx > start && !saw_open {
            return None;
        }
    }
    None
}

fn rewrite_py_line(
    file: &str,
    line: &SourceLine,
    assert_mode: AssertArg,
    report: &mut Report,
) -> Option<String> {
    if line.in_triple_string {
        return Some(line.raw.clone());
    }
    let trimmed = line.trimmed.trim();
    if trimmed.starts_with('#') {
        return Some(line.raw.clone());
    }

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
            report.warn_unsupported_rewrite_boundary(file, line.no, kind, message);
        } else if kind == "codon-jit-ignored" {
            report.warn_interop(file, line.no, kind, message);
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

    if contains_ndarray_type(trimmed) {
        report.warn_semantic(
            file,
            line.no,
            "ndarray-type-softened",
            "Codon ndarray dtype/ndim semantics are not checked in Python debug target",
        );
    }

    if trimmed == "import codon" || trimmed.starts_with("import codon ") {
        report.warn_interop(
            file,
            line.no,
            "codon-import-debug",
            "Python debug target does not require Codon JIT semantics; @codon decorators are ignored when lowered",
        );
    }

    let mut out = if let Some(import) = crate::ast::parse_from_python_import_line(&line.raw) {
        match import {
            crate::ast::FromPythonImport::Module { replacement } => {
                report.rewritten_imports += 1;
                replacement
            }
            crate::ast::FromPythonImport::Typed => {
                report.rewritten_imports += 1;
                report.warn(
                    file,
                    line.no,
                    "typed-python-interop-unsupported",
                    "typed Python interop import cannot be represented as a plain Python import; use #%ifpy/#%ifcodon to provide an explicit Python import or wrapper",
                );
                format!(
                    "{}# codonx: unsupported typed Python interop; use #%ifpy/#%ifcodon to provide a Python import/wrapper",
                    " ".repeat(line.indent)
                )
            }
        }
    } else {
        line.raw.clone()
    };

    out = lower_class_signature(file, line.no, &out, report);
    out = erase_type_params_from_signature(file, line.no, &out, report);
    out = lower_static_range(file, line.no, &out, report);
    out = lower_scalar_casts(file, line.no, &out, assert_mode, report);
    out = lower_assert_statement(file, line.no, &out, report);
    Some(out)
}

fn decorator_warning(trimmed: &str) -> Option<(&'static str, &'static str)> {
    if trimmed.starts_with("@codon.jit") || trimmed.starts_with("@codon.convert") {
        return Some((
            "codon-jit-ignored",
            "Codon JIT/convert decorator is ignored in Python debug target",
        ));
    }
    match trimmed {
        "@export" => Some((
            "export-ignored",
            "Codon export visibility has no Python debug equivalent",
        )),
        "@tuple" => Some((
            "tuple-class-semantics",
            "Codon tuple class layout is not simulated in Python debug target",
        )),
        "@overload" => Some((
            "overload-ignored",
            "Codon overload dispatch is not simulated in Python debug target",
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

fn contains_ndarray_type(trimmed: &str) -> bool {
    trimmed.contains("ndarray[")
}

fn lower_class_signature(file: &str, line_no: usize, line: &str, report: &mut Report) -> String {
    let Some(rewritten) = crate::ast::rewrite_class_signature_for_python(line) else {
        return line.to_string();
    };
    if rewritten.erased_type_params > 0 {
        report.erased_generics += rewritten.erased_type_params;
        report.warn_semantic(
            file,
            line_no,
            "generic-type-param-erased",
            "Codon class generic type parameters written as `T: type` are lowered to Python 3.12 type parameter syntax",
        );
    }
    if rewritten.lowered_static_inheritance {
        report.warn_semantic(
            file,
            line_no,
            "static-inheritance-lowered",
            "Codon Static[...] inheritance is lowered to normal Python inheritance in debug target",
        );
    }
    rewritten.line
}

fn erase_type_params_from_signature(
    file: &str,
    line_no: usize,
    line: &str,
    report: &mut Report,
) -> String {
    let Some(sig) = crate::ast::parse_def_signature_line(line, leading_indent(line)) else {
        return line.to_string();
    };
    let erased = sig
        .params
        .iter()
        .filter(|param| param.is_type_param)
        .count();
    let Some(out) = crate::ast::rewrite_def_signature_for_python(line) else {
        return line.to_string();
    };
    if erased > 0 {
        report.erased_generics += erased;
        report.warn_semantic(
            file,
            line_no,
            "generic-type-param-erased",
            "Codon generic type parameters passed as `type` arguments are erased in Python debug target",
        );
    }
    out
}

fn lower_static_range(file: &str, line_no: usize, line: &str, report: &mut Report) -> String {
    let (out, count) = rewrite_static_range_tokens(line);
    if count > 0 {
        report.warn_semantic(
            file,
            line_no,
            "static-range-lowered",
            "static.range(...) is lowered to runtime range(...) in Python debug target",
        );
    }
    out
}

fn lower_scalar_casts(
    file: &str,
    line_no: usize,
    line: &str,
    assert_mode: AssertArg,
    report: &mut Report,
) -> String {
    let (out, stats) = rewrite_scalar_cast_tokens(line, guards_enabled(assert_mode));
    report.lowered_casts += stats.int_casts + stats.float_casts;
    if stats.lossy_float_casts > 0 {
        report.warn_semantic(
            file,
            line_no,
            "float32-precision",
            "Python debug target lowers f32/float32 casts to Python float and does not simulate 32-bit rounding",
        );
    }
    out
}

fn lower_assert_statement(file: &str, line_no: usize, line: &str, report: &mut Report) -> String {
    let Some(rewritten) = crate::ast::rewrite_assert_statement_for_python(line) else {
        return line.to_string();
    };
    if rewritten.lowered_type_tokens > 0 {
        report.warn_semantic(
            file,
            line_no,
            "assert-type-lowered",
            "Codon type tokens in assert statements are lowered for Python 3.12 debug execution",
        );
    }
    rewritten.line
}

#[derive(Default)]
struct CastRewriteStats {
    int_casts: usize,
    float_casts: usize,
    lossy_float_casts: usize,
}

fn rewrite_static_range_tokens(line: &str) -> (String, usize) {
    let mut out = String::with_capacity(line.len());
    let mut pos = 0;
    let mut count = 0;
    for segment in code_segments(line) {
        let mut cursor = segment.start;
        while let Some(rel) = line[cursor..segment.end].find("static") {
            let start = cursor + rel;
            let end = start + "static".len();
            if !is_token_boundary(line, start, end) {
                cursor = end;
                continue;
            }
            let mut dot = skip_ascii_ws(line, end);
            if line.as_bytes().get(dot) != Some(&b'.') {
                cursor = end;
                continue;
            }
            dot += 1;
            let range_start = skip_ascii_ws(line, dot);
            let range_end = range_start + "range".len();
            if line.get(range_start..range_end) != Some("range")
                || !is_token_boundary(line, range_start, range_end)
            {
                cursor = end;
                continue;
            }
            let open = skip_ascii_ws(line, range_end);
            if line.as_bytes().get(open) != Some(&b'(') {
                cursor = range_end;
                continue;
            }
            out.push_str(&line[pos..start]);
            out.push_str("range");
            out.push_str(&line[open..open + 1]);
            pos = open + 1;
            cursor = open + 1;
            count += 1;
        }
    }
    out.push_str(&line[pos..]);
    (out, count)
}

fn rewrite_scalar_cast_tokens(line: &str, guard_casts: bool) -> (String, CastRewriteStats) {
    let mut out = String::with_capacity(line.len());
    let mut pos = 0;
    let mut stats = CastRewriteStats::default();

    for segment in code_segments(line) {
        let mut cursor = segment.start;
        while cursor < segment.end {
            let Some((cast, name_start, _name_end, open, close)) =
                parse_cast_call(line, cursor, segment.end)
            else {
                break;
            };
            let expr = &line[open + 1..close];
            out.push_str(&line[pos..name_start]);
            match cast {
                ScalarCast::Int(label) => {
                    stats.int_casts += 1;
                    if guard_casts {
                        out.push_str(&format!(
                            "__codonx_cast_int({}, \"__codonx_{}\")",
                            expr, label
                        ));
                    } else {
                        out.push_str(&format!("int({})", expr));
                    }
                }
                ScalarCast::Float { lossy } => {
                    stats.float_casts += 1;
                    if lossy {
                        stats.lossy_float_casts += 1;
                    }
                    out.push_str(&format!("float({})", expr));
                }
            }
            pos = close + 1;
            cursor = close + 1;
        }
    }

    out.push_str(&line[pos..]);
    (out, stats)
}

#[derive(Debug, Clone)]
enum ScalarCast {
    Int(String),
    Float { lossy: bool },
}

fn parse_cast_call(
    line: &str,
    mut cursor: usize,
    end: usize,
) -> Option<(ScalarCast, usize, usize, usize, usize)> {
    while cursor < end {
        let ch = line[cursor..].chars().next()?;
        if !is_ident_start(ch) {
            cursor += ch.len_utf8();
            continue;
        }
        let name_start = cursor;
        let name_end = take_ident(line, name_start)?;
        if !is_token_boundary(line, name_start, name_end) {
            cursor = name_end;
            continue;
        }
        let Some((cast, type_end)) = parse_scalar_cast_type(line, name_start, name_end) else {
            cursor = name_end;
            continue;
        };
        let open = skip_ascii_ws(line, type_end);
        if open >= end || line.as_bytes().get(open) != Some(&b'(') {
            cursor = type_end;
            continue;
        }
        let Some(close) = find_matching_code_delim(line, open, '(', ')') else {
            cursor = open + 1;
            continue;
        };
        if close > end {
            cursor = open + 1;
            continue;
        }
        return Some((cast, name_start, type_end, open, close));
    }
    None
}

fn parse_scalar_cast_type(
    line: &str,
    name_start: usize,
    name_end: usize,
) -> Option<(ScalarCast, usize)> {
    let name = &line[name_start..name_end];
    if matches!(
        name,
        "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64"
    ) {
        return Some((ScalarCast::Int(name.to_string()), name_end));
    }
    if matches!(
        name,
        "f32" | "float32" | "f64" | "float16" | "bfloat16" | "float128"
    ) {
        return Some((
            ScalarCast::Float {
                lossy: name != "f64",
            },
            name_end,
        ));
    }
    if !matches!(name, "Int" | "UInt") {
        return None;
    }
    let bracket_open = skip_ascii_ws(line, name_end);
    if line.as_bytes().get(bracket_open) != Some(&b'[') {
        return None;
    }
    let bracket_close = find_matching_code_delim(line, bracket_open, '[', ']')?;
    let bits = line[bracket_open + 1..bracket_close].trim();
    if bits.is_empty() || !bits.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    Some((
        ScalarCast::Int(format!("{}[{}]", name, bits)),
        bracket_close + 1,
    ))
}

#[derive(Debug, Clone, Copy)]
struct CodeSegment {
    start: usize,
    end: usize,
}

fn code_segments(line: &str) -> Vec<CodeSegment> {
    let mut out = Vec::new();
    let mut segment_start = 0;
    let mut i = 0;
    while i < line.len() {
        let ch = line[i..].chars().next().unwrap();
        if ch == '#' {
            if segment_start < i {
                out.push(CodeSegment {
                    start: segment_start,
                    end: i,
                });
            }
            return out;
        }
        if ch == '\'' || ch == '"' {
            if segment_start < i {
                out.push(CodeSegment {
                    start: segment_start,
                    end: i,
                });
            }
            i = skip_string_literal(line, i);
            segment_start = i;
            continue;
        }
        i += ch.len_utf8();
    }
    if segment_start < line.len() {
        out.push(CodeSegment {
            start: segment_start,
            end: line.len(),
        });
    }
    out
}

fn skip_string_literal(line: &str, start: usize) -> usize {
    let quote = line.as_bytes()[start];
    let triple = line.as_bytes().get(start + 1) == Some(&quote)
        && line.as_bytes().get(start + 2) == Some(&quote);
    let mut i = start + if triple { 3 } else { 1 };
    while i < line.len() {
        if !triple && line.as_bytes()[i] == b'\\' {
            i = (i + 2).min(line.len());
            continue;
        }
        if triple {
            if line.as_bytes().get(i) == Some(&quote)
                && line.as_bytes().get(i + 1) == Some(&quote)
                && line.as_bytes().get(i + 2) == Some(&quote)
            {
                return i + 3;
            }
        } else if line.as_bytes()[i] == quote {
            return i + 1;
        }
        i += 1;
    }
    line.len()
}

fn find_matching_code_delim(
    line: &str,
    open: usize,
    open_ch: char,
    close_ch: char,
) -> Option<usize> {
    let mut depth = 0_i32;
    let mut i = open;
    while i < line.len() {
        let ch = line[i..].chars().next()?;
        match ch {
            '\'' | '"' => {
                i = skip_string_literal(line, i);
                continue;
            }
            '#' => return None,
            c if c == open_ch => depth += 1,
            c if c == close_ch => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
        i += ch.len_utf8();
    }
    None
}

fn skip_ascii_ws(line: &str, mut i: usize) -> usize {
    while line.as_bytes().get(i).is_some_and(u8::is_ascii_whitespace) {
        i += 1;
    }
    i
}

fn leading_indent(line: &str) -> usize {
    line.chars()
        .take_while(|ch| *ch == ' ' || *ch == '\t')
        .map(|ch| if ch == '\t' { 4 } else { 1 })
        .sum()
}

fn take_ident(s: &str, start: usize) -> Option<usize> {
    let mut chars = s[start..].char_indices();
    let (_, first) = chars.next()?;
    if !is_ident_start(first) {
        return None;
    }
    let mut end = start + first.len_utf8();
    for (rel, ch) in chars {
        if !is_ident_continue(ch) {
            break;
        }
        end = start + rel + ch.len_utf8();
    }
    Some(end)
}

fn is_token_boundary(line: &str, start: usize, end: usize) -> bool {
    let before = line[..start].chars().next_back();
    let after = line[end..].chars().next();
    !before.is_some_and(is_ident_continue) && !after.is_some_and(is_ident_continue)
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}
