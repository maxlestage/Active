---
title: Deploy to Heroku
description: Put this documentation site, or your own app built the same way, online with Heroku from a phone, with one-tap deploys and automatic updates.
---

This guide puts the active-react website (this documentation and its presentation page) online with Heroku, using only a phone and a web browser. The same steps work for your own app if it is built the same way (see [what the repository provides](#what-the-repository-provides)).

There are two options. **Option A** creates the app in a few taps. **Option B** connects it to GitHub so every change merged into `master` goes live automatically. You can do A, then B.

## Before you start

You need:

- **A GitHub account.** Option A works with the public repository directly. Option B needs a repository you administer: fork `maxlestage/Active` first (open the repository, tap **Fork**), or use your own.
- **A Heroku account** ([signup.heroku.com](https://signup.heroku.com)) with a **payment method** added in Account settings, Billing. Heroku has no free plan anymore. The cheapest option, **Eco dynos, costs about $5 per month** for a pool of hours shared by all your Eco apps. Eco apps sleep after 30 minutes without visits, so the first visit after a pause takes a few seconds. Check Heroku's pricing page for current prices.

> **Tip:** Heroku's dashboard works in a mobile browser. If a menu looks cramped, switch your browser to "Desktop site" mode.

## Option A: one-tap deploy

1. Open this link on your phone: [Deploy to Heroku](https://heroku.com/deploy?template=https://github.com/maxlestage/Active). For your fork, replace `maxlestage/Active` with your repository in the address: `https://heroku.com/deploy?template=https://github.com/<you>/Active`.
2. Log in to Heroku if asked.
3. **App name:** pick a name in lowercase, such as `my-active-docs`. It becomes part of your address, which ends in `.herokuapp.com`.
4. **Region:** choose United States or Europe, whichever is closer to your visitors.
5. **SITE_URL** (optional): leave it empty. The site then uses the address visitors typed. Fill it in later if you add a custom domain.
6. Tap **Deploy app**. Heroku downloads the code, installs dependencies and builds the site. The log scrolls by; it takes a few minutes.
7. When "Your app was successfully deployed" appears, tap **View** to open your site, or **Manage App** to open its dashboard.

Done. The app won't update by itself when the code changes, though: that is Option B.

## Option B: automatic deploys from GitHub

1. Open [dashboard.heroku.com](https://dashboard.heroku.com) in your mobile browser and tap your app. No app yet? Tap **New**, then **Create new app**, and choose a name and region.
2. Open the **Deploy** tab.
3. Under **Deployment method**, tap **GitHub**, then **Connect to GitHub**, and authorize Heroku in the window that opens.
4. In **Search for a repository to connect to**, type `Active` (or the name of your repository), tap **Search**, then **Connect** next to the right result.
5. Under **Automatic deploys**, choose the branch **`master`**.
6. Optionally tick **Wait for CI to pass before deploy**. Heroku then waits for the repository's GitHub Actions checks to succeed, so a broken build never goes live.
7. Tap **Enable Automatic Deploys**.
8. For the very first deploy, scroll to **Manual deploy** and tap **Deploy Branch**.

From now on, **every merge into `master` redeploys the site.** You can follow each build in the **Activity** tab.

## What the repository provides

Heroku needs to know how to build and start the app. The repository includes everything, so there is nothing to configure:

| File | Role |
| --- | --- |
| `app.json` | Describes the app for the Deploy button: name, Node.js buildpack, optional `SITE_URL` variable, one web dyno |
| `Procfile` | `web: npm start`, the command Heroku runs to start the site |
| `package.json`, `heroku-postbuild` | Heroku runs this script after installing dependencies. It builds the library, then the site (`site/`) |
| `package.json`, `engines` | `"node": ">=20"`: the Node.js version Heroku installs |
| `.slugignore` | Leaves benchmarks, examples, tests and Rust sources out of the deployed bundle |
| `site/server.js` | The production server, with no dependencies |

`site/server.js` serves the prerendered pages and:

- sends visitors of `/` to their language (`/en/`, `/fr/` or `/es/`) based on their browser settings;
- fills in the site address in canonical and Open Graph URLs: `SITE_URL` if set, otherwise the address of the request;
- compresses responses with Brotli or gzip;
- caches fingerprinted assets for a year and revalidates HTML pages;
- listens on the port Heroku provides in the `PORT` environment variable.

> **Note:** Heroku's Node.js build has no Rust compiler. The site's WebAssembly module is compiled ahead of time and committed in `site/src/wasm`. If your own app uses Rust, commit the generated `pkg` files the same way, or build them in CI.

## Custom domain

1. In your app, open **Settings**, scroll to **Domains** and tap **Add domain**.
2. Type your domain, for example `www.example.com`, and confirm. Heroku shows a **DNS target** ending in `herokudns.com`.
3. At your domain registrar (most have a mobile-friendly site), add a **CNAME** record: name `www`, value the DNS target. For a root domain (`example.com`), use an ALIAS, ANAME or "CNAME flattening" record if your provider offers one.
4. In **Settings**, **Config Vars**, set `SITE_URL` to `https://www.example.com` so canonical URLs always use your domain.

DNS changes can take from a few minutes to a few hours. For HTTPS on your own domain, use **Configure SSL** in the same section; Heroku's automatic certificates are not offered on every dyno type, so check what yours includes.

## Logs

In your app, tap **More** (top right), then **View logs**. You see the server output and every request in real time. Build logs are in the **Activity** tab: tap **View build log** next to a build.

## Troubleshooting

### The build fails

Open **Activity**, then the failed build's log, and look at the last red lines.

- `npm ci` complains that `package.json` and `package-lock.json` are out of sync: run `npm install` and commit the updated lock file.
- A TypeScript or Vite error: the same error happens locally with `npm run build` at the repository root and `npm --prefix site run build`.
- `cargo` or `wasm-pack` not found: Heroku has no Rust; commit the generated WebAssembly files (see the note above).

### Application error (H10 App crashed)

The page shows "Application error" and the logs contain `code=H10`. The server crashed when starting. Look for the error just above in the logs. The usual causes:

- the server doesn't listen on `process.env.PORT` (Heroku then reports `R10 Boot timeout` after 60 seconds);
- the `start` script or the `Procfile` points to a missing file;
- a dependency needed at runtime is in `devDependencies`, which Heroku removes after the build.

`H14 No web processes running` means no dyno is running: open **Resources** and turn the `web` dyno on.

### Wrong Node.js version

Heroku picks the Node.js version from `engines.node` in the root `package.json`. To pin a major version, set for example:

```json
{
  "engines": {
    "node": "22.x"
  }
}
```

Commit, and Heroku uses it on the next deploy. The build log shows the resolved version near the top.
