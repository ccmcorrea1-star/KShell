use std::mem::MaybeUninit;
use std::time::{SystemTime, UNIX_EPOCH};

const WEEKDAYS: [&str; 7] = ["dom", "seg", "ter", "qua", "qui", "sex", "sáb"];
const MONTHS: [&str; 12] = [
    "jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClockText {
    pub date: String,
    pub time: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LocalParts {
    weekday: i32,
    day: i32,
    month: i32,
    hour: i32,
    minute: i32,
}

pub fn now() -> ClockText {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    local_parts(seconds as libc::time_t)
        .and_then(format_parts)
        .unwrap_or_else(fallback)
}

fn local_parts(timestamp: libc::time_t) -> Option<LocalParts> {
    let mut local = MaybeUninit::<libc::tm>::uninit();
    let result = unsafe { libc::localtime_r(&timestamp, local.as_mut_ptr()) };
    if result.is_null() {
        return None;
    }

    let local = unsafe { local.assume_init() };
    Some(LocalParts {
        weekday: local.tm_wday,
        day: local.tm_mday,
        month: local.tm_mon,
        hour: local.tm_hour,
        minute: local.tm_min,
    })
}

fn format_parts(parts: LocalParts) -> Option<ClockText> {
    let weekday = WEEKDAYS.get(usize::try_from(parts.weekday).ok()?)?;
    let month = MONTHS.get(usize::try_from(parts.month).ok()?)?;
    if !(1..=31).contains(&parts.day)
        || !(0..=23).contains(&parts.hour)
        || !(0..=59).contains(&parts.minute)
    {
        return None;
    }

    Some(ClockText {
        date: format!("{weekday} {:02} {month}", parts.day),
        time: format!("{:02}:{:02}", parts.hour, parts.minute),
    })
}

fn fallback() -> ClockText {
    ClockText {
        date: "-- -- ---".to_owned(),
        time: "--:--".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{format_parts, ClockText, LocalParts};

    #[test]
    fn formats_portuguese_date_and_time_like_the_mockup() {
        assert_eq!(
            format_parts(LocalParts {
                weekday: 0,
                day: 9,
                month: 7,
                hour: 13,
                minute: 12,
            }),
            Some(ClockText {
                date: "dom 09 ago".to_owned(),
                time: "13:12".to_owned(),
            })
        );
    }

    #[test]
    fn rejects_invalid_calendar_components() {
        assert!(format_parts(LocalParts {
            weekday: 0,
            day: 32,
            month: 7,
            hour: 13,
            minute: 12,
        })
        .is_none());
    }
}
