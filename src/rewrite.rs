use crate::cli::AssertArg;
use crate::emit::Target;
use crate::guard::{
    canonical_guard_type, guard_return_lines, guard_stmts_for_assignment, guard_stmts_for_params,
    guards_enabled, normalize_param_pairs, python_guard_prelude,
};
use crate::report::Report;
use crate::source::SourceLine;
use crate::type_parse::{parse_ann_assign, parse_def_signature, translate_annotations_in_line};

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

    if guards_enabled(assert_mode) {
        out.push(python_guard_prelude().trim_end().to_string());
    }

    for line in lines {
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
            let guards = guard_stmts_for_assignment(
                &name,
                &canonical_guard_type(&ty),
                assert_mode,
                line.indent,
            );
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
        return None;
    }

    if trimmed.starts_with("@gpu.kernel") {
        report.warn(
            file,
            line.no,
            "gpu-fallback",
            "removed Codon @gpu.kernel annotation in Python target; GPU semantics are not simulated",
        );
        return None;
    }

    if trimmed.starts_with("@python") {
        return None;
    }

    let mut out = if let Some(import_line) = rewrite_from_python_import(&line.raw) {
        report.rewritten_imports += 1;
        import_line
    } else {
        line.raw.clone()
    };

    out = translate_annotations_in_line(&out);
    Some(out)
}

fn rewrite_from_python_import(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let indent_len = line.len() - trimmed.len();
    let rest = trimmed.strip_prefix("from python import ")?;
    Some(format!("{}import {}", &line[..indent_len], rest))
}
