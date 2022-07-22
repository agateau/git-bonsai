## Prepare

- [ ] Setup

    pip install invoke
    export VERSION=<the-new-version>

- [ ] Prepare

    invoke prepare-release

## Tag

- [ ] Wait for CI to be happy

- [ ] Create tag

    invoke tag

## Publish

    invoke download-artifacts
    invoke publish

## Post publish

- [ ] Bump version to x.y.z+1-alpha.1

    VERSION=<the-new-version> invoke update-version

- [ ] Write blog post
