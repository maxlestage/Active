---
title: Deploy on GitHub Pages
description: Publish a site built with active for free on GitHub Pages from a phone: export it to static files, switch Pages on once, then every push to master is online a few minutes later.
---

GitHub Pages hosts static files for free, at `https://<owner>.github.io/<repository>/`. A site built with `active` can be published there: its pages are written to files (see [Static sites](../static-site/)), then a GitHub Actions workflow deploys them. Everything below can be done from a phone. The examples use this repository's website, which has an `export` command; the website itself is served by Heroku, at <https://active-c1883a966650.herokuapp.com/>.

Unlike [Heroku](../deploy-heroku/), Pages runs no server: no cost, no machine to keep running, but also no code on the server side. The export replaces the few things the server computed for each request (see [What changes on a static host](#what-changes-on-a-static-host)).

## Switch Pages on (once)

1. Open the repository on GitHub (the GitHub app or the website in a mobile browser).
2. Go to **Settings**, then **Pages**.
3. Under **Build and deployment**, set **Source** to **GitHub Actions**.

That's all: there is no branch or folder to choose, the workflow uploads the site itself.

> **Tip:** add `workflow_dispatch:` to the workflow's `on:` to publish right away from the **Actions** tab (**Pages**, then **Run workflow**). Otherwise the next push to `master` publishes the site.

## How publishing works

1. Add a workflow that builds the site, exports it and deploys it, such as the one in [Static sites](../static-site/#github-pages), as `.github/workflows/pages.yml`.
2. Push to `master` (from the GitHub app too): the workflow builds the WebAssembly bundle, exports the site and deploys it. A few minutes later, the new version is online.

A failed build publishes nothing: the site online always comes from a version that built. The Actions tab shows each run and, for Pages, the address of the published site.

> **Note:** if your CI merges branches into `master` itself, start the Pages workflow when CI succeeds (`workflow_run`) instead of on pushes to `master`, because a push made by a workflow does not start other workflows.

## The export command

The build and the export are commands you can also run on your computer. For this repository's website:

```sh
cargo build -p active-site --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/active_site.wasm
BASE_PATH=/Active SITE_URL=https://<owner>.github.io/Active cargo run --release -p active-site -- export dist
```

`dist/` then holds every page in the three languages (`en/docs/seo/index.html`…), the static files, the browser bundle, `404.html`, `sitemap.xml`, `robots.txt` and the search indexes. Each file comes from the server's own handlers, so the static site and the server send the same HTML.

| Variable | Role | On GitHub Pages |
| --- | --- | --- |
| `BASE_PATH` | Prefix of every link, because a project site lives under `/<repository>/` | `/<repository>`, here `/Active` (given by `actions/configure-pages`) |
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

In **Settings**, **Pages**, enter your domain under **Custom domain** and follow GitHub's DNS instructions. The site is then served at the root of the domain: export it with an empty base path and your domain as the address (a workflow that reads them from `actions/configure-pages` needs no change).

## Your own site

For an `active` project of your own, generate the pages with a small `main` (see [Static sites](../static-site/)) and use its workflow, replacing the build and export steps with your own commands if your site has a server, as this repository's website does.
