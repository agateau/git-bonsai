// SPDX-FileCopyrightText: 2026 Aurélien Gâteau <mail@agateau.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

pub trait Task {
    fn start(&mut self);
    fn update(&mut self);
    fn title(&self) -> &str;
    fn output(&self) -> &str;
    fn success(&self) -> Option<bool>;
}
