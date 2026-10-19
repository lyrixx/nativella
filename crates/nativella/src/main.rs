//! The `nativella` command, and the entry point of standalone applications.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use nativella_lib::{Project, embedded};

const USAGE: &str = "Usage:
  nativella [run] [<project-dir>]                  Run a PHP project as a desktop application
  nativella build [<project-dir>] [-o <output>]    Build a standalone executable of the project
  nativella bundle [<project-dir>] [-o <output>]   Bundle the project for a mobile build (NATIVELLA_BUNDLE)
  nativella --version";

fn main() -> ExitCode {
    // A standalone application runs its own project, whatever its arguments:
    // they belong to the application (a second instance forwards them).
    match embedded::find() {
        Ok(Some(project)) => return run(project),
        Ok(None) => {}
        Err(error) => return fail(&format!("The embedded application cannot start: {error}")),
    }

    let mut args = env::args().skip(1);
    let command = args.next();

    match command.as_deref() {
        None => load(None).map_or_else(|error| fail(&error), run),
        Some("run") => load(args.next()).map_or_else(|error| fail(&error), run),
        Some("build") => package(args, Package::Executable),
        Some("bundle") => package(args, Package::Bundle),
        Some("-V" | "--version") => {
            println!("nativella {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        // `nativella path/to/project`
        Some(dir) if !dir.starts_with('-') => {
            load(Some(dir.to_owned())).map_or_else(|error| fail(&error), run)
        }
        Some(other) => fail(&format!("Unknown command \"{other}\".\n\n{USAGE}")),
    }
}

fn load(dir: Option<String>) -> Result<Project, String> {
    let dir = dir.map_or_else(|| PathBuf::from("."), PathBuf::from);

    Project::load(&dir).map_err(|error| error.to_string())
}

fn run(project: Project) -> ExitCode {
    match nativella_lib::run(project, |builder| builder) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fail(&error),
    }
}

enum Package {
    /// A standalone executable: this one, with the project appended.
    Executable,
    /// The project alone, for a mobile build to compile in.
    Bundle,
}

fn package(mut args: impl Iterator<Item = String>, package: Package) -> ExitCode {
    let mut dir = None;
    let mut output = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-o" | "--output" => match args.next() {
                Some(path) => output = Some(PathBuf::from(path)),
                None => return fail(&format!("{arg} takes a path.\n\n{USAGE}")),
            },
            _ if dir.is_none() && !arg.starts_with('-') => dir = Some(arg),
            _ => return fail(&format!("Unexpected argument \"{arg}\".\n\n{USAGE}")),
        }
    }

    let project = match load(dir) {
        Ok(project) => project,
        Err(error) => return fail(&error),
    };
    let output = output.unwrap_or_else(|| {
        let name = project.manifest.name.to_lowercase().replace(' ', "-");
        PathBuf::from("dist").join(match package {
            Package::Executable if cfg!(windows) => format!("{name}.exe"),
            Package::Executable => name,
            Package::Bundle => format!("{name}.bundle"),
        })
    });

    eprintln!(
        "Packaging {} {} into {}…",
        project.manifest.name,
        project.manifest.version,
        output.display()
    );
    let result = match package {
        Package::Executable => embedded::build(&project, &output),
        Package::Bundle => embedded::bundle(&project, &output),
    };

    match result {
        Ok(local_overrides) => {
            if !local_overrides.is_empty() {
                eprintln!(
                    "Left out {}: local overrides never ship. Set the production environment in a file that does, like the .env.local.php of `composer dump-env prod` with Symfony.",
                    local_overrides.join(", ")
                );
            }
            eprintln!("Done: {}", output.display());
            ExitCode::SUCCESS
        }
        Err(error) => fail(&format!("The packaging failed: {error}")),
    }
}

fn fail(message: &str) -> ExitCode {
    eprintln!("{message}");

    ExitCode::FAILURE
}
