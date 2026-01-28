use std::fs::File;
use std::io::BufReader;
use std::os::unix::fs::FileTypeExt;
use std::path::{Path, PathBuf};

use fs_err as fs;

use anyhow::{bail, Result};
use clap::Parser;
use serde::Deserialize;

use crate::bail_self;
use crate::utilities::LogPrefix;

#[derive(Parser, Debug)]
#[command(
	author = "Pawel Zalewski",
	version = "1.0",
	about = "Krill Kounter: monitor the system block devices."
)]
pub struct Args {
	#[arg(short('c'), long, value_name = "KK_CONFIG_JSON_PATH", required = true)]
	pub config_path: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DaemonConfig {
	#[serde(skip)]
	pub config_file_path: String,
	pub stats_file_path: String,
	pub update_rate: u64,
}

impl DaemonConfig {
	/// # Panics
	///
	/// Will panic if config path does not exist
	/// # Errors
	/// Will return `Err` if JSON onfig path cannot be read
	///
	pub fn new(args: Args) -> Result<Self> {
		let config_file_path = PathBuf::from(args.config_path);

		let file = File::open(&config_file_path)?;
		let reader = BufReader::new(file);
		let config: DaemonConfig = serde_json::from_reader(reader)?;

		let config_file_path_string = config_file_path
			.to_str()
			.expect("No config path")
			.to_string();

		if config.stats_file_path.is_empty() {
			bail_self!(
				config,
				"statsFilePath is empty! define a valid path for stats storage."
			);
		}

		if config.update_rate == 0 {
			bail_self!(config, "updateRate cannot be zero.");
		}

		Ok(Self {
			config_file_path: config_file_path_string,
			stats_file_path: config.stats_file_path,
			update_rate: config.update_rate,
		})
	}
}

impl LogPrefix for DaemonConfig {
	fn log_prefix(&self) -> String {
		"JSON Config: ".to_string()
	}

	fn log(&self, msg: &str) -> String {
		format!("{}: {}", self.log_prefix(), msg)
	}
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeviceConfig {
	pub device_paths: Vec<String>,
	pub stats_file_path: String,
}

/// Initialize and validate the JSON config path and return a validated `DeviceConfig` struct
impl DeviceConfig {
	/// # Errors
	/// Will return `Err` if JSON config path cannot be read
	///
	pub fn new(path: &Path) -> Result<Self> {
		let file = File::open(path)?;
		let reader = BufReader::new(file);
		let config: DeviceConfig = serde_json::from_reader(reader)?;

		if config.device_paths.is_empty() {
			bail_self!(
				config,
				"devicePaths array is empty! Choose at least one block device under /dev"
			);
		}

		for device in &config.device_paths {
			let path = Path::new(device);

			if !path.exists() {
				bail_self!(config, "Path: {} {}", device, "does not exist !");
			}

			let file_meta = fs::metadata(path)?;

			if !file_meta.file_type().is_block_device() {
				bail_self!(config, "Path: {} {}", device, "is not a block device !");
			}
		}

		Ok(config)
	}
}

impl LogPrefix for DeviceConfig {
	fn log_prefix(&self) -> String {
		"JSON Config: ".to_string()
	}

	fn log(&self, msg: &str) -> String {
		format!("{}: {}", self.log_prefix(), msg)
	}
}
