use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::{collections::HashMap, os::unix::fs::MetadataExt};

use anyhow::Result;
use fs_err as fs;

use serial_test::serial;

use tokio::{sync::Mutex, task, time::Duration};
use tokio_util::sync::CancellationToken;

use krillkounter::{config::DaemonConfig, daemon::KrillKounter, device::DeviceEntry};

#[path = "common/mod.rs"]
mod common;

#[derive(Debug, Default)]
struct TestRig {
	app: Arc<Mutex<KrillKounter>>,
	handle: Option<task::JoinHandle<Result<()>>>,
	cancellation_token: CancellationToken,
	stats_file_path: PathBuf,
	update_rate: u64,
}

impl TestRig {
	async fn new() -> Self {
		common::init_env();

		// Get and verify the env variables, if the env variables are wrong all testing is worth nothing..
		let stats_path = common::get_stats();
		let dev = common::get_dev();

		let update_rate = 5;

		let config_file_path =
			common::generate_config_file(vec![dev], stats_path.clone(), update_rate).unwrap();

		let krill_config = DaemonConfig {
			config_file_path: config_file_path
				.to_str()
				.expect("Invalid config path!")
				.to_string(),
			stats_file_path: stats_path.clone(),
			update_rate: update_rate,
		};

		let app = KrillKounter::init(&krill_config).expect("Daemon did not start!");

		tokio::time::sleep(Duration::from_millis(100)).await;

		Self {
			cancellation_token: CancellationToken::new(),
			app: Arc::new(Mutex::new(app)),
			handle: None,
			stats_file_path: PathBuf::from(stats_path),
			update_rate: update_rate,
		}
	}

	async fn start(&mut self) -> () {
		let app = Arc::clone(&self.app);
		let token = self.cancellation_token.clone();
		self.handle = Some(task::spawn(async move {
			let mut app = app.lock().await;
			app.run(token).await
		}));
	}

	async fn stop(self) -> Result<()> {
		self.cancellation_token.cancel();
		Ok(())
	}

	async fn wait(&self) -> () {
		tokio::time::sleep(Duration::from_secs(self.update_rate * 3)).await;
	}

	async fn clean(&self) -> () {
		let dev_mount = common::get_dev_mount();
		let stats_path = common::get_stats();

		let write_command = format!("{} {}{}", "rm -f ", dev_mount, "/dummy_file_test_*");
		self.run_system_command(&write_command)
			.await
			.expect("Failed to delete the dummy file!");

		let write_command = format!("{} {}", "rm -f ", stats_path);
		self.run_system_command(&write_command)
			.await
			.expect("Failed to delete the stat file!");

		let write_command = format!("{}", "sync");
		self.run_system_command(&write_command)
			.await
			.expect("Failed to sync!");
	}

	async fn run_system_command(&self, cmd: &str) -> Result<String> {
		let output = tokio::process::Command::new("sh")
			.arg("-c")
			.arg(cmd)
			.output()
			.await?;

		Ok(String::from_utf8_lossy(&output.stdout).to_string())
	}

	async fn test_bytes_written(&self, bytes: u128, units: &str) -> () {
		let dev_mount = common::get_dev_mount();

		let initial_stats = self.read_daemon_stats().unwrap();

		let initial_bytes = self.read_block_device_write_sectors() as u128 * common::SECTOR_SIZE;

		assert!(units == "M" || units == "k");

		let write_command = format!(
			"{}{}{}{} {}{} {}{}",
			"dd if=/dev/urandom of=",
			dev_mount,
			"/dummy_file_test_",
			bytes,
			"bs=1",
			units,
			"count=",
			bytes,
		);

		// Write bytes to the SD card
		self.run_system_command(&write_command)
			.await
			.expect("Failed to create the dummy file!");

		let write_command = format!("{}", "sync");

		self.run_system_command(&write_command)
			.await
			.expect("Failed to sync!");

		self.wait().await;

		let new_stats = self.read_daemon_stats().unwrap();

		let post_bytes = self.read_block_device_write_sectors() as u128 * common::SECTOR_SIZE;

		let bytes_written = new_stats.total_bytes_written - initial_stats.total_bytes_written;
		let bytes_written_theory = post_bytes - initial_bytes;

		let mut diff =
			(bytes_written as i128 - bytes_written_theory as i128) / common::SECTOR_SIZE as i128;

		diff = diff.abs();

		self.clean().await;

		assert!(
			diff < 2 as i128,
			"Sector difference larger than 1 sectors, Wrote {} but app logged {}, diff is {} in sectors {}",
			bytes_written_theory,
			bytes_written,
			diff * common::SECTOR_SIZE as i128,
			diff,
		);

		println!(
			"{}{}{}",
			"The write difference was ", diff, " sectors and below 10%."
		);
	}

	fn read_daemon_stats(&self) -> Result<DeviceEntry> {
		let mut file = fs::File::open(&self.stats_file_path)?;
		let mut contents = Vec::new();
		file.read_to_end(&mut contents)?;

		let stats: HashMap<String, DeviceEntry> = serde_json::from_slice(&contents)?;
		assert_eq!(stats.len(), 1);

		let device_entry = stats.values().next().expect("No entries found!").clone();
		Ok(device_entry)
	}

	fn read_block_device_write_sectors(&self) -> u64 {
		let dev = common::get_dev();
		let dev_stripped = dev
			.strip_prefix("/dev/")
			.ok_or(anyhow::anyhow!(" no /dev prefix !"))
			.expect("No dev name !")
			.to_string();
		let dev_stat_path = format!("{}{}{}", "/sys/block/", &dev_stripped, "/stat");
		let path: &Path = Path::new(&dev_stat_path);

		assert!(
			path.exists(),
			"Path: {} does not exist - check your .env file",
			dev_stat_path,
		);

		let file = fs::read_to_string(path).unwrap();
		let contents: Vec<String> = file.split_whitespace().map(String::from).collect();

		// write secotrs are at index 6
		let write_sectors = contents[6].trim().parse::<u64>().unwrap_or_default();

		write_sectors
	}
}

#[tokio::test]
#[serial]
async fn test_krill_kounter_writes_1_mb() {
	println!("Caution this test is not really scientific, just a lazy effort.");
	let mut test = TestRig::new().await;
	test.start().await;

	test.wait().await;

	test.test_bytes_written(2, "M").await;

	test.stop().await.expect("Failed to stop the daemon!");
}

#[tokio::test]
#[serial]
async fn test_krill_kounter_writes_8_mb() {
	println!("Caution this test is not really scientific, just a lazy effort.");
	let mut test = TestRig::new().await;
	test.start().await;

	test.wait().await;

	test.test_bytes_written(8, "M").await;

	test.stop().await.expect("Failed to stop the daemon!");
}

#[tokio::test]
#[serial]
async fn test_krill_kounter_writes_42_kb() {
	println!("Caution this test is not really scientific, just a lazy effort.");
	let mut test = TestRig::new().await;
	test.start().await;

	test.wait().await;

	test.test_bytes_written(42, "k").await;

	test.stop().await.expect("Failed to stop the daemon!");
}

#[tokio::test]
#[serial]
async fn test_krill_kounter_writes_128_kb() {
	println!("Caution this test is not really scientific, just a lazy effort.");
	let mut test = TestRig::new().await;
	test.start().await;

	test.wait().await;

	test.test_bytes_written(128, "k").await;

	test.stop().await.expect("Failed to stop the daemon!");
}
