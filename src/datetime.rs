use std::io;

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, Timelike, Utc};
use console::{style, Key, Term};
use std::cmp::{max, min};
use theme::{get_default_theme, TermThemeRenderer, Theme};

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
        self.default = Some(
            DateTime::parse_from_rfc3339(datetime)
                .expect("date format must match rfc3339")
                .naive_local(),
        );
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
        self.min = DateTime::parse_from_rfc3339(val)
            .expect("date format must match rfc3339")
            .naive_local();
        self
    }
    /// Sets max value for Date or DateTime.
    pub fn max(&mut self, val: &'a str) -> &mut Self {
        self.max = DateTime::parse_from_rfc3339(val)
            .expect("date format must match rfc3339")
            .naive_local();
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
                if pos == 0 {
                    style(val.year()).bold()
                } else {
                    style(val.year()).dim()
                },
                if pos == 1 {
                    style(val.month()).bold()
                } else {
                    style(val.month()).dim()
                },
                if pos == 2 {
                    style(val.day()).bold()
                } else {
                    style(val.day()).dim()
                },
            ),
            DateType::Time => format!(
                "{:02}:{:02}:{:02}",
                if pos == 0 {
                    style(val.hour()).bold()
                } else {
                    style(val.hour()).dim()
                },
                if pos == 1 {
                    style(val.minute()).bold()
                } else {
                    style(val.minute()).dim()
                },
                if pos == 2 {
                    style(val.second()).bold()
                } else {
                    style(val.second()).dim()
                },
            ),
            DateType::DateTime => format!(
                "{}-{:02}-{:02} {:02}:{:02}:{:02}",
                if pos == 0 {
                    style(val.year()).bold()
                } else {
                    style(val.year()).dim()
                },
                if pos == 1 {
                    style(val.month()).bold()
                } else {
                    style(val.month()).dim()
                },
                if pos == 2 {
                    style(val.day()).bold()
                } else {
                    style(val.day()).dim()
                },
                if pos == 3 {
                    style(val.hour()).bold()
                } else {
                    style(val.hour()).dim()
                },
                if pos == 4 {
                    style(val.minute()).bold()
                } else {
                    style(val.minute()).dim()
                },
                if pos == 5 {
                    style(val.second()).bold()
                } else {
                    style(val.second()).dim()
                },
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
            Utc::now()
                .with_hour(0)
                .unwrap()
                .with_minute(0)
                .unwrap()
                .with_second(0)
                .unwrap()
                .naive_utc()
        });

        date_val = self.check_date(date_val);
        let mut render = TermThemeRenderer::new(term, self.theme);

        // Set vars for handling changing datetimes.
        let mut pos = 0;
        let max_pos = match &self.date_type {
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
                let str_num: Vec<String> = digits.iter().cloned().map(|c| c.to_string()).collect();
                let str_num: String = str_num.join("");
                term.write_line(&str_num[..])?;
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
                    let date_str = match &self.date_type {
                        DateType::Date => format!(
                            "{}-{:02}-{:02}",
                            date_val.year(),
                            date_val.month(),
                            date_val.day(),
                        ),
                        DateType::Time => format!(
                            "{:02}:{:02}:{:02}",
                            date_val.hour(),
                            date_val.minute(),
                            date_val.second(),
                        ),
                        DateType::DateTime => format!(
                            "{}-{:02}-{:02} {:02}:{:02}:{:02}",
                            date_val.year(),
                            date_val.month(),
                            date_val.day(),
                            date_val.hour(),
                            date_val.minute(),
                            date_val.second(),
                        ),
                    };
                    return Ok(date_str.to_owned());
                }
                Key::ArrowRight | Key::Char('l') => {
                    pos = if pos == max_pos { 0 } else { pos + 1 };
                    digits = Vec::with_capacity(4);
                }
                Key::ArrowLeft | Key::Char('h') => {
                    pos = if pos == 0 { max_pos } else { pos - 1 };
                    digits = Vec::with_capacity(4);
                }
                // Increment datetime by 1.
                Key::ArrowUp | Key::Char('j') => {
                    date_val = match (&self.date_type, pos) {
                        (DateType::Date, 0) => date_val.with_year(date_val.year() + 1).unwrap(),
                        (DateType::Date, 1) => {
                            if date_val.month() + 1 > 12 {
                                date_val
                                    .with_year(date_val.year() + 1)
                                    .unwrap()
                                    .with_month(1)
                                    .unwrap()
                            } else {
                                date_val.with_month(date_val.month() + 1).unwrap()
                            }
                        }
                        (DateType::Date, 2) => {
                            date_val.checked_add_signed(Duration::days(1)).unwrap()
                        }
                        (DateType::Time, 0) => {
                            date_val.checked_add_signed(Duration::hours(1)).unwrap()
                        }
                        (DateType::Time, 1) => {
                            date_val.checked_add_signed(Duration::minutes(1)).unwrap()
                        }
                        (DateType::Time, 2) => {
                            date_val.checked_add_signed(Duration::seconds(1)).unwrap()
                        }
                        (DateType::DateTime, 0) => date_val.with_year(date_val.year() + 1).unwrap(),
                        (DateType::DateTime, 1) => {
                            if date_val.month() + 1 > 12 {
                                date_val
                                    .with_year(date_val.year() + 1)
                                    .unwrap()
                                    .with_month(1)
                                    .unwrap()
                            } else {
                                date_val.with_month(date_val.month() + 1).unwrap()
                            }
                        }
                        (DateType::DateTime, 2) => {
                            date_val.checked_add_signed(Duration::days(1)).unwrap()
                        }
                        (DateType::DateTime, 3) => {
                            date_val.checked_add_signed(Duration::hours(1)).unwrap()
                        }
                        (DateType::DateTime, 4) => {
                            date_val.checked_add_signed(Duration::minutes(1)).unwrap()
                        }
                        (DateType::DateTime, 5) => {
                            date_val.checked_add_signed(Duration::seconds(1)).unwrap()
                        }
                        (DateType::Date, _) => panic!("stepped out of bounds on Date"),
                        (DateType::Time, _) => panic!("stepped out of bounds on Time"),
                        (DateType::DateTime, _) => panic!("stepped out of bounds on DateTime"),
                    };
                    digits = Vec::with_capacity(4);
                }
                // Decrement the datetime by 1.
                Key::ArrowDown | Key::Char('k') => {
                    date_val = match (&self.date_type, pos) {
                        (DateType::Date, 0) => date_val.with_year(date_val.year() - 1).unwrap(),
                        (DateType::Date, 1) => {
                            if date_val.month() - 1 == 0 {
                                date_val
                                    .with_year(date_val.year() - 1)
                                    .unwrap()
                                    .with_month(12)
                                    .unwrap()
                            } else {
                                date_val.with_month(date_val.month() - 1).unwrap()
                            }
                        }
                        (DateType::Date, 2) => {
                            date_val.checked_sub_signed(Duration::days(1)).unwrap()
                        }
                        (DateType::Time, 0) => {
                            date_val.checked_sub_signed(Duration::hours(1)).unwrap()
                        }
                        (DateType::Time, 1) => {
                            date_val.checked_sub_signed(Duration::minutes(1)).unwrap()
                        }
                        (DateType::Time, 2) => {
                            date_val.checked_sub_signed(Duration::seconds(1)).unwrap()
                        }
                        (DateType::DateTime, 0) => date_val.with_year(date_val.year() - 1).unwrap(),
                        (DateType::DateTime, 1) => {
                            if date_val.month() - 1 == 0 {
                                date_val
                                    .with_year(date_val.year() - 1)
                                    .unwrap()
                                    .with_month(12)
                                    .unwrap()
                            } else {
                                date_val.with_month(date_val.month() - 1).unwrap()
                            }
                        }
                        (DateType::DateTime, 2) => {
                            date_val.checked_sub_signed(Duration::days(1)).unwrap()
                        }
                        (DateType::DateTime, 3) => {
                            date_val.checked_sub_signed(Duration::hours(1)).unwrap()
                        }
                        (DateType::DateTime, 4) => {
                            date_val.checked_sub_signed(Duration::minutes(1)).unwrap()
                        }
                        (DateType::DateTime, 5) => {
                            date_val.checked_sub_signed(Duration::seconds(1)).unwrap()
                        }
                        (DateType::Date, _) => panic!("stepped out of bounds on Date"),
                        (DateType::Time, _) => panic!("stepped out of bounds on Time"),
                        (DateType::DateTime, _) => panic!("stepped out of bounds on DateTime"),
                    };
                    digits = Vec::with_capacity(4);
                }
                // Allow numerical inputs.
                Key::Char(val) => {
                    if val.is_digit(10) {
                        digits.push(val.to_digit(10).unwrap());
                        // Need 4 digits to set year
                        if pos == 0 && digits.len() == 4 {
                            let num =
                                digits[0] * 1000 + digits[1] * 100 + digits[2] * 10 + digits[3];
                            date_val = match &self.date_type {
                                DateType::Date => date_val.with_year(num as i32).unwrap(),
                                DateType::DateTime => date_val.with_year(num as i32).unwrap(),
                                DateType::Time => panic!("Time not supported for 4 digits"),
                            };
                            digits = Vec::with_capacity(4);
                        // Have 2 digits in any position, including 0 if hours.
                        } else if digits.len() == 2 && (pos > 0 || self.date_type == DateType::Time)
                        {
                            let num = digits[0] * 10 + digits[1];
                            date_val = match (&self.date_type, pos) {
                                (DateType::Date, 1) => date_val.with_month(num).unwrap_or(date_val),
                                (DateType::Date, 2) => date_val.with_day(num).unwrap_or(date_val),
                                (DateType::Time, 0) => date_val.with_hour(num).unwrap_or(date_val),
                                (DateType::Time, 1) => {
                                    date_val.with_minute(num).unwrap_or(date_val)
                                }
                                (DateType::Time, 2) => {
                                    date_val.with_second(num).unwrap_or(date_val)
                                }
                                (DateType::DateTime, 1) => {
                                    date_val.with_month(num).unwrap_or(date_val)
                                }
                                (DateType::DateTime, 2) => {
                                    date_val.with_day(num).unwrap_or(date_val)
                                }
                                (DateType::DateTime, 3) => {
                                    date_val.with_hour(num).unwrap_or(date_val)
                                }
                                (DateType::DateTime, 4) => {
                                    date_val.with_minute(num).unwrap_or(date_val)
                                }
                                (DateType::DateTime, 5) => {
                                    date_val.with_second(num).unwrap_or(date_val)
                                }
                                (DateType::Date, _) => panic!("stepped out of bounds on Date"),
                                (DateType::Time, _) => panic!("stepped out of bounds on Time"),
                                (DateType::DateTime, _) => {
                                    panic!("stepped out of bounds on DateTime")
                                }
                            };
                            digits = Vec::with_capacity(4);
                        }
                    } else {
                        digits = Vec::with_capacity(4);
                    }
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
        assert_eq!(
            datetime_select.default,
            Some(NaiveDate::from_ymd(2019, 1, 1).and_hms_milli(0, 0, 0, 0))
        );
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
}
