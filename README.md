KrillKounter-rs
-------------

# Description
A lightweight utility for monitoring and logging block device stats to a JSON file at a regular time interval. The aim is to have a single and fairly lightweight binary for monitoring the accumualated usage for wear and tear on embedded devices that are running Linux.

This is a `Rust` rewrite of existing `C++` project, but as it was a small codebase it was actually possible.

# Dependencies
- See [Cargo.toml](Cargo.toml)
- lsblk-2.37.2 might be required for correct detection of USB SD card adapters.

# Compilation
1. Clone this repo to a local directory using the following command:
 ```bash
 git clone https://github.com/The-Good-Penguin/krill-kounter-rs.git
 ```
2. From inside the local repo directory, run the following command to build KrillKounter:
 ```bash
 make build
 ```

# Installation
1. From inside the local repo directory, run the following command:
 ```bash
 sudo make install
 ```

2. KrillKounter-rs can be configured by editing `Environment` values within
 `/lib/systemd/system/krill-kounter-rs.service`:

- `KK_CONFIG_JSON_PATH` - path to the JSON file to be used for configuring the daemon -
one needs to create this file for krill-kounter-rs binary to start.

- `KRILL_KOUNTER_LOG` - the desired dbeug level


3. Enable and start the KrillKounter systemd service using the following command:
 ```bash
 systemctl enable --now krill-kounter-rs.service
 ```
 The state of the KrillKounter service can be monitored using the command:
 ```bash
 systemctl status krill-kounter-rs
 ```

## Config File Format
```JSON
{
    "devicePaths": [
        "/dev/sda",
        "/dev/mmcblk0"
    ],
    "updateRate": 3600,
    "statsFilePath": "/usr/share/KrillKounter/stats.json"
}
```
- `devicePaths` - an array of device paths you wish to monitor under the device node.
- `updateRate` - time interval to wait between JSON file updates, in seconds.
- `statsFilePath` - the destination file for storing the block statistics.

# Contributing
Before contributing run the tests using `cargo test`, for that you need to rename `.env_sanmple` to `.env` and fill out the following variables:


- `TEST_STATS_JSON_PATH` - path where the temporary stat file will be created.

- `TEST_CONFIG_MMCBLK_DEV`- /dev/ node path to the DUT ie. an actual block device.

- `TEST_CONFIG_MMCBLK_DEV_MOUNT` - mount point of the above block device.

- `TEST_CONFIG_MMCBLK_SERIAL` - known serial of the above block device.

- `KRILL_KOUNTER_LOG` - the desired debug level.


Furthermore initialise the `pre-commit`[1] hooks by issuing the following commands:

```bash
pip install pre-commit
pre-commit install
```

Then you can commit your changes and issue a PR via github.

# Security
If you believe you have found a security vulnerability, please submit your report to <security@thegoodpenguin.co.uk>

# License
Licensed under MIT

# Maintainer
Pawel Zalewski <pzalewski@thegoodpenguin.co.uk>

[1] https://pre-commit.com/
