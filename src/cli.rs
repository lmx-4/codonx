use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AssertArg {
    Off,
    Shallow,
    Full,
}

#[derive(Debug, Parser)]
#[command(name = "codonx")]
#[command(version)]
#[command(about = "Codon-first preprocessor that wraps Python debug output and the Codon CLI.")]
pub struct Cli {
    /// Generate a Python debug file from the input source.
    #[arg(long)]
    pub dbg: bool,

    /// Output path for --dbg or codon generation.
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Assertion mode for Python debug output.
    #[arg(long, value_enum, default_value_t = AssertArg::Shallow)]
    pub assert: AssertArg,

    /// Optional JSON warning report path.
    #[arg(long)]
    pub report: Option<PathBuf>,

    /// Keep the preprocessed Codon file used by run/build.
    #[arg(long)]
    pub keep_pre: bool,

    /// Explicit path to the Codon compiler. Defaults to CODONX_CODON_BIN, then codon.
    #[arg(long)]
    pub codon_bin: Option<PathBuf>,

    /// Input file for --dbg mode.
    pub input: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate a pure Codon file and exit.
    Codon {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Preprocess, then invoke `codon run` with matching arguments.
    Run {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Preprocess, then invoke `codon build` with matching arguments.
    Build {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Check directive structure and Python syntax.
    Check {
        input: PathBuf,
        #[arg(long, value_enum, default_value_t = AssertArg::Shallow)]
        assert: AssertArg,
    },

    /// Parse Python 3.12 source with Ruff and emit a CodonX debug JSON dump.
    Ir {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Parse Python 3.12 source with Ruff and emit Python semantic assert IR.
    AssertIr {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Parse Python 3.12 source with Ruff and emit a conservative Codon candidate.
    PyCodon {
        input: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate a conservative Codon candidate from Python, then invoke `codon run`.
    PyRun {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Generate a conservative Codon candidate from Python, then invoke `codon build`.
    PyBuild {
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
}
