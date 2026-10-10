---
title: Publishing releases
description: Release the active-react npm package and the active Rust crate from a phone by publishing a GitHub Release. A workflow does the rest.
---

Publishing a new version takes one action: **publish a GitHub Release with a tag like `v0.2.0`**. A GitHub Actions workflow then tests the code and publishes the npm package and the Rust crate. Everything happens on the GitHub, npm and crates.io websites, so a phone is enough.

This page is for maintainers of the repository (or of your fork).

## How the release workflow works

The workflow lives in `.github/workflows/release.yml` and runs when a release is published:

1. **Read the version from the tag.** `v0.2.0` gives `0.2.0`. A tag that doesn't look like `v1.2.3` stops the workflow with an error.
2. **npm package** (only if the `NPM_TOKEN` secret exists): install, run the tests, write the version into `package.json`, then `npm publish --access public`.
3. **Rust crate** (only if the `CARGO_REGISTRY_TOKEN` secret exists): write the version into `crates/active/Cargo.toml`, run `cargo test`, then `cargo publish`.

The version comes from the tag, so **you never edit version numbers by hand**. The files in the repository keep their old number; only the published packages carry the new one.

The npm and crate jobs are independent: if one registry has no secret, its job is skipped with a warning and the other still publishes. That lets you start with npm only, for example.

## 1. Create an npm token

1. Open [npmjs.com](https://www.npmjs.com) and log in (create an account if needed).
2. Tap your **avatar** (top right), then **Access Tokens**.
3. Tap **Generate New Token**, then **Granular Access Token**.
4. Give it a name, such as `github-release`, and an expiration date.
5. Under **Packages and scopes**, set the permission to **Read and write**. Choose **All packages** for the first release, since `active-react` doesn't exist on npm yet. After the first release you can create a new token limited to `active-react` only.
6. If your account requires two-factor authentication to publish, enable the token option that lets it bypass 2FA; otherwise the workflow can't answer the 2FA prompt.
7. Tap **Generate Token** and **copy it now**: npm shows it only once.

> **Warning:** npm limits how long write tokens stay valid. Note the expiry date and replace the secret before it expires, or the next release will fail at the publish step.

## 2. Create a crates.io token

1. Open [crates.io](https://crates.io) and tap **Log in with GitHub**.
2. Open your avatar menu, then **Account Settings**. Add and **verify your email address**: crates.io refuses to publish without one.
3. Open **API Tokens** and tap **New Token**.
4. Name it, for example `github-release`, and pick an expiration.
5. Tick the scopes **publish-new** (to create the crate the first time) and **publish-update** (for later versions).
6. Tap **Generate Token** and copy it.

## 3. Add the secrets to GitHub

The GitHub mobile app doesn't show repository settings, so use the website in your browser.

1. Open the repository on [github.com](https://github.com), then **Settings**.
2. In the menu, open **Secrets and variables**, then **Actions**.
3. Tap **New repository secret**. Name: `NPM_TOKEN`. Secret: paste the npm token. Tap **Add secret**.
4. Tap **New repository secret** again. Name: `CARGO_REGISTRY_TOKEN`. Secret: paste the crates.io token. Tap **Add secret**.

Secret names must match exactly. Once saved, nobody can read a secret back, not even you: to change one, update it with a new value.

## 4. Publish a release

1. Open the repository's **Releases** page. On a phone, the quickest way is to go straight to `https://github.com/maxlestage/Active/releases/new` (replace the owner for a fork).
2. If you are on the Releases list, tap **Draft a new release**.
3. Tap **Choose a tag**, type `v0.1.0`, and select **Create new tag: v0.1.0 on publish**.
4. Keep `master` as the target branch.
5. Set the release title, for example `v0.1.0`. Tap **Generate release notes** if you want GitHub to list the merged changes.
6. Tap **Publish release**.

## 5. Watch it run

Open the **Actions** tab of the repository: a **Release** run appears within seconds. Tap it to see the three jobs (`version`, `npm`, `crate`) and their logs. A green check means the job succeeded. A job whose secret is missing also ends green, with a warning such as "NPM_TOKEN secret missing — npm publish skipped" in the run summary.

Then check the result:

- [npmjs.com/package/active-react](https://www.npmjs.com/package/active-react) shows the new version.
- [crates.io/crates/active](https://crates.io/crates/active) shows the new version, and [docs.rs/active](https://docs.rs/active) builds the API docs automatically a few minutes later.

### If something fails

- **Tests fail:** nothing is published by that job. Fix the code, merge, and publish a new release with the next patch version.
- **npm says the version already exists:** registries never accept the same version twice, even after unpublishing. Publish the next version instead.
- **401 or 403 errors:** the token is missing, expired or lacks write permission. Create a new one and update the secret.
- **Only one registry got the new version:** fix the cause and release the next patch version, so both registries end up in sync.

## Versioning

Follow [semantic versioning](https://semver.org): `MAJOR.MINOR.PATCH`.

- **PATCH** (`0.1.0` to `0.1.1`): bug fixes only.
- **MINOR** (`0.1.1` to `0.2.0`): new features. While the version starts with `0.`, also use a minor bump for breaking changes: npm and Cargo treat `0.1.x` and `0.2.x` as incompatible.
- **MAJOR** (`1.0.0` to `2.0.0`): breaking changes, once you reach `1.0.0`.

Use plain `vMAJOR.MINOR.PATCH` tags. Pre-release tags such as `v0.2.0-beta.1` pass the workflow's check, but the workflow has no special handling for them.

Keep the npm package and the crate on the same version number: one release publishes both, which keeps them easy to match.

## After the first release

Once both packages are published, users install them with:

```sh
npm i active-react
```

```sh
cargo add active
```

Remember to update the [installation](../installation/) instructions so they no longer point to GitHub.
