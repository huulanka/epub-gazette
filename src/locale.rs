//! Localized labels and date formatting. German is the reference language;
//! English is provided so the project is useful beyond its first user.

use chrono::{Datelike, NaiveDate, Weekday};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Labels {
    pub front_page: &'static str,
    pub index: &'static str,
    pub minutes: &'static str,
    pub context: &'static str,
    pub sources: &'static str,
    pub colophon: &'static str,
    pub volume: &'static str,
    pub number: &'static str,
    pub weather: &'static str,
    pub front_image: &'static str,
    pub typeset_with: &'static str,
}

static DE: Labels = Labels {
    front_page: "Titelseite",
    index: "Heute im Blatt",
    minutes: "Min.",
    context: "Einordnung",
    sources: "Quellen",
    colophon: "Zu dieser Ausgabe",
    volume: "Jahrgang",
    number: "Nr.",
    weather: "Wetter",
    front_image: "Titelbild",
    typeset_with: "Gesetzt mit epub-gazette",
};

static EN: Labels = Labels {
    front_page: "Front Page",
    index: "In This Issue",
    minutes: "min",
    context: "Analysis",
    sources: "Sources",
    colophon: "About This Issue",
    volume: "Vol.",
    number: "No.",
    weather: "Weather",
    front_image: "Front image",
    typeset_with: "Typeset with epub-gazette",
};

fn is_en(lang: &str) -> bool {
    lang.eq_ignore_ascii_case("en") || lang.to_ascii_lowercase().starts_with("en-")
}

pub fn labels(lang: &str) -> &'static Labels {
    if is_en(lang) { &EN } else { &DE }
}

const DE_DAYS: [&str; 7] = [
    "Montag",
    "Dienstag",
    "Mittwoch",
    "Donnerstag",
    "Freitag",
    "Samstag",
    "Sonntag",
];
const DE_DAYS_SHORT: [&str; 7] = ["Mo.", "Di.", "Mi.", "Do.", "Fr.", "Sa.", "So."];
const DE_MONTHS: [&str; 12] = [
    "Januar",
    "Februar",
    "März",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];
const EN_DAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
const EN_MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn day_idx(d: NaiveDate) -> usize {
    d.weekday().num_days_from_monday() as usize
}

/// "Samstag, 14. November 2026" / "Saturday, November 14, 2026"
pub fn date_long(d: NaiveDate, lang: &str) -> String {
    let m = d.month0() as usize;
    if is_en(lang) {
        format!(
            "{}, {} {}, {}",
            EN_DAYS[day_idx(d)],
            EN_MONTHS[m],
            d.day(),
            d.year()
        )
    } else {
        format!(
            "{}, {}. {} {}",
            DE_DAYS[day_idx(d)],
            d.day(),
            DE_MONTHS[m],
            d.year()
        )
    }
}

/// "Sa., 14.11.2026" / "Sat, 2026-11-14" – used in the book title.
pub fn date_short(d: NaiveDate, lang: &str) -> String {
    if is_en(lang) {
        format!("{}, {}", &EN_DAYS[day_idx(d)][..3], d.format("%Y-%m-%d"))
    } else {
        format!("{} {}", DE_DAYS_SHORT[day_idx(d)], d.format("%d.%m.%Y"))
    }
}

pub fn is_sunday(d: NaiveDate) -> bool {
    d.weekday() == Weekday::Sun
}

/// Roman numeral for volume numbers (1..=3999).
pub fn roman(mut n: u32) -> Option<String> {
    if n == 0 || n > 3999 {
        return None;
    }
    const TABLE: [(u32, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut out = String::new();
    for (v, s) in TABLE {
        while n >= v {
            out.push_str(s);
            n -= v;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn german_dates() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 14).unwrap();
        assert_eq!(date_long(d, "de"), "Samstag, 14. November 2026");
        assert_eq!(date_short(d, "de"), "Sa. 14.11.2026");
        assert!(!is_sunday(d));
    }

    #[test]
    fn english_dates() {
        let d = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        assert_eq!(date_long(d, "en-GB"), "Sunday, March 1, 2026");
        assert_eq!(date_short(d, "en"), "Sun, 2026-03-01");
    }

    #[test]
    fn roman_numerals() {
        assert_eq!(roman(1).as_deref(), Some("I"));
        assert_eq!(roman(14).as_deref(), Some("XIV"));
        assert_eq!(roman(2026).as_deref(), Some("MMXXVI"));
        assert_eq!(roman(0), None);
    }
}
