"""
SPDX-FileCopyrightText: 2022 Aurélien Gâteau <mail@agateau.com>

SPDX-License-Identifier: GPL-3.0-or-later

A set of tasks to simplify the release process. See RELEASE_CHECK_LIST.md
for details.
"""

import os
import re
import shutil
import sys

from pathlib import Path
from typing import List

from invoke import task, run


VERSION = os.environ["VERSION"]

ARTIFACTS_DIR = Path("artifacts")

MAIN_BRANCH = "master"


def erun(*args, **kwargs):
    """Like run, but with echo on"""
    kwargs["echo"] = True
    run(*args, **kwargs)


def cerun(c, *args, **kwargs):
    """Like Context.run, but with echo on"""
    kwargs["echo"] = True
    c.run(*args, **kwargs)


def ask(msg: str) -> str:
    """Show a message, wait for input and returns it"""
    print(msg, end=" ")
    return input()


def is_ok(msg: str) -> bool:
    """Show a message, append (y/n) and return True if user select y or Y"""
    answer = ask(f"{msg} (y/n)").lower()
    return answer == "y"


@task
def update_version(c):
    path = Path("Cargo.toml")
    text = path.read_text()
    text, count = re.subn(r"^version = .*", f"version = \"{VERSION}\"", text,
                          flags=re.MULTILINE)
    assert count == 0 or count == 1
    path.write_text(text)


@task
def prepare_release(c):
    run(f"gh issue list -m {VERSION}", pty=True)
    run("gh pr list", pty=True)
    if not is_ok("Continue?"):
        sys.exit(1)

    erun(f"git checkout {MAIN_BRANCH}")
    erun("git pull")
    erun("git status -s")
    if not is_ok("Continue?"):
        sys.exit(1)

    prepare_release2(c)


@task
def prepare_release2(c):
    erun("git checkout -b prep-release")

    update_version(c)

    erun(f"changie batch {VERSION}")
    print(f"Review/edit changelog (.changes/{VERSION}.md)")
    if not is_ok("Looks good?"):
        sys.exit(1)
    erun("changie merge")
    print("Review CHANGELOG.md")

    if not is_ok("Looks good?"):
        sys.exit(1)

    prepare_release3(c)


@task
def prepare_release3(c):
    erun("git add Cargo.toml CHANGELOG.md .changes")
    erun(f"git commit -m 'Prepare {VERSION}'")
    erun("git push -u origin prep-release")

    erun("cargo publish --dry-run")
    erun("cargo package --list")


@task
def tag(c):
    erun(f"git checkout {MAIN_BRANCH}")
    erun("git merge --no-ff prep-release")
    erun(f"git tag -a {VERSION} -m 'Releasing version {VERSION}'")

    if not is_ok("Push tag?"):
        sys.exit(1)

    erun("git push")
    erun("git push --tags")
    erun("git push -d origin prep-release")
    erun("git branch -d prep-release")


def get_artifact_list() -> List[Path]:
    assert ARTIFACTS_DIR.exists()
    return list(ARTIFACTS_DIR.glob("*.tar.bz2"))


@task
def download_artifacts(c):
    if ARTIFACTS_DIR.exists():
        shutil.rmtree(ARTIFACTS_DIR)
    ARTIFACTS_DIR.mkdir()
    erun(f"gh run download --dir {ARTIFACTS_DIR}", pty=True)


@task
def publish(c):
    files_str = " ".join(str(x) for x in get_artifact_list())
    erun(f"gh release create {VERSION} -F.changes/{VERSION}.md {files_str}")
    erun("cargo publish")
