# Releasing

The suite releases in lockstep: the five crates and marlin-py move to one
version in one `release: prepare vX.Y.Z` commit. The order below is the
part that matters; each step's command is in the justfile.

1. **Fuzz.** `just fuzz-release` runs every target for one CPU-hour each.
   Zero artifacts is the bar; record the executions on the TODO card that
   asked for the run, if one did.
2. **Prepare.** Bump `[workspace.package] version` and the two pins under
   `[workspace.dependencies]` in `Cargo.toml`; the bindings crate's
   `Cargo.toml`, `pyproject.toml` and the version test; move every
   CHANGELOG's `[Unreleased]` content under `## [X.Y.Z] - yyyy-mm-dd`,
   leaving `[Unreleased]` empty, with a "lockstep bump, no behavioral
   change" entry for a crate without changes. `cargo update -w` refreshes
   the lockfile. `just release-check` confirms the fields agree.
3. **Gate.** `just ci` and `just py-ci` green. `just ci` starts by
   confirming the local stable rustc is the one CI runs.
4. **Push main and wait for green.** Both workflows must pass on the
   commit before it is tagged: the tag's own workflow publishes marlin-py
   and needs the test jobs green.
5. **Tag that commit.** `git tag -a vX.Y.Z` on the green commit, then
   `git push origin vX.Y.Z`. The Python bindings workflow builds the
   wheels and publishes to PyPI through trusted publishing; watch it with
   `gh run watch`.
6. **Publish the crates** from the tagged checkout, leaf first, each
   waiting for the previous one's index entry: `marlin-field`,
   `marlin-nmea-envelope`, `marlin-nmea-0183`, `marlin-ais`, `marlin-klv`.
   `cargo publish --dry-run` works only for a crate whose dependencies
   are already on crates.io.
7. **Confirm.** crates.io reports the version for all five; docs.rs builds
   `marlin-field`; PyPI lists the wheels and the sdist.

Downstream consumers (nexus, bosun) pin exact versions and migrate in
their own repos, with `docs/migration-<minor>.md` as the guide when the
release is breaking.
