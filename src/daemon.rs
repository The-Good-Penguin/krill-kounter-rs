use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use env_logger::{Builder, Env};
use futures::future::join_all;
use log::{error, info};

use tokio::{select, task, time};
use tokio_util::sync::CancellationToken;

use crate::config::{DaemonConfig, DeviceConfig};
use crate::device::{
	init_device_stats, merge_existing_device_stats, update_devices_stats, write_device_stats,
	DeviceEntry,
};
use crate::utilities::LogPrefix;

#[derive(Default, Debug)]
pub struct KrillKounter {
	update_rate: u64,
	stats: HashMap<String, DeviceEntry>,
	stats_file_path: String,
}

impl KrillKounter {
	/// Returns a `KrillKounter` object and tries to merge old stats if they exist
	/// # Errors
	///
	/// Will return `Err` if `DeviceConfig` cannot be initialized
	/// Will return `Err` if `init_device_stats` fails
	/// Will return `Err` if `merge_existing_device_stats` fails
	pub fn init(config: &DaemonConfig) -> Result<Self> {
		let env = Env::new().filter_or("KRILL_KOUNTER_LOG", "info");

		let _ = Builder::from_env(env).try_init();

		let device_config = DeviceConfig::new(Path::new(&config.config_file_path))?;

		let mut stats = init_device_stats(Path::new(&config.stats_file_path))?;

		let update_rate = config.update_rate;
		let stats_file_path = config.stats_file_path.clone();

		merge_existing_device_stats(&device_config.device_paths, &mut stats)?;

		info!("Starting Krill Kounter daemon with update rate of {update_rate}s");

		Ok(Self {
			update_rate,
			stats,
			stats_file_path,
		})
	}

	/// Main loop of the application that calls into `update_stats`
	/// # Errors
	///
	/// Will return `Err` if `update_stats` fails.
	pub async fn run(&mut self, cancellation_token: CancellationToken) -> Result<()> {
		let mut interval = time::interval(time::Duration::from_secs(self.update_rate));

		loop {
			select! {
				() = cancellation_token.cancelled() => {break Ok(());},
				_ = interval.tick() => {

					self.update_stats().await?;
				}
			}
		}
	}

	/// Updates the statistics for every active `DeviceEntry`, spawns a tokio task for each path
	async fn update_stats(&mut self) -> Result<bool> {
		/*
		We want to read the stats in parallel, wait for results, then modify the shared mut buffer after join
		but to do that we need to use fields from the said shared mut buffer which in turn requires a clone
		to transfer the ownership into these tasks
		*/
		let tasks: Vec<_> = self
			.stats
			.iter()
			.filter(|(_serial, device_entry)| device_entry.is_active)
			.map(|(serial, device_entry)| {
				let mut entry = device_entry.clone();
				let serial = serial.clone();

				task::spawn_blocking(move || {
					// We check for None in update_devices_stats below, hence it is ok to use ok() here to convert from Result to Option.
					let current_stat = entry.get_new_stats().ok();
					(serial, current_stat)
				})
			})
			.collect();

		let results = join_all(tasks)
			.await
			.into_iter()
			.filter_map(|res| match res {
				Ok(value) => Some(value),
				Err(err) => {
					error!("Task has panicked {err}");
					None
				}
			})
			.collect::<Vec<_>>();

		let ret = update_devices_stats(&results, &mut self.stats)?;

		if ret {
			let path = Path::new(&self.stats_file_path);
			write_device_stats(path, &self.stats)?;
		}

		Ok(ret)
	}
}

impl LogPrefix for KrillKounter {
	fn log_prefix(&self) -> String {
		"KrillKounter: ".to_string()
	}

	fn log(&self, msg: &str) -> String {
		format!("{}: {}", self.log_prefix(), msg)
	}
}
