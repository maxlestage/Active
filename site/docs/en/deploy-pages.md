---
title: Deploy on GitHub Pages
description: Publish the active website for free on GitHub Pages from a phone: switch Pages on once, then every merge into master is online a few minutes later.
---

GitHub Pages hosts static files for free, at `https://<owner>.github.io/<repository>/`. This repository publishes its website there automatically: the `pages.yml` workflow turns the Rust server's pages into files with `active-site export`, then deploys them. Everything below can be done from a phone.

Unlike [Heroku](../deploy-heroku/), Pages runs no server: no cost, no machine to keep running, but also no code on the server side. The export replaces the few things the server computed for each request (see [What changes on a static host](#what-changes-on-a-static-host)).

## Switch Pages on (once)

1. Open the repository on GitHub (the GitHub app or the website in a mobile browser).
2. Go to **Settings**, then **Pages**.
3. Under **Build and deployment**, set **Source** to **GitHub Actions**.

That's all: there is no branch or folder to choose, the workflow uploads the site itself. For a fork, do the same in your fork; its address becomes `https://<you>.github.io/Active/`.

> **Tip:** to publish right away, open the **Actions** tab, choose **Pages**, then **Run workflow**. Otherwise the next merge into `master` publishes the site.

## How publishing works

1. You push a branch, or Claude does.
2. The CI workflow checks it (format, lints, tests, builds, export) and, when everything is green, merges it into `master`. Branches whose name starts with `wip/` are checked but never merged.
3. When CI succeeds, the **Pages** workflow starts: it builds the WebAssembly bundle, exports the site and deploys it. A few minutes later, the new version is online.

A red CI publishes nothing: the site online always comes from a version that passed every check. The Actions tab shows each run and, for Pages, the address of the published site.

> **Note:** Pages waits for CI instead of listening to pushes on `master`, because a push made by a workflow (the automatic merge) does not start other workflows.

Until Pages is switched on, the Pages workflow stops at its first job with a warning instead of failing.

## The export command

The workflow runs the same command you can run on your computer:

```sh
cargo build -p active-site --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/active_site.wasm
BASE_PATH=/Active SITE_URL=https://maxlestage.github.io/Active cargo run --release -p active-site -- export dist
```

`dist/` then holds every page in the three languages (`en/docs/seo/index.html`…), the static files, the browser bundle, `404.html`, `sitemap.xml`, `robots.txt` and the search indexes. Each file comes from the server's own handlers, so the static site and the server send the same HTML.

| Variable | Role | On GitHub Pages |
| --- | --- | --- |
| `BASE_PATH` | Prefix of every link, because a project site lives under `/<repository>/` | `/Active` (given by `actions/configure-pages`) |
| `SITE_URL` | Public address, for canonical URLs, Open Graph, hreflang and the sitemap | `https://<owner>.github.io/Active` |

Leave `BASE_PATH` empty for a site served at the root of its domain (a custom domain, or another static host).

To check an export locally, serve the folder above `dist/` under the same path, for example with [`miniserve`](https://crates.io/crates/miniserve):

```sh
mkdir -p preview && cp -r dist preview/Active
cargo install miniserve
miniserve preview --index index.html
```

then open `http://localhost:8080/Active/`.

## What changes on a static host

| On the server | In the static export |
| --- | --- |
| `/` redirects to the visitor's language (`Accept-Language`) | `/` sends to the English home page and links the three languages |
| The opening curtain plays once per visit (a cookie) | No curtain |
| Unknown pages answer 404 in the visitor's language | GitHub serves `404.html`, in English |
| Brotli and gzip compression, cache headers | GitHub's own compression and cache |

The docs search works the same way in both: the browser downloads the index of its language once (`/search/en.txt`, a few kilobytes) and searches locally with the same Rust code as the server.

## A custom domain

In **Settings**, **Pages**, enter your domain under **Custom domain** and follow GitHub's DNS instructions. The site is then served at the root of the domain: `actions/configure-pages` gives an empty base path and your domain as the address, with no change to the workflow.

## Your own site

For an `active` project of your own, generate the pages with a small `main` (see [Static sites](../static-site/)) and reuse its workflow, or copy `pages.yml` and replace the build and export steps with your own commands.
