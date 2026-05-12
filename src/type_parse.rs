use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamAnn {
    pub name: String,
    pub ty: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefSig {
    pub indent: usize,
    pub params: Vec<ParamAnn>,
    pub ret: Option<String>,
}

pub fn translate_annotations_in_line(line: &str) -> String {
    let mut out = line.to_string();
    for (from, to) in [
        ("List", "list"),
        ("Dict", "dict"),
        ("Set", "set"),
        ("Tuple", "tuple"),
    ] {
        let re = Regex::new(&format!(r"\b{}\b", from)).unwrap();
        out = re.replace_all(&out, to).to_string();
    }
    for codon_ty in ["i8", "u8", "i16", "u16", "i32", "u32", "i64", "u64"] {
        let re = Regex::new(&format!(r"\b{}\b", regex::escape(codon_ty))).unwrap();
        out = re.replace_all(&out, "int").to_string();
    }
    out = Regex::new(r"\bU?Int\[\s*\d+\s*\]")
        .unwrap()
        .replace_all(&out, "int")
        .to_string();
    out = Regex::new(r"\bbyte\b")
        .unwrap()
        .replace_all(&out, "int")
        .to_string();
    out = Regex::new(r"\bf32\b")
        .unwrap()
        .replace_all(&out, "float")
        .to_string();
    out = Regex::new(r"\bfloat32\b")
        .unwrap()
        .replace_all(&out, "float")
        .to_string();
    out = Regex::new(r"\b(?:float16|bfloat16|float128)\b")
        .unwrap()
        .replace_all(&out, "float")
        .to_string();
    out = Regex::new(r"\bf64\b")
        .unwrap()
        .replace_all(&out, "float")
        .to_string();
    out = Regex::new(r"\bOptional\[[^\]]+\]")
        .unwrap()
        .replace_all(&out, "object")
        .to_string();
    out = Regex::new(r"\bUnion\[[^\]]+\]")
        .unwrap()
        .replace_all(&out, "object")
        .to_string();
    out = Regex::new(r"\bLiteral\[\s*int\s*\]")
        .unwrap()
        .replace_all(&out, "int")
        .to_string();
    out = Regex::new(r"\bLiteral\[\s*str\s*\]")
        .unwrap()
        .replace_all(&out, "str")
        .to_string();
    out = Regex::new(r"\bLiteral\[\s*bool\s*\]")
        .unwrap()
        .replace_all(&out, "bool")
        .to_string();
    out = Regex::new(r"\bNoneType\b")
        .unwrap()
        .replace_all(&out, "object")
        .to_string();
    out = Regex::new(r"\bPtr\[[^\]]+\]")
        .unwrap()
        .replace_all(&out, "object")
        .to_string();
    out = Regex::new(r"\bcobj\b")
        .unwrap()
        .replace_all(&out, "object")
        .to_string();
    out = Regex::new(r"\bndarray\[[^\]]+\]")
        .unwrap()
        .replace_all(&out, "object")
        .to_string();
    out
}

pub fn split_top_level_commas(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0_usize;
    for (i, c) in s.char_indices() {
        match c {
            '[' | '(' | '{' => depth += 1,
            ']' | ')' | '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(s[start..i].trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
    }
    let last = s[start..].trim();
    if !last.is_empty() {
        out.push(last.to_string());
    }
    out
}

pub fn parse_def_signature(line: &str, indent: usize) -> Option<DefSig> {
    let re = Regex::new(
        r"^\s*def\s+[A-Za-z_][A-Za-z0-9_]*(?:\[[^\]]+\])?\s*\((?P<params>.*)\)\s*(?:->\s*(?P<ret>[^:]+))?:",
    )
    .ok()?;
    let caps = re.captures(line)?;
    let params_src = caps.name("params")?.as_str();
    let mut params = Vec::new();

    for p in split_top_level_commas(params_src) {
        let p = p.trim();
        if p.is_empty() || p == "self" || p == "cls" || p.starts_with('*') {
            continue;
        }
        let Some((name_part, ty_part)) = p.split_once(':') else {
            continue;
        };
        let name = name_part.trim().to_string();
        let ty = ty_part
            .split('=')
            .next()
            .unwrap_or(ty_part)
            .trim()
            .to_string();
        if ty == "type" {
            continue;
        }
        if !name.is_empty() && !ty.is_empty() {
            params.push(ParamAnn { name, ty });
        }
    }

    let ret = caps
        .name("ret")
        .map(|m| m.as_str().trim().to_string())
        .filter(|s| !s.is_empty());
    Some(DefSig {
        indent,
        params,
        ret,
    })
}

pub fn parse_ann_assign(line: &str) -> Option<(String, String)> {
    let re = Regex::new(r"^\s*(?P<name>[A-Za-z_][A-Za-z0-9_]*)\s*:\s*(?P<ty>[^=#]+?)\s*=").ok()?;
    let caps = re.captures(line)?;
    let name = caps.name("name")?.as_str().to_string();
    let ty = caps.name("ty")?.as_str().trim().to_string();
    Some((name, ty))
}
