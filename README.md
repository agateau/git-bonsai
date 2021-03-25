# Git Bonsai

Git Bonsai is a command-line tool to help you tend the branches of your git garden.

## Usage

Just run `git bonsai` in a git repository checkout.

## What does it do?

Git Bonsai does the following:

1. Fetches remote changes.

2. Iterates on all your local tracking branches and update them to their remote counterparts.

3. Lists branches which can be safely deleted and lets you select the ones to delete.

## Is it safe?

Git Bonsai takes several precautions to ensure it does not delete anything precious:

1. It refuses to run if there are any uncommitted changes. This includes unknown files.

2. It always prompt you before deleting any branch, and explains why this branch is safe to remove.

3. It refuses to delete a branch if it is not contained in another branch.

4. Git Bonsai never touches the remote repository.

## Demo

Here is an example repository:

```
$ git log --oneline --all --graph

* b87566d (duplicate2, duplicate1) Create duplicate1
*   e02d47e (HEAD -> master) Merging topic1
|\
| *   020b54c (topic1) Merging topic1-1
| |\
| | * cd060dc (topic1-1) Create topic1-1
| |/
| * f356524 Create topic1
|/
| * 85a9880 (topic2) Create topic2
|/
* 0f209d0 Init
```

(You can create this repository with the `create-demo-repository` script)

`topic1` and `topic1-1` branches can be safely deleted. `topic2` cannot. One of `duplicate1` and `duplicate2` can also be deleted, but not both.

Let's run Git Bonsai:

```
$ git bonsai
Info: Fetching changes
These branches point to the same commit, but no other branch contains this
commit, so you can delete all of them but one.

Select branches to delete:
> [x] duplicate1
  [x] duplicate2
```

I press `Space` to uncheck `duplicate1`, then `Enter` to continue.

```
Info: Deleting duplicate2
Select branches to delete:
> [x] topic1, contained in:
      - master
      - duplicate1

  [x] topic1-1, contained in:
      - topic1
      - duplicate1
      - master
```

Looks good to me, so I press `Enter`.

```
Info: Deleting topic1
Info: Deleting topic1-1
```

Let's look at the repository now:

```
$ git log --oneline --all --graph

* 0dfd179 (duplicate1) Create duplicate1
*   5d06a2d (HEAD -> master) Merging topic1
|\
| *   6a3b1de Merging topic1-1
| |\
| | * 7671947 Create topic1-1
| |/
| * c328fee Create topic1
|/
| * 1616d9e (topic2) Create topic2
|/
* 71913d9 Init
```

## Installation

### Stable version

The easiest way to install is to download an archive from the [release page][release], unpack it and copy the `git-bonsai` binary in a directory in `$PATH`.

[release]: https://github.com/agateau/git-bonsai/releases

### Git snapshots

Snapshots from the master branch are available from [builds.agateau.com/git-bonsai](https://builds.agateau.com/git-bonsai).

## Building it

Git Bonsai is written in [Rust][]. To build it, install Rust and then run:

    cargo install git-bonsai

[Rust]: https://www.rust-lang.org

## Why yet another git cleaning tool?

I created Git Bonsai because I wanted a tool like this but also as a way to learn Rust. There definitely are similar tools, probably more capable, and the Rust code probably needs work, pull requests are welcome!
