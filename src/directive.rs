use crate::emit::Target;
use crate::{error::CodonxError, source::SourceLine};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Directive {
    IfDebug,
    Else,
    Endif,
}

#[derive(Debug, Clone)]
struct Frame {
    parent_active: bool,
    if_branch_active: bool,
    in_else: bool,
    line: usize,
}

pub fn parse_directive(line: &SourceLine) -> Option<Directive> {
    if line.in_triple_string {
        return None;
    }
    let t = line.trimmed.trim();
    match t {
        "#%ifdebug" => Some(Directive::IfDebug),
        "#%else" => Some(Directive::Else),
        "#%endif" => Some(Directive::Endif),
        _ => None,
    }
}

fn current_active(stack: &[Frame]) -> bool {
    stack
        .last()
        .map(|f| {
            if f.in_else {
                f.parent_active && !f.if_branch_active
            } else {
                f.parent_active && f.if_branch_active
            }
        })
        .unwrap_or(true)
}

pub fn select_target_lines(
    file: &str,
    lines: &[SourceLine],
    target: Target,
) -> Result<Vec<SourceLine>, CodonxError> {
    let mut out = Vec::new();
    let mut stack: Vec<Frame> = Vec::new();

    for line in lines {
        if let Some(directive) = parse_directive(line) {
            match directive {
                Directive::IfDebug => {
                    let parent = current_active(&stack);
                    let if_active = matches!(target, Target::Py);
                    stack.push(Frame {
                        parent_active: parent,
                        if_branch_active: if_active,
                        in_else: false,
                        line: line.no,
                    });
                }
                Directive::Else => {
                    let Some(top) = stack.last_mut() else {
                        return Err(CodonxError::Directive {
                            file: file.to_string(),
                            line: line.no,
                            message: "#%else without #%ifdebug".to_string(),
                        });
                    };
                    if top.in_else {
                        return Err(CodonxError::Directive {
                            file: file.to_string(),
                            line: line.no,
                            message: "duplicate #%else".to_string(),
                        });
                    }
                    top.in_else = true;
                }
                Directive::Endif => {
                    if stack.pop().is_none() {
                        return Err(CodonxError::Directive {
                            file: file.to_string(),
                            line: line.no,
                            message: "#%endif without #%ifdebug".to_string(),
                        });
                    }
                }
            }
            continue;
        }

        if current_active(&stack) {
            out.push(line.clone());
        }
    }

    if let Some(frame) = stack.last() {
        return Err(CodonxError::Directive {
            file: file.to_string(),
            line: frame.line,
            message: "unclosed #%ifdebug".to_string(),
        });
    }

    Ok(out)
}
