//! Translations and formatting: messages with placeholders, plural and select rules (ICU
//! MessageFormat, as in vue-i18n, Angular, FormatJS), numbers, currencies, percentages and
//! dates in the visitor's language (Angular's pipes, `Intl`), without any dependency.
//!
//! [`I18n`] holds the current locale in a [`State`]: every text built with
//! [`I18n::t`] inside `text_dyn` follows it, so switching the language updates the page in
//! place.
//!
//! ```
//! use active::i18n::{I18n, Date, DateStyle};
//!
//! let i18n = I18n::new("en")
//!     .messages("en", [("inbox", "{name}, you have {count, plural, =0 {no messages} one {# message} other {# messages}}")])
//!     .messages("fr", [("inbox", "{name}, vous avez {count, plural, =0 {aucun message} one {# message} other {# messages}}")]);
//! assert_eq!(i18n.t_with("inbox", &[("name", "Ada".into()), ("count", 1200.into())]), "Ada, you have 1,200 messages");
//! i18n.set_locale("fr-CA");
//! assert_eq!(i18n.t_with("inbox", &[("name", "Ada".into()), ("count", 0.into())]), "Ada, vous avez aucun message");
//! assert_eq!(i18n.date(Date::new(2026, 10, 1), DateStyle::Long), "1er octobre 2026");
//! ```
//!
//! The rules cover English, French, Spanish, German, Italian, Portuguese, Dutch, the
//! Scandinavian languages, Polish, Czech, Slovak, Russian, Ukrainian, Arabic, Hebrew,
//! Japanese, Chinese and Korean; other languages get the English rules.

use std::collections::HashMap;

use crate::reactive::{State, use_state};

/// Narrow no-break space: the French thousands separator.
const NNBSP: char = match char::from_u32(0x202F) {
    Some(c) => c,
    None => ' ',
};
/// No-break space.
const NBSP: char = match char::from_u32(0xA0) {
    Some(c) => c,
    None => ' ',
};

/// The language of a locale, lowercase: `"pt"` for `"pt-BR"`.
fn language(locale: &str) -> String {
    locale
        .split(['-', '_'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// The region of a locale, uppercase: `"BR"` for `"pt-BR"`.
fn region(locale: &str) -> String {
    locale
        .split(['-', '_'])
        .nth(1)
        .unwrap_or("")
        .to_ascii_uppercase()
}

/// A plural category (CLDR): which form of a word goes with a number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Plural {
    /// Arabic `0`.
    Zero,
    /// `1` in English, `0` and `1` in French, `21` in Russian…
    One,
    /// Arabic and Hebrew `2`.
    Two,
    /// Polish and Russian `2`–`4`, `22`–`24`…
    Few,
    /// Polish and Russian `5`–`20`, `25`…
    Many,
    /// Everything else.
    Other,
}

impl Plural {
    /// The keyword of the category in a message: `"one"`, `"few"`, `"other"`…
    pub fn keyword(self) -> &'static str {
        match self {
            Plural::Zero => "zero",
            Plural::One => "one",
            Plural::Two => "two",
            Plural::Few => "few",
            Plural::Many => "many",
            Plural::Other => "other",
        }
    }
}

/// The plural category of `n` in `locale`.
///
/// ```
/// use active::i18n::{plural, Plural};
///
/// assert_eq!(plural("en", 1.0), Plural::One);
/// assert_eq!(plural("en", 0.0), Plural::Other);
/// assert_eq!(plural("fr", 0.0), Plural::One);
/// assert_eq!(plural("ru", 22.0), Plural::Few);
/// assert_eq!(plural("pl", 25.0), Plural::Many);
/// assert_eq!(plural("ar", 2.0), Plural::Two);
/// assert_eq!(plural("ja", 1.0), Plural::Other);
/// ```
pub fn plural(locale: &str, n: f64) -> Plural {
    let n = n.abs();
    let whole = n.fract() == 0.0;
    let i = n.trunc() as u64;
    let (i10, i100) = (i % 10, i % 100);
    match language(locale).as_str() {
        "ja" | "zh" | "ko" | "vi" | "th" | "id" | "ms" | "lo" | "my" => Plural::Other,
        "fr" | "pt" if region(locale) != "PT" => {
            if i <= 1 {
                Plural::One
            } else {
                Plural::Other
            }
        }
        "ru" | "uk" | "be" => match () {
            _ if !whole => Plural::Other,
            _ if i10 == 1 && i100 != 11 => Plural::One,
            _ if (2..=4).contains(&i10) && !(12..=14).contains(&i100) => Plural::Few,
            _ => Plural::Many,
        },
        "pl" => match () {
            _ if !whole => Plural::Other,
            _ if i == 1 => Plural::One,
            _ if (2..=4).contains(&i10) && !(12..=14).contains(&i100) => Plural::Few,
            _ => Plural::Many,
        },
        "cs" | "sk" => match () {
            _ if !whole => Plural::Many,
            _ if i == 1 => Plural::One,
            _ if (2..=4).contains(&i) => Plural::Few,
            _ => Plural::Other,
        },
        "ar" => match () {
            _ if !whole => Plural::Other,
            _ if i == 0 => Plural::Zero,
            _ if i == 1 => Plural::One,
            _ if i == 2 => Plural::Two,
            _ if (3..=10).contains(&i100) => Plural::Few,
            _ if (11..=99).contains(&i100) => Plural::Many,
            _ => Plural::Other,
        },
        "he" => match () {
            _ if whole && i == 1 => Plural::One,
            _ if whole && i == 2 => Plural::Two,
            _ => Plural::Other,
        },
        _ => {
            if whole && i == 1 {
                Plural::One
            } else {
                Plural::Other
            }
        }
    }
}

/// Thousands separator, decimal separator and minimum number of digits before grouping.
fn separators(locale: &str) -> (Option<char>, char, usize) {
    match language(locale).as_str() {
        "fr" => (Some(NNBSP), ',', 4),
        "es" | "pl" => (
            Some(if language(locale) == "pl" { NBSP } else { '.' }),
            ',',
            5,
        ),
        "nb" | "no" | "nn" | "sv" | "fi" | "cs" | "sk" | "ru" | "uk" | "be" | "bg" | "hu"
        | "et" | "lv" | "lt" => (Some(NBSP), ',', 4),
        "de" if region(locale) == "CH" => (Some('\''), '.', 4),
        "de" | "it" | "pt" | "nl" | "da" | "tr" | "id" | "el" | "ro" | "hr" | "sl" | "sr"
        | "vi" => (Some('.'), ',', 4),
        _ => (Some(','), '.', 4),
    }
}

/// `n` with exactly `decimals` digits after the separator, grouped by thousands as in
/// `locale`.
///
/// ```
/// use active::i18n::format_fixed;
///
/// assert_eq!(format_fixed("en", 1234567.891, 2), "1,234,567.89");
/// assert_eq!(format_fixed("de", -1234.5, 2), "-1.234,50");
/// assert_eq!(format_fixed("es", 1234.0, 0), "1234"); // Spanish groups from 5 digits
/// ```
pub fn format_fixed(locale: &str, n: f64, decimals: usize) -> String {
    if !n.is_finite() {
        return if n.is_nan() {
            "NaN".into()
        } else if n > 0.0 {
            "∞".into()
        } else {
            "-∞".into()
        };
    }
    let (group, decimal, min) = separators(locale);
    let text = format!("{:.*}", decimals, n.abs());
    let (int, frac) = text.split_once('.').unwrap_or((&text, ""));
    let negative = n < 0.0 && text.bytes().any(|b| (b'1'..=b'9').contains(&b));
    let mut out = String::with_capacity(text.len() + 4);
    if negative {
        out.push('-');
    }
    match group {
        Some(sep) if int.len() >= min => {
            for (i, digit) in int.chars().enumerate() {
                if i > 0 && (int.len() - i) % 3 == 0 {
                    out.push(sep);
                }
                out.push(digit);
            }
        }
        _ => out.push_str(int),
    }
    if !frac.is_empty() {
        out.push(decimal);
        out.push_str(frac);
    }
    out
}

/// `n` as in `locale`, with up to three decimals (no trailing zeros), like
/// `Intl.NumberFormat`.
///
/// ```
/// use active::i18n::format_number;
///
/// assert_eq!(format_number("en", 1234.5), "1,234.5");
/// assert_eq!(format_number("fr", 1234.5).replace('\u{202f}', " "), "1 234,5");
/// assert_eq!(format_number("de", 0.12345), "0,123");
/// ```
pub fn format_number(locale: &str, n: f64) -> String {
    let mut text = format_fixed(locale, n, 3);
    let decimal = separators(locale).1;
    if text.contains(decimal) && n.is_finite() {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with(decimal) {
            text.pop();
        }
    }
    text
}

/// A ratio as a percentage in `locale` (`0.25` → `"25%"`, `"25 %"` in French), with
/// `decimals` digits after the separator.
pub fn format_percent(locale: &str, ratio: f64, decimals: usize) -> String {
    let number = format_fixed(locale, ratio * 100.0, decimals);
    match language(locale).as_str() {
        "fr" => format!("{number}{NNBSP}%"),
        "de" | "es" | "nb" | "no" | "sv" | "fi" | "cs" | "sk" | "ru" | "uk" | "da" => {
            format!("{number}{NBSP}%")
        }
        _ => format!("{number}%"),
    }
}

/// An amount of money in `locale`, with the symbol of the ISO 4217 `currency` code
/// (`"EUR"`, `"USD"`…; other codes are written as such) and its usual number of decimals.
///
/// ```
/// use active::i18n::format_currency;
///
/// assert_eq!(format_currency("en", 1234.5, "USD"), "$1,234.50");
/// assert_eq!(format_currency("de", 1234.5, "EUR").replace('\u{a0}', " "), "1.234,50 €");
/// assert_eq!(format_currency("ja", 1234.0, "JPY"), "¥1,234");
/// ```
pub fn format_currency(locale: &str, amount: f64, currency: &str) -> String {
    let code = currency.to_ascii_uppercase();
    let decimals = match code.as_str() {
        "JPY" | "KRW" | "VND" | "CLP" | "ISK" => 0,
        _ => 2,
    };
    let symbol = match code.as_str() {
        "EUR" => "€",
        "USD" => "$",
        "GBP" => "£",
        "JPY" | "CNY" => "¥",
        "KRW" => "₩",
        "INR" => "₹",
        "RUB" => "₽",
        "UAH" => "₴",
        "PLN" => "zł",
        "BRL" => "R$",
        "ILS" => "₪",
        "TRY" => "₺",
        _ => code.as_str(),
    };
    let number = format_fixed(locale, amount.abs(), decimals);
    let sign = if amount < 0.0 && number.bytes().any(|b| (b'1'..=b'9').contains(&b)) {
        "-"
    } else {
        ""
    };
    match language(locale).as_str() {
        "en" | "ja" | "zh" | "ko" | "he" | "th" => {
            let space = if symbol.chars().all(char::is_alphabetic) {
                " "
            } else {
                ""
            };
            format!("{sign}{symbol}{space}{number}")
        }
        "nl" => format!("{symbol}{NBSP}{sign}{number}"),
        _ => format!("{sign}{number}{NBSP}{symbol}"),
    }
}

/// A calendar date (proleptic Gregorian), for [`format_date`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    /// The year (`2026`).
    pub year: i32,
    /// The month, `1` to `12`.
    pub month: u8,
    /// The day of the month, `1` to `31`.
    pub day: u8,
}

impl Date {
    /// The date `year-month-day`.
    pub fn new(year: i32, month: u8, day: u8) -> Self {
        Date { year, month, day }
    }

    /// The UTC date of a Unix timestamp in seconds (`js_sys::Date::now() / 1000.0` in the
    /// browser, `SystemTime` on the server).
    pub fn from_unix(seconds: i64) -> Self {
        // Howard Hinnant's `civil_from_days`.
        let z = seconds.div_euclid(86_400) + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u8;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
        let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
        Date { year, month, day }
    }

    /// The day of the week, `0` for Sunday to `6` for Saturday.
    pub fn weekday(self) -> u8 {
        let (y, m) = if self.month <= 2 {
            (i64::from(self.year) - 1, i64::from(self.month) + 9)
        } else {
            (i64::from(self.year), i64::from(self.month) - 3)
        };
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let doy = (153 * m + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        let days = era * 146_097 + doe - 719_468;
        (days + 4).rem_euclid(7) as u8
    }
}

/// How much of a date [`format_date`] writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DateStyle {
    /// Digits: `10/15/2026`, `15/10/2026`, `15.10.2026`.
    Short,
    /// The month abbreviated: `Oct 15, 2026`, `15 oct. 2026`.
    Medium,
    /// The month in full: `October 15, 2026`, `15 octobre 2026`.
    Long,
    /// With the day of the week: `Thursday, October 15, 2026`.
    Full,
}

fn months(lang: &str) -> Option<[&'static str; 12]> {
    Some(match lang {
        "en" => [
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
        ],
        "fr" => [
            "janvier",
            "février",
            "mars",
            "avril",
            "mai",
            "juin",
            "juillet",
            "août",
            "septembre",
            "octobre",
            "novembre",
            "décembre",
        ],
        "es" => [
            "enero",
            "febrero",
            "marzo",
            "abril",
            "mayo",
            "junio",
            "julio",
            "agosto",
            "septiembre",
            "octubre",
            "noviembre",
            "diciembre",
        ],
        "de" => [
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
        ],
        "it" => [
            "gennaio",
            "febbraio",
            "marzo",
            "aprile",
            "maggio",
            "giugno",
            "luglio",
            "agosto",
            "settembre",
            "ottobre",
            "novembre",
            "dicembre",
        ],
        "pt" => [
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
        ],
        "nl" => [
            "januari",
            "februari",
            "maart",
            "april",
            "mei",
            "juni",
            "juli",
            "augustus",
            "september",
            "oktober",
            "november",
            "december",
        ],
        _ => return None,
    })
}

fn weekdays(lang: &str) -> [&'static str; 7] {
    match lang {
        "fr" => [
            "dimanche", "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi",
        ],
        "es" => [
            "domingo",
            "lunes",
            "martes",
            "miércoles",
            "jueves",
            "viernes",
            "sábado",
        ],
        "de" => [
            "Sonntag",
            "Montag",
            "Dienstag",
            "Mittwoch",
            "Donnerstag",
            "Freitag",
            "Samstag",
        ],
        "it" => [
            "domenica",
            "lunedì",
            "martedì",
            "mercoledì",
            "giovedì",
            "venerdì",
            "sabato",
        ],
        "pt" => [
            "domingo",
            "segunda-feira",
            "terça-feira",
            "quarta-feira",
            "quinta-feira",
            "sexta-feira",
            "sábado",
        ],
        "nl" => [
            "zondag",
            "maandag",
            "dinsdag",
            "woensdag",
            "donderdag",
            "vrijdag",
            "zaterdag",
        ],
        _ => [
            "Sunday",
            "Monday",
            "Tuesday",
            "Wednesday",
            "Thursday",
            "Friday",
            "Saturday",
        ],
    }
}

/// The abbreviated month, as in `Intl.DateTimeFormat`'s `month: "short"`.
fn short_month(lang: &str, month: &str) -> String {
    match lang {
        "fr" => match month {
            "mai" | "juin" | "mars" | "août" => month.to_string(),
            "juillet" => "juil.".into(),
            "février" => "févr.".into(),
            "septembre" => "sept.".into(),
            "janvier" => "janv.".into(),
            _ => format!("{}.", month.chars().take(3).collect::<String>()),
        },
        "pt" => format!("{}.", month.chars().take(3).collect::<String>()),
        "es" if month == "septiembre" => "sept".into(),
        _ => month.chars().take(3).collect(),
    }
}

/// A date as written in `locale`.
///
/// ```
/// use active::i18n::{format_date, Date, DateStyle};
///
/// let d = Date::new(2026, 10, 15);
/// assert_eq!(format_date("en-US", d, DateStyle::Short), "10/15/2026");
/// assert_eq!(format_date("en-GB", d, DateStyle::Long), "15 October 2026");
/// assert_eq!(format_date("fr", d, DateStyle::Full), "jeudi 15 octobre 2026");
/// assert_eq!(format_date("es", d, DateStyle::Long), "15 de octubre de 2026");
/// assert_eq!(format_date("de", d, DateStyle::Short), "15.10.2026");
/// ```
pub fn format_date(locale: &str, date: Date, style: DateStyle) -> String {
    let lang = language(locale);
    let us = lang == "en" && matches!(region(locale).as_str(), "" | "US" | "PH");
    let Date { year, month, day } = date;
    let Some(names) = months(&lang) else {
        // Unknown language: ISO 8601 for digits, English otherwise.
        return match style {
            DateStyle::Short => format!("{year:04}-{month:02}-{day:02}"),
            _ => format_date("en", date, style),
        };
    };
    let name = names[usize::from(month.clamp(1, 12)) - 1];
    if style == DateStyle::Short || (style == DateStyle::Medium && lang == "de") {
        return match lang.as_str() {
            "en" if us => format!("{month}/{day}/{year}"),
            "de" => format!("{day:02}.{month:02}.{year}"),
            "nl" => format!("{day:02}-{month:02}-{year}"),
            _ => format!("{day:02}/{month:02}/{year}"),
        };
    }
    let month_name = if style == DateStyle::Medium {
        short_month(&lang, name)
    } else {
        name.to_string()
    };
    let day_text = if lang == "fr" && day == 1 {
        "1er".to_string()
    } else {
        day.to_string()
    };
    let body = match lang.as_str() {
        "en" if us => format!("{month_name} {day}, {year}"),
        "es" | "pt" if style != DateStyle::Medium || lang == "pt" => {
            format!("{day} de {month_name} de {year}")
        }
        "de" => format!("{day}. {month_name} {year}"),
        _ => format!("{day_text} {month_name} {year}"),
    };
    if style != DateStyle::Full {
        return body;
    }
    let weekday = weekdays(&lang)[usize::from(date.weekday())];
    match lang.as_str() {
        "fr" | "it" | "nl" => format!("{weekday} {body}"),
        _ => format!("{weekday}, {body}"),
    }
}

/// A value given to a message: text or a number (formatted in the message's locale).
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    /// Inserted as is; matched by `select`.
    Text(String),
    /// Formatted with [`format_number`]; chooses the branch of a `plural`.
    Number(f64),
}

impl From<&str> for Arg {
    fn from(s: &str) -> Self {
        Arg::Text(s.to_string())
    }
}
impl From<String> for Arg {
    fn from(s: String) -> Self {
        Arg::Text(s)
    }
}
macro_rules! number_args {
    ($($t:ty),*) => {$(
        impl From<$t> for Arg {
            fn from(n: $t) -> Self {
                Arg::Number(n as f64)
            }
        }
    )*};
}
number_args!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

/// Formats an ICU message: `{name}` placeholders, `{n, plural, =0 {…} one {# item} other {#
/// items}}` (`#` is the number) and `{kind, select, a {…} other {…}}`, nested as needed. A
/// missing value leaves its placeholder.
///
/// ```
/// use active::i18n::format_message;
///
/// let msg = "{who, select, she {She} he {He} other {They}} liked {n, plural, one {# photo} other {# photos}}";
/// assert_eq!(format_message("en", msg, &[("who", "she".into()), ("n", 3.into())]), "She liked 3 photos");
/// assert_eq!(format_message("ru", "{n, plural, one {# файл} few {# файла} many {# файлов} other {# файла}}", &[("n", 23.into())]), "23 файла");
/// ```
pub fn format_message(locale: &str, message: &str, args: &[(&str, Arg)]) -> String {
    let mut out = String::with_capacity(message.len());
    write_message(&mut out, locale, message, args, None);
    out
}

/// The text up to the `}` that closes the `{` just before `s` (nested braces skipped).
fn closing(s: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (i, c) in s.char_indices() {
        match c {
            '{' => depth += 1,
            '}' if depth == 0 => return Some(i),
            '}' => depth -= 1,
            _ => {}
        }
    }
    None
}

fn write_message(
    out: &mut String,
    locale: &str,
    message: &str,
    args: &[(&str, Arg)],
    hash: Option<f64>,
) {
    let mut rest = message;
    while let Some(i) = rest.find(['{', '#']) {
        out.push_str(&rest[..i]);
        if rest[i..].starts_with('#') {
            match hash {
                Some(n) => out.push_str(&format_number(locale, n)),
                None => out.push('#'),
            }
            rest = &rest[i + 1..];
            continue;
        }
        let after = &rest[i + 1..];
        let Some(end) = closing(after) else {
            out.push_str(&rest[i..]);
            return;
        };
        write_placeholder(out, locale, &after[..end], args, &rest[i..i + end + 2]);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
}

fn write_placeholder(
    out: &mut String,
    locale: &str,
    inner: &str,
    args: &[(&str, Arg)],
    whole: &str,
) {
    let mut parts = inner.splitn(3, ',');
    let name = parts.next().unwrap_or("").trim();
    let arg = args.iter().find(|(n, _)| *n == name).map(|(_, a)| a);
    let kind = parts.next().map(str::trim);
    let (Some(arg), kind) = (arg, kind) else {
        out.push_str(whole);
        return;
    };
    let options = parts.next().unwrap_or("");
    match (kind, arg) {
        (None, Arg::Text(s)) => out.push_str(s),
        (None, Arg::Number(n)) => out.push_str(&format_number(locale, *n)),
        (Some("plural"), Arg::Number(n)) => {
            let options = branches(options);
            let exact = options.iter().find(|(k, _)| {
                k.strip_prefix('=')
                    .and_then(|v| v.parse::<f64>().ok())
                    .is_some_and(|v| v == *n)
            });
            let category = plural(locale, *n).keyword();
            let chosen = exact
                .or_else(|| options.iter().find(|(k, _)| *k == category))
                .or_else(|| options.iter().find(|(k, _)| *k == "other"));
            if let Some((_, body)) = chosen {
                write_message(out, locale, body, args, Some(*n));
            }
        }
        (Some("select"), arg) => {
            let value = match arg {
                Arg::Text(s) => s.clone(),
                Arg::Number(n) => format_number("en", *n),
            };
            let options = branches(options);
            let chosen = options
                .iter()
                .find(|(k, _)| *k == value)
                .or_else(|| options.iter().find(|(k, _)| *k == "other"));
            if let Some((_, body)) = chosen {
                write_message(out, locale, body, args, None);
            }
        }
        (Some("number"), Arg::Number(n)) => out.push_str(&format_number(locale, *n)),
        _ => out.push_str(whole),
    }
}

/// `key {body} key {body}…` → `[(key, body)]`.
fn branches(options: &str) -> Vec<(&str, &str)> {
    let mut found = Vec::new();
    let mut rest = options;
    while let Some(open) = rest.find('{') {
        let key = rest[..open].trim();
        let after = &rest[open + 1..];
        let Some(end) = closing(after) else {
            break;
        };
        found.push((key, &after[..end]));
        rest = &after[end + 1..];
    }
    found
}

#[derive(Default)]
struct Catalog {
    fallback: String,
    messages: HashMap<String, HashMap<String, String>>,
}

/// Translations and the current locale, reactive: [`t`](I18n::t) and the formatting methods
/// read the locale state, so a view that uses them inside `text_dyn` follows
/// [`set_locale`](I18n::set_locale). `Copy`: move it into closures, or hand it to the whole
/// app with [`provide`](crate::provide).
///
/// A message is looked up in the current locale (`"fr-CA"`), then its language (`"fr"`),
/// then the locale given to [`new`](I18n::new); a missing message shows its key.
#[derive(Clone, Copy, Debug)]
pub struct I18n {
    locale: State<String>,
    catalog: State<Catalog>,
}

impl I18n {
    /// Translations in `locale`, which is also the fallback for missing messages.
    pub fn new(locale: &str) -> Self {
        I18n {
            locale: use_state(locale.to_string()),
            catalog: use_state(Catalog {
                fallback: locale.to_string(),
                messages: HashMap::new(),
            }),
        }
    }

    /// Adds messages for `locale` (ICU syntax, see [`format_message`]).
    #[must_use]
    pub fn messages<'a>(
        self,
        locale: &str,
        entries: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Self {
        self.catalog.update(|c| {
            let table = c.messages.entry(locale.to_string()).or_default();
            for (key, message) in entries {
                table.insert(key.to_string(), message.to_string());
            }
        });
        self
    }

    /// The current locale (subscribes the running effect).
    pub fn locale(self) -> String {
        self.locale.get()
    }

    /// The state holding the current locale, e.g. to bind a `<select>` to it.
    pub fn locale_state(self) -> State<String> {
        self.locale
    }

    /// Switches the language: every text that reads it is updated.
    pub fn set_locale(self, locale: &str) {
        if self.locale.with(|l| l != locale) {
            self.locale.set(locale.to_string());
        }
    }

    /// The message `key` in the current locale, without values.
    pub fn t(self, key: &str) -> String {
        self.t_with(key, &[])
    }

    /// The message `key` in the current locale, with `args` for its placeholders.
    pub fn t_with(self, key: &str, args: &[(&str, Arg)]) -> String {
        let locale = self.locale.get();
        self.catalog.with(|c| {
            let candidates = [locale.clone(), language(&locale), c.fallback.clone()];
            let message = candidates
                .iter()
                .find_map(|l| c.messages.get(l).and_then(|m| m.get(key)));
            match message {
                Some(m) => format_message(&locale, m, args),
                None => key.to_string(),
            }
        })
    }

    /// [`format_number`] in the current locale.
    pub fn number(self, n: f64) -> String {
        format_number(&self.locale.get(), n)
    }

    /// [`format_currency`] in the current locale.
    pub fn currency(self, amount: f64, currency: &str) -> String {
        format_currency(&self.locale.get(), amount, currency)
    }

    /// [`format_percent`] in the current locale.
    pub fn percent(self, ratio: f64, decimals: usize) -> String {
        format_percent(&self.locale.get(), ratio, decimals)
    }

    /// [`format_date`] in the current locale.
    pub fn date(self, date: Date, style: DateStyle) -> String {
        format_date(&self.locale.get(), date, style)
    }

    /// [`plural`] in the current locale.
    pub fn plural(self, n: f64) -> Plural {
        plural(&self.locale.get(), n)
    }
}

impl std::fmt::Debug for Catalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Catalog")
            .field("locales", &self.messages.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::html::p;
    use crate::view::Node;

    #[test]
    fn plural_rules_follow_cldr() {
        let forms =
            |locale: &str, ns: &[f64]| ns.iter().map(|&n| plural(locale, n)).collect::<Vec<_>>();
        use Plural::*;
        assert_eq!(
            forms("en", &[0.0, 1.0, 2.0, 1.5]),
            [Other, One, Other, Other]
        );
        assert_eq!(forms("fr", &[0.0, 1.0, 1.5, 2.0]), [One, One, One, Other]);
        assert_eq!(forms("pt-PT", &[0.0, 1.0]), [Other, One]);
        assert_eq!(
            forms("ru", &[1.0, 2.0, 5.0, 11.0, 21.0, 22.0, 112.0, 1.5]),
            [One, Few, Many, Many, One, Few, Many, Other]
        );
        assert_eq!(
            forms("pl", &[1.0, 3.0, 13.0, 23.0, 21.0]),
            [One, Few, Many, Few, Many]
        );
        assert_eq!(forms("cs", &[1.0, 3.0, 5.0, 0.5]), [One, Few, Other, Many]);
        assert_eq!(
            forms("ar", &[0.0, 1.0, 2.0, 5.0, 50.0, 100.0]),
            [Zero, One, Two, Few, Many, Other]
        );
    }

    #[test]
    fn numbers_currencies_and_percentages() {
        assert_eq!(format_fixed("en", 0.005, 2), "0.01");
        assert_eq!(format_fixed("en", -0.001, 2), "0.00");
        assert_eq!(
            format_fixed("fr", 1234567.0, 0),
            format!("1{NNBSP}234{NNBSP}567")
        );
        assert_eq!(format_fixed("es", 12345.6, 1), "12.345,6");
        assert_eq!(format_fixed("de-CH", 1234.5, 1), "1'234.5");
        assert_eq!(format_number("en", 2.0), "2");
        assert_eq!(format_number("en", f64::INFINITY), "∞");
        assert_eq!(format_currency("fr", -5.0, "EUR"), format!("-5,00{NBSP}€"));
        assert_eq!(format_currency("en", -5.0, "EUR"), "-€5.00");
        assert_eq!(format_currency("en", 3.0, "CHF"), "CHF 3.00");
        assert_eq!(format_currency("nl", 3.0, "EUR"), format!("€{NBSP}3,00"));
        assert_eq!(format_percent("en", 0.256, 1), "25.6%");
        assert_eq!(format_percent("fr", 0.5, 0), format!("50{NNBSP}%"));
    }

    #[test]
    fn dates_from_timestamps_and_in_every_style() {
        assert_eq!(Date::from_unix(0), Date::new(1970, 1, 1));
        assert_eq!(Date::from_unix(1_792_022_400), Date::new(2026, 10, 15));
        assert_eq!(Date::from_unix(-86_400), Date::new(1969, 12, 31));
        assert_eq!(Date::new(2024, 2, 29).weekday(), 4);
        assert_eq!(Date::new(1970, 1, 1).weekday(), 4);
        let d = Date::new(2026, 2, 3);
        assert_eq!(format_date("en", d, DateStyle::Medium), "Feb 3, 2026");
        assert_eq!(
            format_date("en", d, DateStyle::Full),
            "Tuesday, February 3, 2026"
        );
        assert_eq!(format_date("fr", d, DateStyle::Medium), "3 févr. 2026");
        assert_eq!(
            format_date("es", d, DateStyle::Full),
            "martes, 3 de febrero de 2026"
        );
        assert_eq!(format_date("de", d, DateStyle::Long), "3. Februar 2026");
        assert_eq!(
            format_date("pt-BR", d, DateStyle::Medium),
            "3 de fev. de 2026"
        );
        assert_eq!(format_date("ja", d, DateStyle::Short), "2026-02-03");
        assert_eq!(format_date("ja", d, DateStyle::Long), "February 3, 2026");
    }

    #[test]
    fn messages_nest_and_keep_unknown_placeholders() {
        let msg = "{n, plural, =0 {Empty} other {{n} in {place}: # left}}";
        assert_eq!(format_message("en", msg, &[("n", 0.into())]), "Empty");
        assert_eq!(
            format_message("de", msg, &[("n", 1500.into()), ("place", "Lager".into())]),
            "1.500 in Lager: 1.500 left"
        );
        assert_eq!(format_message("en", "Hi {name} #1", &[]), "Hi {name} #1");
        assert_eq!(
            format_message("en", "Broken {x", &[("x", 1.into())]),
            "Broken {x"
        );
    }

    #[test]
    fn translations_follow_the_locale_and_fall_back() {
        let i18n = I18n::new("en")
            .messages("en", [("hello", "Hello {name}"), ("bye", "Bye")])
            .messages("fr", [("hello", "Bonjour {name}")]);
        let view: Node = p()
            .text_dyn(move || i18n.t_with("hello", &[("name", "Ada".into())]))
            .into();
        assert_eq!(view.render(), "<p><!--#-->Hello Ada</p>");
        i18n.set_locale("fr-FR");
        assert_eq!(view.render(), "<p><!--#-->Bonjour Ada</p>");
        assert_eq!(i18n.t("bye"), "Bye");
        assert_eq!(i18n.t("missing"), "missing");
        assert_eq!(i18n.number(1.5), "1,5");
        assert_eq!(i18n.plural(0.0), Plural::One);
    }
}
