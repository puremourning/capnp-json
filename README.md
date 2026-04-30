# capnp-json

[![crates.io](https://img.shields.io/crates/v/capnp-json.svg)](https://crates.io/crates/capnp-json)
[![docs.rs](https://img.shields.io/docsrs/capnp-json)](https://docs.rs/capnp-json)
[![license](https://img.shields.io/crates/l/capnp-json.svg)](crates/capnp-json/LICENSE)

A [Cap'n Proto](https://capnproto.org) JSON codec for
[capnp-rust](https://github.com/capnproto/capnproto-rust), wire-compatible
with the C++ `capnp::JsonCodec`.

The published crate lives in [`crates/capnp-json/`](crates/capnp-json/) — see
its [README](crates/capnp-json/README.md) for usage and supported features,
or the rendered API docs at <https://docs.rs/capnp-json>.

## Releasing

`capnp-json` follows [SemVer](https://semver.org). While the crate is on
`0.x`, breaking changes bump the minor version (`0.1.0` → `0.2.0`) and
backward-compatible changes bump the patch version (`0.1.0` → `0.1.1`).

Releases are cut by the
[`Release`](.github/workflows/release.yml) workflow. To publish a new
version:

1. Make sure `main` is green on CI and contains everything you want in the
   release.
2. From the GitHub **Actions** tab, run **Release** via *Run workflow* and
   enter the new version (e.g. `0.2.0`, no `v` prefix).

The workflow then:

- bumps `version` in the root [`Cargo.toml`](Cargo.toml) (and the install
  snippet in the crate README on minor bumps);
- refreshes `Cargo.lock` and runs `cargo test --workspace`;
- runs `cargo publish --dry-run` as a sanity check;
- commits the bump as `Release vX.Y.Z`, tags `vX.Y.Z`, and pushes both;
- publishes to [crates.io](https://crates.io/crates/capnp-json);
- creates a GitHub release with auto-generated notes.

docs.rs picks up the new version from crates.io automatically — give it a
few minutes and check <https://docs.rs/capnp-json>.

### Required setup

- `CARGO_REGISTRY_TOKEN` repository secret — a crates.io API token from
  <https://crates.io/me>, scoped to `publish-update` for `capnp-json`.
- The default `GITHUB_TOKEN` is enough for the commit/tag push and release
  creation, provided `main` doesn't have branch protection that blocks
  pushes from `github-actions[bot]`. If it does, either relax the rule for
  the bot or swap in a fine-grained PAT.

## License

Licensed under the [MIT License](crates/capnp-json/LICENSE).

Copyright (c) 2025 Ben Jackson and Cap'n Proto contributors.
