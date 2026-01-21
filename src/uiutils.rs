// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::{DateTime, FixedOffset, Utc};

pub fn format_datetime(datetime: &DateTime<FixedOffset>) -> String {
    let delta = datetime.signed_duration_since(Utc::now());
    if delta.num_days() == 0 {
        return datetime.format("Today, %H:%M").to_string();
    }
    if delta.num_days() > -7 {
        // Less than a week ago
        return datetime.format("%A, %H:%M").to_string();
    }
    if delta.num_days() > -365 {
        return datetime.format("%b %d, %H:%M").to_string();
    }
    datetime.format("%Y %b %d, %H:%M").to_string()
}
