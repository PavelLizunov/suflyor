use std::time::SystemTime;

pub fn now_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// "2026-05-24_15-30-12" — no chrono dep needed for a stamp.
pub(crate) fn chrono_like_stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (year, month, day, h, m, s) = unix_to_ymdhms(secs);
    format!("{year:04}-{month:02}-{day:02}_{h:02}-{m:02}-{s:02}")
}

/// Fixed Moscow offset (UTC+3 — no DST in Russia since 2014). Display-only.
pub const MSK_OFFSET_SECS: u64 = 3 * 3600;

/// Moscow wall-clock label for the archive UI: `"11.06.2026 14:30:12 (МСК)"`
/// from unix MILLISECONDS (the `started_at_ms` the indexer stores).
#[must_use]
pub fn format_msk_label(unix_ms: i64) -> String {
    let secs = (unix_ms.max(0) as u64) / 1000 + MSK_OFFSET_SECS;
    let (year, month, day, h, m, s) = unix_to_ymdhms(secs);
    format!("{day:02}.{month:02}.{year:04} {h:02}:{m:02}:{s:02} (МСК)")
}

/// Parse a session-id stamp prefix `"YYYY-MM-DD_HH-MM-SS…"` back to unix seconds.
#[must_use]
pub fn stamp_to_unix_secs(id: &str) -> Option<u64> {
    let date = id.get(0..10)?;
    let time = id.get(11..19)?;
    if id.as_bytes().get(10) != Some(&b'_') {
        return None;
    }
    let mut dp = date.split('-');
    let (y, mo, d) = (
        dp.next()?.parse::<i64>().ok()?,
        dp.next()?.parse::<u64>().ok()?,
        dp.next()?.parse::<u64>().ok()?,
    );
    let mut tp = time.split('-');
    let (h, mi, s) = (
        tp.next()?.parse::<u64>().ok()?,
        tp.next()?.parse::<u64>().ok()?,
        tp.next()?.parse::<u64>().ok()?,
    );
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 59 {
        return None;
    }
    let days = days_from_civil(y, mo, d);
    if days < 0 {
        return None; // pre-1970 stamp — not a real session id
    }
    Some((days as u64) * 86_400 + h * 3600 + mi * 60 + s)
}

/// Civil Y/M/D → days since the unix epoch (Howard Hinnant's `days_from_civil`).
pub(crate) fn days_from_civil(y: i64, m: u64, d: u64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe as i64 - 719_468
}

/// Public domain unix→Y/M/D/H/M/S, days-since-epoch math.
pub(crate) fn unix_to_ymdhms(secs: u64) -> (i32, u32, u32, u32, u32, u32) {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let h = (rem / 3600) as u32;
    let m = ((rem % 3600) / 60) as u32;
    let s = (rem % 60) as u32;

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    (year as i32, month as u32, d as u32, h, m, s)
}
