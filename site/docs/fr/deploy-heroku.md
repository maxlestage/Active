---
title: Déployer sur Heroku
description: Mettez un site Active en ligne sur Heroku avec un simple téléphone : fork sur GitHub, déploiement en un bouton, puis redéploiement à chaque push sur master.
---

Ce guide met en ligne un site construit avec `active` sur [Heroku](https://www.heroku.com), avec seulement un téléphone et un navigateur web. Il prend ce dépôt (le site web d'`active`) comme exemple ; les dernières sections montrent comment déployer votre propre application de la même façon.

Heroku construit le dépôt avec le buildpack Rust [`emk/rust`](https://elements.heroku.com/buildpacks/emk/heroku-buildpack-rust) (serveur Rust et bundle WebAssembly), exécute le serveur et vous donne une adresse se terminant par `herokuapp.com`. Une fois l'application connectée à GitHub, chaque push sur `master` est mis en ligne automatiquement. Le `Dockerfile` du dépôt reste une autre façon de le construire : voir [Avec Docker](#avec-docker).

## Avant de commencer

Il vous faut deux comptes :

- **GitHub.** Un fork du dépôt vous permet de modifier le site et de déployer vos changements. L'application GitHub comme github.com dans un navigateur mobile conviennent.
- **Heroku**, sur [signup.heroku.com](https://signup.heroku.com), avec un **moyen de paiement** ajouté dans Account settings, Billing. Heroku n'a pas d'offre gratuite : voir [Coûts](#couts) plus bas.

> **Astuce :** Le tableau de bord de Heroku fonctionne dans un navigateur mobile. Si une page semble à l'étroit, passez le navigateur en mode « Version pour ordinateur » (« Desktop site »).

## 1. Forker le dépôt

1. Ouvrez [maxlestage.github.io/Active](https://maxlestage.github.io/Active/).
2. Touchez **Fork**, gardez le nom par défaut `Active`, et confirmez avec **Create fork**.

Vous disposez maintenant de `https://github.com/<you>/Active`, que vous pouvez modifier depuis l'application ou le site GitHub. Pour déployer le dépôt d'origine sans modification, vous pouvez sauter cette étape.

## 2. Déployer avec le bouton

Le bouton lit `app.json` et crée une application Heroku sur la stack `heroku-26`, qui construit le dépôt avec le buildpack `emk/rust`.

1. Ouvrez le lien de déploiement de votre dépôt : `https://heroku.com/deploy?template=https://github.com/maxlestage/Active`.
2. Connectez-vous à Heroku si on vous le demande.
3. **App name :** choisissez un nom en minuscules comme `my-active-site`. Votre adresse devient `my-active-site-<random>.herokuapp.com` ou une adresse similaire, affichée à la fin.
4. **Region :** United States ou Europe, selon ce qui est le plus proche de vos visiteurs.
5. **SITE_URL :** laissez-la vide pour l'instant (voir [Configuration](#configuration)).
6. Touchez **Deploy app**.

Heroku exécute alors le buildpack : il installe Rust, puis `heroku-build.sh` compile le serveur en mode release, ajoute la cible WebAssembly, compile le bundle du navigateur et lance `wasm-bindgen`. Le premier build prend plusieurs minutes, car tout est compilé à partir de zéro. Quand « Your app was successfully deployed » apparaît, touchez **View**.

## 3. Redéployer automatiquement depuis GitHub

Le bouton ne déploie qu'une fois. Pour publier chaque changement que vous poussez :

1. Ouvrez [dashboard.heroku.com](https://dashboard.heroku.com) et touchez l'application que vous venez de créer.
2. Ouvrez l'onglet **Deploy**.
3. Sous **Deployment method**, touchez **GitHub**, puis **Connect to GitHub**, et autorisez Heroku.
4. Recherchez `Active` (votre fork), puis touchez **Connect**.
5. Sous **Automatic deploys**, choisissez la branche **`master`** et touchez **Enable Automatic Deploys**.

Désormais, chaque commit qui arrive sur `master` (un fichier modifié sur GitHub, une pull request fusionnée) lance un nouveau build. Suivez les builds dans l'onglet **Activity**. La section **Manual deploy** déploie une branche à la demande.

> **Astuce :** Le bouton n'est pas obligatoire. Une application créée avec **New → Create new app** dans le tableau de bord fonctionne aussi : avant son premier déploiement, ouvrez son onglet **Settings**, sous **Buildpacks** touchez **Add buildpack**, saisissez `emk/rust` et enregistrez. L'application doit être sur une stack classique comme `heroku-26`, et non sur la stack container, qui ignore les buildpacks.

### Avec la CLI Heroku

Sur un ordinateur, depuis un clone du dépôt :

```sh
heroku create --stack heroku-26 --buildpack emk/rust
git push heroku master
```

Une application existante, par exemple sur la stack container, passe au buildpack avec ces commandes ; le déploiement suivant l'utilise :

```sh
heroku stack:set heroku-26 --app <name>
heroku buildpacks:set emk/rust --app <name>
```

### Modifier le site depuis un téléphone

Avec les déploiements automatiques activés, toute la boucle fonctionne depuis l'application ou le site GitHub : ouvrez un fichier de votre fork (par exemple une page de `site/docs/en/`), touchez l'icône en forme de crayon, modifiez, puis faites un commit sur `master`. Heroku reconstruit l'application et le changement est en ligne quelques minutes plus tard.

## Comment le dépôt est déployé

Ces fichiers indiquent à Heroku quoi faire. Vous n'avez pas besoin de les modifier pour déployer ce site.

| Fichier | Rôle |
| --- | --- |
| `app.json` | Lu par le bouton Deploy : description de l'application, `"stack": "heroku-26"`, le buildpack `emk/rust`, la variable facultative `SITE_URL`, un dyno `web` |
| `RustConfig` | Lu par le buildpack : la version de Rust (`stable`), `RUST_SKIP_BUILD=1` pour sauter sa propre compilation ; lance ensuite `heroku-build.sh` |
| `heroku-build.sh` | Compile le serveur dans `target/release` et le bundle WebAssembly dans `site/pkg` |
| `Procfile` | Démarre le processus `web` : `SITE_DIR=site exec target/release/active-site` |

Sans ses commentaires, `RustConfig` tient en trois lignes :

```sh
VERSION=stable
RUST_SKIP_BUILD=1
trap 'status=$?; if [ "$status" -eq 0 ]; then bash "$BUILD_DIR/heroku-build.sh" || exit 1; fi' EXIT
```

Le buildpack lit ce fichier, installe Rust, puis saute sa propre compilation, qui ne lancerait qu'un simple `cargo build --release` ; le `trap` lance `heroku-build.sh` une fois le buildpack terminé. Le script exécute les mêmes commandes que la CI et que la page [Compiler le client](../client-build/), et copie le serveur dans `target/release` quand Cargo compile dans le cache du buildpack (`CARGO_TARGET_DIR`) :

```sh
cargo build --release -p active-site --bin active-site
rustup target add wasm32-unknown-unknown
cargo build -p active-site --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/active_site.wasm
```

Il utilise la version de `wasm-bindgen` inscrite dans `Cargo.lock` : il télécharge le binaire précompilé de cette version, ou le compile avec `cargo install` si le téléchargement échoue. Le serveur s'exécute ensuite depuis le répertoire du dépôt et sert `site/static`, `site/docs` et `site/pkg` à partir de `SITE_DIR=site`.

### Avec Docker

`heroku.yml`, le `Dockerfile` et `.dockerignore` restent en place : une application sur la stack container ignore les buildpacks et construit le `Dockerfile` à la place, et la même image tourne sur n'importe quel hébergeur de conteneurs. Pour les utiliser sur Heroku, passez l'application sur cette stack avec la CLI Heroku (`heroku stack:set container --app <name>`) et redéployez ; `heroku stack:set heroku-26 --app <name>` la ramène au buildpack.

| Fichier | Rôle |
| --- | --- |
| `heroku.yml` | Indique à la stack container de construire le processus `web` à partir du `Dockerfile` |
| `Dockerfile` | Deux étapes : une image Rust compile le bundle et le serveur, une petite image Debian les exécute |
| `.dockerignore` | Exclut du build `target/`, `pkg/`, `bench/` et les données Git |

`heroku.yml` tient en trois lignes :

```yaml
build:
  docker:
    web: Dockerfile
```

Le `Dockerfile` exécute les mêmes commandes que la CI et que la page [Compiler le client](../client-build/) :

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

L'image finale ne contient que le binaire du serveur et les fichiers qu'il sert : elle démarre donc en une fraction de seconde.

## Configuration

Heroku transmet les réglages au serveur sous forme de variables d'environnement, appelées **config vars**. Définissez-les dans l'onglet **Settings** de l'application, sous **Config Vars**, **Reveal Config Vars**. En modifier une redémarre l'application.

| Variable | Définie par | Signification |
| --- | --- | --- |
| `PORT` | Heroku | Le port sur lequel le serveur doit écouter. Le serveur le lit et se rabat sur `3000` en local. Ne la définissez jamais vous-même. |
| `SITE_URL` | vous, facultative | Adresse publique utilisée dans les URL canoniques, les URL Open Graph, `sitemap.xml` et `robots.txt`, par exemple `https://www.example.com`. Si elle est vide, le serveur utilise l'adresse de chaque requête. |

Définissez `SITE_URL` dès que vous utilisez un domaine personnalisé, afin que les moteurs de recherche voient toujours la même adresse.

## Journaux

- **Journaux du serveur :** dans l'application, touchez **More** (en haut à droite), puis **View logs**. Vous y voyez chaque requête et tout ce qu'affiche le serveur.
- **Journaux de build :** dans l'onglet **Activity**, touchez **View build log** à côté d'un build.

Avec la CLI Heroku sur un ordinateur, `heroku logs --tail --app <name>` diffuse les mêmes journaux en continu.

## Domaine personnalisé

1. Dans **Settings**, sous **Domains**, touchez **Add domain** et saisissez votre domaine, par exemple `www.example.com`. Heroku affiche une **DNS target**.
2. Chez votre registraire de domaine, ajoutez un enregistrement **CNAME** : nom `www`, valeur la DNS target. Pour un domaine racine (`example.com`), utilisez un enregistrement ALIAS, ANAME ou CNAME aplati si votre fournisseur le propose.
3. Donnez à la config var `SITE_URL` la valeur `https://www.example.com`.

Les changements DNS prennent de quelques minutes à quelques heures. Pour le HTTPS sur votre domaine, utilisez les options SSL de la même section **Domains** ; les certificats automatiques dépendent du type de dyno, vérifiez donc ce que le vôtre inclut.

## Coûts

Heroku n'a pas d'offre gratuite. Au moment de la rédaction :

- Les dynos **Eco** coûtent environ 5 $ par mois pour un forfait de 1 000 heures de dyno partagé entre toutes vos applications Eco. Une application Eco se met en veille après 30 minutes sans trafic, et la visite suivante attend quelques secondes pendant son réveil.
- Les dynos **Basic** coûtent environ 7 $ par mois et par dyno, et ne se mettent jamais en veille.

Changez le type de dyno dans l'onglet **Resources**. Consultez [heroku.com/pricing](https://www.heroku.com/pricing) pour les prix actuels. Un serveur construit avec `active` consomme très peu de mémoire : le plus petit dyno suffit pour un site de documentation ou un blog.

> **Astuce :** Si votre site n'a aucune logique côté serveur, un [site statique](../static-site/) peut plutôt être hébergé gratuitement sur GitHub Pages ou un service similaire.

## Déployer votre propre application

Pour une crate unique dont le binaire est l'application, le simple `cargo build --release` du buildpack `emk/rust` suffit : ajoutez un `Procfile` avec `web: ./target/release/my-site`, et si vous le souhaitez un `RustConfig` avec `VERSION=1.90` pour fixer la version de Rust.

S'il faut davantage, comme le bundle WebAssembly d'une application organisée comme dans le [Démarrage rapide](../quick-start/) (paquet `my-site`, `pkg/` servi par le serveur), un paquet d'un workspace ou un exemple, faites comme ce dépôt, avec quatre fichiers à la racine de votre dépôt.

`RustConfig` fixe la version de Rust, saute la compilation du buildpack et lance le script de build une fois Rust installé :

```sh
VERSION=stable
RUST_SKIP_BUILD=1
trap 'status=$?; if [ "$status" -eq 0 ]; then bash "$BUILD_DIR/heroku-build.sh" || exit 1; fi' EXIT
```

`heroku-build.sh` compile le serveur, le copie dans `target/release` quand Cargo compile dans le cache du buildpack (`CARGO_TARGET_DIR`), puis compile le bundle WebAssembly dans `pkg/` :

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

`Procfile` démarre le serveur :

```
web: exec target/release/my-site
```

`app.json`, pour le bouton Deploy (adaptez le nom et la description) :

```json
{
  "name": "my-site",
  "description": "My site built with active.",
  "stack": "heroku-26",
  "buildpacks": [{ "url": "emk/rust" }],
  "formation": { "web": { "quantity": 1 } }
}
```

Le script prend la version de `wasm-bindgen-cli` dans `Cargo.lock` : les deux correspondent donc toujours ; la compilation de la CLI ajoute quelques minutes au build. `RustConfig` lance le script avec `bash` : il n'a donc pas besoin du bit exécutable, qu'un fichier créé sur github.com n'a pas.

Faites écouter le serveur sur `PORT`, sur toutes les interfaces :

```rust
use vitesse::prelude::*;

fn main() -> std::io::Result<()> {
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(3000);
    let app = App::new(); // your routes
    app.run(port)
}
```

Déployez ensuite avec `https://heroku.com/deploy?template=https://github.com/<you>/<repo>` et connectez GitHub comme ci-dessus, ou créez l'application dans le tableau de bord ou avec la CLI Heroku et ajoutez le buildpack `emk/rust`.

### Votre application avec Docker

Pour construire plutôt avec Docker, copiez `heroku.yml`, mettez `"stack": "container"` dans `app.json` en retirant ses `buildpacks`, et utilisez ce `Dockerfile` à la place de `RustConfig`, `heroku-build.sh` et `Procfile` :

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

Ajoutez un `.dockerignore` contenant `target` et `pkg`. Gardez la version de `wasm-bindgen-cli` du `Dockerfile` égale à la version de `wasm-bindgen` dans `Cargo.lock`.

## Dépannage

### Le build échoue

Ouvrez **Activity**, puis le journal du build en échec, et lisez les dernières lignes.

- **« No default language could be detected »** : l'application n'a pas de buildpack. Dans **Settings**, sous **Buildpacks**, ajoutez `emk/rust`, puis redéployez, ou créez une nouvelle application avec le bouton Deploy.
- **Le journal montre un build Docker au lieu du buildpack :** l'application est sur la stack container, qui ignore les buildpacks. Changez-la avec la CLI Heroku (voir [Avec la CLI Heroku](#avec-la-cli-heroku)) ou créez une nouvelle application avec le bouton Deploy.
- **Une erreur de schéma ou de version de `wasm-bindgen` :** la version de la CLI diffère de celle du crate `wasm-bindgen` dans `Cargo.lock`. `heroku-build.sh` la lit dans `Cargo.lock` ; avec Docker, rendez égale la version du `Dockerfile`.
- **Une erreur de compilation Rust :** la même erreur apparaît en local avec les commandes de [Compiler le client](../client-build/). Corrigez-la, faites un commit, et le build suivant la prendra en compte.

### Erreur « Application error »

La page affiche « Application error ». Ouvrez les journaux :

- `code=H10` (application plantée) : le serveur s'est arrêté au démarrage. Les lignes juste au-dessus en donnent la raison.
- `R10` (délai de démarrage dépassé) : le serveur n'a pas écouté sur `PORT` dans les 60 secondes. Liez-le à `0.0.0.0` et à la variable `PORT`.
- `code=H14` (aucun processus web en cours d'exécution) : ouvrez **Resources** et activez le dyno `web`.
