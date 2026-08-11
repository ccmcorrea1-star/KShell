//! Battery status source backed by Linux power-supply sysfs.

use std::fs;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BatteryStatus {
    pub percent: u8,
    pub charging: bool,
}

pub fn read() -> Option<BatteryStatus> {
    let entries = fs::read_dir("/sys/class/power_supply").ok()?;
    let mut capacities = Vec::new();
    let mut charging = false;

    for entry in entries.flatten() {
        let path = entry.path();
        if !fs::read_to_string(path.join("type"))
            .map(|kind| kind.trim().eq_ignore_ascii_case("battery"))
            .unwrap_or(false)
        {
            continue;
        }

        let Some(capacity) = fs::read_to_string(path.join("capacity"))
            .ok()
            .and_then(|value| parse_capacity(&value))
        else {
            continue;
        };
        capacities.push(capacity);
        charging |= fs::read_to_string(path.join("status"))
            .map(|status| {
                matches!(
                    status.trim().to_ascii_lowercase().as_str(),
                    "charging" | "full"
                )
            })
            .unwrap_or(false);
    }

    if capacities.is_empty() {
        return None;
    }

    let percent = capacities
        .iter()
        .map(|&value| u32::from(value))
        .sum::<u32>()
        / u32::try_from(capacities.len()).ok()?;
    Some(BatteryStatus {
        percent: u8::try_from(percent).unwrap_or(100),
        charging,
    })
}

fn parse_capacity(value: &str) -> Option<u8> {
    let capacity = value.trim().parse::<u16>().ok()?;
    u8::try_from(capacity.min(100)).ok()
}

#[cfg(test)]
mod tests {
    use super::parse_capacity;

    #[test]
    fn clamps_capacity_to_a_percentage() {
        assert_eq!(parse_capacity("87\n"), Some(87));
        assert_eq!(parse_capacity("120"), Some(100));
        assert_eq!(parse_capacity("unknown"), None);
    }
}
