
# Releasing

1. Update the version number in [Cargo.toml](./Cargo.toml)
2. Update [CHANGELOG.md](./CHANGELOG.md)
3. (if needed) Update the version numbers in [README.md](./README.md)
4. Commit with the following subject line: `release: vX.Y.Z`
5. Run `cargo release` to check the release
6. Run `cargo release --execute` to release
