---
title: Déployer sur Heroku
description: Mettez un site active en ligne sur Heroku avec un simple téléphone : fork sur GitHub, déploiement en un bouton, puis redéploiement à chaque push sur master.
---

Ce guide met en ligne un site construit avec `active` sur [Heroku](https://www.heroku.com), avec seulement un téléphone et un navigateur web. Il prend ce dépôt (le site web d'`active`) comme exemple ; les dernières sections montrent comment déployer votre propre application de la même façon.

Heroku construit le `Dockerfile` du dépôt (serveur Rust et bundle WebAssembly), exécute le serveur et vous donne une adresse se terminant par `herokuapp.com`. Une fois l'application connectée à GitHub, chaque push sur `master` est mis en ligne automatiquement.

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

Le bouton lit `app.json` et crée une application Heroku qui construit le dépôt avec Docker.

1. Ouvrez le lien de déploiement de votre dépôt : `https://heroku.com/deploy?template=<l'adresse de votre dépôt>`.
2. Connectez-vous à Heroku si on vous le demande.
3. **App name :** choisissez un nom en minuscules comme `my-active-site`. Votre adresse devient `my-active-site-<random>.herokuapp.com` ou une adresse similaire, affichée à la fin.
4. **Region :** United States ou Europe, selon ce qui est le plus proche de vos visiteurs.
5. **SITE_URL :** laissez-la vide pour l'instant (voir [Configuration](#configuration)).
6. Touchez **Deploy app**.

Heroku construit alors l'image Docker : il installe la cible WebAssembly et `wasm-bindgen-cli`, puis compile le bundle du navigateur et le serveur en mode release. Le premier build prend plusieurs minutes, car tout est compilé à partir de zéro. Quand « Your app was successfully deployed » apparaît, touchez **View**.

## 3. Redéployer automatiquement depuis GitHub

Le bouton ne déploie qu'une fois. Pour publier chaque changement que vous poussez :

1. Ouvrez [dashboard.heroku.com](https://dashboard.heroku.com) et touchez l'application que vous venez de créer.
2. Ouvrez l'onglet **Deploy**.
3. Sous **Deployment method**, touchez **GitHub**, puis **Connect to GitHub**, et autorisez Heroku.
4. Recherchez `Active` (votre fork), puis touchez **Connect**.
5. Sous **Automatic deploys**, choisissez la branche **`master`** et touchez **Enable Automatic Deploys**.

Désormais, chaque commit qui arrive sur `master` (un fichier modifié sur GitHub, une pull request fusionnée) lance un nouveau build. Suivez les builds dans l'onglet **Activity**. La section **Manual deploy** déploie une branche à la demande.

> **Attention :** Créez l'application avec le bouton, puis connectez-la. Une application créée avec **New → Create new app** dans le tableau de bord utilise la stack de buildpacks par défaut de Heroku, qui ignore `heroku.yml` et ne peut pas construire ce dépôt. Passer une application existante à la stack container nécessite la CLI Heroku (`heroku stack:set container --app <name>`), dont on ne dispose pas sur un téléphone.

### Modifier le site depuis un téléphone

Avec les déploiements automatiques activés, toute la boucle fonctionne depuis l'application ou le site GitHub : ouvrez un fichier de votre fork (par exemple une page de `site/docs/en/`), touchez l'icône en forme de crayon, modifiez, puis faites un commit sur `master`. Heroku reconstruit l'application et le changement est en ligne quelques minutes plus tard.

## Comment le dépôt est déployé

Quatre fichiers indiquent à Heroku quoi faire. Vous n'avez pas besoin de les modifier pour déployer ce site.

| Fichier | Rôle |
| --- | --- |
| `app.json` | Lu par le bouton Deploy : description de l'application, `"stack": "container"`, la variable facultative `SITE_URL`, un dyno `web` |
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

Pour une application organisée comme dans le [Démarrage rapide](../quick-start/) (paquet `my-site`, `pkg/` servi par le serveur), copiez `heroku.yml` et `app.json` dans votre dépôt, adaptez le nom et la description dans `app.json`, et utilisez ce `Dockerfile` :

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

Ajoutez un `.dockerignore` contenant `target` et `pkg`, et faites écouter le serveur sur `PORT`, sur toutes les interfaces :

```rust
#[tokio::main]
async fn main() {
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    let app = axum::Router::new(); // your routes
    axum::serve(listener, app).await.unwrap();
}
```

Gardez la version de `wasm-bindgen-cli` du `Dockerfile` égale à la version de `wasm-bindgen` dans `Cargo.lock`. Déployez ensuite avec `https://heroku.com/deploy?template=https://github.com/<you>/<repo>` et connectez GitHub comme ci-dessus.

## Dépannage

### Le build échoue

Ouvrez **Activity**, puis le journal du build en échec, et lisez les dernières lignes.

- **« No default language could be detected »** ou une erreur de buildpack : l'application n'est pas sur la stack container. Créez une nouvelle application avec le bouton Deploy.
- **Une erreur de schéma ou de version de `wasm-bindgen` :** la version de la CLI dans le `Dockerfile` diffère de celle du crate `wasm-bindgen` dans `Cargo.lock`. Rendez-les égales.
- **Une erreur de compilation Rust :** la même erreur apparaît en local avec les commandes de [Compiler le client](../client-build/). Corrigez-la, faites un commit, et le build suivant la prendra en compte.

### Erreur « Application error »

La page affiche « Application error ». Ouvrez les journaux :

- `code=H10` (application plantée) : le serveur s'est arrêté au démarrage. Les lignes juste au-dessus en donnent la raison.
- `R10` (délai de démarrage dépassé) : le serveur n'a pas écouté sur `PORT` dans les 60 secondes. Liez-le à `0.0.0.0` et à la variable `PORT`.
- `code=H14` (aucun processus web en cours d'exécution) : ouvrez **Resources** et activez le dyno `web`.
