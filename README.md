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

2. To ensure branches are safe to merge, Git Bonsai always deletes branches
   using `git branch -d` not `git branch -D`. To do so, it switches to the
   first branch containing the branch to delete before deleting it. For
   example, given this git commit graph:

```
    o          b2
   / \
  o---o--o     b1
 /        \
o----------o-- master

```

Git Bonsai detects branch `b2` can be merged because `b1` and `master` contain
it and `b1` can be merged because `master` contains it.

To delete them it first switches to `b1` and run `git branch -d b2`. Then it
switches to `master` and run `git branch -d b1`.

3. Git Bonsai never touches the remote repository.

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
