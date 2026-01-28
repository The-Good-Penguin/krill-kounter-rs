use std::collections::HashMap;
use std::io::Write;
use std::os::raw::c_ulong;
use std::path::Path;

use anyhow::Result;
use nix::unistd::Uid;
use tempfile::NamedTempFile;
use tokio::fs;

use krillkounter::block::BlockStats;
use krillkounter::device::{init_device_stats, merge_existing_device_stats, DeviceEntry};

#[path = "common/mod.rs"]
mod common;

#[tokio::test]
async fn json_stats_dir_not_existing() -> Result<()> {
	let path = Path::new("/tmp/krillkounter3/stats.json");

	let stats = init_device_stats(&path);

	assert!(
		stats.is_ok(),
		"Expected no error message, but got '{}'",
		stats.unwrap_err(),
	);

	fs::remove_file(&path).await?;
	fs::remove_dir(&path.parent().unwrap()).await?;

	Ok(())
}

#[tokio::test]
async fn json_stats_file_not_existing_dir_existing() -> Result<()> {
	let path = Path::new("/tmp/krillkounter4/stats.json");

	let parent_dir = path.parent().unwrap();

	fs::create_dir_all(parent_dir).await?;

	let stats = init_device_stats(&path);

	assert!(
		stats.is_ok(),
		"Expected no error message, but got '{}'",
		stats.unwrap_err(),
	);

	fs::remove_file(&path).await?;
	fs::remove_dir(&path.parent().unwrap()).await?;

	Ok(())
}

#[tokio::test]
async fn json_stats_unwritable_stat_path() -> Result<()> {
	//TODO this test currently is lazy, create a rd only dir instead here
	let path = Path::new("/usr/bin/stats.json");

	if Uid::effective().is_root() {
		panic!("You must NOT run the tests as root");
	}

	let expected = "JSON Stats: Path: /usr/bin is unwritable !";

	let stats = init_device_stats(&path).unwrap_err();
	assert!(
		stats.to_string().contains(expected),
		"Expected error message to contain '{}', but got '{}'",
		expected,
		stats,
	);

	Ok(())
}

#[tokio::test]
async fn json_stat_file_existing() -> Result<()> {
	let mut stats: HashMap<String, DeviceEntry> = Default::default();

	common::init_env();

	let dev = common::get_dev();

	stats.insert(
		"0xdeadface".to_string(),
		DeviceEntry {
			first_sithing_date: "31.07.2025".to_string(),
			previous_path: dev.clone(),
			current_path: dev.clone(),
			stored_stats: BlockStats {
				read_io: 10,
				read_merges: 20,
				read_sectors: 30,
				read_ticks: 40,
				write_io: 50,
				write_merges: 60,
				write_sectors: 70,
				write_ticks: 80,
				in_flight: 90,
				io_ticks: 100,
				time_in_queue: 110,
				discard_io: 120,
				discard_merges: 130,
				discard_sectors: 140,
				discard_ticks: 150,
				flush_io: 0,
				flush_ticks: 0,
			},
			total_bytes_written: 200,
			disk_seq: 40,
			previous_stats: BlockStats {
				read_io: 1,
				read_merges: 2,
				read_sectors: 3,
				read_ticks: 4,
				write_io: 5,
				write_merges: 6,
				write_sectors: 7,
				write_ticks: 8,
				in_flight: 9,
				io_ticks: 10,
				time_in_queue: 11,
				discard_io: 12,
				discard_merges: 13,
				discard_sectors: 14,
				discard_ticks: 15,
				flush_io: 0,
				flush_ticks: 0,
			},
			serial_number: ".".to_string(),
			device_name: ".".to_string(),
			stat_path: ".".to_string(),
			is_active: true,
		},
	);

	let stats_json = serde_json::to_string_pretty(&stats)?;

	let mut temp_file = NamedTempFile::with_suffix(".json")?;
	write!(temp_file, "{}", stats_json)?;

	let path = temp_file.into_temp_path();

	let stats_result = init_device_stats(&path);

	assert!(
		stats_result.is_ok(),
		"Expected no error message, but got '{}'",
		stats_result.unwrap_err(),
	);

	Ok(())
}

#[tokio::test]
async fn previous_device_entry_in_stat_file_does_not_exist() -> Result<()> {
	let mut stats: HashMap<String, DeviceEntry> = Default::default();

	common::init_env();

	let dev = common::get_dev();

	stats.insert(
		"0xdeadface".to_string(),
		DeviceEntry {
			first_sithing_date: "31.07.2025".to_string(),
			previous_path: dev.clone(),
			current_path: dev.clone(),
			stored_stats: BlockStats {
				read_io: 10,
				read_merges: 20,
				read_sectors: 30,
				read_ticks: 40,
				write_io: 50,
				write_merges: 60,
				write_sectors: 70,
				write_ticks: 80,
				in_flight: 90,
				io_ticks: 100,
				time_in_queue: 110,
				discard_io: 120,
				discard_merges: 130,
				discard_sectors: 140,
				discard_ticks: 150,
				flush_io: 0,
				flush_ticks: 0,
			},
			total_bytes_written: 200,
			disk_seq: 40,
			previous_stats: BlockStats {
				read_io: 1,
				read_merges: 2,
				read_sectors: 3,
				read_ticks: 4,
				write_io: 5,
				write_merges: 6,
				write_sectors: 7,
				write_ticks: 8,
				in_flight: 9,
				io_ticks: 10,
				time_in_queue: 11,
				discard_io: 12,
				discard_merges: 13,
				discard_sectors: 14,
				discard_ticks: 15,
				flush_io: 0,
				flush_ticks: 0,
			},
			serial_number: ".".to_string(),
			device_name: ".".to_string(),
			stat_path: ".".to_string(),
			is_active: true,
		},
	);

	let mut path: Vec<String> = Vec::new();
	path.push(dev.clone());

	if let Err(e) = merge_existing_device_stats(&path, &mut stats) {
		assert!(false, "{}", e);
	}

	assert!(
		stats.len() == 2,
		"The stat file should be of length 2 and is {}",
		stats.len()
	);

	Ok(())
}

#[tokio::test]
async fn previous_device_entry_in_stat_file_does_exist() -> Result<()> {
	let mut stats: HashMap<String, DeviceEntry> = Default::default();

	common::init_env();

	let dev = common::get_dev();
	let serial = common::get_dev_serial();

	stats.insert(
		serial.clone(),
		DeviceEntry {
			first_sithing_date: "31.07.2025".to_string(),
			previous_path: "/dev/mmcblk4".to_string(),
			current_path: "/dev/mmcblk4".to_string(),
			stored_stats: BlockStats {
				read_io: 10,
				read_merges: 20,
				read_sectors: 30,
				read_ticks: 40,
				write_io: 50,
				write_merges: 60,
				write_sectors: 70,
				write_ticks: 80,
				in_flight: 90,
				io_ticks: 100,
				time_in_queue: 110,
				discard_io: 120,
				discard_merges: 130,
				discard_sectors: 140,
				discard_ticks: 150,
				flush_io: 0,
				flush_ticks: 0,
			},
			total_bytes_written: 200,
			disk_seq: 40,
			previous_stats: BlockStats {
				read_io: 1,
				read_merges: 2,
				read_sectors: 3,
				read_ticks: 4,
				write_io: 5,
				write_merges: 6,
				write_sectors: 7,
				write_ticks: 8,
				in_flight: 9,
				io_ticks: 10,
				time_in_queue: 11,
				discard_io: 12,
				discard_merges: 13,
				discard_sectors: 14,
				discard_ticks: 15,
				flush_io: 0,
				flush_ticks: 0,
			},
			serial_number: ".".to_string(),
			device_name: ".".to_string(),
			stat_path: ".".to_string(),
			is_active: true,
		},
	);

	let mut path: Vec<String> = Vec::new();
	path.push(dev.clone());

	if let Err(e) = merge_existing_device_stats(&path, &mut stats) {
		assert!(false, "{}", e);
	}

	let device = stats.get(&serial).unwrap();

	assert!(
		stats.len() == 1 && device.first_sithing_date == "31.07.2025" && device.current_path == dev,
		"The stat file should be of length 1 and is {} path is {} date is {}",
		stats.len(),
		device.current_path,
		device.first_sithing_date
	);

	Ok(())
}

#[tokio::test]
async fn overflow_handling() -> Result<()> {
	let mut stats: HashMap<String, DeviceEntry> = Default::default();

	common::init_env();

	let dev = common::get_dev();
	let serial = common::get_dev_serial();

	stats.insert(
		serial.clone(),
		DeviceEntry {
			first_sithing_date: "31.07.2025".to_string(),
			previous_path: "/dev/mmcblk4".to_string(),
			current_path: "/dev/mmcblk4".to_string(),
			stored_stats: BlockStats {
				read_io: 10,
				read_merges: 20,
				read_sectors: 30,
				read_ticks: 40,
				write_io: 50,
				write_merges: 60,
				write_sectors: 70,
				write_ticks: 80,
				in_flight: 90,
				io_ticks: 100,
				time_in_queue: 110,
				discard_io: 120,
				discard_merges: 130,
				discard_sectors: 140,
				discard_ticks: 150,
				flush_io: 0,
				flush_ticks: 0,
			},
			total_bytes_written: 200,
			disk_seq: 40,
			previous_stats: BlockStats {
				read_io: 1,
				read_merges: 2,
				read_sectors: 3,
				read_ticks: 4,
				write_io: 5,
				write_merges: 6,
				write_sectors: 7,
				write_ticks: 8,
				in_flight: 9,
				io_ticks: 10,
				time_in_queue: 11,
				discard_io: 12,
				discard_merges: 13,
				discard_sectors: 14,
				discard_ticks: 15,
				flush_io: 0,
				flush_ticks: 0,
			},
			serial_number: ".".to_string(),
			device_name: ".".to_string(),
			stat_path: ".".to_string(),
			is_active: true,
		},
	);

	let mut path: Vec<String> = Vec::new();
	path.push(dev.clone());

	if let Err(e) = merge_existing_device_stats(&path, &mut stats) {
		assert!(false, "{}", e);
	}

	let mut device = stats.get(&serial).unwrap().clone();

	let overflow = 4;

	let old_total_bytes = device.total_bytes_written;

	device
		.compute_total_bytes_written(overflow, c_ulong::MAX - 10)
		.unwrap();
	let correct_value = old_total_bytes + (overflow as u128 + 1 + 10) * common::SECTOR_SIZE;

	assert!(
		device.total_bytes_written == correct_value,
		"Value after overflow is {} but it should be {}",
		device.total_bytes_written,
		correct_value
	);

	Ok(())
}
