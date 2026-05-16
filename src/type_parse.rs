use regex::Regex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamAnn {
    pub name: String,
    pub ty: String,
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
