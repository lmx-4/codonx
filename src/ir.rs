use anyhow::Context;
use ruff_python_ast::{Expr, Parameters, Stmt, StmtFunctionDef, Suite};
use ruff_python_parser::parse_module;
use ruff_text_size::{Ranged, TextRange};
use serde::Serialize;
use std::{fs, path::Path};

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize)]
pub struct CodonxIr {
    pub schema_version: u32,
    pub frontend: String,
    pub python_target: String,
    pub source_path: String,
    pub source_bytes: usize,
    pub macros: Vec<IrMacro>,
    pub nodes: Vec<IrNode>,
    pub diagnostics: Vec<IrDiagnostic>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrNode {
    pub id: usize,
    pub kind: String,
    pub name: Option<String>,
    pub range: IrRange,
    pub macros: Vec<usize>,
    pub conversion: ConversionStatus,
    pub diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrMacro {
    pub id: usize,
    pub line: usize,
    pub text: String,
    pub attached_node: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrDiagnostic {
    pub id: String,
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IrRange {
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversionStatus {
    CodonNative,
    Guarded,
    Fallback,
    Unsupported,
}

#[derive(Debug)]
struct LineIndex {
    starts: Vec<usize>,
}

impl LineIndex {
    fn new(source: &str) -> Self {
        let mut starts = vec![0];
        for (idx, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                starts.push(idx + 1);
            }
        }
        Self { starts }
    }

    fn line_for_byte(&self, byte: usize) -> usize {
        match self.starts.binary_search(&byte) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        }
    }

    fn range(&self, range: TextRange) -> IrRange {
        let start = range.start().to_usize();
        let end = range.end().to_usize();
        IrRange {
            start_byte: start,
            end_byte: end,
            start_line: self.line_for_byte(start),
            end_line: self.line_for_byte(end),
        }
    }
}

pub fn build_ir(input: &Path) -> anyhow::Result<CodonxIr> {
    let source = fs::read_to_string(input)
        .with_context(|| format!("failed to read Python source {}", input.display()))?;
    let parsed = parse_module(&source)
        .map_err(|err| anyhow::anyhow!("Ruff parser failed for {}: {}", input.display(), err))?;
    let lines = LineIndex::new(&source);
    let mut macros = collect_macros(&source);
    let mut nodes = Vec::new();
    collect_suite(parsed.suite(), &lines, &mut macros, &mut nodes);

    let mut diagnostics = Vec::new();
    for mac in &macros {
        if mac.attached_node.is_none() {
            diagnostics.push(IrDiagnostic {
                id: format!("macro-{}-unbound", mac.id),
                line: mac.line,
                message: format!("macro `{}` is not attached to a parsed AST node", mac.text),
            });
        }
    }

    Ok(CodonxIr {
        schema_version: SCHEMA_VERSION,
        frontend: "ruff_python_parser".to_string(),
        python_target: "3.12+".to_string(),
        source_path: input.display().to_string(),
        source_bytes: source.len(),
        macros,
        nodes,
        diagnostics,
    })
}

pub fn render_ir_json(ir: &CodonxIr) -> anyhow::Result<String> {
    Ok(format!("{}\n", serde_json::to_string_pretty(ir)?))
}

pub fn render_assert_ir_python(input: &Path) -> anyhow::Result<String> {
    let source = fs::read_to_string(input)
        .with_context(|| format!("failed to read Python source {}", input.display()))?;
    let parsed = parse_module(&source)
        .map_err(|err| anyhow::anyhow!("Ruff parser failed for {}: {}", input.display(), err))?;
    let mut generator = AssertIrGenerator::new(&source);
    Ok(generator.render(parsed.suite()))
}

fn collect_macros(source: &str) -> Vec<IrMacro> {
    source
        .lines()
        .enumerate()
        .filter_map(|(idx, line)| {
            let trimmed = line.trim_start();
            trimmed.starts_with("#%").then(|| IrMacro {
                id: 0,
                line: idx + 1,
                text: trimmed.to_string(),
                attached_node: None,
            })
        })
        .enumerate()
        .map(|(id, mut mac)| {
            mac.id = id;
            mac
        })
        .collect()
}

fn collect_suite(
    suite: &Suite,
    lines: &LineIndex,
    macros: &mut [IrMacro],
    nodes: &mut Vec<IrNode>,
) {
    for stmt in suite {
        collect_stmt(stmt, lines, macros, nodes);
    }
}

fn collect_stmt(stmt: &Stmt, lines: &LineIndex, macros: &mut [IrMacro], nodes: &mut Vec<IrNode>) {
    let id = nodes.len();
    let range = lines.range(stmt.range());
    let attached_macros = attach_macros(id, range.start_line, macros);
    let diagnostics = diagnostics_for_stmt(stmt);
    nodes.push(IrNode {
        id,
        kind: stmt_kind(stmt).to_string(),
        name: stmt_name(stmt),
        range,
        macros: attached_macros,
        conversion: conversion_for_stmt(stmt),
        diagnostics,
    });

    match stmt {
        Stmt::FunctionDef(function) => collect_suite(&function.body, lines, macros, nodes),
        Stmt::ClassDef(class_def) => collect_suite(&class_def.body, lines, macros, nodes),
        Stmt::For(for_stmt) => {
            collect_suite(&for_stmt.body, lines, macros, nodes);
            collect_suite(&for_stmt.orelse, lines, macros, nodes);
        }
        Stmt::While(while_stmt) => {
            collect_suite(&while_stmt.body, lines, macros, nodes);
            collect_suite(&while_stmt.orelse, lines, macros, nodes);
        }
        Stmt::If(if_stmt) => {
            collect_suite(&if_stmt.body, lines, macros, nodes);
            for clause in &if_stmt.elif_else_clauses {
                collect_suite(&clause.body, lines, macros, nodes);
            }
        }
        Stmt::With(with_stmt) => collect_suite(&with_stmt.body, lines, macros, nodes),
        Stmt::Match(match_stmt) => {
            for case in &match_stmt.cases {
                collect_suite(&case.body, lines, macros, nodes);
            }
        }
        Stmt::Try(try_stmt) => {
            collect_suite(&try_stmt.body, lines, macros, nodes);
            for handler in &try_stmt.handlers {
                match handler {
                    ruff_python_ast::ExceptHandler::ExceptHandler(handler) => {
                        collect_suite(&handler.body, lines, macros, nodes);
                    }
                }
            }
            collect_suite(&try_stmt.orelse, lines, macros, nodes);
            collect_suite(&try_stmt.finalbody, lines, macros, nodes);
        }
        _ => {}
    }
}

fn attach_macros(node_id: usize, start_line: usize, macros: &mut [IrMacro]) -> Vec<usize> {
    let mut ids = Vec::new();
    for mac in macros.iter_mut() {
        if mac.attached_node.is_none() && mac.line < start_line {
            mac.attached_node = Some(node_id);
            ids.push(mac.id);
        }
    }
    ids
}

fn stmt_kind(stmt: &Stmt) -> &'static str {
    match stmt {
        Stmt::FunctionDef(_) => "function",
        Stmt::ClassDef(_) => "class",
        Stmt::Return(_) => "return",
        Stmt::Delete(_) => "delete",
        Stmt::TypeAlias(_) => "type_alias",
        Stmt::Assign(_) => "assign",
        Stmt::AugAssign(_) => "aug_assign",
        Stmt::AnnAssign(_) => "ann_assign",
        Stmt::For(_) => "for",
        Stmt::While(_) => "while",
        Stmt::If(_) => "if",
        Stmt::With(_) => "with",
        Stmt::Match(_) => "match",
        Stmt::Raise(_) => "raise",
        Stmt::Try(_) => "try",
        Stmt::Assert(_) => "assert",
        Stmt::Import(_) => "import",
        Stmt::ImportFrom(_) => "import_from",
        Stmt::Global(_) => "global",
        Stmt::Nonlocal(_) => "nonlocal",
        Stmt::Expr(_) => "expr",
        Stmt::Pass(_) => "pass",
        Stmt::Break(_) => "break",
        Stmt::Continue(_) => "continue",
        Stmt::IpyEscapeCommand(_) => "ipy_escape_command",
    }
}

fn stmt_name(stmt: &Stmt) -> Option<String> {
    match stmt {
        Stmt::FunctionDef(function) => Some(function.name.as_str().to_string()),
        Stmt::ClassDef(class_def) => Some(class_def.name.as_str().to_string()),
        _ => None,
    }
}

fn conversion_for_stmt(stmt: &Stmt) -> ConversionStatus {
    match stmt {
        Stmt::FunctionDef(_) | Stmt::ClassDef(_) | Stmt::Import(_) | Stmt::ImportFrom(_) => {
            ConversionStatus::Guarded
        }
        Stmt::Assign(_)
        | Stmt::AnnAssign(_)
        | Stmt::AugAssign(_)
        | Stmt::Return(_)
        | Stmt::For(_)
        | Stmt::While(_)
        | Stmt::If(_)
        | Stmt::Assert(_)
        | Stmt::Expr(_)
        | Stmt::Pass(_)
        | Stmt::Break(_)
        | Stmt::Continue(_) => ConversionStatus::CodonNative,
        Stmt::With(_) | Stmt::Try(_) | Stmt::Raise(_) | Stmt::Delete(_) => {
            ConversionStatus::Fallback
        }
        Stmt::TypeAlias(_)
        | Stmt::Match(_)
        | Stmt::Global(_)
        | Stmt::Nonlocal(_)
        | Stmt::IpyEscapeCommand(_) => ConversionStatus::Unsupported,
    }
}

fn diagnostics_for_stmt(stmt: &Stmt) -> Vec<String> {
    match stmt {
        Stmt::With(_) => vec!["fallback-with-runtime-context".to_string()],
        Stmt::Try(_) => vec!["fallback-exception-semantics".to_string()],
        Stmt::Raise(_) => vec!["fallback-exception-semantics".to_string()],
        Stmt::Delete(_) => vec!["fallback-python-delete-semantics".to_string()],
        Stmt::TypeAlias(_) => vec!["unsupported-python-type-alias".to_string()],
        Stmt::Match(_) => vec!["unsupported-pattern-matching".to_string()],
        Stmt::Global(_) => vec!["unsupported-global-scope-mutation".to_string()],
        Stmt::Nonlocal(_) => vec!["unsupported-nonlocal-scope-mutation".to_string()],
        Stmt::IpyEscapeCommand(_) => vec!["unsupported-ipython-escape-command".to_string()],
        _ => Vec::new(),
    }
}

struct AssertIrGenerator<'a> {
    source: &'a str,
    line_starts: Vec<usize>,
    out: String,
}

impl<'a> AssertIrGenerator<'a> {
    fn new(source: &'a str) -> Self {
        let mut line_starts = vec![0];
        for (idx, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(idx + 1);
            }
        }
        Self {
            source,
            line_starts,
            out: String::new(),
        }
    }

    fn render(&mut self, suite: &Suite) -> String {
        self.out.push_str(
            "# Generated by codonx assert-ir. This is executable Python 3.12 semantic IR.\n",
        );
        self.out
            .push_str("# The original program shape is preserved; inserted asserts tighten Codon-facing semantics.\n\n");
        self.out.push_str("def __codonx_guard(value, type_name):\n");
        self.out.push_str("    if type_name == \"int\":\n");
        self.out
            .push_str("        return isinstance(value, int) and not isinstance(value, bool)\n");
        self.out.push_str("    if type_name == \"float\":\n");
        self.out.push_str(
            "        return isinstance(value, (int, float)) and not isinstance(value, bool)\n",
        );
        self.out.push_str("    if type_name == \"bool\":\n");
        self.out
            .push_str("        return isinstance(value, bool)\n");
        self.out.push_str("    if type_name == \"str\":\n");
        self.out.push_str("        return isinstance(value, str)\n");
        self.out.push_str("    if type_name == \"list\":\n");
        self.out
            .push_str("        return isinstance(value, list)\n");
        self.out.push_str("    if type_name == \"dict\":\n");
        self.out
            .push_str("        return isinstance(value, dict)\n");
        self.out.push_str("    if type_name == \"tuple\":\n");
        self.out
            .push_str("        return isinstance(value, tuple)\n");
        self.out.push_str("    if type_name == \"set\":\n");
        self.out.push_str("        return isinstance(value, set)\n");
        self.out.push_str("    return True\n\n\n");
        self.render_suite(suite, None);
        self.out.clone()
    }

    fn render_suite(&mut self, suite: &Suite, return_type: Option<&str>) {
        for stmt in suite {
            self.render_stmt(stmt, return_type);
        }
    }

    fn render_stmt(&mut self, stmt: &Stmt, return_type: Option<&str>) {
        for diagnostic in diagnostics_for_stmt(stmt) {
            self.push_line(
                self.indent_for_range(stmt.range()),
                &format!("# codonx: {diagnostic}"),
            );
        }

        match stmt {
            Stmt::FunctionDef(function) => self.render_function(function),
            Stmt::Return(ret) => {
                if let (Some(value), Some(type_name)) = (&ret.value, return_type) {
                    let indent = self.indent_for_range(stmt.range());
                    let expr = self.source_for(value.range()).trim();
                    self.push_line(indent, &format!("__codonx_ret = {expr}"));
                    self.push_line(
                        indent,
                        &format!(
                            "assert __codonx_guard(__codonx_ret, \"{type_name}\"), \"codonx return guard failed: {type_name}\""
                        ),
                    );
                    self.push_line(indent, "return __codonx_ret");
                } else {
                    self.push_source(stmt.range());
                }
            }
            Stmt::AnnAssign(assign) => {
                self.push_source(stmt.range());
                if let Some(type_name) = self.codon_type_name(&assign.annotation) {
                    let target = self.source_for(assign.target.range()).trim();
                    let indent = self.indent_for_range(stmt.range());
                    self.push_line(
                        indent,
                        &format!(
                            "assert __codonx_guard({target}, \"{type_name}\"), \"codonx assignment guard failed: {target}: {type_name}\""
                        ),
                    );
                }
            }
            _ => self.push_source(stmt.range()),
        }
    }

    fn render_function(&mut self, function: &StmtFunctionDef) {
        self.push_function_header(function);
        let body_indent = self.body_indent(function);
        self.render_parameter_asserts(&function.parameters, body_indent);
        let return_type = function
            .returns
            .as_deref()
            .and_then(|expr| self.codon_type_name(expr));
        self.render_suite(&function.body, return_type.as_deref());
        if function.body.is_empty() {
            self.push_line(body_indent, "pass");
        }
    }

    fn render_parameter_asserts(&mut self, parameters: &Parameters, indent: usize) {
        for param in parameters.iter() {
            let parameter = param.as_parameter();
            let Some(annotation) = parameter.annotation.as_deref() else {
                continue;
            };
            let Some(type_name) = self.codon_type_name(annotation) else {
                continue;
            };
            let name = parameter.name.as_str();
            if name == "self" || name == "cls" {
                continue;
            }
            self.push_line(
                indent,
                &format!(
                    "assert __codonx_guard({name}, \"{type_name}\"), \"codonx parameter guard failed: {name}: {type_name}\""
                ),
            );
        }
    }

    fn push_function_header(&mut self, function: &StmtFunctionDef) {
        let start = function.range.start().to_usize();
        let end = function
            .body
            .first()
            .map(|stmt| self.line_start_for_byte(stmt.range().start().to_usize()))
            .unwrap_or_else(|| function.range.end().to_usize());
        self.out
            .push_str(self.source[start..end].trim_end_matches('\n'));
        self.out.push('\n');
    }

    fn body_indent(&self, function: &StmtFunctionDef) -> usize {
        function
            .body
            .first()
            .map(|stmt| self.indent_for_range(stmt.range()))
            .unwrap_or_else(|| self.indent_for_range(function.range) + 4)
    }

    fn push_source(&mut self, range: TextRange) {
        self.out.push_str(&" ".repeat(self.indent_for_range(range)));
        self.out
            .push_str(self.source_for(range).trim_end_matches('\n'));
        self.out.push('\n');
    }

    fn push_line(&mut self, indent: usize, text: &str) {
        self.out.push_str(&" ".repeat(indent));
        self.out.push_str(text);
        self.out.push('\n');
    }

    fn source_for(&self, range: TextRange) -> &'a str {
        &self.source[range.start().to_usize()..range.end().to_usize()]
    }

    fn indent_for_range(&self, range: TextRange) -> usize {
        let byte = range.start().to_usize();
        let line_start = self.line_start_for_byte(byte);
        self.source[line_start..byte]
            .chars()
            .take_while(|ch| *ch == ' ' || *ch == '\t')
            .map(|ch| if ch == '\t' { 4 } else { 1 })
            .sum()
    }

    fn line_start_for_byte(&self, byte: usize) -> usize {
        match self.line_starts.binary_search(&byte) {
            Ok(idx) => self.line_starts[idx],
            Err(idx) => self.line_starts[idx.saturating_sub(1)],
        }
    }

    fn codon_type_name(&self, expr: &Expr) -> Option<String> {
        let text = self.source_for(expr.range()).trim();
        let base = text.split('[').next().unwrap_or(text).trim();
        match base {
            "int" => Some("int".to_string()),
            "float" => Some("float".to_string()),
            "bool" => Some("bool".to_string()),
            "str" => Some("str".to_string()),
            "list" => Some("list".to_string()),
            "dict" => Some("dict".to_string()),
            "tuple" => Some("tuple".to_string()),
            "set" => Some("set".to_string()),
            _ => None,
        }
    }
}
