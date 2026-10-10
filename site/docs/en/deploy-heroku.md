---
title: Deploy to Heroku
description: Put an active site online with Heroku using only a phone: fork on GitHub, deploy with one button, then redeploy automatically on every push to master.
---

This guide puts a site built with `active` online with [Heroku](https://www.heroku.com), using only a phone and a web browser. It uses this repository (the `active` website) as the example; the last sections show how to deploy your own app the same way.

Heroku builds the repository's `Dockerfile` (Rust server and WebAssembly bundle), runs the server, and gives you an address ending in `herokuapp.com`. Once connected to GitHub, every push to `master` goes live by itself.

## Before you start

You need two accounts:

- **GitHub.** A fork of the repository lets you change the site and deploy your changes. The GitHub app and [github.com](https://github.com) in a mobile browser both work.
- **Heroku**, at [signup.heroku.com](https://signup.heroku.com), with a **payment method** added under Account settings, Billing. Heroku has no free plan: see [Costs](#costs) below.

> **Tip:** Heroku's dashboard works in a mobile browser. If a page looks cramped, switch the browser to "Desktop site" mode.

## 1. Fork the repository

1. Open [github.com/maxlestage/Active](https://github.com/maxlestage/Active).
2. Tap **Fork**, keep the default name `Active`, and confirm with **Create fork**.

You now have `https://github.com/<you>/Active`, which you can edit from the GitHub app or website. To deploy the original repository without changes, you can skip this step.

## 2. Deploy with the button

The button reads `app.json` and creates a Heroku app that builds the repository with Docker.

1. Open the deploy link. For the original repository it is [heroku.com/deploy?template=https://github.com/maxlestage/Active](https://heroku.com/deploy?template=https://github.com/maxlestage/Active). For your fork, use `https://heroku.com/deploy?template=https://github.com/<you>/Active`.
2. Log in to Heroku if asked.
3. **App name:** choose a lowercase name such as `my-active-site`. Your address becomes `my-active-site-<random>.herokuapp.com` or similar, shown at the end.
4. **Region:** United States or Europe, whichever is closer to your visitors.
5. **SITE_URL:** leave it empty for now (see [Configuration](#configuration)).
6. Tap **Deploy app**.

Heroku now builds the Docker image: it installs the WebAssembly target and `wasm-bindgen-cli`, compiles the browser bundle and the server in release mode. The first build takes several minutes because everything is compiled from scratch. When "Your app was successfully deployed" appears, tap **View**.

## 3. Redeploy automatically from GitHub

The button deploys once. To publish every change you push:

1. Open [dashboard.heroku.com](https://dashboard.heroku.com) and tap the app you just created.
2. Open the **Deploy** tab.
3. Under **Deployment method**, tap **GitHub**, then **Connect to GitHub**, and authorize Heroku.
4. Search for `Active` (your fork), then tap **Connect**.
5. Under **Automatic deploys**, choose the branch **`master`** and tap **Enable Automatic Deploys**.

From now on, every commit that reaches `master` (a file edited on GitHub, a merged pull request) starts a new build. Follow builds in the **Activity** tab. The **Manual deploy** section deploys a branch on demand.

> **Warning:** Create the app with the button, then connect it. An app created with **New → Create new app** in the dashboard uses Heroku's default buildpack stack, which ignores `heroku.yml` and cannot build this repository. Moving an existing app to the container stack requires the Heroku CLI (`heroku stack:set container --app <name>`), which a phone does not have.

### Editing the site from a phone

With automatic deploys on, the whole loop works from the GitHub app or website: open a file in your fork (for example a page in `site/docs/en/`), tap the pencil icon, edit, and commit to `master`. Heroku rebuilds and the change is online a few minutes later.

## How the repository is deployed

Four files tell Heroku what to do. You do not need to change them to deploy this site.

| File | Role |
| --- | --- |
| `app.json` | Read by the Deploy button: app description, `"stack": "container"`, the optional `SITE_URL` variable, one `web` dyno |
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

For an app laid out like the [Quick start](../quick-start/) (package `my-site`, `pkg/` served by the server), copy `heroku.yml` and `app.json` to your repository, adapt `app.json`'s name and description, and use this `Dockerfile`:

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

Add a `.dockerignore` containing `target` and `pkg`, and make the server listen on `PORT` on all interfaces:

```rust
#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    let app = axum::Router::new(); // your routes
    axum::serve(listener, app).await.unwrap();
}
```

Keep the `wasm-bindgen-cli` version in the `Dockerfile` equal to the `wasm-bindgen` version in `Cargo.lock`. Then deploy with `https://heroku.com/deploy?template=https://github.com/<you>/<repo>` and connect GitHub as above.

## Troubleshooting

### The build fails

Open **Activity**, then the failed build's log, and read the last lines.

- **"No default language could be detected"** or a buildpack error: the app is not on the container stack. Create a new app with the Deploy button.
- **A `wasm-bindgen` schema or version error:** the CLI version in the `Dockerfile` differs from the `wasm-bindgen` crate in `Cargo.lock`. Make them equal.
- **A Rust compile error:** the same error appears locally with the commands from [Building the client](../client-build/). Fix it, commit, and the next build picks it up.

### Application error

The page shows "Application error". Open the logs:

- `code=H10` (app crashed): the server stopped at startup. The lines just above say why.
- `R10` (boot timeout): the server did not listen on `PORT` within 60 seconds. Bind `0.0.0.0` and the `PORT` variable.
- `code=H14` (no web processes running): open **Resources** and turn the `web` dyno on.
