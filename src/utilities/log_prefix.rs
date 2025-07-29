pub trait LogPrefix {
	fn log_prefix(&self) -> String;

	fn log(&self, msg: &str) -> String {
		format!("{}: {}", self.log_prefix(), msg)
	}
}
