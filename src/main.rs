mod cli;
mod directive;
mod emit;
mod error;
mod guard;
mod report;
mod rewrite;
mod source;
mod type_parse;

use anyhow::{anyhow, bail, Context};
use clap::Parser;
use cli::{AssertArg, Cli, Command};
use directive::select_target_lines;
use emit::Target;
use report::Report;
use rewrite::rewrite_lines;
use source::read_source;
use std::{
    ffi::OsString,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
};

fn emit_to_string(
    input: &Path,
    target: Target,
    assert_mode: AssertArg,
    report: &mut Report,
) -> anyhow::Result<String> {
    let file = input.display().to_string();
    let lines = read_source(input).with_context(|| format!("failed to read {}", file))?;
    let selected = select_target_lines(&file, &lines, target)?;
    Ok(rewrite_lines(&file, &selected, target, assert_mode, report))
}

fn write_or_stdout(output: Option<PathBuf>, text: &str) -> anyhow::Result<()> {
    if let Some(path) = output {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(path, text)?;
    } else {
        print!("{}", text);
    }
    Ok(())
}

fn write_report(report: &Report, path: Option<PathBuf>) -> anyhow::Result<()> {
    if let Some(path) = path {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        report.write_json(&path)?;
    }
    Ok(())
}

fn default_dbg_output(input: &Path) -> PathBuf {
    sibling_with_suffix(input, "_dbg", "py")
}

fn default_codon_output(input: &Path) -> PathBuf {
    sibling_with_suffix(input, "_pre", "codon")
}

fn sibling_with_suffix(input: &Path, suffix: &str, ext: &str) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("codonx");
    let file_name = format!("{}{}.{}", stem, suffix, ext);
    input.with_file_name(file_name)
}

fn codon_bin(cli: &Cli) -> OsString {
    cli.codon_bin
        .as_ref()
        .map(|p| p.as_os_str().to_os_string())
        .or_else(|| std::env::var_os("CODONX_CODON_BIN"))
        .unwrap_or_else(|| OsString::from("codon"))
}

fn codon_option_takes_value(arg: &str) -> bool {
    matches!(
        arg,
        "-o" | "-module"
            | "-linker-flags"
            | "-disable-opt"
            | "-log"
            | "-march"
            | "-mcpu"
            | "--relocation-model"
    )
}

fn find_codon_input_arg(args: &[String]) -> Option<usize> {
    let mut skip_value = false;
    for (i, arg) in args.iter().enumerate() {
        if skip_value {
            skip_value = false;
            continue;
        }
        if arg == "--" {
            return args.get(i + 1).map(|_| i + 1);
        }
        if codon_option_takes_value(arg) {
            skip_value = true;
            continue;
        }
        if arg.starts_with("-o") && arg.len() > 2 {
            continue;
        }
        if arg.starts_with('-') {
            continue;
        }
        return Some(i);
    }
    None
}

fn has_output_arg(args: &[String]) -> bool {
    args.iter().any(|a| a == "-o" || a.starts_with("-o="))
}

fn build_output_for_original(input: &Path, args: &[String]) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("a.out");
    let ext = if args.iter().any(|a| a == "-lib") {
        Some("so")
    } else if args.iter().any(|a| a == "-obj" || a == "-pyext") {
        Some("o")
    } else if args.iter().any(|a| a == "-asm") {
        Some("s")
    } else if args.iter().any(|a| a == "-llvm") {
        Some("ll")
    } else {
        None
    };

    match ext {
        Some(ext) => input.with_file_name(format!("{}.{}", stem, ext)),
        None => input.with_file_name(stem),
    }
}

fn write_preprocessed_codon(input: &Path, output: &Path) -> anyhow::Result<()> {
    let mut rep = Report::default();
    let text = emit_to_string(input, Target::Codon, AssertArg::Off, &mut rep)?;
    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)?;
        }
    }
    fs::write(output, text)?;
    Ok(())
}

fn invoke_codon(
    cli: &Cli,
    subcommand: &str,
    original_args: &[String],
    keep_pre: bool,
) -> anyhow::Result<i32> {
    let input_idx = find_codon_input_arg(original_args)
        .ok_or_else(|| anyhow!("codonx {} requires a source file argument", subcommand))?;
    let input = PathBuf::from(&original_args[input_idx]);
    if input.as_os_str() == "-" {
        bail!("codonx cannot preprocess stdin input '-'");
    }

    let pre = default_codon_output(&input);
    write_preprocessed_codon(&input, &pre)?;

    let mut args = original_args.to_vec();
    args[input_idx] = pre.display().to_string();

    if subcommand == "build" && !has_output_arg(&args) {
        let output = build_output_for_original(&input, original_args);
        args.insert(input_idx, output.display().to_string());
        args.insert(input_idx, "-o".to_string());
    }

    let status = ProcessCommand::new(codon_bin(cli))
        .arg(subcommand)
        .args(&args)
        .status()
        .with_context(|| format!("failed to run codon {}", subcommand))?;

    if !keep_pre {
        let _ = fs::remove_file(&pre);
    }

    Ok(status.code().unwrap_or(1))
}

fn check_python_syntax(text: &str) -> anyhow::Result<()> {
    let tmp = std::env::temp_dir().join(format!("codonx_check_{}.py", std::process::id()));
    fs::write(&tmp, text)?;
    let status = ProcessCommand::new("python3")
        .arg("-m")
        .arg("py_compile")
        .arg(&tmp)
        .status();
    let _ = fs::remove_file(&tmp);
    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => Err(anyhow!("python3 -m py_compile failed with status {}", s)),
        Err(e) => Err(anyhow!("failed to run python3 for syntax check: {}", e)),
    }
}

fn run_dbg(cli: &Cli, input: PathBuf) -> anyhow::Result<()> {
    let mut rep = Report::default();
    let text = emit_to_string(&input, Target::Py, cli.assert, &mut rep)?;
    let output = cli
        .output
        .clone()
        .unwrap_or_else(|| default_dbg_output(&input));
    write_or_stdout(Some(output), &text)?;
    write_report(&rep, cli.report.clone())?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.dbg {
        let input = cli
            .input
            .clone()
            .ok_or_else(|| anyhow!("codonx --dbg requires an input file"))?;
        run_dbg(&cli, input)?;
        return Ok(());
    }

    match &cli.command {
        Some(Command::Codon { input, output }) => {
            let output = output
                .clone()
                .or_else(|| cli.output.clone())
                .unwrap_or_else(|| default_codon_output(input));
            write_preprocessed_codon(input, &output)?;
        }
        Some(Command::Run { args }) => {
            let code = invoke_codon(&cli, "run", args, cli.keep_pre)?;
            std::process::exit(code);
        }
        Some(Command::Build { args }) => {
            let code = invoke_codon(&cli, "build", args, cli.keep_pre)?;
            std::process::exit(code);
        }
        Some(Command::Check { input, assert }) => {
            let mut rep_py = Report::default();
            let py = emit_to_string(input, Target::Py, *assert, &mut rep_py)?;
            check_python_syntax(&py)?;

            let mut rep_codon = Report::default();
            let _codon = emit_to_string(input, Target::Codon, AssertArg::Off, &mut rep_codon)?;

            let mut stderr = std::io::stderr();
            writeln!(stderr, "[codonx check] ok: {}", input.display())?;
            writeln!(
                stderr,
                "[codonx check] py warnings: {}",
                rep_py.warnings.len()
            )?;
            writeln!(
                stderr,
                "[codonx check] codon warnings: {}",
                rep_codon.warnings.len()
            )?;
        }
        None => bail!("expected --dbg or a subcommand: codon, run, build, check"),
    }

    Ok(())
}
