# Changelog

## 0.3.0 - 2022-11-13

### Changed

- Git Bonsai now detects the default branch and always considers it protected.

## 0.2.2 - 2022-07-22

### Added

- Make it possible to configure protected branches, using `git config`. See `git-bonsai --help` for details.

## 0.2.1 - 2021-05-25

### Changed

- Internal: code is more Rust-like now (#4).
- Internal: CI now checks formatting and runs clippy linter.

### Fixed

- git-bonsai no longer fails when a branch is checked out in a separate worktree. Worktree branches are just ignored (#5).

## 0.2.0 - 2020-03-29

### Added

- Added a --no-fetch option.
- Implemented removal of identical branches.
- Added integration tests.
- The CI now builds git-bonsai on Windows and macOS.

### Changed

- Improved README.

## 0.1.0 - 2020-03-22

First release.
