# Publishing on crates.io

Active is published on [crates.io](https://crates.io/crates/active) as the `active` crate,
under its [proprietary license](LICENSE): the source is visible there so that it can be
audited and built, and anyone may depend on it.

## One release = one tag

1. Bump `version` in [`crates/active/Cargo.toml`](crates/active/Cargo.toml) and commit.
2. Open **Releases → Draft a new release**, create the tag `vX.Y.Z` (the same version with a
   `v` in front) and tap **Publish release**.

The [`release.yml`](.github/workflows/release.yml) workflow runs the tests and publishes the
crate. It works from the GitHub website on a phone; nothing is installed locally. The whole
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
