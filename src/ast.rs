use crate::type_parse::{split_top_level_commas, ParamAnn};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeAnn {
    pub span: Span,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefParam {
    pub name: String,
    pub ty: Option<TypeAnn>,
    pub is_type_param: bool,
    pub source_span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefSig {
    pub indent: usize,
    pub params: Vec<DefParam>,
    pub ret: Option<TypeAnn>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnAssign {
    pub name: String,
    pub ty: TypeAnn,
    pub has_value: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassSig {
    pub type_params: Vec<DefParam>,
    pub static_base: Option<TypeAnn>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassRewrite {
    pub line: String,
    pub erased_type_params: usize,
    pub lowered_static_inheritance: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FromPythonImport {
    Module { replacement: String },
    Typed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchKind {
    Replace(String),
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Patch {
    pub span: Span,
    pub kind: PatchKind,
}

pub fn parse_def_signature_line(line: &str, indent: usize) -> Option<DefSig> {
    let def_pos = line.find("def ")?;
    if !line[..def_pos].trim().is_empty() {
        return None;
    }

    let mut i = def_pos + 4;
    let name_start = i;
    i = take_ident(line, i)?;
    if i == name_start {
        return None;
    }

    i = skip_ws(line, i);
    if line.as_bytes().get(i) == Some(&b'[') {
        i = find_matching(line, i, '[', ']')? + 1;
        i = skip_ws(line, i);
    }

    if line.as_bytes().get(i) != Some(&b'(') {
        return None;
    }
    let open = i;
    let close = find_matching(line, open, '(', ')')?;
    let after_close = skip_ws(line, close + 1);
    let (ret, colon) = if line[after_close..].starts_with("->") {
        let ret_start = skip_ws(line, after_close + 2);
        let colon = find_top_level_char(line, ret_start, ':')?;
        let ret_end = trim_end_index(line, ret_start, colon);
        let ret = if ret_start < ret_end {
            Some(TypeAnn {
                span: Span {
                    start: ret_start,
                    end: ret_end,
                },
                text: line[ret_start..ret_end].to_string(),
            })
        } else {
            None
        };
        (ret, colon)
    } else {
        (None, after_close)
    };
    if line.as_bytes().get(colon) != Some(&b':') {
        return None;
    }

    let params = parse_params(line, open + 1, close);
    Some(DefSig {
        indent,
        params,
        ret,
    })
}

pub fn def_guard_signature(sig: &DefSig) -> (Vec<ParamAnn>, Option<String>) {
    let params = sig
        .params
        .iter()
        .filter_map(|p| {
            if p.name == "self" || p.name == "cls" || p.name.starts_with('*') || p.is_type_param {
                return None;
            }
            let ty = p.ty.as_ref()?;
            Some(ParamAnn {
                name: p.name.clone(),
                ty: ty.text.clone(),
            })
        })
        .collect();
    let ret = sig.ret.as_ref().map(|r| r.text.clone());
    (params, ret)
}

pub fn rewrite_def_signature_for_python(line: &str) -> Option<String> {
    let sig = parse_def_signature_line(line, leading_indent(line))?;
    let mut patches = Vec::new();

    for param in &sig.params {
        if param.is_type_param {
            patches.push(Patch {
                span: param.source_span,
                kind: PatchKind::Delete,
            });
            continue;
        }
        let Some(ty) = &param.ty else {
            continue;
        };
        let lowered = lower_type_annotation(&ty.text);
        if lowered != ty.text {
            patches.push(Patch {
                span: ty.span,
                kind: PatchKind::Replace(lowered),
            });
        }
    }

    if let Some(ret) = &sig.ret {
        let lowered = lower_type_annotation(&ret.text);
        if lowered != ret.text {
            patches.push(Patch {
                span: ret.span,
                kind: PatchKind::Replace(lowered),
            });
        }
    }

    if patches.is_empty() {
        return Some(line.to_string());
    }
    apply_patches(line, &patches)
}

pub fn parse_ann_assign_line(line: &str) -> Option<AnnAssign> {
    let start = leading_indent_bytes(line);
    let name_end = take_ident(line, start)?;
    let name = line[start..name_end].to_string();
    let after_name = skip_ws(line, name_end);
    if line.as_bytes().get(after_name) != Some(&b':') {
        return None;
    }

    let ty_start = skip_ws(line, after_name + 1);
    let eq = find_top_level_char(line, ty_start, '=');
    let comment = find_top_level_char(line, ty_start, '#');
    let value_or_comment = match (eq, comment) {
        (Some(eq), Some(comment)) => Some(eq.min(comment)),
        (Some(eq), None) => Some(eq),
        (None, Some(comment)) => Some(comment),
        (None, None) => None,
    };
    let ty_end = trim_end_index(line, ty_start, value_or_comment.unwrap_or(line.len()));
    if ty_start >= ty_end {
        return None;
    }

    Some(AnnAssign {
        name,
        ty: TypeAnn {
            span: Span {
                start: ty_start,
                end: ty_end,
            },
            text: line[ty_start..ty_end].to_string(),
        },
        has_value: eq.is_some(),
    })
}

pub fn rewrite_ann_assign_for_python(line: &str) -> Option<String> {
    let ann = parse_ann_assign_line(line)?;
    let lowered = lower_type_annotation(&ann.ty.text);
    if lowered == ann.ty.text {
        return Some(line.to_string());
    }
    apply_patches(
        line,
        &[Patch {
            span: ann.ty.span,
            kind: PatchKind::Replace(lowered),
        }],
    )
}

pub fn parse_class_signature_line(line: &str) -> Option<ClassSig> {
    let class_pos = line.find("class ")?;
    if !line[..class_pos].trim().is_empty() {
        return None;
    }

    let mut i = class_pos + 6;
    let name_start = i;
    i = take_ident(line, i)?;
    if i == name_start {
        return None;
    }
    i = skip_ws(line, i);

    let mut type_params = Vec::new();
    if line.as_bytes().get(i) == Some(&b'[') {
        let close = find_matching(line, i, '[', ']')?;
        type_params = parse_params(line, i + 1, close);
        i = skip_ws(line, close + 1);
    }

    let mut static_base = None;
    if line.as_bytes().get(i) == Some(&b'(') {
        let close = find_matching(line, i, '(', ')')?;
        let bases_start = i + 1;
        for (rel_start, rel_end) in split_top_level_ranges(&line[bases_start..close]) {
            let raw_start = bases_start + rel_start;
            let raw_end = bases_start + rel_end;
            let trim_start = raw_start + line[raw_start..raw_end].len()
                - line[raw_start..raw_end].trim_start().len();
            let trim_end = raw_end
                - (line[raw_start..raw_end].len() - line[raw_start..raw_end].trim_end().len());
            let base = &line[trim_start..trim_end];
            if base.starts_with("Static[") && base.ends_with(']') {
                let inner_start = trim_start + "Static[".len();
                let inner_end = trim_end - 1;
                if inner_start < inner_end {
                    static_base = Some(TypeAnn {
                        span: Span {
                            start: trim_start,
                            end: trim_end,
                        },
                        text: line[inner_start..inner_end].to_string(),
                    });
                }
            }
        }
        i = skip_ws(line, close + 1);
    }

    if line.as_bytes().get(i) != Some(&b':') {
        return None;
    }

    Some(ClassSig {
        type_params,
        static_base,
    })
}

pub fn rewrite_class_signature_for_python(line: &str) -> Option<ClassRewrite> {
    let sig = parse_class_signature_line(line)?;
    let mut patches = Vec::new();
    let mut erased_type_params = 0;

    for param in &sig.type_params {
        let Some(ty) = &param.ty else {
            continue;
        };
        if ty.text.trim() == "type" {
            patches.push(Patch {
                span: Span {
                    start: param.name.len() + param.source_span.start,
                    end: param.source_span.end,
                },
                kind: PatchKind::Delete,
            });
            erased_type_params += 1;
        }
    }

    let lowered_static_inheritance = sig.static_base.is_some();
    if let Some(base) = &sig.static_base {
        let replacement = if base.text.trim() == "object" {
            String::new()
        } else {
            base.text.clone()
        };
        patches.push(Patch {
            span: base.span,
            kind: PatchKind::Replace(replacement),
        });
    }

    let mut line = if patches.is_empty() {
        line.to_string()
    } else {
        apply_patches(line, &patches)?
    };
    line = clean_empty_class_bases(&line);

    Some(ClassRewrite {
        line,
        erased_type_params,
        lowered_static_inheritance,
    })
}

pub fn parse_from_python_import_line(line: &str) -> Option<FromPythonImport> {
    let trimmed = line.trim_start();
    let indent_len = line.len() - trimmed.len();
    let rest = trimmed.strip_prefix("from python import ")?;
    if rest.trim().is_empty() {
        return None;
    }
    if rest.contains("->") || rest.contains('(') || rest.contains(')') {
        return Some(FromPythonImport::Typed);
    }
    Some(FromPythonImport::Module {
        replacement: format!("{}import {}", &line[..indent_len], rest),
    })
}

fn parse_params(line: &str, start: usize, end: usize) -> Vec<DefParam> {
    split_top_level_ranges(&line[start..end])
        .into_iter()
        .filter_map(|(rel_start, rel_end)| {
            let raw_start = start + rel_start;
            let raw_end = start + rel_end;
            let trim_start = raw_start + line[raw_start..raw_end].len()
                - line[raw_start..raw_end].trim_start().len();
            let trim_end = raw_end
                - (line[raw_start..raw_end].len() - line[raw_start..raw_end].trim_end().len());
            if trim_start >= trim_end {
                return None;
            }
            parse_param(line, trim_start, trim_end)
        })
        .collect()
}

fn parse_param(line: &str, start: usize, end: usize) -> Option<DefParam> {
    let text = &line[start..end];
    let colon_rel = find_top_level_char(text, 0, ':');
    let eq_rel = find_top_level_char(text, 0, '=');
    let name_end_rel = colon_rel.or(eq_rel).unwrap_or(text.len());
    let name = text[..name_end_rel].trim().to_string();

    let ty = colon_rel.and_then(|colon| {
        let ty_start =
            start + colon + 1 + text[colon + 1..].len() - text[colon + 1..].trim_start().len();
        let default_rel = find_top_level_char(text, colon + 1, '=').unwrap_or(text.len());
        let ty_end = start + trim_end_index(text, colon + 1, default_rel);
        if ty_start >= ty_end {
            return None;
        }
        Some(TypeAnn {
            span: Span {
                start: ty_start,
                end: ty_end,
            },
            text: line[ty_start..ty_end].to_string(),
        })
    });
    let is_type_param = ty.as_ref().is_some_and(|t| t.text.trim() == "type");

    Some(DefParam {
        name,
        ty,
        is_type_param,
        source_span: Span { start, end },
    })
}

pub fn lower_type_annotation(ty: &str) -> String {
    let trimmed = ty.trim();
    if matches!(
        trimmed,
        "byte" | "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64"
    ) || is_sized_int(trimmed)
    {
        return "int".to_string();
    }
    if matches!(
        trimmed,
        "f32" | "float32" | "f64" | "float16" | "bfloat16" | "float128"
    ) {
        return "float".to_string();
    }
    if matches!(trimmed, "NoneType" | "cobj")
        || looks_like_generic(trimmed, "Optional")
        || looks_like_generic(trimmed, "Union")
        || looks_like_generic(trimmed, "Ptr")
        || looks_like_generic(trimmed, "ndarray")
    {
        return "object".to_string();
    }
    if looks_like_literal_builtin(trimmed, "int") {
        return "int".to_string();
    }
    if looks_like_literal_builtin(trimmed, "str") {
        return "str".to_string();
    }
    if looks_like_literal_builtin(trimmed, "bool") {
        return "bool".to_string();
    }

    lower_type_tokens(ty)
}

fn lower_type_tokens(ty: &str) -> String {
    let mut out = String::with_capacity(ty.len());
    let mut i = 0;
    while i < ty.len() {
        let Some(ch) = ty[i..].chars().next() else {
            break;
        };
        if is_ident_start(ch) {
            let start = i;
            i += ch.len_utf8();
            while i < ty.len() {
                let Some(next) = ty[i..].chars().next() else {
                    break;
                };
                if !is_ident_continue(next) {
                    break;
                }
                i += next.len_utf8();
            }
            let token = &ty[start..i];
            if matches!(token, "Int" | "UInt") {
                if let Some(end) = consume_sized_int_suffix(ty, i) {
                    out.push_str("int");
                    i = end;
                    continue;
                }
            }
            out.push_str(match token {
                "List" => "list",
                "Dict" => "dict",
                "Set" => "set",
                "Tuple" => "tuple",
                "byte" | "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" => "int",
                "f32" | "float32" | "f64" | "float16" | "bfloat16" | "float128" => "float",
                "NoneType" | "cobj" => "object",
                _ => token,
            });
        } else {
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

fn apply_patches(line: &str, patches: &[Patch]) -> Option<String> {
    let mut patches = patches.to_vec();
    patches.sort_by_key(|p| (p.span.start, p.span.end));
    let mut out = String::with_capacity(line.len());
    let mut pos = 0;
    let mut previous: Option<Span> = None;
    for patch in patches {
        if patch.span.start < pos || patch.span.end > line.len() {
            return None;
        }
        if previous.is_some_and(|prev| patch.span.start < prev.end) {
            return None;
        }
        out.push_str(&line[pos..patch.span.start]);
        match patch.kind {
            PatchKind::Replace(text) => out.push_str(&text),
            PatchKind::Delete => {}
        }
        pos = patch.span.end;
        previous = Some(patch.span);
    }
    out.push_str(&line[pos..]);
    if line.contains('\n') {
        Some(out)
    } else {
        Some(clean_param_commas(&out))
    }
}

fn clean_param_commas(line: &str) -> String {
    let Some(open) = line.find('(') else {
        return line.to_string();
    };
    let Some(close) = find_matching(line, open, '(', ')') else {
        return line.to_string();
    };
    let params = &line[open + 1..close];
    let kept = split_top_level_commas(params)
        .into_iter()
        .filter(|param| !param.trim().is_empty())
        .collect::<Vec<_>>();
    format!(
        "{}({}){}",
        &line[..open],
        kept.join(", "),
        &line[close + 1..]
    )
}

fn clean_empty_class_bases(line: &str) -> String {
    let Some(class_pos) = line.find("class ") else {
        return line.to_string();
    };
    if !line[..class_pos].trim().is_empty() {
        return line.to_string();
    }

    let mut i = class_pos + 6;
    let Some(name_end) = take_ident(line, i) else {
        return line.to_string();
    };
    i = skip_ws(line, name_end);
    if line.as_bytes().get(i) == Some(&b'[') {
        let Some(type_params_end) = find_matching(line, i, '[', ']') else {
            return line.to_string();
        };
        i = skip_ws(line, type_params_end + 1);
    }
    if line.as_bytes().get(i) != Some(&b'(') {
        return line.to_string();
    }
    let Some(close) = find_matching(line, i, '(', ')') else {
        return line.to_string();
    };
    if !line[i + 1..close].trim().is_empty() {
        return line.to_string();
    }
    let after_close = skip_ws(line, close + 1);
    if line.as_bytes().get(after_close) != Some(&b':') {
        return line.to_string();
    }

    format!("{}{}", &line[..i], &line[after_close..])
}

fn split_top_level_ranges(s: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0_usize;
    for (i, c) in s.char_indices() {
        match c {
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push((start, i));
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push((start, s.len()));
    out
}

fn find_matching(s: &str, open: usize, open_ch: char, close_ch: char) -> Option<usize> {
    let mut depth = 0_i32;
    for (idx, ch) in s.char_indices().skip_while(|(idx, _)| *idx < open) {
        match ch {
            c if c == open_ch => depth += 1,
            c if c == close_ch => {
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

fn find_top_level_char(s: &str, start: usize, target: char) -> Option<usize> {
    let mut depth = 0_i32;
    for (idx, ch) in s.char_indices().skip_while(|(idx, _)| *idx < start) {
        match ch {
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' => depth -= 1,
            c if c == target && depth == 0 => return Some(idx),
            _ => {}
        }
    }
    None
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

fn skip_ws(s: &str, mut i: usize) -> usize {
    while i < s.len() && s.as_bytes()[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn trim_end_index(s: &str, start: usize, end: usize) -> usize {
    end - (s[start..end].len() - s[start..end].trim_end().len())
}

fn leading_indent(s: &str) -> usize {
    s.chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum()
}

fn leading_indent_bytes(s: &str) -> usize {
    s.char_indices()
        .find_map(|(idx, ch)| {
            if ch == ' ' || ch == '\t' {
                None
            } else {
                Some(idx)
            }
        })
        .unwrap_or(s.len())
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric()
}

fn is_sized_int(ty: &str) -> bool {
    let Some(bits) = ty.strip_prefix("Int[").and_then(|s| s.strip_suffix(']')) else {
        return ty
            .strip_prefix("UInt[")
            .and_then(|s| s.strip_suffix(']'))
            .is_some_and(|bits| bits.trim().parse::<u32>().is_ok());
    };
    bits.trim().parse::<u32>().is_ok()
}

fn consume_sized_int_suffix(s: &str, start: usize) -> Option<usize> {
    let rest = s.get(start..)?;
    let close_rel = rest.strip_prefix('[')?.find(']')?;
    let close = close_rel + 1;
    let inside = &rest[1..close];
    if !inside.trim().parse::<u32>().is_ok() {
        return None;
    }
    Some(start + close + 1)
}

fn looks_like_generic(ty: &str, name: &str) -> bool {
    ty.strip_prefix(name)
        .and_then(|rest| rest.strip_prefix('['))
        .is_some_and(|rest| rest.ends_with(']'))
}

fn looks_like_literal_builtin(ty: &str, name: &str) -> bool {
    let Some(inner) = ty
        .strip_prefix("Literal[")
        .and_then(|rest| rest.strip_suffix(']'))
    else {
        return false;
    };
    inner.trim() == name
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_function_signature_without_regex() {
        let sig =
            parse_def_signature_line("def f[T](x: list[i32], y: Dict[str, i64] = {}) -> i32:", 0)
                .unwrap();
        let (params, ret) = def_guard_signature(&sig);
        assert_eq!(params[0].name, "x");
        assert_eq!(params[0].ty, "list[i32]");
        assert_eq!(params[1].ty, "Dict[str, i64]");
        assert_eq!(ret.as_deref(), Some("i32"));
    }

    #[test]
    fn rewrites_function_signature_with_spans() {
        let out = rewrite_def_signature_for_python(
            "def f(x: List[i32], T: type, y: Optional[str] = None) -> i64:",
        )
        .unwrap();
        assert_eq!("def f(x: list[int], y: object = None) -> int:", out);
    }

    #[test]
    fn rewrites_annotation_assignment_with_spans() {
        let ann = parse_ann_assign_line("    total: Dict[str, UInt[16]] = {}").unwrap();
        assert_eq!(ann.name, "total");
        assert_eq!(ann.ty.text, "Dict[str, UInt[16]]");
        assert!(ann.has_value);
        let out = rewrite_ann_assign_for_python("    total: Dict[str, UInt[16]] = {}").unwrap();
        assert_eq!("    total: dict[str, int] = {}", out);
    }

    #[test]
    fn rewrites_simple_annotation_declaration_with_spans() {
        let ann = parse_ann_assign_line("value: Optional[i32]").unwrap();
        assert_eq!(ann.name, "value");
        assert!(!ann.has_value);
        let out = rewrite_ann_assign_for_python("value: Optional[i32]").unwrap();
        assert_eq!("value: object", out);
    }

    #[test]
    fn rewrites_class_signature_with_type_params_and_static_base() {
        let rewritten = rewrite_class_signature_for_python("class Box[T: type](Static[Base]):")
            .expect("class signature");
        assert_eq!("class Box[T](Base):", rewritten.line);
        assert_eq!(1, rewritten.erased_type_params);
        assert!(rewritten.lowered_static_inheritance);
    }

    #[test]
    fn drops_static_object_base_for_python_generic_classes() {
        let rewritten = rewrite_class_signature_for_python("class Box[T: type](Static[object]):")
            .expect("class signature");
        assert_eq!("class Box[T]:", rewritten.line);
        assert_eq!(1, rewritten.erased_type_params);
        assert!(rewritten.lowered_static_inheritance);
    }

    #[test]
    fn parses_from_python_import_statement() {
        let import = parse_from_python_import_line("    from python import numpy as np").unwrap();
        assert_eq!(
            FromPythonImport::Module {
                replacement: "    import numpy as np".to_string()
            },
            import
        );
        assert_eq!(
            Some(FromPythonImport::Typed),
            parse_from_python_import_line("from python import numpy.array(pyobj) -> pyobj")
        );
    }
}
