use std::process::Command;

use super::{
    DiskVolumeInformation, SystemInformation, SystemInformationError, SystemInformationErrors,
    SystemInformationResponse,
};

const MEM_TOTAL_PREFIX: &str = "MemTotal";
const MEM_AVAILABLE_PREFIX: &str = "MemAvailable";

pub fn collect_system_information() -> SystemInformationResponse {
    let mut errors = SystemInformationErrors::default();

    let (memory_maximum, memory_current) = match collect_memory_information() {
        Ok(values) => values,
        Err(error) => {
            errors.memory_information_error = Some(error);
            (None, None)
        }
    };

    let (file_descriptor_maximum, file_descriptor_current) = match collect_file_descriptor_information() {
        Ok(values) => values,
        Err(error) => {
            errors.file_descriptors_information_error = Some(error);
            (None, None)
        }
    };

    let (os_system, host_name, os_version, os_release, architecture) = match collect_os_information() {
        Ok(values) => values,
        Err(error) => {
            errors.operating_system_information_error = Some(error);
            (None, None, None, None, None)
        }
    };

    let disk = match collect_disk_information() {
        Ok(disks) => Some(disks),
        Err(error) => {
            errors.disk_volume_information_error = Some(error);
            None
        }
    };

    SystemInformationResponse {
        info: Some(SystemInformation {
            memory_maximum,
            memory_current,
            file_descriptor_maximum,
            file_descriptor_current,
            os_system,
            host_name,
            os_version,
            os_release,
            architecture,
            cloudflared_version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            go_version: Some(rustc_version()),
            go_arch: Some(crate::environment::TARGET_ARCH.to_owned()),
            disk,
        }),
        // Match Go's successful shape quirk: `errors` is serialized as `{}`.
        errors: Some(errors),
    }
}

fn collect_memory_information() -> Result<(Option<u64>, Option<u64>), SystemInformationError> {
    let raw = std::fs::read_to_string("/proc/meminfo").map_err(|error| SystemInformationError {
        error: format!("error reading /proc/meminfo: {error}"),
        raw_info: String::new(),
    })?;

    parse_memory_information(&raw)
        .map(|(maximum, current)| (Some(maximum), Some(current)))
        .map_err(|error| SystemInformationError { error, raw_info: raw })
}

fn collect_file_descriptor_information() -> Result<(Option<u64>, Option<u64>), SystemInformationError> {
    let raw = command_stdout("sysctl", &["-n", "fs.file-nr"]).map_err(|error| SystemInformationError {
        error,
        raw_info: String::new(),
    })?;

    parse_file_descriptor_information(&raw)
        .map(|(maximum, current)| (Some(maximum), Some(current)))
        .map_err(|error| SystemInformationError { error, raw_info: raw })
}

fn collect_disk_information() -> Result<Vec<DiskVolumeInformation>, SystemInformationError> {
    let raw = command_stdout("df", &["-k"]).map_err(|error| SystemInformationError {
        error,
        raw_info: String::new(),
    })?;

    parse_disk_information(&raw).map_err(|error| SystemInformationError { error, raw_info: raw })
}

type OsInfo = (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
);

fn collect_os_information() -> Result<OsInfo, SystemInformationError> {
    let raw = command_stdout("uname", &["-a"]).map_err(|error| SystemInformationError {
        error,
        raw_info: String::new(),
    })?;

    parse_uname_information(&raw).map_err(|error| SystemInformationError { error, raw_info: raw })
}

fn command_stdout(command: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(command)
        .args(args)
        .output()
        .map_err(|error| format!("error retrieving output from command '{command}': {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "error retrieving output from command '{command}': exit {}",
            output.status
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_memory_information(raw: &str) -> Result<(u64, u64), String> {
    let total = find_kib_key(raw, MEM_TOTAL_PREFIX)?;
    let available = find_kib_key(raw, MEM_AVAILABLE_PREFIX)?;
    Ok((total, total.saturating_sub(available)))
}

fn find_kib_key(raw: &str, prefix: &str) -> Result<u64, String> {
    let line = raw
        .lines()
        .find(|line| line.starts_with(prefix))
        .ok_or_else(|| format!("parsing memory information: key not found, key={prefix}"))?;
    let value = line
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| format!("parsing memory information: key not found, key={prefix}"))?;

    value
        .parse::<u64>()
        .map_err(|error| format!("error parsing memory field '{value}': {error}"))
}

fn parse_file_descriptor_information(raw: &str) -> Result<(u64, u64), String> {
    let fields: Vec<&str> = raw.split_whitespace().collect();
    if fields.len() != 3 {
        return Err(format!(
            "expected file descriptor information to have 3 fields got {}: insufficient fields",
            fields.len()
        ));
    }

    let current = fields[0]
        .parse::<u64>()
        .map_err(|error| format!("error parsing files current field '{}': {error}", fields[0]))?;
    let maximum = fields[2]
        .parse::<u64>()
        .map_err(|error| format!("error parsing files max field '{}': {error}", fields[2]))?;

    Ok((maximum, current))
}

fn parse_disk_information(raw: &str) -> Result<Vec<DiskVolumeInformation>, String> {
    let mut disks = Vec::new();

    for line in raw.lines().skip(1) {
        if line.trim().is_empty() {
            continue;
        }

        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 3 {
            return Err(format!(
                "expected disk volume to have 3 fields got {}: insufficient fields",
                fields.len()
            ));
        }

        let size_maximum = match fields[1].parse::<u64>() {
            Ok(value) => value,
            Err(_) => continue,
        };
        let size_current = match fields[2].parse::<u64>() {
            Ok(value) => value,
            Err(_) => continue,
        };

        disks.push(DiskVolumeInformation {
            name: fields[0].to_owned(),
            size_maximum,
            size_current,
        });
    }

    if disks.is_empty() {
        return Err("no disk volume information found".to_owned());
    }

    Ok(disks)
}

fn parse_uname_information(raw: &str) -> Result<OsInfo, String> {
    let fields: Vec<&str> = raw.split_whitespace().collect();
    if fields.len() < 6 {
        return Err(format!(
            "expected system information to have 6 fields got {}: insufficient fields",
            fields.len()
        ));
    }

    let architecture_index = fields.len().saturating_sub(2);
    Ok((
        Some(fields[0].to_owned()),
        Some(fields[1].to_owned()),
        Some(fields[2].to_owned()),
        Some(fields[3..architecture_index].join(" ")),
        Some(fields[architecture_index].to_owned()),
    ))
}

fn rustc_version() -> String {
    match Command::new("rustc").arg("-V").output() {
        Ok(output) if output.status.success() => String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        _ => "rust".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_memory_information_matches_go_shape() {
        let raw = "MemTotal:       1024 kB\nMemAvailable:    256 kB\n";
        let (maximum, current) = parse_memory_information(raw).expect("parse meminfo");
        assert_eq!(maximum, 1024);
        assert_eq!(current, 768);
    }

    #[test]
    fn parse_file_descriptor_information_matches_go_shape() {
        let raw = "512\t0\t1024\n";
        let (maximum, current) = parse_file_descriptor_information(raw).expect("parse fds");
        assert_eq!(maximum, 1024);
        assert_eq!(current, 512);
    }

    #[test]
    fn parse_disk_information_matches_go_shape() {
        let raw = "Filesystem 1K-blocks Used Available Use% Mounted on\n/dev/sda1 100 50 50 50% /\n";
        let disks = parse_disk_information(raw).expect("parse disks");
        assert_eq!(disks[0].name, "/dev/sda1");
        assert_eq!(disks[0].size_maximum, 100);
        assert_eq!(disks[0].size_current, 50);
    }

    #[test]
    fn parse_uname_information_matches_go_shape() {
        let raw = "Linux test-host 6.8.0-1 #1 SMP PREEMPT_DYNAMIC x86_64 GNU/Linux\n";
        let info = parse_uname_information(raw).expect("parse uname");
        assert_eq!(info.0.as_deref(), Some("Linux"));
        assert_eq!(info.1.as_deref(), Some("test-host"));
        assert_eq!(info.2.as_deref(), Some("6.8.0-1"));
        assert_eq!(info.4.as_deref(), Some("x86_64"));
    }

    #[test]
    fn collect_system_information_returns_wrapper() {
        let response = collect_system_information();
        assert!(response.info.is_some());
        assert!(response.errors.is_some());
    }
}
