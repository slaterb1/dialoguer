use std::io;

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, SecondsFormat, TimeZone, Timelike, Utc};
use console::{style, Key, Term};
use std::cmp::{max, min};
use theme::{get_default_theme, TermThemeRenderer, Theme};

trait DateAdjust {
    fn increment_year(&self) -> Self;
    fn decrement_year(&self) -> Self;
    fn increment_month(&self) -> Self;
    fn decrement_month(&self) -> Self;
}

static MONTH_END_DAYS: &[u32] = &[0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

impl<T> DateAdjust for T
where
    T: Datelike,
{
    fn increment_year(&self) -> Self {
        self.with_year(self.year() + 1).unwrap_or_else(|| {
            // If we're currently on a leap day we know how to handle a failure
            assert_eq!(self.month(), 2, "Unexpected failure in year increment. Please open a bug ticket with the current case.");
            assert_eq!(self.day(), 29, "Unexpected failure in year increment. Please open a bug ticket with the current case.");

            self.with_day(28).unwrap().with_year(self.year() + 1).unwrap()
        })
    }

    fn decrement_year(&self) -> Self {
        self.with_year(self.year() - 1).unwrap_or_else(|| {
            // If we're currently on a leap day we know how to handle a failure
            assert_eq!(self.month(), 2, "Unexpected failure in year decrement. Please open a bug ticket with the current case.");
            assert_eq!(self.day(), 29, "Unexpected failure in year decrement. Please open a bug ticket with the current case.");

            self.with_day(28).unwrap().with_year(self.year() - 1).unwrap()
        })
    }

    fn increment_month(&self) -> Self {
        let new_month = self.month() + 1;
        if new_month > 12 {
            // This case should be infallible since both December and January have 31 days
            self.with_year(self.year() + 1).unwrap().with_month(1).unwrap()
        } else {
            self.with_month(new_month).unwrap_or_else(|| {
                // We've stepped off the end of the month most likely, adjust the day if so
                assert!(
                    self.day() > MONTH_END_DAYS[new_month as usize],
                    "Unexpected failure in month increment. Please open a bug ticket with the current case."
                );

                self.with_day(MONTH_END_DAYS[new_month as usize]).unwrap().with_month(new_month).unwrap()
            })
        }
    }

    fn decrement_month(&self) -> Self {
        let new_month = self.month() - 1;
        if new_month < 1 {
            // This case should be infallible since both December and January have 31 days
            self.with_year(self.year() - 1).unwrap().with_month(12).unwrap()
        } else {
            self.with_month(new_month).unwrap_or_else(|| {
                // We've stepped off the end of the month most likely, adjust the day if so
                assert!(
                    self.day() > MONTH_END_DAYS[new_month as usize],
                    "Unexpected failure in month decrement. Please open a bug ticket with the current case."
                );

                self.with_day(MONTH_END_DAYS[new_month as usize]).unwrap().with_month(new_month).unwrap()
            })
        }
    }
}

/// The possible types of datetime selections that can be made.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DateType {
    Date,
    Time,
    DateTime,
}

/// Renders a datetime selection interactive text.
///
/// prompt question is optional and not shown by default.
/// weekday that is displayed can be turned off.
/// date_type allows you to specify "date", "time" or "datetime"
/// default starting time can be set if following rfc3339 format "%Y-%m-%dT%H:%M:%s%Z"
/// min and max DateTime can be set to help with selection.
///
/// Note: Date values can be changed by UP/DOWN/j/k or specifying numerical values.
pub struct DateTimeSelect<'a> {
    prompt: Option<String>,
    default: Option<NaiveDateTime>,
    theme: &'a dyn Theme,
    weekday: bool,
    date_type: DateType,
    min: NaiveDateTime,
    max: NaiveDateTime,
    clear: bool,
    show_match: bool,
}

impl<'a> DateTimeSelect<'a> {
    pub fn new() -> DateTimeSelect<'static> {
        DateTimeSelect::with_theme(get_default_theme())
    }

    /// Creates a datetime with a specific theme.
    pub fn with_theme(theme: &'a dyn Theme) -> DateTimeSelect<'a> {
        DateTimeSelect {
            prompt: None,
            default: None,
            theme,
            weekday: true,
            date_type: DateType::DateTime,
            min: NaiveDate::from_ymd(0, 1, 1).and_hms(0, 0, 0),
            max: NaiveDate::from_ymd(9999, 12, 31).and_hms(23, 59, 59),
            clear: true,
            show_match: false,
        }
    }
    /// Sets the datetime prompt.
    pub fn with_prompt(&mut self, prompt: &str) -> &mut Self {
        self.prompt = Some(prompt.into());
        self
    }
    /// Sets default time to start with.
    pub fn default(&mut self, datetime: &str) -> &mut Self {
        self.default = Some(DateTime::parse_from_rfc3339(datetime).expect("date format must match rfc3339").naive_local());
        self
    }
    /// Sets whether to show weekday or not.
    pub fn weekday(&mut self, val: bool) -> &mut Self {
        self.weekday = val;
        self
    }
    /// Sets date selector to date, time, or datetime format.
    pub fn date_type(&mut self, val: DateType) -> &mut Self {
        self.date_type = val;
        self
    }
    /// Sets min value for Date or DateTime.
    pub fn min(&mut self, val: &str) -> &mut Self {
        self.min = DateTime::parse_from_rfc3339(val).expect("date format must match rfc3339").naive_local();
        assert!(self.max >= self.min, "maximum must be larger than minimum");
        self
    }
    /// Sets max value for Date or DateTime.
    pub fn max(&mut self, val: &'a str) -> &mut Self {
        self.max = DateTime::parse_from_rfc3339(val).expect("date format must match rfc3339").naive_local();
        assert!(self.max >= self.min, "maximum must be larger than minimum");
        self
    }
    /// Sets whether to clear inputs from terminal.
    pub fn clear(&mut self, val: bool) -> &mut Self {
        self.clear = val;
        self
    }
    /// Sets whether to show match string or not.
    pub fn show_match(&mut self, val: bool) -> &mut Self {
        self.show_match = val;
        self
    }

    fn check_date(&self, val: NaiveDateTime) -> NaiveDateTime {
        min(max(val, self.min), self.max)
    }

    fn terminal_format(&self, val: NaiveDateTime, pos: isize) -> String {
        match self.date_type {
            DateType::Date => format!(
                "{}-{:02}-{:02}",
                if pos == 0 { style(val.year()).bold() } else { style(val.year()).dim() },
                if pos == 1 { style(val.month()).bold() } else { style(val.month()).dim() },
                if pos == 2 { style(val.day()).bold() } else { style(val.day()).dim() },
            ),
            DateType::Time => format!(
                "{:02}:{:02}:{:02}",
                if pos == 0 { style(val.hour()).bold() } else { style(val.hour()).dim() },
                if pos == 1 { style(val.minute()).bold() } else { style(val.minute()).dim() },
                if pos == 2 { style(val.second()).bold() } else { style(val.second()).dim() },
            ),
            DateType::DateTime => format!(
                "{}-{:02}-{:02} {:02}:{:02}:{:02}",
                if pos == 0 { style(val.year()).bold() } else { style(val.year()).dim() },
                if pos == 1 { style(val.month()).bold() } else { style(val.month()).dim() },
                if pos == 2 { style(val.day()).bold() } else { style(val.day()).dim() },
                if pos == 3 { style(val.hour()).bold() } else { style(val.hour()).dim() },
                if pos == 4 { style(val.minute()).bold() } else { style(val.minute()).dim() },
                if pos == 5 { style(val.second()).bold() } else { style(val.second()).dim() },
            ),
        }
    }

    /// Enables user interaction and returns the result.
    ///
    /// The dialog is rendered on stderr.
    pub fn interact(&self) -> io::Result<String> {
        self.interact_on(&Term::stderr())
    }
    /// Like `interact` but allows a specific terminal to be set.
    fn interact_on(&self, term: &Term) -> io::Result<String> {
        let mut date_val = self.default.unwrap_or_else(|| {
            // Current date in UTC is used as default time if override not set.
            Utc::today().and_hms(0, 0, 0).naive_utc()
        });

        date_val = self.check_date(date_val);
        let mut render = TermThemeRenderer::new(term, self.theme);

        // Set vars for handling changing datetimes.
        let mut pos = 0;
        let max_pos = match self.date_type {
            DateType::Date => 2,
            DateType::Time => 2,
            DateType::DateTime => 5,
        };
        let mut digits: Vec<u32> = Vec::with_capacity(4);

        loop {
            // Styling is added to highlight pos being changed.
            let date_str = self.terminal_format(date_val, pos);

            // Add weekday if specified.
            let date_str = match &self.weekday {
                true => format!("{}, {:?}", date_str, date_val.weekday()),
                false => date_str,
            };

            // Render current state of datetime string.
            render.datetime(&self.prompt, &date_str)?;

            // Display typed numbers if show_match is true.
            if self.show_match {
                let str_num: Vec<String> = digits.iter().map(|c| c.to_string()).collect();
                term.write_line(&str_num.join(""))?;
            }

            match term.read_key()? {
                Key::Enter => {
                    // Clean up terminal.
                    if self.clear {
                        render.clear()?
                    }
                    if self.show_match {
                        term.clear_last_lines(1)?;
                    }
                    // Clean up formatting of returned string.
                    let date_str = match self.date_type {
                        DateType::Date => date_val.format("%Y-%m-%d").to_string(),
                        DateType::Time => date_val.format("%H:%M:%S").to_string(),
                        DateType::DateTime => Utc.from_utc_datetime(&date_val).to_rfc3339_opts(SecondsFormat::Secs, true),
                    };
                    return Ok(date_str);
                }
                Key::ArrowRight | Key::Char('l') => {
                    pos = if pos == max_pos { 0 } else { pos + 1 };
                    digits.clear();
                }
                Key::ArrowLeft | Key::Char('h') => {
                    pos = if pos == 0 { max_pos } else { pos - 1 };
                    digits.clear();
                }
                // Increment datetime by 1.
                Key::ArrowUp | Key::Char('j') => {
                    date_val = match (self.date_type, pos) {
                        (DateType::DateTime, 0) | (DateType::Date, 0) => date_val.increment_year(),
                        (DateType::DateTime, 1) | (DateType::Date, 1) => date_val.increment_month(),
                        (DateType::DateTime, 2) | (DateType::Date, 2) => date_val + Duration::days(1),
                        (DateType::DateTime, 3) | (DateType::Time, 0) => date_val + Duration::hours(1),
                        (DateType::DateTime, 4) | (DateType::Time, 1) => date_val + Duration::minutes(1),
                        (DateType::DateTime, 5) | (DateType::Time, 2) => date_val + Duration::seconds(1),
                        (DateType::Date, _) => panic!("stepped out of bounds on Date"),
                        (DateType::Time, _) => panic!("stepped out of bounds on Time"),
                        (DateType::DateTime, _) => panic!("stepped out of bounds on DateTime"),
                    };
                    digits.clear();
                }
                // Decrement the datetime by 1.
                Key::ArrowDown | Key::Char('k') => {
                    date_val = match (self.date_type, pos) {
                        (DateType::DateTime, 0) | (DateType::Date, 0) => date_val.decrement_year(),
                        (DateType::DateTime, 1) | (DateType::Date, 1) => date_val.decrement_month(),
                        (DateType::DateTime, 2) | (DateType::Date, 2) => date_val - Duration::days(1),
                        (DateType::DateTime, 3) | (DateType::Time, 0) => date_val - Duration::hours(1),
                        (DateType::DateTime, 4) | (DateType::Time, 1) => date_val - Duration::minutes(1),
                        (DateType::DateTime, 5) | (DateType::Time, 2) => date_val - Duration::seconds(1),
                        (DateType::Date, _) => panic!("stepped out of bounds on Date"),
                        (DateType::Time, _) => panic!("stepped out of bounds on Time"),
                        (DateType::DateTime, _) => panic!("stepped out of bounds on DateTime"),
                    };
                    digits = Vec::with_capacity(4);
                }
                // Allow numerical inputs.
                Key::Char(val) => {
                    if let Some(digit) = val.to_digit(10) {
                        digits.push(digit);
                        // Need 4 digits to set year
                        if pos == 0 && digits.len() == 4 {
                            let num = digits[0] * 1000 + digits[1] * 100 + digits[2] * 10 + digits[3];

                            date_val = match self.date_type {
                                DateType::Date | DateType::DateTime => date_val.with_year(num as i32),
                                DateType::Time => panic!("Time not supported for 4 digits"),
                            }
                            .unwrap_or(date_val);

                            digits.clear();
                        // Have 2 digits in any position, including 0 if hours.
                        } else if digits.len() == 2 && (pos > 0 || self.date_type == DateType::Time) {
                            let num = digits[0] * 10 + digits[1];
                            date_val = match (self.date_type, pos) {
                                (DateType::DateTime, 1) | (DateType::Date, 1) => date_val.with_month(num),
                                (DateType::DateTime, 2) | (DateType::Date, 2) => date_val.with_day(num),
                                (DateType::DateTime, 3) | (DateType::Time, 0) => date_val.with_hour(num),
                                (DateType::DateTime, 4) | (DateType::Time, 1) => date_val.with_minute(num),
                                (DateType::DateTime, 5) | (DateType::Time, 2) => date_val.with_second(num),
                                (DateType::Date, _) => panic!("stepped out of bounds on Date"),
                                (DateType::Time, _) => panic!("stepped out of bounds on Time"),
                                (DateType::DateTime, _) => panic!("stepped out of bounds on DateTime"),
                            }
                            .unwrap_or(date_val);
                            digits.clear();
                        }
                    } else {
                        digits.clear();
                    }
                }
                Key::Backspace => {
                    digits.pop();
                }
                _ => {}
            }
            date_val = self.check_date(date_val);
            render.clear()?;
            if self.show_match {
                term.clear_last_lines(1)?;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let datetime_select = DateTimeSelect::new();
        assert_eq!(datetime_select.prompt, None);
        assert_eq!(datetime_select.weekday, true);
        assert_eq!(datetime_select.date_type, DateType::DateTime);
    }
    #[test]
    fn test_setting_proper_rfc3339_default() {
        let mut datetime_select = DateTimeSelect::new();
        datetime_select.default("2019-01-01T00:00:00-00:00");
        assert_eq!(datetime_select.default, Some(NaiveDate::from_ymd(2019, 1, 1).and_hms_milli(0, 0, 0, 0)));
    }
    #[test]
    fn test_setting_prompt() {
        let mut datetime_select = DateTimeSelect::new();
        datetime_select.with_prompt("test");
        assert_eq!(datetime_select.prompt, Some("test".to_owned()));
    }
    #[test]
    fn test_setting_weekday() {
        let mut datetime_select = DateTimeSelect::new();
        datetime_select.weekday(false);
        assert_eq!(datetime_select.weekday, false);
    }
    #[test]
    fn test_setting_valid_date_type() {
        let mut datetime_select = DateTimeSelect::new();
        datetime_select.date_type(DateType::Date);
        assert_eq!(datetime_select.date_type, DateType::Date);
    }
    #[test]
    fn test_max_min_datetimes() {
        let mut datetime_select = DateTimeSelect::new();

        datetime_select.min("2020-02-20T02:20:25Z");
        let min_date = NaiveDate::from_ymd(2020, 2, 20).and_hms(2, 20, 25);
        assert_eq!(datetime_select.min, min_date);

        datetime_select.max("2022-11-30T00:00:00Z");
        let max_date = NaiveDate::from_ymd(2022, 11, 30).and_hms(0, 0, 0);
        assert_eq!(datetime_select.max, max_date);

        let in_range_date = NaiveDate::from_ymd(2020, 7, 8).and_hms(17, 1, 30);
        assert_eq!(datetime_select.check_date(in_range_date), in_range_date);

        assert_eq!(datetime_select.check_date(NaiveDate::from_ymd(2000, 1, 1).and_hms(0, 0, 0)), min_date);
        assert_eq!(datetime_select.check_date(NaiveDate::from_ymd(2030, 1, 1).and_hms(0, 0, 0)), max_date);
    }
}
