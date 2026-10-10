---
title: Publier une version
description: Publiez le paquet npm active-react et le crate Rust active depuis un téléphone, en publiant une GitHub Release. Un workflow s'occupe du reste.
---

Publier une nouvelle version ne demande qu'une action : **publier une GitHub Release avec un tag comme `v0.2.0`**. Un workflow GitHub Actions teste alors le code, puis publie le paquet npm et le crate Rust. Tout se passe sur les sites de GitHub, npm et crates.io : un téléphone suffit.

Cette page s'adresse aux mainteneurs du dépôt (ou de votre fork).

## Fonctionnement du workflow de release

Le workflow se trouve dans `.github/workflows/release.yml` et s'exécute lorsqu'une release est publiée :

1. **Lecture de la version depuis le tag.** `v0.2.0` donne `0.2.0`. Un tag qui ne ressemble pas à `v1.2.3` arrête le workflow avec une erreur.
2. **Paquet npm** (uniquement si le secret `NPM_TOKEN` existe) : installation, exécution des tests, écriture de la version dans `package.json`, puis `npm publish --access public`.
3. **Crate Rust** (uniquement si le secret `CARGO_REGISTRY_TOKEN` existe) : écriture de la version dans `crates/active/Cargo.toml`, exécution de `cargo test`, puis `cargo publish`.

La version provient du tag : **vous ne modifiez jamais les numéros de version à la main**. Les fichiers du dépôt conservent leur ancien numéro ; seuls les paquets publiés portent le nouveau.

Les jobs npm et crate sont indépendants : si un registre n'a pas de secret, son job est ignoré avec un avertissement, et l'autre publie quand même. Vous pouvez ainsi commencer avec npm seulement, par exemple.

## 1. Créer un token npm

1. Ouvrez [npmjs.com](https://www.npmjs.com) et connectez-vous (créez un compte si nécessaire).
2. Touchez votre **avatar** (en haut à droite), puis **Access Tokens**.
3. Touchez **Generate New Token**, puis **Granular Access Token**.
4. Donnez-lui un nom, comme `github-release`, et une date d'expiration.
5. Sous **Packages and scopes**, réglez la permission sur **Read and write**. Choisissez **All packages** pour la première release, puisque `active-react` n'existe pas encore sur npm. Après la première release, vous pourrez créer un nouveau token limité à `active-react`.
6. Si votre compte exige l'authentification à deux facteurs pour publier, activez l'option du token qui lui permet de contourner la 2FA ; sinon, le workflow ne peut pas répondre à la demande de 2FA.
7. Touchez **Generate Token** et **copiez-le immédiatement** : npm ne l'affiche qu'une seule fois.

> **Attention :** npm limite la durée de validité des tokens d'écriture. Notez la date d'expiration et remplacez le secret avant qu'il n'expire, sinon la prochaine release échouera à l'étape de publication.

## 2. Créer un token crates.io

1. Ouvrez [crates.io](https://crates.io) et touchez **Log in with GitHub**.
2. Ouvrez le menu de votre avatar, puis **Account Settings**. Ajoutez et **vérifiez votre adresse e-mail** : crates.io refuse de publier sans adresse vérifiée.
3. Ouvrez **API Tokens** et touchez **New Token**.
4. Nommez-le, par exemple `github-release`, et choisissez une date d'expiration.
5. Cochez les scopes **publish-new** (pour créer le crate la première fois) et **publish-update** (pour les versions suivantes).
6. Touchez **Generate Token** et copiez-le.

## 3. Ajouter les secrets à GitHub

L'application mobile GitHub n'affiche pas les réglages des dépôts : utilisez le site web dans votre navigateur.

1. Ouvrez le dépôt sur [github.com](https://github.com), puis **Settings**.
2. Dans le menu, ouvrez **Secrets and variables**, puis **Actions**.
3. Touchez **New repository secret**. Name : `NPM_TOKEN`. Secret : collez le token npm. Touchez **Add secret**.
4. Touchez de nouveau **New repository secret**. Name : `CARGO_REGISTRY_TOKEN`. Secret : collez le token crates.io. Touchez **Add secret**.

Les noms des secrets doivent correspondre exactement. Une fois enregistré, un secret ne peut plus être relu par personne, pas même par vous : pour le changer, mettez-le à jour avec une nouvelle valeur.

## 4. Publier une release

1. Ouvrez la page **Releases** du dépôt. Sur un téléphone, le plus rapide est d'aller directement sur `https://github.com/maxlestage/Active/releases/new` (remplacez le propriétaire s'il s'agit d'un fork).
2. Si vous êtes sur la liste des releases, touchez **Draft a new release**.
3. Touchez **Choose a tag**, tapez `v0.1.0`, puis sélectionnez **Create new tag: v0.1.0 on publish**.
4. Gardez `master` comme branche cible.
5. Indiquez le titre de la release, par exemple `v0.1.0`. Touchez **Generate release notes** si vous voulez que GitHub liste les modifications fusionnées.
6. Touchez **Publish release**.

## 5. Suivre l'exécution

Ouvrez l'onglet **Actions** du dépôt : une exécution **Release** apparaît en quelques secondes. Touchez-la pour voir les trois jobs (`version`, `npm`, `crate`) et leurs logs. Une coche verte signifie que le job a réussi. Un job dont le secret manque se termine lui aussi en vert, avec un avertissement comme « NPM_TOKEN secret missing — npm publish skipped » dans le résumé de l'exécution.

Vérifiez ensuite le résultat :

- [npmjs.com/package/active-react](https://www.npmjs.com/package/active-react) affiche la nouvelle version.
- [crates.io/crates/active](https://crates.io/crates/active) affiche la nouvelle version, et [docs.rs/active](https://docs.rs/active) génère automatiquement la documentation de l'API quelques minutes plus tard.

### En cas d'échec

- **Les tests échouent :** ce job ne publie rien. Corrigez le code, fusionnez, puis publiez une nouvelle release avec la version de correctif suivante.
- **npm indique que la version existe déjà :** les registres n'acceptent jamais deux fois la même version, même après une dépublication. Publiez plutôt la version suivante.
- **Erreurs 401 ou 403 :** le token est absent, expiré ou n'a pas la permission d'écriture. Créez-en un nouveau et mettez à jour le secret.
- **Un seul registre a reçu la nouvelle version :** corrigez la cause et publiez la version de correctif suivante, pour que les deux registres soient de nouveau synchronisés.

## Gestion des versions

Suivez le [versionnage sémantique](https://semver.org) : `MAJOR.MINOR.PATCH`.

- **PATCH** (`0.1.0` vers `0.1.1`) : corrections de bugs uniquement.
- **MINOR** (`0.1.1` vers `0.2.0`) : nouvelles fonctionnalités. Tant que la version commence par `0.`, utilisez aussi une montée de version mineure pour les changements incompatibles : npm et Cargo considèrent `0.1.x` et `0.2.x` comme incompatibles.
- **MAJOR** (`1.0.0` vers `2.0.0`) : changements incompatibles, une fois la version `1.0.0` atteinte.

Utilisez de simples tags `vMAJOR.MINOR.PATCH`. Les tags de préversion comme `v0.2.0-beta.1` passent la vérification du workflow, mais celui-ci ne leur réserve aucun traitement particulier.

Gardez le paquet npm et le crate sur le même numéro de version : une seule release publie les deux, ce qui permet de les faire correspondre facilement.

## Après la première release

Une fois les deux paquets publiés, les utilisateurs les installent avec :

```sh
npm i active-react
```

```sh
cargo add active
```

Pensez à mettre à jour les instructions d'[installation](../installation/) pour qu'elles ne renvoient plus vers GitHub.
