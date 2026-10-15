---
title: Déployer sur GitHub Pages
description: Publiez gratuitement un site fait avec active sur GitHub Pages depuis un téléphone : exportez-le en fichiers statiques, activez Pages une fois, chaque push sur master est en ligne peu après.
---

GitHub Pages héberge gratuitement des fichiers statiques, à l'adresse `https://<propriétaire>.github.io/<dépôt>/`. Un site fait avec `active` peut y être publié : ses pages sont écrites dans des fichiers (voir [Sites statiques](../static-site/)), puis un workflow GitHub Actions les déploie. Tout ce qui suit se fait depuis un téléphone. Les exemples utilisent le site de ce dépôt, qui a une commande `export` ; le site lui-même est servi par Heroku, à l'adresse <https://active-c1883a966650.herokuapp.com/>.

Contrairement à [Heroku](../deploy-heroku/), Pages n'exécute aucun serveur : pas de coût, pas de machine à faire tourner, mais pas non plus de code côté serveur. L'export remplace les quelques choses que le serveur calculait à chaque requête (voir [Ce qui change sur un hébergeur statique](#ce-qui-change-sur-un-hebergeur-statique)).

## Activer Pages (une seule fois)

1. Ouvrez le dépôt sur GitHub (l'application GitHub ou le site dans un navigateur mobile).
2. Allez dans **Settings**, puis **Pages**.
3. Sous **Build and deployment**, réglez **Source** sur **GitHub Actions**.

C'est tout : il n'y a ni branche ni dossier à choisir, le workflow envoie le site lui-même.

> **Astuce :** ajoutez `workflow_dispatch:` au `on:` du workflow pour publier tout de suite depuis l'onglet **Actions** (**Pages**, puis **Run workflow**). Sinon, le prochain push sur `master` publie le site.

## Comment se passe la publication

1. Ajoutez un workflow qui compile le site, l'exporte et le déploie, comme celui de [Sites statiques](../static-site/#github-pages), sous le nom `.github/workflows/pages.yml`.
2. Poussez sur `master` (depuis l'application GitHub aussi) : le workflow compile le bundle WebAssembly, exporte le site et le déploie. Quelques minutes plus tard, la nouvelle version est en ligne.

Un build qui échoue ne publie rien : le site en ligne vient toujours d'une version qui a compilé. L'onglet Actions montre chaque exécution et, pour Pages, l'adresse du site publié.

> **Remarque :** si votre CI fusionne elle-même les branches dans `master`, démarrez le workflow Pages quand la CI réussit (`workflow_run`) au lieu d'écouter les pushs sur `master`, car un push fait par un workflow ne démarre pas d'autres workflows.

## La commande d'export

Le build et l'export sont des commandes que vous pouvez aussi lancer sur votre ordinateur. Pour le site de ce dépôt :

```sh
cargo build -p active-site --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/active_site.wasm
BASE_PATH=/Active SITE_URL=https://<propriétaire>.github.io/Active cargo run --release -p active-site -- export dist
```

`dist/` contient alors toutes les pages dans les trois langues (`en/docs/seo/index.html`…), les fichiers statiques, le bundle navigateur, `404.html`, `sitemap.xml`, `robots.txt` et les index de recherche. Chaque fichier vient des handlers du serveur lui-même : le site statique et le serveur envoient le même HTML.

| Variable | Rôle | Sur GitHub Pages |
| --- | --- | --- |
| `BASE_PATH` | Préfixe de chaque lien, car un site de projet vit sous `/<dépôt>/` | `/<dépôt>`, ici `/Active` (fourni par `actions/configure-pages`) |
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

Dans **Settings**, **Pages**, saisissez votre domaine sous **Custom domain** et suivez les instructions DNS de GitHub. Le site est alors servi à la racine du domaine : exportez-le avec un chemin de base vide et votre domaine comme adresse (un workflow qui les lit dans `actions/configure-pages` n'a rien à changer).

## Votre propre site

Pour un projet `active` à vous, générez les pages avec un petit `main` (voir [Sites statiques](../static-site/)) et reprenez son workflow, en remplaçant les étapes de build et d'export par vos propres commandes si votre site a un serveur, comme le site de ce dépôt.
