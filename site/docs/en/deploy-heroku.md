---
title: Deploy to Heroku
description: Put an Active site online with Heroku using only a phone: fork on GitHub, deploy with one button, then redeploy automatically on every push to master.
---

This guide puts a site built with `active` online with [Heroku](https://www.heroku.com), using only a phone and a web browser. It uses this repository (the `active` website) as the example; the last sections show how to deploy your own app the same way.

Heroku builds the repository with the Rust buildpack [`emk/rust`](https://elements.heroku.com/buildpacks/emk/heroku-buildpack-rust) (Rust server and WebAssembly bundle), runs the server, and gives you an address ending in `herokuapp.com`. Once connected to GitHub, every push to `master` goes live by itself. The repository's `Dockerfile` remains another way to build it: see [With Docker](#with-docker).

## Before you start

You need two accounts:

- **GitHub.** A fork of the repository lets you change the site and deploy your changes. The GitHub app and github.com in a mobile browser both work.
- **Heroku**, at [signup.heroku.com](https://signup.heroku.com), with a **payment method** added under Account settings, Billing. Heroku has no free plan: see [Costs](#costs) below.

> **Tip:** Heroku's dashboard works in a mobile browser. If a page looks cramped, switch the browser to "Desktop site" mode.

## 1. Fork the repository

1. Open [maxlestage.github.io/Active](https://maxlestage.github.io/Active/).
2. Tap **Fork**, keep the default name `Active`, and confirm with **Create fork**.

You now have `https://github.com/<you>/Active`, which you can edit from the GitHub app or website. To deploy the original repository without changes, you can skip this step.

## 2. Deploy with the button

The button reads `app.json` and creates a Heroku app on the `heroku-26` stack that builds the repository with the `emk/rust` buildpack.

1. Open the deploy link of your repository: `https://heroku.com/deploy?template=https://github.com/maxlestage/Active`.
2. Log in to Heroku if asked.
3. **App name:** choose a lowercase name such as `my-active-site`. Your address becomes `my-active-site-<random>.herokuapp.com` or similar, shown at the end.
4. **Region:** United States or Europe, whichever is closer to your visitors.
5. **SITE_URL:** leave it empty for now (see [Configuration](#configuration)).
6. Tap **Deploy app**.

Heroku now runs the buildpack: it installs Rust, then `heroku-build.sh` compiles the server in release mode, adds the WebAssembly target, compiles the browser bundle and runs `wasm-bindgen`. The first build takes several minutes because everything is compiled from scratch. When "Your app was successfully deployed" appears, tap **View**.

## 3. Redeploy automatically from GitHub

The button deploys once. To publish every change you push:

1. Open [dashboard.heroku.com](https://dashboard.heroku.com) and tap the app you just created.
2. Open the **Deploy** tab.
3. Under **Deployment method**, tap **GitHub**, then **Connect to GitHub**, and authorize Heroku.
4. Search for `Active` (your fork), then tap **Connect**.
5. Under **Automatic deploys**, choose the branch **`master`** and tap **Enable Automatic Deploys**.

From now on, every commit that reaches `master` (a file edited on GitHub, a merged pull request) starts a new build. Follow builds in the **Activity** tab. The **Manual deploy** section deploys a branch on demand.

> **Tip:** The button is not required. An app created with **New → Create new app** in the dashboard works too: before its first deploy, open its **Settings** tab, under **Buildpacks** tap **Add buildpack**, enter `emk/rust` and save. The app must be on a regular stack such as `heroku-26`, not on the container stack, which ignores buildpacks.

### With the Heroku CLI

On a computer, from a clone of the repository:

```sh
heroku create --stack heroku-26 --buildpack emk/rust
git push heroku master
```

An existing app, for example one on the container stack, switches to the buildpack with these commands; the next deploy uses it:

```sh
heroku stack:set heroku-26 --app <name>
heroku buildpacks:set emk/rust --app <name>
```

### Editing the site from a phone

With automatic deploys on, the whole loop works from the GitHub app or website: open a file in your fork (for example a page in `site/docs/en/`), tap the pencil icon, edit, and commit to `master`. Heroku rebuilds and the change is online a few minutes later.

## How the repository is deployed

These files tell Heroku what to do. You do not need to change them to deploy this site.

| File | Role |
| --- | --- |
| `app.json` | Read by the Deploy button: app description, `"stack": "heroku-26"`, the `emk/rust` buildpack, the optional `SITE_URL` variable, one `web` dyno |
| `RustConfig` | Read by the buildpack: the Rust version (`stable`), `RUST_SKIP_BUILD=1` to skip its own build, then runs `heroku-build.sh` |
| `heroku-build.sh` | Builds the server into `target/release` and the WebAssembly bundle into `site/pkg` |
| `Procfile` | Starts the `web` process: `SITE_DIR=site exec target/release/active-site` |

Without its comments, `RustConfig` is three lines:

```sh
VERSION=stable
RUST_SKIP_BUILD=1
trap 'status=$?; if [ "$status" -eq 0 ]; then bash "$BUILD_DIR/heroku-build.sh" || exit 1; fi' EXIT
```

The buildpack reads this file, installs Rust, then skips its own build, which would only run a plain `cargo build --release`; the `trap` runs `heroku-build.sh` once the buildpack is done. The script runs the same commands as the CI and as [Building the client](../client-build/), and copies the server into `target/release` when Cargo builds into the buildpack's cache (`CARGO_TARGET_DIR`):

```sh
cargo build --release -p active-site --bin active-site
rustup target add wasm32-unknown-unknown
cargo build -p active-site --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/active_site.wasm
```

It uses the `wasm-bindgen` version recorded in `Cargo.lock`: it downloads that release's prebuilt binary, or compiles it with `cargo install` if the download fails. The server then runs from the repository's directory and serves `site/static`, `site/docs` and `site/pkg` from `SITE_DIR=site`.

### With Docker

`heroku.yml`, the `Dockerfile` and `.dockerignore` remain: an app on the container stack ignores buildpacks and builds the `Dockerfile` instead, and the same image runs on any container host. To use them on Heroku, switch the app with the Heroku CLI (`heroku stack:set container --app <name>`) and deploy again; `heroku stack:set heroku-26 --app <name>` brings it back to the buildpack.

| File | Role |
| --- | --- |
| `heroku.yml` | Tells the container stack to build the `web` process from `Dockerfile` |
| `Dockerfile` | Two stages: a Rust image builds the bundle and the server, a small Debian image runs them |
| `.dockerignore` | Keeps `target/`, `pkg/`, `bench/` and Git data out of the build |

`heroku.yml` is three lines:

```yaml
build:
  docker:
    web: Dockerfile
```

The `Dockerfile` runs the same commands as the CI and as [Building the client](../client-build/):

```dockerfile
FROM rust:1-slim-bookworm AS build
RUN rustup target add wasm32-unknown-unknown \
 && cargo install wasm-bindgen-cli --version 0.2.129 --locked
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY examples examples
COPY site site
RUN cargo build -p active-site --lib --target wasm32-unknown-unknown --profile wasm-release \
 && wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/active_site.wasm \
 && cargo build -p active-site --bin active-site --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /src/target/release/active-site /app/active-site
COPY --from=build /src/site/static /app/site/static
COPY --from=build /src/site/docs /app/site/docs
COPY --from=build /src/site/pkg /app/site/pkg
ENV SITE_DIR=/app/site
CMD ["/app/active-site"]
```

The final image contains only the server binary and the files it serves, so it starts in a fraction of a second.

## Configuration

Heroku passes settings to the server as environment variables, called **config vars**. Set them in the app's **Settings** tab, under **Config Vars**, **Reveal Config Vars**. Changing one restarts the app.

| Variable | Set by | Meaning |
| --- | --- | --- |
| `PORT` | Heroku | The port the server must listen on. The server reads it and falls back to `3000` locally. Never set it yourself. |
| `SITE_URL` | you, optional | Public address used in canonical URLs, Open Graph URLs, `sitemap.xml` and `robots.txt`, for example `https://www.example.com`. Empty: the server uses the address of each request. |

Set `SITE_URL` once you use a custom domain, so search engines always see the same address.

## Logs

- **Server logs:** in the app, tap **More** (top right), then **View logs**. You see each request and anything the server prints.
- **Build logs:** in the **Activity** tab, tap **View build log** next to a build.

With the Heroku CLI on a computer, `heroku logs --tail --app <name>` streams the same logs.

## Custom domain

1. In **Settings**, under **Domains**, tap **Add domain** and type your domain, for example `www.example.com`. Heroku shows a **DNS target**.
2. At your domain registrar, add a **CNAME** record: name `www`, value the DNS target. For a root domain (`example.com`), use an ALIAS, ANAME or flattened CNAME record if your provider offers one.
3. Set the `SITE_URL` config var to `https://www.example.com`.

DNS changes take from a few minutes to a few hours. For HTTPS on your domain, use the SSL options in the same **Domains** section; automatic certificates depend on the dyno type, so check what yours includes.

## Costs

Heroku has no free tier. At the time of writing:

- **Eco** dynos cost about $5 per month for a pool of 1,000 dyno hours shared by all your Eco apps. An Eco app sleeps after 30 minutes without traffic, and the next visit waits a few seconds while it wakes up.
- **Basic** dynos cost about $7 per month per dyno and never sleep.

Change the dyno type in the **Resources** tab. Check [heroku.com/pricing](https://www.heroku.com/pricing) for current prices. A server built with `active` needs very little memory, so the smallest dyno is enough for a documentation site or a blog.

> **Tip:** If your site has no server-side logic, a [static site](../static-site/) can be hosted for free on GitHub Pages or similar services instead.

## Deploy your own app

For a single crate whose binary is the app, the `emk/rust` buildpack's plain `cargo build --release` is enough: add a `Procfile` with `web: ./target/release/my-site`, and optionally a `RustConfig` with `VERSION=1.90` to pin the Rust version.

When more is needed, such as the WebAssembly bundle of an app laid out like the [Quick start](../quick-start/) (package `my-site`, `pkg/` served by the server), a package of a workspace or an example, do as this repository, with four files at the root of your repository.

`RustConfig` sets the Rust version, skips the buildpack's own build, and runs the build script once Rust is installed:

```sh
VERSION=stable
RUST_SKIP_BUILD=1
trap 'status=$?; if [ "$status" -eq 0 ]; then bash "$BUILD_DIR/heroku-build.sh" || exit 1; fi' EXIT
```

`heroku-build.sh` builds the server, copies it into `target/release` when Cargo builds into the buildpack's cache (`CARGO_TARGET_DIR`), then builds the WebAssembly bundle into `pkg/`:

```bash
#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
target="${CARGO_TARGET_DIR:-target}"
cargo build --release --bin my-site
mkdir -p target/release
if [ "$target" != target ]; then
  cp "$target/release/my-site" target/release/
fi
rustup target add wasm32-unknown-unknown
cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
version=$(grep -A1 '^name = "wasm-bindgen"$' Cargo.lock | sed -n 's/^version = "\(.*\)"$/\1/p')
bin="$target/wasm-bindgen-$version"
[ -x "$bin/bin/wasm-bindgen" ] || cargo install wasm-bindgen-cli --version "$version" --locked --root "$bin"
"$bin/bin/wasm-bindgen" --target web --no-typescript --out-dir pkg "$target/wasm32-unknown-unknown/wasm-release/my_site.wasm"
```

`Procfile` starts the server:

```
web: exec target/release/my-site
```

`app.json`, for the Deploy button (adapt the name and description):

```json
{
  "name": "my-site",
  "description": "My site built with active.",
  "stack": "heroku-26",
  "buildpacks": [{ "url": "emk/rust" }],
  "formation": { "web": { "quantity": 1 } }
}
```

The script takes the `wasm-bindgen-cli` version from `Cargo.lock`, so the two always match; compiling the CLI adds a few minutes to the build. `RustConfig` runs the script with `bash`, so it does not need the executable bit, which a file created on github.com does not have.

Make the server listen on `PORT` on all interfaces:

```rust
use vitesse::prelude::*;

fn main() -> std::io::Result<()> {
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3000);
    let app = App::new(); // your routes
    app.run(port)
}
```

Then deploy with `https://heroku.com/deploy?template=https://github.com/<you>/<repo>` and connect GitHub as above, or create the app in the dashboard or with the Heroku CLI and add the `emk/rust` buildpack.

### Your app with Docker

To build with Docker instead, copy `heroku.yml`, set `"stack": "container"` in `app.json` and remove its `buildpacks`, and use this `Dockerfile` instead of `RustConfig`, `heroku-build.sh` and `Procfile`:

```dockerfile
FROM rust:1-slim-bookworm AS build
RUN rustup target add wasm32-unknown-unknown \
 && cargo install wasm-bindgen-cli --version 0.2.129 --locked
WORKDIR /src
COPY . .
RUN cargo build --lib --target wasm32-unknown-unknown --profile wasm-release \
 && wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_site.wasm \
 && cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /src/target/release/my-site /app/my-site
COPY --from=build /src/pkg /app/pkg
CMD ["/app/my-site"]
```

Add a `.dockerignore` containing `target` and `pkg`. Keep the `wasm-bindgen-cli` version in the `Dockerfile` equal to the `wasm-bindgen` version in `Cargo.lock`.

## Troubleshooting

### The build fails

Open **Activity**, then the failed build's log, and read the last lines.

- **"No default language could be detected"**: the app has no buildpack. In **Settings**, under **Buildpacks**, add `emk/rust`, then deploy again, or create a new app with the Deploy button.
- **The log shows a Docker build instead of the buildpack:** the app is on the container stack, which ignores buildpacks. Switch it with the Heroku CLI (see [With the Heroku CLI](#with-the-heroku-cli)) or create a new app with the Deploy button.
- **A `wasm-bindgen` schema or version error:** the CLI version differs from the `wasm-bindgen` crate in `Cargo.lock`. `heroku-build.sh` reads it from `Cargo.lock`; with Docker, make the version in the `Dockerfile` equal.
- **A Rust compile error:** the same error appears locally with the commands from [Building the client](../client-build/). Fix it, commit, and the next build picks it up.

### Application error

The page shows "Application error". Open the logs:

- `code=H10` (app crashed): the server stopped at startup. The lines just above say why.
- `R10` (boot timeout): the server did not listen on `PORT` within 60 seconds. Bind `0.0.0.0` and the `PORT` variable.
- `code=H14` (no web processes running): open **Resources** and turn the `web` dyno on.
