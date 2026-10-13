# Publishing on crates.io

Active is published on [crates.io](https://crates.io/crates/active) as the `active` crate,
under its [proprietary license](LICENSE): the source is visible there so that it can be
audited and built, and anyone may depend on it.

## One release = one version bump

Bump `version` in [`crates/active/Cargo.toml`](crates/active/Cargo.toml) and push it to
`master`. Nothing else to fill in.

Once CI passes, the [`release.yml`](.github/workflows/release.yml) workflow sees a version
without a tag, creates the tag `vX.Y.Z` (the same version with a `v` in front) and the GitHub
release (notes generated from the commits), runs the tests and publishes the crate. A GitHub
release published by hand with a tag `vX.Y.Z` publishes too, taking the version from the tag.
It works from the GitHub website on a phone; nothing is installed locally. The whole
guide, with screenshots-level detail: [`site/docs/en/publishing.md`](site/docs/en/publishing.md).

## One-time setup

- A crates.io account with a verified e-mail address.
- A token from [crates.io/settings/tokens](https://crates.io/settings/tokens) with the scopes
  **publish-new** and **publish-update**, saved as the repository secret
  `CARGO_REGISTRY_TOKEN` (GitHub → Settings → Secrets and variables → Actions).

## From a computer

```sh
cargo login                          # paste the token, once
cargo publish -p active --dry-run    # packages and builds exactly what would be uploaded
cargo publish -p active
```

## The ecosystem, in order

The seven projects by Maxime Nathan Lestage depend on each other through crates.io, so they
are published in this order, each one after the previous one is online:

1. **Active** — `active` (this repository)
2. **Vitesse** — `vitesse`
3. **Securite** — `securite`
4. **Memoire** — `memoire-orm-derive`, then `memoire-orm`
5. **Paquets** — `paquets-macros`, then `paquets` (depends on the four above)
6. **FerroSwift** — `ferroswift-macros`, `ferroswift-transpiler`, `ferroswift`, `ferroswift-cli`
7. **Solidify** — `solidify` (and `solidify-rs` on npm; depends on none of the above)
