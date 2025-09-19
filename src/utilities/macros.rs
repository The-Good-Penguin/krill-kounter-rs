#[macro_export]
macro_rules! string_to_u64_result {
	($string:expr) => {{
		let result = $string.trim().parse::<u64>()?;
		result
	}};
}

#[macro_export]
macro_rules! string_to_u64_or_default {
	($string:expr) => {{
		let result = $string.trim().parse::<u64>().unwrap_or_default();
		result
	}};
}

#[macro_export]
macro_rules! file_content_to_string {
	($file:expr) => {{
		let mut string = String::new();
		$file.read_to_string(&mut string)?;
		let string_cleaned = string.trim().to_string();
		string_cleaned
	}};
}

// There must be a more clever way of doing this ?
#[macro_export]
macro_rules! log_self {
	($self:expr, $($arg:tt)*) => {
		$self.log(&format!($($arg)*))
	};
}

#[macro_export]
macro_rules! debug_self {
	($self:expr, $($arg:tt)*) => {
		debug!("{}", $self.log(&format!($($arg)*)))
	};
}

#[macro_export]
macro_rules! info_self {
	($self:expr, $($arg:tt)*) => {
		info!("{}", $self.log(&format!($($arg)*)))
	};
}

#[macro_export]
macro_rules! error_self {
	($self:expr, $($arg:tt)*) => {
		error!("{}", $self.log(&format!($($arg)*)))
	};
}

#[macro_export]
macro_rules! bail_self {
	($self:expr, $($arg:tt)*) => {
		bail!("{}", $self.log(&format!($($arg)*)))
	};
}
