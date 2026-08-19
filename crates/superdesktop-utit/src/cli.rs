use std::{path::PathBuf, str::FromStr};

use crate::Suite;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandLine {
    pub workspace: PathBuf,
    pub command: UtitCommand,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UtitCommand {
    List {
        json: bool,
    },
    Run {
        suite: Suite,
        cases: Vec<String>,
        tags: Vec<String>,
        output: Option<PathBuf>,
        dry_run: bool,
        fail_fast: bool,
        replace_run: bool,
    },
    ValidateReport {
        path: PathBuf,
    },
}

pub fn parse_args<I>(arguments: I, current_dir: PathBuf) -> Result<CommandLine, String>
where
    I: IntoIterator<Item = String>,
{
    let mut arguments = arguments.into_iter().peekable();
    let command_name = arguments
        .next()
        .ok_or("usage: superdesktop-utit <list|run|validate-report>")?;
    let mut workspace = current_dir;
    let command = match command_name.as_str() {
        "list" => {
            let mut json = false;
            while let Some(argument) = arguments.next() {
                match argument.as_str() {
                    "--json" => json = true,
                    "--workspace" => {
                        workspace =
                            PathBuf::from(arguments.next().ok_or("missing --workspace value")?)
                    }
                    _ => return Err(format!("unknown list argument: {argument}")),
                }
            }
            UtitCommand::List { json }
        }
        "run" => {
            let mut suite = Suite::Smoke;
            let mut cases = Vec::new();
            let mut tags = Vec::new();
            let mut output = None;
            let mut dry_run = false;
            let mut fail_fast = false;
            let mut replace_run = false;
            while let Some(argument) = arguments.next() {
                match argument.as_str() {
                    "--suite" => {
                        suite = Suite::from_str(&arguments.next().ok_or("missing --suite value")?)?
                    }
                    "--case" => cases.push(arguments.next().ok_or("missing --case value")?),
                    "--tag" => tags.push(arguments.next().ok_or("missing --tag value")?),
                    "--output" => {
                        output = Some(PathBuf::from(
                            arguments.next().ok_or("missing --output value")?,
                        ))
                    }
                    "--workspace" => {
                        workspace =
                            PathBuf::from(arguments.next().ok_or("missing --workspace value")?)
                    }
                    "--dry-run" => dry_run = true,
                    "--fail-fast" => fail_fast = true,
                    "--replace-run" => replace_run = true,
                    _ => return Err(format!("unknown run argument: {argument}")),
                }
            }
            let mut unique_cases = cases.clone();
            unique_cases.sort();
            unique_cases.dedup();
            if unique_cases.len() != cases.len() {
                return Err("duplicate --case filter".into());
            }
            let mut unique_tags = tags.clone();
            unique_tags.sort();
            unique_tags.dedup();
            if unique_tags.len() != tags.len() {
                return Err("duplicate --tag filter".into());
            }
            UtitCommand::Run {
                suite,
                cases,
                tags,
                output,
                dry_run,
                fail_fast,
                replace_run,
            }
        }
        "validate-report" => {
            let path = PathBuf::from(arguments.next().ok_or("missing report path")?);
            while let Some(argument) = arguments.next() {
                match argument.as_str() {
                    "--workspace" => {
                        workspace =
                            PathBuf::from(arguments.next().ok_or("missing --workspace value")?)
                    }
                    _ => return Err(format!("unknown validate-report argument: {argument}")),
                }
            }
            UtitCommand::ValidateReport { path }
        }
        _ => return Err(format!("unknown command: {command_name}")),
    };
    Ok(CommandLine { workspace, command })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_command_and_rejects_unknown_or_duplicate_filters() {
        let cwd = PathBuf::from(r"D:\workspace");
        assert_eq!(
            parse_args(["list".into(), "--json".into()], cwd.clone())
                .unwrap()
                .command,
            UtitCommand::List { json: true }
        );
        let run = parse_args(
            [
                "run",
                "--suite",
                "shell-parity",
                "--case",
                "gui-start",
                "--tag",
                "gui",
                "--dry-run",
            ]
            .into_iter()
            .map(ToString::to_string),
            cwd.clone(),
        )
        .unwrap();
        assert!(matches!(
            run.command,
            UtitCommand::Run {
                suite: Suite::ShellParity,
                dry_run: true,
                ..
            }
        ));
        assert!(
            parse_args(
                ["run", "--case", "x", "--case", "x"]
                    .into_iter()
                    .map(ToString::to_string),
                cwd.clone()
            )
            .is_err()
        );
        assert!(parse_args(["unknown".into()], cwd).is_err());
    }
}
