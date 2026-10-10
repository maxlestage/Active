---
title: Déployer sur GitHub Pages
description: Publiez gratuitement le site d'active sur GitHub Pages depuis un téléphone : activez Pages une fois, chaque fusion dans master est en ligne peu après.
---

[GitHub Pages](https://pages.github.com) héberge gratuitement des fichiers statiques, à l'adresse `https://<propriétaire>.github.io/<dépôt>/`. Ce dépôt y publie automatiquement son site : le workflow [`pages.yml`](https://github.com/maxlestage/Active/blob/master/.github/workflows/pages.yml) transforme les pages du serveur Rust en fichiers avec `active-site export`, puis les déploie. Tout ce qui suit se fait depuis un téléphone.

Contrairement à [Heroku](../deploy-heroku/), Pages n'exécute aucun serveur : pas de coût, pas de machine à faire tourner, mais pas non plus de code côté serveur. L'export remplace les quelques choses que le serveur calculait à chaque requête (voir [Ce qui change sur un hébergeur statique](#ce-qui-change-sur-un-hebergeur-statique)).

## Activer Pages (une seule fois)

1. Ouvrez le dépôt sur GitHub (l'application GitHub ou le site dans un navigateur mobile).
2. Allez dans **Settings**, puis **Pages**.
3. Sous **Build and deployment**, réglez **Source** sur **GitHub Actions**.

C'est tout : il n'y a ni branche ni dossier à choisir, le workflow envoie le site lui-même. Pour un fork, faites la même chose dans votre fork ; son adresse devient `https://<vous>.github.io/Active/`.

> **Astuce :** pour publier tout de suite, ouvrez l'onglet **Actions**, choisissez **Pages**, puis **Run workflow**. Sinon, la prochaine fusion dans `master` publie le site.

## Comment se passe la publication

1. Vous poussez une branche, ou Claude le fait.
2. Le workflow de [CI](https://github.com/maxlestage/Active/blob/master/.github/workflows/ci.yml) la vérifie (format, lints, tests, builds, export) et, quand tout est vert, la fusionne dans `master`. Les branches dont le nom commence par `wip/` sont vérifiées mais jamais fusionnées.
3. Quand la CI réussit, le workflow **Pages** démarre : il compile le bundle WebAssembly, exporte le site et le déploie. Quelques minutes plus tard, la nouvelle version est en ligne.

Une CI rouge ne publie rien : le site en ligne vient toujours d'une version qui a passé toutes les vérifications. L'onglet Actions montre chaque exécution et, pour Pages, l'adresse du site publié.

> **Remarque :** Pages attend la CI au lieu d'écouter les pushs sur `master`, car un push fait par un workflow (la fusion automatique) ne démarre pas d'autres workflows.

Tant que Pages n'est pas activé, le workflow Pages s'arrête à son premier job avec un avertissement au lieu d'échouer.

## La commande d'export

Le workflow lance la même commande que vous pouvez lancer sur votre ordinateur :

```sh
cargo build -p active-site --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/active_site.wasm
BASE_PATH=/Active SITE_URL=https://maxlestage.github.io/Active cargo run --release -p active-site -- export dist
```

`dist/` contient alors toutes les pages dans les trois langues (`en/docs/seo/index.html`…), les fichiers statiques, le bundle navigateur, `404.html`, `sitemap.xml`, `robots.txt` et les index de recherche. Chaque fichier vient des handlers du serveur lui-même : le site statique et le serveur envoient le même HTML.

| Variable | Rôle | Sur GitHub Pages |
| --- | --- | --- |
| `BASE_PATH` | Préfixe de chaque lien, car un site de projet vit sous `/<dépôt>/` | `/Active` (fourni par `actions/configure-pages`) |
| `SITE_URL` | Adresse publique, pour les URL canoniques, Open Graph, hreflang et le sitemap | `https://<propriétaire>.github.io/Active` |

Laissez `BASE_PATH` vide pour un site servi à la racine de son domaine (un domaine personnalisé, ou un autre hébergeur statique).

Pour vérifier un export en local, servez le dossier au-dessus de `dist/` sous le même chemin, par exemple avec [`miniserve`](https://crates.io/crates/miniserve) :

```sh
mkdir -p preview && cp -r dist preview/Active
cargo install miniserve
miniserve preview --index index.html
```

puis ouvrez `http://localhost:8080/Active/`.

## Ce qui change sur un hébergeur statique

| Sur le serveur | Dans l'export statique |
| --- | --- |
| `/` redirige vers la langue du visiteur (`Accept-Language`) | `/` envoie vers l'accueil en anglais et propose les trois langues |
| Le rideau d'ouverture se joue une fois par visite (un cookie) | Pas de rideau |
| Les pages inconnues répondent 404 dans la langue du visiteur | GitHub sert `404.html`, en anglais |
| Compression Brotli et gzip, en-têtes de cache | Compression et cache de GitHub |

La recherche dans la documentation fonctionne de la même façon dans les deux cas : le navigateur télécharge une fois l'index de sa langue (`/search/fr.txt`, quelques kilo-octets) et cherche localement avec le même code Rust que le serveur.

## Un domaine personnalisé

Dans **Settings**, **Pages**, saisissez votre domaine sous **Custom domain** et suivez les instructions DNS de GitHub. Le site est alors servi à la racine du domaine : `actions/configure-pages` fournit un chemin de base vide et votre domaine comme adresse, sans rien changer au workflow.

## Votre propre site

Pour un projet `active` à vous, générez les pages avec un petit `main` (voir [Sites statiques](../static-site/)) et reprenez son workflow, ou copiez `pages.yml` en remplaçant les étapes de build et d'export par vos propres commandes.
