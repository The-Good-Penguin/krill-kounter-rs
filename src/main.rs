use anyhow::Result;
use clap::Parser;
use log::info;

use tokio::{
	select,
	signal::unix::{signal, SignalKind},
	task,
};
use tokio_util::sync::CancellationToken;

use krillkounter::{
	config::{Args, DaemonConfig},
	daemon::KrillKounter,
};

#[tokio::main]
async fn main() -> Result<()> {
	let args = Args::parse();

	let krill_config = DaemonConfig::new(args)?;
	let mut krill_kounter = KrillKounter::init(&krill_config)?;

	let cancellation_token = CancellationToken::new();
	let cancellation_clone = cancellation_token.clone();

	let handle = task::spawn(async move { krill_kounter.run(cancellation_token).await });

	wait_for_shutdown_signal(cancellation_clone).await?;
	let _ = handle.await?;

	info!("Shutting down Krill Kounter daemon");

	Ok(())
}

async fn wait_for_shutdown_signal(cancellation_token: CancellationToken) -> Result<()> {
	let mut sig_term = signal(SignalKind::terminate())?;
	let mut sig_int = signal(SignalKind::interrupt())?;

	select! {
		_ = sig_term.recv() => {
			info!("received terminate signal, quitting !");
			cancellation_token.cancel();
			Ok(())
		}
		_ = sig_int.recv() => {
			info!("received interrupt signal, quitting !");
			cancellation_token.cancel();
			Ok(())
		}
	}
}
