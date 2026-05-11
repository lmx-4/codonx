# Security Policy

codonx is an experimental source preprocessor and CLI wrapper.

## Supported Versions

At the current stage, only the latest `main` branch and the latest tagged 0.x release are considered supported for security-relevant fixes.

| Version | Supported |
| ------- | --------- |
| latest main | yes |
| latest 0.x tag | yes |
| older 0.x tags | no |

## Reporting a Vulnerability

Please report security issues privately if they involve:

- unexpected command execution;
- unsafe path handling;
- environment variable injection;
- generated code that silently changes intended behavior in a dangerous way;
- release artifact compromise.

If private reporting is not configured on GitHub, open a minimal issue that does not include exploit details and ask for a private contact channel.

## Scope

Security-sensitive areas include:

- `codonx run` / `codonx build` subprocess invocation;
- `#%define CODON_PYTHON` and `#%define CODON_DEBUG`;
- temporary file handling;
- generated file path handling;
- report generation paths.

## Non-Security Bugs

Incorrect syntax lowering, missing warnings, or Python/Codon semantic mismatches are usually correctness bugs rather than security vulnerabilities. Please report them as normal issues unless they can be used for command execution, path escape, or unexpected file overwrite.
