// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crossterm::event::KeyCode;

#[derive(Debug, Clone)]
pub struct Action<T>
where
    T: Clone,
{
    pub name: String,
    pub keycode: KeyCode,
    pub enabled: bool,
    pub command: T,
}

impl<T> Action<T>
where
    T: Clone,
{
    pub fn new(name: String, keycode: KeyCode, command: T) -> Self {
        Self {
            name,
            keycode,
            enabled: true,
            command,
        }
    }
}
