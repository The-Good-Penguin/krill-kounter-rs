use std::io::Write;
use std::os::unix::fs::FileTypeExt;
use std::path::Path;

use anyhow::Result;
use fs_err as fs;

use tempfile::{NamedTempFile, TempPath};

pub const SECTOR_SIZE: u128 = 512;

pub fn init_env() -> () {
	dotenvy::dotenv().ok();
}

pub fn generate_config_file(
	device_paths: Vec<String>,
	stats_path: String,
	update_rate: u64,
) -> Result<TempPath> {
	let config_json = serde_json::json!({
		"devicePaths": device_paths,
		"statsFilePath": stats_path,
		"updateRate": update_rate,
	});

	let mut temp_file = NamedTempFile::with_suffix(".json")?;
	write!(temp_file, "{}", config_json)?;

	let path = temp_file.into_temp_path();

	Ok(path)
}

pub fn get_stats() -> String {
	let stats_path = std::env::var("TEST_STATS_JSON_PATH")
		.expect("TEST_STATS_JSON_PATH environment variable must be set");

	assert!(!stats_path.is_empty());

	stats_path
}

pub fn get_dev() -> String {
	let device = std::env::var("TEST_CONFIG_MMCBLK_DEV")
		.expect("TEST_CONFIG_MMCBLK_DEV environment variable must be set eg. /dev/mmcblk0");

	assert!(!device.is_empty());

	let path = Path::new(&device);

	assert!(path.exists());

	let file_meta = fs::metadata(path).expect("No metadata!");

	assert!(file_meta.file_type().is_block_device());

	device
}

pub fn get_dev_mount() -> String {
	let device = std::env::var("TEST_CONFIG_MMCBLK_DEV_MOUNT")
		.expect("TEST_CONFIG_MMCBLK_DEV_MOUNT environment variable must be set eg. /media/loop1 for the DUT");

	assert!(!device.is_empty());

	let path = Path::new(&device);

	assert!(path.exists());

	device
}

pub fn get_dev_serial() -> String {
	let serial = std::env::var("TEST_CONFIG_MMCBLK_SERIAL")
		.expect("TEST_CONFIG_MMCBLK_SERIAL environment variable must be set");

	assert!(!serial.is_empty());

	serial
}
