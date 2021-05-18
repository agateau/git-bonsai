- [ ] Check working tree is up to date and clean:

    git checkout dev
    git bonsai

- [ ] Bump version number in Cargo.toml

- [ ] Update CHANGELOG.md:

    r!git log --pretty=format:'- \%s (\%an)' x.y.z-1..HEAD

- [ ] Commit and push

    git commit
    git push

- [ ] Prepare Cargo package

    cargo publish --dry-run
    cargo package --list

- [ ] Merge

    git bonsai
    git checkout master
    git merge --no-ff dev

- [ ] Tag

    git tag -a x.y.z

- [ ] Push

    git push
    git push --tags

- [ ] Download build artifacts from CI

- [ ] Publish Cargo package

    cargo publish

- [ ] Publish binaries on GitHub

- [ ] Bump version to x.y.z+1-alpha

- [ ] Write blog post
