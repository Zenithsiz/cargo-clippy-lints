//! Cargo subcommand to run `clippy`
//! with external lints defined in a `lints.toml`

// Features
#![feature(command_access)]

// Imports
use std::{
	env,
	ffi::OsString,
	fs,
	path::{Path, PathBuf},
	process::{Command, ExitStatus},
};

use anyhow::Context;

/// All lints defined in file
#[derive(Clone, Default, Debug)]
#[derive(serde::Serialize, serde::Deserialize)]
struct Lints {
	#[serde(default)]
	deny:  Vec<String>,
	#[serde(default)]
	allow: Vec<String>,
	#[serde(default)]
	warn:  Vec<String>,
}

impl Lints {
	/// Lints filename
	const FILE_NAME: &'static str = "lints.toml";
}

impl Lints {
	/// Finds the config path in the current directory or any
	/// parent directory
	pub fn find_config_path() -> Result<Option<PathBuf>, anyhow::Error> {
		// Get the current path to start looking
		let mut cur_path = env::current_dir().context("Failed to get current directory")?;

		// Then keep ascending until we find it
		loop {
			// Get the path
			let lints_path = cur_path.join(Lints::FILE_NAME);

			// Then check if it exists
			match lints_path.exists() {
				// If it did, return it
				true => break Ok(Some(lints_path)),

				// Else check if we still have a parent
				false => match cur_path.parent() {
					// If so, retry
					Some(parent) => cur_path = parent.to_path_buf(),
					// Else return `None`
					None => return Ok(None),
				},
			}
		}
	}

	/// Parses the lints from config
	pub fn from_config() -> Result<Self, anyhow::Error> {
		Self::find_config_path()?.map_or_else(|| Ok(Lints::default()), |path| Self::from_config_with_path(&path))
	}

	/// Parses the lints from a path
	pub fn from_config_with_path(path: &Path) -> Result<Self, anyhow::Error> {
		fs::read_to_string(path)
			.context("Failed to read config")
			.map(|s| toml::from_str(&s))?
			.context("Failed to parse config")
	}

	/// Constructs all deny flags
	fn deny_flags(&self) -> Vec<String> {
		self.deny
			.iter()
			.flat_map(|lint| vec!["-D".to_owned(), lint.clone()].into_iter())
			.collect()
	}

	/// Constructs all warn flags
	fn warn_flags(&self) -> Vec<String> {
		self.warn
			.iter()
			.flat_map(|lint| vec!["-W".to_owned(), lint.clone()].into_iter())
			.collect()
	}

	/// Constructs all allow flags
	fn allow_flags(&self) -> Vec<String> {
		self.allow
			.iter()
			.flat_map(|lint| vec!["-A".to_owned(), lint.clone()].into_iter())
			.collect()
	}

	/// Runs clippy with `args`
	pub fn run_clippy(&self, args: impl IntoIterator<Item = OsString>) -> Result<ExitStatus, anyhow::Error> {
		// Build the command
		let mut cmd = Command::new("cargo");
		let cmd = cmd
			.arg("clippy")
			.args(args)
			.arg("--")
			.args(self.warn_flags())
			.args(self.deny_flags())
			.args(self.allow_flags());

		// Print what we're running
		eprint!("Running \"cargo\"");
		for arg in cmd.get_args() {
			eprint!(", {:?}", arg);
		}
		eprintln!();

		// Spawn it and wait
		cmd.spawn()
			.context("Unable to start clippy")?
			.wait()
			.context("Unable to wait for clippy")
	}
}

fn main() -> Result<(), anyhow::Error> {
	// Get the lints
	let lints = Lints::from_config()?;

	// Then run clippy
	let get_args = || std::env::args_os();
	let status = match get_args().nth(1) {
		// If we were run with `cargo`, skip the next argument (which will be our filename)
		// Note: When running with cargo, we're run with `clippy-lints` in the 2nd argument
		Some(arg) if arg == "clippy-lints" => lints.run_clippy(get_args().skip(2))?,
		_ => lints.run_clippy(get_args().skip(1))?,
	};

	// And check the status
	match status {
		status if status.success() => Ok(()),
		_ => anyhow::bail!("Clippy returned non-0 status: {}", status),
	}
}
