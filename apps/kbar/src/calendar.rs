use std::mem::MaybeUninit;
use std::time::{SystemTime, UNIX_EPOCH};

pub const GRID_SIZE: usize = 42;

pub const WEEKDAYS: [&str; 7] = ["dom", "seg", "ter", "qua", "qui", "sex", "sáb"];

const MONTHS: [&str; 12] = [
    "janeiro",
    "fevereiro",
    "março",
    "abril",
    "maio",
    "junho",
    "julho",
    "agosto",
    "setembro",
    "outubro",
    "novembro",
    "dezembro",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

impl Date {
    pub const fn new(year: i32, month: u32, day: u32) -> Self {
        Self { year, month, day }
    }

    pub const fn start_of_month(self) -> Self {
        Self::new(self.year, self.month, 1)
    }

    pub fn previous_month(self) -> Self {
        if self.month == 1 {
            Self::new(self.year - 1, 12, 1)
        } else {
            Self::new(self.year, self.month - 1, 1)
        }
    }

    pub fn next_month(self) -> Self {
        if self.month == 12 {
            Self::new(self.year + 1, 1, 1)
        } else {
            Self::new(self.year, self.month + 1, 1)
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CalendarDay {
    pub date: Date,
    pub current_month: bool,
}

pub fn today() -> Option<Date> {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    local_date(seconds as libc::time_t)
}

pub fn month_title(month: Date) -> String {
    let month_name = MONTHS
        .get(month.month.saturating_sub(1) as usize)
        .copied()
        .unwrap_or("mês");
    format!("{month_name} {}", month.year)
}

pub fn date_label(date: Date) -> String {
    let month_name = MONTHS
        .get(date.month.saturating_sub(1) as usize)
        .copied()
        .unwrap_or("mês");
    format!("{} de {month_name} de {}", date.day, date.year)
}

pub fn month_grid(month: Date) -> [CalendarDay; GRID_SIZE] {
    let month = month.start_of_month();
    let first_weekday = weekday(month.year, month.month, month.day);
    let mut date = month;
    for _ in 0..first_weekday {
        date = previous_day(date);
    }

    let mut grid = [CalendarDay {
        date: Date::new(0, 1, 1),
        current_month: false,
    }; GRID_SIZE];
    for cell in &mut grid {
        *cell = CalendarDay {
            date,
            current_month: date.month == month.month && date.year == month.year,
        };
        date = next_day(date);
    }
    grid
}

fn local_date(timestamp: libc::time_t) -> Option<Date> {
    let mut local = MaybeUninit::<libc::tm>::uninit();
    let result = unsafe { libc::localtime_r(&timestamp, local.as_mut_ptr()) };
    if result.is_null() {
        return None;
    }

    let local = unsafe { local.assume_init() };
    let year = local.tm_year.checked_add(1900)?;
    let month = u32::try_from(local.tm_mon.checked_add(1)?).ok()?;
    let day = u32::try_from(local.tm_mday).ok()?;
    (1..=12)
        .contains(&month)
        .then_some(Date::new(year, month, day))
}

fn previous_day(date: Date) -> Date {
    if date.day > 1 {
        Date::new(date.year, date.month, date.day - 1)
    } else {
        let month = date.previous_month();
        Date::new(
            month.year,
            month.month,
            days_in_month(month.year, month.month),
        )
    }
}

fn next_day(date: Date) -> Date {
    if date.day < days_in_month(date.year, date.month) {
        Date::new(date.year, date.month, date.day + 1)
    } else {
        date.next_month()
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

// Returns Sunday as 0, matching the Portuguese weekday order used by the UI.
fn weekday(year: i32, month: u32, day: u32) -> u32 {
    let mut year = year;
    let mut month = month as i32;
    if month < 3 {
        month += 12;
        year -= 1;
    }

    let day = day as i32;
    let century = year / 100;
    let year_in_century = year % 100;
    let saturday_first = (day
        + (13 * (month + 1)) / 5
        + year_in_century
        + year_in_century / 4
        + century / 4
        + 5 * century)
        % 7;
    ((saturday_first + 6) % 7) as u32
}

#[cfg(test)]
mod tests {
    use super::{date_label, month_grid, month_title, Date};

    #[test]
    fn month_grid_starts_on_sunday_and_keeps_six_weeks() {
        let grid = month_grid(Date::new(2026, 8, 1));

        assert_eq!(grid.len(), 42);
        assert_eq!(grid[0].date, Date::new(2026, 7, 26));
        assert!(!grid[0].current_month);
        assert_eq!(grid[6].date, Date::new(2026, 8, 1));
        assert!(grid[6].current_month);
        assert_eq!(grid[36].date, Date::new(2026, 8, 31));
        assert_eq!(grid[37].date, Date::new(2026, 9, 1));
        assert!(!grid[37].current_month);
    }

    #[test]
    fn month_grid_handles_leap_years() {
        let leap_year = month_grid(Date::new(2024, 2, 1));
        let common_year = month_grid(Date::new(2025, 2, 1));

        assert!(leap_year
            .iter()
            .any(|cell| cell.date == Date::new(2024, 2, 29)));
        assert!(!common_year
            .iter()
            .any(|cell| cell.date == Date::new(2025, 2, 29)));
    }

    #[test]
    fn navigation_wraps_between_years() {
        assert_eq!(
            Date::new(2026, 1, 1).previous_month(),
            Date::new(2025, 12, 1)
        );
        assert_eq!(Date::new(2025, 12, 1).next_month(), Date::new(2026, 1, 1));
    }

    #[test]
    fn labels_use_the_bar_locale() {
        assert_eq!(month_title(Date::new(2026, 8, 1)), "agosto 2026");
        assert_eq!(date_label(Date::new(2026, 8, 9)), "9 de agosto de 2026");
    }
}
