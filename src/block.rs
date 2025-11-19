use std::ops::{AddAssign, Sub};

use anyhow::Result;
use log::info;

use fs_err as fs;

use serde::{Deserialize, Serialize};

use crate::utilities::LogPrefix;
use crate::{info_self, string_to_u64_or_default};

/// The `BlockStats` is based on information found at <https://elixir.bootlin.com/linux/v6.17.4/source/block/genhd.c#L1075>
/// for laziness reason we store all of the values as u64
#[derive(Deserialize, Serialize, Clone, Default, PartialEq, PartialOrd, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BlockStats {
	pub read_io: u64,
	pub read_merges: u64,
	pub read_sectors: u64,
	pub read_ticks: u64,
	pub write_io: u64,
	pub write_merges: u64,
	pub write_sectors: u64,
	pub write_ticks: u64,
	pub in_flight: u64,
	pub io_ticks: u64,
	pub time_in_queue: u64,
	pub discard_io: u64,
	pub discard_merges: u64,
	pub discard_sectors: u64,
	pub discard_ticks: u64,
	pub flush_io: u64,
	pub flush_ticks: u64,
}

impl BlockStats {
	/// Returns a `BlockStats` object populated with current sysfs stats
	/// # Errors
	///
	/// Will return `Err` if sysfs stats path does not exist
	pub fn new(path: &String) -> Result<Self> {
		let mut block_stats = Self {
			..Default::default()
		};
		block_stats.get_current_stats(path)?;
		Ok(block_stats)
	}

	/// Get current sysfs stats
	/// # Errors
	///
	/// Will return `Err` if sysfs statspath does not exist
	pub fn get_current_stats(&mut self, path: &String) -> Result<()> {
		let file = fs::read_to_string(path)?;
		let contents: Vec<String> = file.split_whitespace().map(String::from).collect();

		/*
		Convert to struct, I am sure that there is more clever way,
		but obstructing it with a macro does not feel right
		the stat file is a single file separated by whitespace
		with new values appended at the end with the next kernel versions
		*/
		for (index, value) in contents.iter().enumerate() {
			match index {
				0 => self.read_io = string_to_u64_or_default!(value),
				1 => self.read_merges = string_to_u64_or_default!(value),
				2 => self.read_sectors = string_to_u64_or_default!(value),
				3 => self.read_ticks = string_to_u64_or_default!(value),
				4 => self.write_io = string_to_u64_or_default!(value),
				5 => self.write_merges = string_to_u64_or_default!(value),
				6 => self.write_sectors = string_to_u64_or_default!(value),
				7 => self.write_ticks = string_to_u64_or_default!(value),
				8 => self.in_flight = string_to_u64_or_default!(value),
				9 => self.io_ticks = string_to_u64_or_default!(value),
				10 => self.time_in_queue = string_to_u64_or_default!(value),
				11 => self.discard_io = string_to_u64_or_default!(value),
				12 => self.discard_merges = string_to_u64_or_default!(value),
				13 => self.discard_sectors = string_to_u64_or_default!(value),
				14 => self.discard_ticks = string_to_u64_or_default!(value),
				15 => self.flush_io = string_to_u64_or_default!(value),
				16 => self.flush_ticks = string_to_u64_or_default!(value),
				_ => {}
			}
		}

		Ok(())
	}

	/// Store and accumulate the difference between current and previous stats
	pub fn accumulate_stats(&mut self, current_stats: BlockStats, prev_stats: BlockStats) {
		if prev_stats > current_stats {
			info_self!(
				self,
				"Prev stats larger than current stats, this should not happen !"
			);
		}
		*self += current_stats - prev_stats;
	}
}

impl AddAssign for BlockStats {
	fn add_assign(&mut self, rhs: Self) {
		*self = Self {
			read_io: self.read_io + rhs.read_io,
			read_merges: self.read_merges + rhs.read_merges,
			read_sectors: self.read_sectors + rhs.read_sectors,
			read_ticks: self.read_ticks + rhs.read_ticks,
			write_io: self.write_io + rhs.write_io,
			write_merges: self.write_merges + rhs.write_merges,
			write_sectors: self.write_sectors + rhs.write_sectors,
			write_ticks: self.write_ticks + rhs.write_ticks,
			in_flight: self.in_flight + rhs.in_flight,
			io_ticks: self.io_ticks + rhs.io_ticks,
			time_in_queue: self.time_in_queue + rhs.time_in_queue,
			discard_io: self.discard_io + rhs.discard_io,
			discard_merges: self.discard_merges + rhs.discard_merges,
			discard_sectors: self.discard_sectors + rhs.discard_sectors,
			discard_ticks: self.discard_ticks + rhs.discard_ticks,
			flush_io: self.flush_io + rhs.flush_io,
			flush_ticks: self.flush_ticks + rhs.flush_ticks,
		};
	}
}

impl Sub for BlockStats {
	type Output = Self;

	fn sub(self, rhs: Self) -> Self::Output {
		Self {
			read_io: self.read_io - rhs.read_io,
			read_merges: self.read_merges - rhs.read_merges,
			read_sectors: self.read_sectors - rhs.read_sectors,
			read_ticks: self.read_ticks - rhs.read_ticks,
			write_io: self.write_io - rhs.write_io,
			write_merges: self.write_merges - rhs.write_merges,
			write_sectors: self.write_sectors - rhs.write_sectors,
			write_ticks: self.write_ticks - rhs.write_ticks,
			in_flight: self.in_flight - rhs.in_flight,
			io_ticks: self.io_ticks - rhs.io_ticks,
			time_in_queue: self.time_in_queue - rhs.time_in_queue,
			discard_io: self.discard_io - rhs.discard_io,
			discard_merges: self.discard_merges - rhs.discard_merges,
			discard_sectors: self.discard_sectors - rhs.discard_sectors,
			discard_ticks: self.discard_ticks - rhs.discard_ticks,
			flush_io: self.flush_io - rhs.flush_io,
			flush_ticks: self.flush_ticks - rhs.flush_ticks,
		}
	}
}

impl LogPrefix for BlockStats {
	fn log_prefix(&self) -> String {
		"BlockStats: ".to_string()
	}

	fn log(&self, msg: &str) -> String {
		format!("{}: {}", self.log_prefix(), msg)
	}
}
