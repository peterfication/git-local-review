# Release Process

The x.x.x stands for the version that should be released.

1. Create a new branch "release-x-x-x"
2. Update the version in the [Cargo.toml](Cargo.toml)
3. Add the version and the current date to the changes under `Unreleased` in the [CHANGELOG.md](CHANGELOG.md) (while keeping the `Unreleased` heading for the next changes)
4. Execute `cargo build` to update `Cargo.lock` and add it to the commit
5. Create a commit with the message "Release version x.x.x"
6. Create a tag on this commit in the format "vx.x.x"
7. Push the branch with the tag, create a pull request and merge it after the CI succeeded (`git push origin release-x-x-x && git push origin vx.x.x`)
8. Create a release on GitHub for the newly created tag with the changelog entry for this version.
9. Add release binaries and checksums to the release assets.

## Release binaries

TODO
