---
title: Publier le crate
description: Publiez une nouvelle version d'active sur crates.io en changeant sa version depuis un téléphone, ou avec cargo publish, et voyez comment la CI vérifie chaque push.
---

Cette page s'adresse aux mainteneurs du dépôt ou d'un fork. Une publication du crate `active` tient en une seule action : **modifier la version dans `crates/active/Cargo.toml` et la pousser sur `master`**, par exemple de `0.1.0` à `0.2.0`. Une fois la CI passée, un workflow GitHub Actions crée le tag de la version et la publication GitHub (GitHub Release), teste le crate et le publie sur [crates.io](https://crates.io). Tout se passe sur les sites de GitHub et de crates.io : un téléphone suffit.

## Fonctionnement du workflow de publication

Le workflow est `.github/workflows/release.yml`. Il s'exécute chaque fois que la CI réussit sur un push sur `master`, et :

1. **Lit la version** dans `crates/active/Cargo.toml`. Si son tag (`v0.2.0` pour `0.2.0`) existe déjà, la version a déjà été publiée et le workflow s'arrête là.
2. **Crée le tag et la publication GitHub.** Il crée le tag `v0.2.0` sur le commit testé par la CI, et la publication GitHub du même nom, avec des notes générées par GitHub à partir des commits depuis la publication précédente.
3. **Vérifie le jeton.** Sans le secret `CARGO_REGISTRY_TOKEN`, le job se termine avec l'avertissement « CARGO_REGISTRY_TOKEN secret missing — cargo publish skipped » et ne publie rien sur crates.io.
4. **Lance les tests** avec `cargo test -p active`. Un test en échec arrête la publication.
5. **Publie** avec `cargo publish -p active --allow-dirty`, sauf si cette version est déjà sur crates.io.

Une publication GitHub créée à la main (étape 4 ci-dessous) lance la même publication : la version est alors lue dans le tag de la publication, `v0.2.0` donnant `0.2.0`, et écrite dans `crates/active/Cargo.toml` avant de publier. Un tag qui ne ressemble pas à `v1.2.3` (éventuellement avec un suffixe de préversion comme `-beta.1`) arrête le workflow avec une erreur.

## 1. Créer un jeton crates.io

1. Ouvrez [crates.io](https://crates.io) et touchez **Log in with GitHub**.
2. Ouvrez le menu de votre avatar, puis **Account Settings**. Ajoutez et **vérifiez votre adresse e-mail** : crates.io refuse de publier sans elle.
3. Ouvrez **API Tokens** et touchez **New Token**.
4. Donnez-lui un nom, par exemple `github-release`, et choisissez une date d'expiration.
5. Sélectionnez les portées **publish-new** (nécessaire pour la toute première publication, qui crée le crate) et **publish-update** (pour les versions suivantes).
6. Touchez **Generate Token** et copiez le jeton immédiatement : crates.io ne l'affiche qu'une seule fois.

## 2. Ajouter le secret sur GitHub

L'application mobile GitHub n'affiche pas les paramètres des dépôts : utilisez donc le site web dans le navigateur de votre téléphone.

1. Ouvrez le dépôt sur github.com, puis **Settings**.
2. Ouvrez **Secrets and variables**, puis **Actions**.
3. Touchez **New repository secret**. Name : `CARGO_REGISTRY_TOKEN`. Secret : collez le jeton. Touchez **Add secret**.

Le nom doit correspondre exactement. Personne ne peut relire un secret une fois enregistré ; pour le remplacer, mettez-le à jour avec une nouvelle valeur.

## 3. Incrémenter la version

1. Dans le dépôt, ouvrez `crates/active/Cargo.toml` et touchez l'icône en forme de crayon (**Edit**).
2. Modifiez la ligne `version = "…"` de la section `[package]`, par exemple de `0.1.0` à `0.2.0`.
3. Touchez **Commit changes** et faites le commit sur `master` (ou sur une branche : l'auto-merge la fusionne dans `master` une fois la CI verte, et lance lui-même la publication).

Ce commit est la publication : une fois la CI passée, le workflow crée le tag `v0.2.0` et la publication GitHub, puis publie le crate. Aucun formulaire à remplir.

Suivez le [versionnage sémantique](https://semver.org) : avant `1.0.0`, incrémentez le numéro mineur (de `0.1.x` à `0.2.0`) pour les changements incompatibles ou les nouvelles fonctionnalités, et le numéro de correctif (de `0.2.0` à `0.2.1`) pour les simples corrections. Cargo considère `0.1.x` et `0.2.x` comme incompatibles.

## 4. Créer une publication à la main (facultatif)

Inutile après l'étape 3. Une publication créée à la main publie toujours le crate, par exemple pour une version dont vous voulez créer le tag vous-même :

1. Ouvrez `https://github.com/<owner>/Active/releases/new` (ou **Releases**, puis **Draft a new release**).
2. Touchez **Choose a tag**, tapez `v0.2.0` (la version de l'étape 3 précédée d'un `v`) et sélectionnez **Create new tag on publish**.
3. Gardez `master` comme cible.
4. Définissez le titre, par exemple `v0.2.0`. **Generate release notes** liste pour vous les changements fusionnés.
5. Touchez **Publish release**.

## 5. Vérifier le résultat

Ouvrez l'onglet **Actions** : une exécution **Release** apparaît une fois que l'exécution **CI** du push a réussi. Une coche verte signifie que le crate est publié (ou que le jeton manquait : ouvrez l'exécution pour voir l'avertissement). Ensuite :

- **Releases** affiche le nouveau tag `v0.2.0` et ses notes ;
- [crates.io/crates/active](https://crates.io/crates/active) affiche la nouvelle version ;
- [docs.rs/active](https://docs.rs/active) génère la documentation de l'API quelques minutes plus tard, avec la fonctionnalité `client` activée.

### En cas d'échec

- **La CI échoue :** aucun tag n'est créé et rien n'est publié. Corrigez le code et poussez à nouveau : la version est publiée dès que la CI réussit.
- **Les tests échouent dans l'exécution Release :** le tag et la publication GitHub existent, mais rien n'est sur crates.io. Corrigez le code, puis passez à la version de correctif suivante et poussez.
- **La version existe déjà :** crates.io n'accepte jamais deux fois la même version, même après un yank. Publiez la version suivante.
- **401 ou 403 :** le jeton est absent, expiré ou n'a pas la bonne portée. Créez-en un nouveau et mettez à jour le secret.
- **Le nom appartient à quelqu'un d'autre** (première publication uniquement) : un autre compte possède `active` sur crates.io. Renommez le paquet dans `crates/active/Cargo.toml` (par exemple `active-web`) et mettez à jour la documentation.

## Publier depuis un ordinateur

La même publication, sans GitHub Actions :

```sh
cargo login                         # paste the crates.io token once
cargo test -p active
cargo publish -p active --dry-run   # builds the package exactly as it will be uploaded
cargo publish -p active
```

Seuls les fichiers listés dans le champ `include` du crate (`src`, `README.md`, `LICENSE`) sont envoyés. `cargo package -p active --list` les affiche. Poussez ensuite le changement de version sur `master` : le workflow crée le tag et la publication GitHub, et ignore crates.io puisque cette version y est déjà.

## Intégration continue

`.github/workflows/ci.yml` s'exécute à chaque push et à chaque pull request :

| Étape | Commande |
| --- | --- |
| Formatage | `cargo fmt --all --check` |
| Lint, serveur | `cargo clippy --workspace --all-targets -- -D warnings` |
| Lint, navigateur | `cargo clippy -p active --features client --target wasm32-unknown-unknown -- -D warnings`, puis la même chose pour le site et les exemples (`--lib`) |
| Tests | `cargo test --workspace` |
| Compilation du site | les mêmes commandes que le `Dockerfile` utilisé par [Heroku](../deploy-heroku/) |
| Empaquetage | `cargo package -p active --allow-dirty` |

La dernière étape détecte les problèmes d'empaquetage (un fichier manquant, des métadonnées invalides) avant qu'une publication ne les révèle.

Le workflow comporte aussi un job `merge` : quand une branche est poussée et que toutes les vérifications ci-dessus réussissent, il fusionne cette branche dans `master` et pousse le résultat. Les branches dont le nom commence par `wip/` et les pull requests sont seulement vérifiées, jamais fusionnées automatiquement. Après la fusion, [`pages.yml`](../deploy-pages/) publie `master` sur GitHub Pages, et avec les déploiements automatiques de Heroku activés, le changement est aussi mis en ligne là-bas.
