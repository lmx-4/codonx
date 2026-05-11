use crate::emit::Target;
use crate::{error::CodonxError, source::SourceLine};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Directive {
    IfPy,
    IfCodon,
    IfDebugDeprecated,
    Else,
    Endif,
}

#[derive(Debug, Clone)]
struct Frame {
    parent_active: bool,
    if_branch_active: bool,
    in_else: bool,
    line: usize,
    directive: Directive,
}

pub fn parse_directive(line: &SourceLine) -> Option<Directive> {
    if line.in_triple_string {
        return None;
    }
    let t = line.trimmed.trim();
    match t {
        "#%ifpy" => Some(Directive::IfPy),
        "#%ifcodon" => Some(Directive::IfCodon),
        "#%ifdebug" => Some(Directive::IfDebugDeprecated),
        "#%else" => Some(Directive::Else),
        "#%endif" => Some(Directive::Endif),
        _ => None,
    }
}

fn directive_name(directive: Directive) -> &'static str {
    match directive {
        Directive::IfPy => "#%ifpy",
        Directive::IfCodon => "#%ifcodon",
        Directive::IfDebugDeprecated => "#%ifdebug",
        Directive::Else => "#%else",
        Directive::Endif => "#%endif",
    }
}

fn target_selects_if_branch(directive: Directive, target: Target) -> bool {
    match directive {
        Directive::IfPy | Directive::IfDebugDeprecated => matches!(target, Target::Py),
        Directive::IfCodon => matches!(target, Target::Codon),
        Directive::Else | Directive::Endif => unreachable!("not a branch-start directive"),
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
                Directive::IfPy | Directive::IfCodon | Directive::IfDebugDeprecated => {
                    let parent = current_active(&stack);
                    let if_active = target_selects_if_branch(directive, target);
                    stack.push(Frame {
                        parent_active: parent,
                        if_branch_active: if_active,
                        in_else: false,
                        line: line.no,
                        directive,
                    });
                }
                Directive::Else => {
                    let Some(top) = stack.last_mut() else {
                        return Err(CodonxError::Directive {
                            file: file.to_string(),
                            line: line.no,
                            message: "#%else without active conditional".to_string(),
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
                            message: "#%endif without active conditional".to_string(),
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
            message: format!(
                "unclosed conditional directive started by {}",
                directive_name(frame.directive)
            ),
        });
    }

    Ok(out)
}
