use anyhow::Result;

use krillkounter::config::DeviceConfig;

#[path = "common/mod.rs"]
mod common;

#[tokio::test]
async fn json_config_bad_dev_path() -> Result<()> {
	common::init_env();

	let dev = common::get_dev();

	let path = common::generate_config_file(
		vec![dev, "/dev/abcdef".to_string()],
		"/tmp/krillkounter0/stats.json".to_string(),
		1000,
	)?;

	let expected = "JSON Config: : Path: /dev/abcdef does not exist !";

	let config = DeviceConfig::new(&path).unwrap_err();
	assert!(
		config.to_string().contains(expected),
		"Expected error message to contain '{}', but got '{}'",
		expected,
		config,
	);

	Ok(())
}

#[tokio::test]
async fn json_config_dev_not_block_device() -> Result<()> {
	common::init_env();

	let path = common::generate_config_file(
		vec!["/dev/tty".to_string()],
		"/tmp/krillkounter1/stats.json".to_string(),
		1000,
	)?;

	let expected = "JSON Config: : Path: /dev/tty is not a block device !";

	let config = DeviceConfig::new(&path).unwrap_err();
	assert!(
		config.to_string().contains(expected),
		"Expected error message to contain '{}', but got '{}'",
		expected,
		config,
	);

	Ok(())
}

#[tokio::test]
async fn json_config_empty_device_paths() -> Result<()> {
	common::init_env();

	let path =
		common::generate_config_file(vec![], "/tmp/krillkounter2/stats.json".to_string(), 1000)?;

	let expected =
		"JSON Config: : devicePaths array is empty! Choose at least one block device under /dev";

	let config = DeviceConfig::new(&path).unwrap_err();
	assert!(
		config.to_string().contains(expected),
		"Expected error message to contain '{}', but got '{}'",
		expected,
		config,
	);

	Ok(())
}
