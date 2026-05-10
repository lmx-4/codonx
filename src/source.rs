use std::{fs, path::Path};

#[derive(Debug, Clone)]
pub struct SourceLine {
    pub no: usize,
    pub raw: String,
    pub indent: usize,
    pub trimmed: String,
    pub in_triple_string: bool,
}

fn count_indent(s: &str) -> usize {
    s.chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .map(|c| if c == '\t' { 4 } else { 1 })
        .sum()
}

fn count_occurrences(line: &str, needle: &str) -> usize {
    let mut count = 0;
    let mut rest = line;
    while let Some(pos) = rest.find(needle) {
        count += 1;
        rest = &rest[pos + needle.len()..];
    }
    count
}

pub fn read_source(path: &Path) -> std::io::Result<Vec<SourceLine>> {
    let text = fs::read_to_string(path)?;
    let mut lines = Vec::new();
    let mut in_triple = false;
    let mut triple_delim = "".to_string();

    for (idx, raw_line) in text.lines().enumerate() {
        let raw = raw_line.to_string();
        let trimmed = raw.trim_start().to_string();
        let indent = count_indent(&raw);
        let line_started_in_triple = in_triple;

        if in_triple {
            if count_occurrences(&raw, &triple_delim) % 2 == 1 {
                in_triple = false;
                triple_delim.clear();
            }
        } else {
            let dq = count_occurrences(&raw, "\"\"\"");
            let sq = count_occurrences(&raw, "'''");
            if dq % 2 == 1 {
                in_triple = true;
                triple_delim = "\"\"\"".to_string();
            } else if sq % 2 == 1 {
                in_triple = true;
                triple_delim = "'''".to_string();
            }
        }

        lines.push(SourceLine {
            no: idx + 1,
            raw,
            indent,
            trimmed,
            in_triple_string: line_started_in_triple,
        });
    }

    Ok(lines)
}
