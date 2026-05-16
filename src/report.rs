use serde::Serialize;
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize)]
pub struct Warning {
    pub file: String,
    pub line: usize,
    pub kind: String,
    pub message: String,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Report {
    pub warnings: Vec<Warning>,
    pub rewritten_imports: usize,
    pub removed_parallel_annotations: usize,
    pub inserted_guards: usize,
    pub unknown_guard_types: usize,
    pub unchecked_dynamic_types: usize,
    pub semantic_warnings: usize,
    pub lowered_casts: usize,
    pub erased_generics: usize,
    pub interop_warnings: usize,
    pub unsupported_rewrite_boundaries: usize,
}

impl Report {
    pub fn warn(&mut self, file: &str, line: usize, kind: &str, message: impl Into<String>) {
        self.warnings.push(Warning {
            file: file.to_string(),
            line,
            kind: kind.to_string(),
            message: message.into(),
        });
    }

    pub fn warn_unknown_guard_type(&mut self, file: &str, line: usize, ty: &str) {
        self.unknown_guard_types += 1;
        self.warn(
            file,
            line,
            "unknown-guard-type",
            format!(
                "unknown guard type `{}` is soft-passed in Python debug target",
                ty
            ),
        );
    }

    pub fn warn_unchecked_dynamic_type(&mut self, file: &str, line: usize, ty: &str) {
        self.unchecked_dynamic_types += 1;
        self.warn(
            file,
            line,
            "unchecked-dynamic-type",
            format!(
                "dynamic guard type `{}` is not checked in Python debug target",
                ty
            ),
        );
    }

    pub fn warn_semantic(
        &mut self,
        file: &str,
        line: usize,
        kind: &str,
        message: impl Into<String>,
    ) {
        self.semantic_warnings += 1;
        self.warn(file, line, kind, message);
    }

    pub fn warn_interop(
        &mut self,
        file: &str,
        line: usize,
        kind: &str,
        message: impl Into<String>,
    ) {
        self.interop_warnings += 1;
        self.warn(file, line, kind, message);
    }

    pub fn warn_unsupported_rewrite_boundary(
        &mut self,
        file: &str,
        line: usize,
        kind: &str,
        message: impl Into<String>,
    ) {
        self.unsupported_rewrite_boundaries += 1;
        self.warn(file, line, kind, message);
    }

    pub fn write_json(&self, path: &Path) -> anyhow::Result<()> {
        let text = serde_json::to_string_pretty(self)?;
        fs::write(path, text)?;
        Ok(())
    }
}
