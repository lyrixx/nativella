//! The `nativella` command, and the entry point of standalone applications.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nativella_lib::apk::{self, Key};
use nativella_lib::{Project, embedded, notices};

const USAGE: &str = "Usage:
  nativella run [<project-dir>]                    Run a PHP project as a desktop application
  nativella build [<project-dir>] [-o <output>]    Build a standalone executable of the project
  nativella android [<project-dir>] --template <apk> [--key <pem>] [-o <output>]
                                                   Make an Android application (APK) of the project
  nativella bundle [<project-dir>] [-o <output>]   Pack the project into a file to embed in an iOS app
  nativella licenses                               Print the licenses of the software nativella contains
  nativella --version                              Print the version

The project directory defaults to the current one.
Documentation: https://lyrixx.github.io/nativella/docs/";

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
        Some("run") => load(args.next()).map_or_else(|error| fail(&error), run),
        Some("build") => package(args, Package::Executable),
        Some("android") => package(args, Package::Android),
        Some("bundle") => package(args, Package::Bundle),
        Some("licenses") => licenses(),
        Some("-V" | "--version") => {
            println!("nativella {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        None | Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            ExitCode::SUCCESS
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
    /// An Android application: the template APK, with the project.
    Android,
    /// The project alone, for an iOS build to compile in.
    Bundle,
}

fn package(mut args: impl Iterator<Item = String>, package: Package) -> ExitCode {
    let mut dir = None;
    let mut output = None;
    let mut template = None;
    let mut key = None;
    while let Some(arg) = args.next() {
        let path = match arg.as_str() {
            "-o" | "--output" => &mut output,
            "--template" if matches!(package, Package::Android) => &mut template,
            "--key" if matches!(package, Package::Android) => &mut key,
            _ if dir.is_none() && !arg.starts_with('-') => {
                dir = Some(arg);
                continue;
            }
            _ => return fail(&format!("Unexpected argument \"{arg}\".\n\n{USAGE}")),
        };
        match args.next() {
            Some(value) => *path = Some(PathBuf::from(value)),
            None => return fail(&format!("{arg} takes a path.\n\n{USAGE}")),
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
            Package::Android => format!("{name}.apk"),
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
        Package::Executable => embedded::build(&project, &output)
            .and_then(|local_overrides| write_notices(&output).map(|()| local_overrides)),
        Package::Android => android(&project, template.as_deref(), key.as_deref(), &output),
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

/// Prints the third-party notices this executable carries.
fn licenses() -> ExitCode {
    match notices::text() {
        Ok(Some(text)) => match io::stdout().write_all(text.as_bytes()) {
            Ok(()) => ExitCode::SUCCESS,
            // `nativella licenses | head` closes the pipe before the end.
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
            Err(error) => fail(&format!("The notices cannot be printed: {error}")),
        },
        Ok(None) => fail(
            "This nativella is a development build, without the notices of the software it contains: the releases have them, and so does castor nativella:build.",
        ),
        Err(error) => fail(&format!("The notices cannot be read: {error}")),
    }
}

/// Writes the third-party notices next to the standalone executable
/// `output`, which carries the same software as this one: they ship with
/// it.
fn write_notices(output: &Path) -> io::Result<()> {
    let Some(text) = notices::text()? else {
        eprintln!(
            "Warning: this nativella is a development build, without the notices of the software it contains: build the applications you distribute with a release, or with castor nativella:build."
        );
        return Ok(());
    };

    let path = output.with_file_name(notices::FILE);
    fs::write(&path, text)?;
    eprintln!(
        "Wrote the licenses of the software the application contains to {}: distribute them with it.",
        path.display()
    );

    Ok(())
}

/// The APK of `project`, signed with the key in `key`, or the debug key.
/// Returns the local overrides it left out.
fn android(
    project: &Project,
    template: Option<&Path>,
    key: Option<&Path>,
    output: &Path,
) -> io::Result<Vec<String>> {
    let template = template.ok_or_else(|| {
        io::Error::other(format!(
            "--template is missing: the template APK of Nativella {}, from its release.",
            env!("CARGO_PKG_VERSION")
        ))
    })?;
    let key = match key {
        Some(key) => Key::load(key)?,
        None => {
            let key = Key::debug()?;
            eprintln!(
                "Signing with the debug key of this computer, {}: to publish the application, sign it with a key of your own (--key).",
                key.path().display()
            );
            key
        }
    };

    apk::package(project, template, output, &key)
}

fn fail(message: &str) -> ExitCode {
    eprintln!("{message}");

    ExitCode::FAILURE
}
