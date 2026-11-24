//! Dates en français : noms des jours et des mois, libellés de semaine, heures.

use chrono::{Datelike, Duration, NaiveDate, NaiveTime, Timelike};

pub const WEEKDAY_SHORT: [&str; 7] = ["LUN.", "MAR.", "MER.", "JEU.", "VEN.", "SAM.", "DIM."];
pub const WEEKDAY_LONG: [&str; 7] = [
    "lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche",
];
pub const WEEKDAY_LETTER: [&str; 7] = ["L", "M", "M", "J", "V", "S", "D"];
pub const MONTHS: [&str; 12] = [
    "janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre",
    "octobre", "novembre", "décembre",
];

/// Index du jour dans la semaine : 0 = lundi … 6 = dimanche.
pub fn weekday_index(date: NaiveDate) -> u8 {
    date.weekday().num_days_from_monday() as u8
}

/// Lundi de la semaine qui contient `date`.
pub fn monday_of(date: NaiveDate) -> NaiveDate {
    date - Duration::days(i64::from(weekday_index(date)))
}

/// Les sept jours de la semaine qui commence au lundi donné.
pub fn week_days(monday: NaiveDate) -> [NaiveDate; 7] {
    std::array::from_fn(|i| monday + Duration::days(i as i64))
}

pub fn capitalize(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

fn month_name(date: NaiveDate) -> &'static str {
    MONTHS[date.month0() as usize]
}

/// Titre principal : « Novembre 2026 », « Novembre – Décembre 2026 »
/// ou « Décembre 2026 – Janvier 2027 ».
pub fn header_title(monday: NaiveDate) -> String {
    let sunday = monday + Duration::days(6);
    if monday.month() == sunday.month() {
        format!("{} {}", capitalize(month_name(monday)), monday.year())
    } else if monday.year() == sunday.year() {
        format!(
            "{} – {} {}",
            capitalize(month_name(monday)),
            capitalize(month_name(sunday)),
            sunday.year()
        )
    } else {
        format!(
            "{} {} – {} {}",
            capitalize(month_name(monday)),
            monday.year(),
            capitalize(month_name(sunday)),
            sunday.year()
        )
    }
}

/// « Du 23 au 29 novembre », « Du 30 novembre au 6 décembre »,
/// « Du 29 décembre 2025 au 4 janvier 2026 ».
pub fn range_label(monday: NaiveDate) -> String {
    let sunday = monday + Duration::days(6);
    if monday.month() == sunday.month() {
        format!("Du {} au {} {}", monday.day(), sunday.day(), month_name(sunday))
    } else if monday.year() == sunday.year() {
        format!(
            "Du {} {} au {} {}",
            monday.day(),
            month_name(monday),
            sunday.day(),
            month_name(sunday)
        )
    } else {
        format!(
            "Du {} {} {} au {} {} {}",
            monday.day(),
            month_name(monday),
            monday.year(),
            sunday.day(),
            month_name(sunday),
            sunday.year()
        )
    }
}

/// Position de la semaine affichée par rapport à la semaine actuelle.
pub fn relative_label(monday: NaiveDate, today: NaiveDate) -> String {
    let weeks = (monday - monday_of(today)).num_days() / 7;
    match weeks {
        0 => "Cette semaine".to_owned(),
        1 => "Semaine prochaine".to_owned(),
        -1 => "Semaine dernière".to_owned(),
        n if n > 1 => format!("Dans {n} semaines"),
        n => format!("Il y a {} semaines", -n),
    }
}

/// Numéro de semaine ISO (celui des calendriers français).
pub fn iso_week(monday: NaiveDate) -> u32 {
    monday.iso_week().week()
}

/// « mardi 24 novembre », avec l'année si elle diffère de `today`.
pub fn long_date(date: NaiveDate, today: NaiveDate) -> String {
    let base = format!(
        "{} {} {}",
        WEEKDAY_LONG[weekday_index(date) as usize],
        date.day(),
        month_name(date)
    );
    if date.year() == today.year() {
        base
    } else {
        format!("{base} {}", date.year())
    }
}

pub fn format_time(time: NaiveTime) -> String {
    format!("{:02}:{:02}", time.hour(), time.minute())
}

/// Lit une heure saisie à la main : « 14:30 », « 14h30 », « 14h », « 9 », « 9.15 ».
/// Une chaîne vide donne `Ok(None)` (pas d'heure).
pub fn parse_time(input: &str) -> Result<Option<NaiveTime>, ()> {
    let s = input.trim().to_lowercase();
    if s.is_empty() {
        return Ok(None);
    }
    let (h, m) = match s.find([':', 'h', '.']) {
        Some(pos) => (&s[..pos], &s[pos + 1..]),
        None => (s.as_str(), ""),
    };
    let all_digits = |p: &str| p.chars().all(|c| c.is_ascii_digit());
    if h.is_empty() || h.len() > 2 || m.len() > 2 || !all_digits(h) || !all_digits(m) {
        return Err(());
    }
    let hour: u32 = h.parse().map_err(|_| ())?;
    let minute: u32 = if m.is_empty() { 0 } else { m.parse().map_err(|_| ())? };
    NaiveTime::from_hms_opt(hour, minute, 0).map(Some).ok_or(())
}

/// « Sport 18:30 » → (« Sport », 18:30). Le titre n'est découpé que
/// s'il reste du texte devant l'heure.
pub fn split_trailing_time(title: &str) -> (String, Option<NaiveTime>) {
    let trimmed = title.trim();
    if let Some((head, tail)) = trimmed.rsplit_once(char::is_whitespace) {
        let looks_like_time = tail.chars().any(|c| c.is_ascii_digit())
            && tail.contains([':', 'h'])
            && !head.trim().is_empty();
        if looks_like_time
            && let Ok(Some(time)) = parse_time(tail) {
                return (head.trim().to_owned(), Some(time));
            }
    }
    (trimmed.to_owned(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn week_helpers() {
        assert_eq!(monday_of(d(2026, 11, 24)), d(2026, 11, 23));
        assert_eq!(monday_of(d(2026, 11, 29)), d(2026, 11, 23));
        assert_eq!(monday_of(d(2026, 11, 23)), d(2026, 11, 23));
        assert_eq!(iso_week(d(2026, 11, 23)), 48);
        assert_eq!(week_days(d(2026, 11, 23))[6], d(2026, 11, 29));
    }

    #[test]
    fn labels() {
        assert_eq!(header_title(d(2026, 11, 23)), "Novembre 2026");
        assert_eq!(header_title(d(2026, 11, 30)), "Novembre – Décembre 2026");
        assert_eq!(header_title(d(2026, 12, 28)), "Décembre 2026 – Janvier 2027");
        assert_eq!(range_label(d(2026, 11, 23)), "Du 23 au 29 novembre");
        assert_eq!(range_label(d(2026, 11, 30)), "Du 30 novembre au 6 décembre");
        assert_eq!(
            range_label(d(2026, 12, 28)),
            "Du 28 décembre 2026 au 3 janvier 2027"
        );
        let today = d(2026, 11, 24);
        assert_eq!(relative_label(d(2026, 11, 23), today), "Cette semaine");
        assert_eq!(relative_label(d(2026, 11, 30), today), "Semaine prochaine");
        assert_eq!(relative_label(d(2026, 11, 16), today), "Semaine dernière");
        assert_eq!(relative_label(d(2026, 12, 14), today), "Dans 3 semaines");
        assert_eq!(relative_label(d(2026, 11, 2), today), "Il y a 3 semaines");
        assert_eq!(long_date(d(2026, 11, 24), today), "mardi 24 novembre");
        assert_eq!(long_date(d(2027, 1, 4), today), "lundi 4 janvier 2027");
    }

    #[test]
    fn time_parsing() {
        let t = |h, m| Some(NaiveTime::from_hms_opt(h, m, 0).unwrap());
        assert_eq!(parse_time(""), Ok(None));
        assert_eq!(parse_time("14:30"), Ok(t(14, 30)));
        assert_eq!(parse_time("14h30"), Ok(t(14, 30)));
        assert_eq!(parse_time("14H"), Ok(t(14, 0)));
        assert_eq!(parse_time("9"), Ok(t(9, 0)));
        assert_eq!(parse_time(" 9.05 "), Ok(t(9, 5)));
        assert_eq!(parse_time("25:00"), Err(()));
        assert_eq!(parse_time("12:60"), Err(()));
        assert_eq!(parse_time("midi"), Err(()));
        assert_eq!(format_time(t(9, 5).unwrap()), "09:05");
    }

    #[test]
    fn trailing_time() {
        let t = |h, m| Some(NaiveTime::from_hms_opt(h, m, 0).unwrap());
        assert_eq!(split_trailing_time("Sport 18:30"), ("Sport".into(), t(18, 30)));
        assert_eq!(split_trailing_time("Marché 9h"), ("Marché".into(), t(9, 0)));
        assert_eq!(split_trailing_time("Facturation 2026"), ("Facturation 2026".into(), None));
        assert_eq!(split_trailing_time("18:30"), ("18:30".into(), None));
        assert_eq!(split_trailing_time("  Relire  "), ("Relire".into(), None));
    }
}
