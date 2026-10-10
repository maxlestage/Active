---
title: Publier le crate
description: Publiez une nouvelle version d'active sur crates.io depuis un téléphone (GitHub Release) ou avec cargo publish, et voyez comment la CI vérifie chaque push.
---

Cette page s'adresse aux mainteneurs du dépôt ou d'un fork. Une publication du crate `active` tient en une seule action : **créer une publication GitHub (GitHub Release) dont le tag est la version**, par exemple `v0.2.0`. Un workflow GitHub Actions teste le crate et le publie sur [crates.io](https://crates.io). Tout se passe sur les sites de GitHub et de crates.io : un téléphone suffit.

## Fonctionnement du workflow de publication

Le workflow est `.github/workflows/release.yml`. Il s'exécute à la mise en ligne d'une publication GitHub, et :

1. **Lit la version dans le tag.** `v0.2.0` donne `0.2.0`. Un tag qui ne ressemble pas à `v1.2.3` (éventuellement avec un suffixe de préversion comme `-beta.1`) arrête le workflow avec une erreur.
2. **Vérifie le jeton.** Sans le secret `CARGO_REGISTRY_TOKEN`, le job se termine avec l'avertissement « CARGO_REGISTRY_TOKEN secret missing — cargo publish skipped » et ne publie rien.
3. **Écrit la version** du tag dans `crates/active/Cargo.toml`.
4. **Lance les tests** avec `cargo test -p active`. Un test en échec arrête la publication.
5. **Publie** avec `cargo publish -p active --allow-dirty`.

Comme la version vient du tag, le crate publié correspond toujours à la publication, même si vous oubliez de modifier `Cargo.toml`. L'incrémenter aussi dans le dépôt (étape 3 ci-dessous) permet au code source et à crates.io de rester cohérents.

## 1. Créer un jeton crates.io

1. Ouvrez [crates.io](https://crates.io) et touchez **Log in with GitHub**.
2. Ouvrez le menu de votre avatar, puis **Account Settings**. Ajoutez et **vérifiez votre adresse e-mail** : crates.io refuse de publier sans elle.
3. Ouvrez **API Tokens** et touchez **New Token**.
4. Donnez-lui un nom, par exemple `github-release`, et choisissez une date d'expiration.
5. Sélectionnez les portées **publish-new** (nécessaire pour la toute première publication, qui crée le crate) et **publish-update** (pour les versions suivantes).
6. Touchez **Generate Token** et copiez le jeton immédiatement : crates.io ne l'affiche qu'une seule fois.

## 2. Ajouter le secret sur GitHub

L'application mobile GitHub n'affiche pas les paramètres des dépôts : utilisez donc le site web dans le navigateur de votre téléphone.

1. Ouvrez le dépôt sur [github.com](https://github.com), puis **Settings**.
2. Ouvrez **Secrets and variables**, puis **Actions**.
3. Touchez **New repository secret**. Name : `CARGO_REGISTRY_TOKEN`. Secret : collez le jeton. Touchez **Add secret**.

Le nom doit correspondre exactement. Personne ne peut relire un secret une fois enregistré ; pour le remplacer, mettez-le à jour avec une nouvelle valeur.

## 3. Incrémenter la version

1. Dans le dépôt, ouvrez `crates/active/Cargo.toml` et touchez l'icône en forme de crayon (**Edit**).
2. Modifiez la ligne `version = "…"` de la section `[package]`, par exemple de `0.1.0` à `0.2.0`.
3. Touchez **Commit changes** et faites le commit sur `master`.

Suivez le [versionnage sémantique](https://semver.org) : avant `1.0.0`, incrémentez le numéro mineur (de `0.1.x` à `0.2.0`) pour les changements incompatibles ou les nouvelles fonctionnalités, et le numéro de correctif (de `0.2.0` à `0.2.1`) pour les simples corrections. Cargo considère `0.1.x` et `0.2.x` comme incompatibles.

## 4. Créer une publication

1. Ouvrez `https://github.com/<owner>/Active/releases/new` (ou **Releases**, puis **Draft a new release**).
2. Touchez **Choose a tag**, tapez `v0.2.0` (la version de l'étape 3 précédée d'un `v`) et sélectionnez **Create new tag on publish**.
3. Gardez `master` comme cible.
4. Définissez le titre, par exemple `v0.2.0`. **Generate release notes** liste pour vous les changements fusionnés.
5. Touchez **Publish release**.

## 5. Vérifier le résultat

Ouvrez l'onglet **Actions** : une exécution **Release** apparaît en quelques secondes. Une coche verte signifie que le crate est publié (ou que le jeton manquait : ouvrez l'exécution pour voir l'avertissement). Ensuite :

- [crates.io/crates/active](https://crates.io/crates/active) affiche la nouvelle version ;
- [docs.rs/active](https://docs.rs/active) génère la documentation de l'API quelques minutes plus tard, avec la fonctionnalité `client` activée.

### En cas d'échec

- **Les tests échouent :** rien n'est publié. Corrigez le code, puis créez une nouvelle publication avec la version de correctif suivante.
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

Seuls les fichiers listés dans le champ `include` du crate (`src`, `README.md`, `LICENSE`) sont envoyés. `cargo package -p active --list` les affiche. Taguez ensuite le commit (`git tag v0.2.0 && git push --tags`) pour que le dépôt garde la trace de ce qui a été publié.

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

Le workflow comporte aussi un job `merge` : quand une branche dont le nom commence par `claude/` est poussée et que toutes les vérifications ci-dessus réussissent, il fusionne cette branche dans `master` et pousse le résultat. Les autres branches et les pull requests sont seulement vérifiées, jamais fusionnées automatiquement. Avec les déploiements automatiques de Heroku activés, une fusion dans `master` met alors le changement en ligne.
