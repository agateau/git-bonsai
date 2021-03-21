# Git Bonsai

Git Bonsai is a command-line tool to help you tend the branches of your git
garden.

## Usage

Just run `git bonsai` in a git repository checkout.

## What does it do?

At startup, Git Bonsai fetches remote changes and iterate on all your local
branches to update the tracking ones with their remote counter-parts.

Then it shows you a list of the merged branches and asks if you want to delete
them.

## Is it safe?

Git Bonsai takes several precautions to ensure it does not delete anything
precious:

1. It refuses to run if there are any uncommitted changes. This includes
   unknown files.

2. It always prompt you before deleting any branch, and explains why this
   branch is safe to remove.

3. It refuses to delete a branch if it is not contained in another branch.

4. Git Bonsai never touches the remote repository.

## Installation

The easiest way to install is to download an archive from the [release
page][release], unpack it and copy the `git-bonsai` binary in a directory in
$PATH.

[release]: https://github.com/agateau/git-bonsai/releases

## Building it

Git Bonsai is written in [Rust][]. To build it, install Rust and then run:

    cargo install git-bonsai

[Rust]: https://www.rust-lang.org

## Why yet another git cleaning tool?

I created Git Bonsai because I wanted a tool like this but also as a way to
learn Rust. There definitely are similar tools, probably more capable, and the
Rust code definitely needs work, pull requests are welcome!
