use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

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

		let initial_stats = self.read_daemon_stats().await.unwrap();

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

		let new_stats = self.read_daemon_stats().await.unwrap();

		let bytes_written = new_stats.total_bytes_written - initial_stats.total_bytes_written;

		let bytes_written_theory: u128 = match units {
			"M" => bytes * 1024 * 1024,
			"k" => bytes * 1024,
			&_ => todo!(),
		};

		let diff =
			(bytes_written as i128 - bytes_written_theory as i128) / common::SECTOR_SIZE as i128;

		self.clean().await;

		assert!(
			diff.abs() <= ((bytes_written_theory/common::SECTOR_SIZE) as f64 *0.1) as i128,
			"Sector difference larger than 10%, Wrote {} but app logged {}, diff is {} in sectors {}",
			bytes_written_theory,
			bytes_written,
			diff*common::SECTOR_SIZE as i128,
			diff,
		);

		println!(
			"{}{}{}",
			"The write difference was ", diff, " sectors and below 10%."
		);
	}

	async fn read_daemon_stats(&self) -> Result<DeviceEntry> {
		let mut file = fs::File::open(&self.stats_file_path)?;
		let mut contents = Vec::new();
		file.read_to_end(&mut contents)?;

		let stats: HashMap<String, DeviceEntry> = serde_json::from_slice(&contents)?;
		assert_eq!(stats.len(), 1);

		let device_entry = stats.values().next().expect("No entries found!").clone();
		Ok(device_entry)
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
