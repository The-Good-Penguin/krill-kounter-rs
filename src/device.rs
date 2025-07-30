use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::os::raw::c_ulong;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Result};
use log::{debug, error, info};

use fs_err as fs;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::block::BlockStats;
use crate::utilities::LogPrefix;
use crate::{bail_self, debug_self, file_content_to_string, log_self, string_to_u64_result};

const SECTOR_SIZE: u128 = 512;

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DeviceEntry {
	pub first_sithing_date: String,

	#[serde(rename = "path", skip_serializing)]
	pub previous_path: String,
	#[serde(rename = "path", skip_deserializing)]
	pub current_path: String,

	#[serde(rename = "stats")]
	pub stored_stats: BlockStats,

	// We use u128 to deal with overflows for u32 and u64 the same way
	// to avoid testing for 32-bit Architecture overflows alone
	pub total_bytes_written: u128,
	pub disk_seq: u64,

	// The below entries exist only in memory and are not stored.
	#[serde(skip)]
	pub previous_stats: BlockStats,
	#[serde(skip)]
	pub is_active: bool,
	#[serde(skip)]
	pub serial_number: String,
	#[serde(skip)]
	pub device_name: String,
	#[serde(skip)]
	pub stat_path: String,
}

impl DeviceEntry {
	/// Returns a `DeviceEntry` object that needs to be initialised
	/// # Panics
	///
	/// Will panic if `device_name` does not exist
	#[must_use]
	pub fn new(device_path: &str) -> Self {
		// We know this is a valid path at this point
		let device_name = device_path
			.strip_prefix("/dev/")
			.ok_or(anyhow::anyhow!(" no /dev prefix !"))
			.expect("No dev name !")
			.to_string();

		Self {
			device_name,
			current_path: device_path.to_owned(),
			..Default::default()
		}
	}

	/// Returns new `BlockStats` struct populated from `stat_path`
	/// # Errors
	///
	/// Will return `Err` if a new `BlockStats` struct cannot be created
	pub fn get_new_stats(&mut self) -> Result<BlockStats> {
		let current_stats: BlockStats = BlockStats::new(&self.stat_path)?;
		Ok(current_stats)
	}

	/// Accumulate stats and store in `stored_stats`, returns tru if there was an update
	/// # Errors
	///
	/// Will return `Err` if a new `get_new_disk_seq` fails
	pub fn update_stats(&mut self, current_stats: BlockStats) -> Result<bool> {
		let prev_disk_seq = self.disk_seq;
		self.disk_seq = self.get_new_disk_seq()?;

		if prev_disk_seq != self.disk_seq {
			self.previous_stats = BlockStats::default();
			debug_self!(
				self,
				"Disk sequence changed: prev: {} new: {} \n",
				prev_disk_seq,
				self.disk_seq
			);
		}

		// No changes = nothing to do
		if self.previous_stats == current_stats {
			debug_self!(self, "Nothing to do!");
			return Ok(false);
		}

		self.compute_total_bytes_written(
			current_stats.write_sectors,
			self.previous_stats.write_sectors,
		)?;

		// Accumulate the stat difference for storage
		self.stored_stats
			.accumulate_stats(current_stats, self.previous_stats.clone());

		/*
		 Get new stats here to include the writes to the JSON output file,
		 this is only important if the stats file is stored on the block device
		 being monitored. Without this, the next time the function is called,
		 we will detect the stats changing due to the JSON output and cause an
		 infinite loop.
		*/
		self.previous_stats.get_current_stats(&self.stat_path)?;

		Ok(true)
	}

	/// Initialise : populate `stat_path`, get serial number and current stats, set to active
	/// # Errors
	///
	/// Will return `Err` if `now_local` fails
	/// Will return `Err` if `get_serial_number` fails
	/// Will return `Err` if `get_current_stats` fails
	pub fn init_device_entry(&mut self) -> Result<()> {
		if self.first_sithing_date.is_empty() {
			let now = OffsetDateTime::now_local()?;
			self.first_sithing_date = now.to_string();
		}

		self.stat_path = format!("{}{}{}", "/sys/block/", self.device_name, "/stat");

		let path: &Path = Path::new(&self.stat_path);

		if !path.exists() {
			bail_self!(self, "Path: {} {}", self.device_name, "does not exist");
		}

		self.serial_number = self.get_serial_number()?;
		self.previous_stats.get_current_stats(&self.stat_path)?;
		self.is_active = true;

		Ok(())
	}

	/// Compute total bytes written with ULONG_MAX-overflow-proof accumulator
	/// # Errors
	///
	/// Will return `Err` if `total_bytes_written` would overflow
	pub fn compute_total_bytes_written(
		&mut self,
		current_write_sectors: u64,
		previous_write_sectors: u64,
	) -> Result<()> {
		let previous_write_sectors_u128: u128 = u128::from(previous_write_sectors);
		let current_write_sectors_u128: u128 = u128::from(current_write_sectors);

		let byte_difference: u128 = if current_write_sectors < previous_write_sectors {
			/* Overflow has occurred, calculate the difference:
			ULONG_MAX from limits.h is used to get the max value for the
			writeSector field in the kernel. This can either be 32-bit or 64-bit, depending on the machine.
			<https://www.kernel.org/doc/html/v6.1/admin-guide/iostats.html>
			*/

			let mut diff: u128 =
				(u128::from(c_ulong::MAX) + 1 - previous_write_sectors_u128) * SECTOR_SIZE;
			diff += current_write_sectors_u128 * SECTOR_SIZE;
			diff
		} else {
			let diff: u128 =
				(current_write_sectors_u128 - previous_write_sectors_u128) * SECTOR_SIZE;
			diff
		};

		match self.total_bytes_written.checked_add(byte_difference) {
			Some(result) => self.total_bytes_written = result,
			None => bail_self!(self, "340 undecillion of bytes written, well done !"),
		}

		debug_self!(
			self,
			"current write sectors: {} previous write sectors: {} diff: {} total_bytes_written: {}",
			current_write_sectors_u128,
			previous_write_sectors_u128,
			byte_difference,
			self.total_bytes_written
		);

		Ok(())
	}

	/// Get serial number, try fallback on fialure as it might be an USB adapter
	fn get_serial_number(&self) -> Result<String> {
		let serial_number = self.get_serial_number_block();

		let serial_number = match serial_number {
			Ok(s) if !s.is_empty() => Ok(s),
			_ => self.get_serial_number_fallback(),
		};

		let serial_string = serial_number?;

		if serial_string.is_empty() {
			bail_self!(self, "Could not get a serial number for block device");
		}

		Ok(serial_string)
	}

	/// Get serial number from sysfs
	fn get_serial_number_block(&self) -> Result<String> {
		let serial_path = format!("{}{}{}", "/sys/block/", self.device_name, "/device/serial");

		debug_self!(
			self,
			"Trying to get block serial number for : {}",
			serial_path
		);

		let mut file = fs::File::open(&serial_path)?;

		let serial_cleaned = file_content_to_string!(file);

		Ok(serial_cleaned)
	}

	/// Get serial number via lsblk
	fn get_serial_number_fallback(&self) -> Result<String> {
		let command_string = format!("{}{}", "/dev/", self.device_name);

		let command = Command::new("lsblk")
			.arg("-r")
			.arg("-n")
			.arg("-o")
			.arg("serial")
			.arg(&command_string)
			.arg("-a")
			.output()?;

		debug_self!(
			self,
			"Trying to get USB block serial number for : {}",
			self.device_name
		);

		if command.status.success() {
			Ok(String::from_utf8(command.stdout)?.trim().to_string())
		} else {
			let err = String::from_utf8_lossy(&command.stderr);
			Err(anyhow::anyhow!(log_self!(
				self,
				"The lsblk command failed: unable to get serial number! \n\
										Exit status is : {} \nStderr {} ",
				command.status.code().expect("No Stderr !"),
				err
			)))
		}
	}

	/// Get `disk_seq` from sysfs
	///
	/// # Errors
	/// Will return `Err` if `disk_seq` path exists but is an empty string
	fn get_new_disk_seq(&self) -> Result<u64> {
		let sysfs_path = format!("{}{}{}", "/sys/block/", self.device_name, "/diskseq");

		let file = fs::File::open(&sysfs_path);

		// Handle pre 5.12 kernel sysfs
		if file.is_err() {
			debug_self!(
				self,
				"Failed to read diskseq, drive swaps will not be detected!"
			);
			return Ok(0);
		}

		let disk_seq = file_content_to_string!(file?);

		if disk_seq.is_empty() {
			Err(anyhow::anyhow!(log_self!(self, "Diskseq is empty!")))
		} else {
			Ok(string_to_u64_result!(disk_seq))
		}
	}

	/// Copy over fields that are volatile and not stored in JSON
	fn copy_volatile_fields(&mut self, entry: &DeviceEntry) {
		// Serial number does not change
		self.is_active = entry.is_active;
		self.current_path.clone_from(&entry.current_path);
		self.device_name.clone_from(&entry.device_name);
		self.previous_stats = entry.previous_stats.clone();
		self.stat_path.clone_from(&entry.stat_path);
	}
}

impl LogPrefix for DeviceEntry {
	fn log_prefix(&self) -> String {
		format!("DeviceEntry:{}: ", self.device_name)
	}

	fn log(&self, msg: &str) -> String {
		format!("{}: {}", self.log_prefix(), msg)
	}
}

/// Read existing JSON stats if they exist if not create the file: always returns a hashmap object
/// # Panics
///
/// Will panic if config JSON path does is empty
/// # Errors
///
/// Will return `Err` if new JSON stat file is at root
/// Will return `Err` if new JSON stat file is not writtable
/// Will return `Err` if existing JSON stat file cannot be read
/// Will return `Err` if existing JSON stat file cannot be deserialised
pub fn init_device_stats(path: &Path) -> Result<HashMap<String, DeviceEntry>> {
	if path.extension().and_then(|s| s.to_str()) != Some("json") {
		let msg = format!(
			"JSON Stats: {} {}",
			path.to_str().expect("Path is empty!"),
			"is not a JSON format !"
		);
		bail!(msg);
	}

	let Some(parent_dir) = path.parent() else {
		let msg = "JSON Stats: Do not store the stat json file in rootdir !".to_string();
		bail!(msg);
	};

	if path.exists() {
		info!(
			"JSON Stats: Found existing stat file at {}",
			path.to_str().expect("No path!")
		);

		let mut file = fs::File::open(path)?;
		let mut contents = Vec::new();
		file.read_to_end(&mut contents)?;

		let stats: HashMap<String, DeviceEntry> = serde_json::from_slice(&contents)?;

		Ok(stats)
	} else {
		if !parent_dir.exists() && fs::create_dir_all(parent_dir).is_err() {
			let msg = format!(
				"JSON Stats: Path: {} {}",
				parent_dir.to_str().expect("No parent!"),
				"is unwritable !"
			);
			bail!(msg);
		}

		if fs::write(path, b".").is_err() {
			let msg = format!(
				"JSON Stats: Path: {} {}",
				parent_dir.to_str().expect("No parent!"),
				"is unwritable !"
			);
			bail!(msg);
		}

		let stats: HashMap<String, DeviceEntry> = HashMap::default();

		info!(
			"JSON Stats: New stat file will be created at {}",
			path.to_str().expect("No path!")
		);

		Ok(stats)
	}
}

/// # Errors
///
/// Will return `Err` if the write to JSON stat file fails
pub fn write_device_stats(path: &Path, stats: &HashMap<String, DeviceEntry>) -> Result<()> {
	debug!("Stats: Writing out stats to json.");
	let stats_json = serde_json::to_value(stats)?;
	let file = File::create(path)?;
	let mut writer = BufWriter::new(file);
	serde_json::to_writer(&mut writer, &stats_json)?;
	writer.flush()?;
	Ok(())
}

/// Merge volatile fields if the stat already exists in JSON, othetwise create a new entry
/// # Panics
///
/// Can be ignored as the panic condition is highly unlikely
/// # Errors
///
/// Will return `Err` if `init_device_entry` fails
pub fn merge_existing_device_stats(
	device_paths: &[String],
	stats: &mut HashMap<String, DeviceEntry>,
) -> Result<()> {
	let mut device_entries: HashMap<String, DeviceEntry> = HashMap::default();

	for device_path in device_paths {
		let mut device_entry = DeviceEntry::new(device_path);

		device_entry.init_device_entry()?;

		device_entries
			.entry(device_entry.serial_number.clone())
			.or_insert(device_entry);
	}

	for (serial_number, device_entry) in &device_entries {
		debug!(
			"Stats: Processing device {}: SN {} ",
			device_entry.current_path, serial_number
		);

		if stats.contains_key(serial_number) {
			let old_entry = stats.get_mut(serial_number).expect("WTF");

			old_entry.copy_volatile_fields(device_entry);

			let msg = format!(
				"Stats merge: Found existing stats for device {} \n: SN {}",
				old_entry.current_path, serial_number
			);
			debug!("{}", &msg);
		} else {
			stats
				.entry(device_entry.serial_number.clone())
				.or_insert(device_entry.clone());
		}
	}

	Ok(())
}

/// Update statistics in the `HashMap` with new ones from a `BlockStats` vector, returns true if they have changed
/// # Panics
///
/// Can be ignored as the panic condition is highly unlikely
/// # Errors
///
/// Will return `Err` if `update_stats` fails
pub fn update_devices_stats(
	current_block_stats: &[(String, Option<BlockStats>)],
	stats: &mut HashMap<String, DeviceEntry>,
) -> Result<bool> {
	let mut stats_have_changed = false;

	for (serial_number, current_stat) in current_block_stats {
		if current_stat.is_none() {
			error!("Stats: Error reading new stats for {serial_number}, skipping !");
			continue;
		}

		if stats.contains_key(serial_number) {
			let entry = stats.get_mut(serial_number).expect("WTF");

			let stat = current_stat.clone().expect("Could not clone BlockStats");

			stats_have_changed |= entry.update_stats(stat)?;

			let msg = format!(
				"Stats: Update stats for device {} SN: {}",
				entry.current_path, serial_number
			);
			debug!("{}", &msg);
		}
	}

	Ok(stats_have_changed)
}
