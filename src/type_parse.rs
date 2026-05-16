#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamAnn {
    pub name: String,
    pub ty: String,
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
