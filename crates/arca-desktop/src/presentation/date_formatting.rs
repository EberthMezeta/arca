//! Fechas legibles en el idioma activo (lo que antes hacía `Intl.DateTimeFormat`).

use chrono::{DateTime, Datelike, Local, Timelike, Utc};

/// Estrategia de formato por idioma: añadir un idioma es añadir una entrada.
struct DateStyle {
    language_code: &'static str,
    abbreviated_month_names: [&'static str; 12],
    format_date_time: fn(&DateStyle, &DateTime<Local>) -> String,
}

const DATE_STYLES: [DateStyle; 2] = [
    DateStyle {
        language_code: "es",
        abbreviated_month_names: ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic"],
        format_date_time: |date_style, local_date_time| {
            format!(
                "{} {} {}, {:02}:{:02}",
                local_date_time.day(),
                date_style.month_name(local_date_time),
                local_date_time.year(),
                local_date_time.hour(),
                local_date_time.minute()
            )
        },
    },
    DateStyle {
        language_code: "en",
        abbreviated_month_names: ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"],
        format_date_time: |date_style, local_date_time| {
            let (is_afternoon, twelve_hour) = local_date_time.hour12();
            let day_period = if is_afternoon { "PM" } else { "AM" };
            format!(
                "{} {}, {}, {}:{:02} {day_period}",
                date_style.month_name(local_date_time),
                local_date_time.day(),
                local_date_time.year(),
                twelve_hour,
                local_date_time.minute()
            )
        },
    },
];

impl DateStyle {
    fn month_name(&self, local_date_time: &DateTime<Local>) -> &'static str {
        self.abbreviated_month_names[local_date_time.month0() as usize]
    }
}

pub fn format_date_time(language_code: &str, utc_date_time: DateTime<Utc>) -> String {
    let local_date_time = utc_date_time.with_timezone(&Local);
    let date_style =
        DATE_STYLES.iter().find(|date_style| date_style.language_code == language_code).unwrap_or(&DATE_STYLES[1]);
    (date_style.format_date_time)(date_style, &local_date_time)
}

/// "1.5 MB": misma regla que la UI anterior (una cifra decimal salvo en bytes).
pub fn format_file_size(size_bytes: u64) -> String {
    const SIZE_UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut unit_index = 0;
    #[allow(clippy::cast_precision_loss)]
    let mut scaled_size = size_bytes as f64;
    while scaled_size >= 1024.0 && unit_index < SIZE_UNITS.len() - 1 {
        scaled_size /= 1024.0;
        unit_index += 1;
    }
    if unit_index == 0 {
        return format!("{size_bytes} B");
    }
    format!("{scaled_size:.1} {}", SIZE_UNITS[unit_index])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_sizes_use_the_largest_fitting_unit() {
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(50 * 1024 * 1024), "50.0 MB");
    }

    #[test]
    fn dates_are_formatted_per_language() {
        let utc_date_time = DateTime::parse_from_rfc3339("2026-10-22T12:00:00Z").expect("fecha").with_timezone(&Utc);

        assert!(format_date_time("es", utc_date_time).contains("oct 2026"));
        assert!(format_date_time("en", utc_date_time).starts_with("Oct"));
    }
}
