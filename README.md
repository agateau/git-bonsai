# Git Bonsai

Git Bonsai is a TUI tool to help you tend the branches of your git garden.

## Usage

Run `git bonsai` in a git repository checkout.

## What can it do?

Git Bonsai lists your branches and their state compared to their remote counterparts.

### Sync command

The Sync command fetches remote changes then iterates on all your local tracking branches and update them to their remote counterparts if they can be fast-forwarded.

### Delete command

The Delete command lets you delete branches. It is smart enough not to ask you to confirm before deletion if the branch is safe to delete. A branch is considered to be safe to delete if it is contained in another local branch, or in its remote tracking one.

### Checkout command

The Checkout command lets you change your current branch.

## Installation

### Stable version

The easiest way to install is to download an archive from the [release page][release], unpack it and copy the `git-bonsai` binary in a directory in `$PATH`.

[release]: https://github.com/agateau/git-bonsai/releases

### Git snapshots

Snapshots from the master branch are available from [builds.agateau.com/git-bonsai](https://builds.agateau.com/git-bonsai).

## Building it

Git Bonsai is written in [Rust][]. To build it, install Rust and then run:

```
cargo install git-bonsai
```

[Rust]: https://www.rust-lang.org

## Debugging

You can get a log file of what happens using the `--log PATH` option.
