---
title: Compiler le client
description: Compilez vos composants en WebAssembly avec cargo et wasm-bindgen, servez le bundle, chargez-le depuis la page et automatisez la compilation en CI et Docker.
---

La partie navigateur d'un site `active` est votre crate de bibliothèque compilé en WebAssembly. Deux commandes le produisent : `cargo` compile le crate pour la cible `wasm32-unknown-unknown`, puis `wasm-bindgen` génère le fichier `.wasm` final et un petit chargeur JavaScript. Cette page couvre ces deux commandes, la mise à disposition du résultat et l'automatisation de la compilation.

## Ce qui est produit

```text
src/lib.rs ──cargo build --target wasm32-unknown-unknown──▶ target/…/my_site.wasm
                                                                 │
                       wasm-bindgen --target web --no-typescript ▼
                                       pkg/my_site.js         le chargeur (un module JavaScript)
                                       pkg/my_site_bg.wasm    votre code, servi aux navigateurs
```

Le chargeur est généré : vous ne le modifiez jamais. La page le démarre avec l'unique ligne écrite par [`Document::client`](../documents/#charger-le-bundle-du-navigateur).

## Prérequis

- La cible : `rustup target add wasm32-unknown-unknown`.
- La CLI, **exactement** à la version du crate `wasm-bindgen` présente dans votre `Cargo.lock` : `cargo install wasm-bindgen-cli --version 0.2.129 --locked`.
- Un crate configuré pour deux cibles, comme dans [Installation](../installation/#un-crate-deux-cibles) : `crate-type = ["cdylib", "rlib"]`, `active` avec la fonctionnalité `client` et `wasm-bindgen` sous `[target.'cfg(target_arch = "wasm32")'.dependencies]`, ainsi qu'une fonction `#[wasm_bindgen(start)]`.

## Commandes de compilation

```sh
cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_site.wasm
```

- `--lib` ne compile que la bibliothèque. Votre serveur (`src/main.rs`) n'est pas compilé pour le navigateur.
- `--profile wasm-release` utilise le profil optimisé pour la taille défini dans [Installation](../installation/#ajouter-les-profils-de-compilation). Sans lui, utilisez `--release` et le chemin `…/release/my_site.wasm`.
- Le fichier `.wasm` porte le nom du paquet, où `-` est remplacé par `_` : le paquet `my-site` donne `my_site.wasm`.
- `--target web` produit un chargeur sous forme de module ES standard, ce qu'attend `Document::client`.
- `--no-typescript` évite de générer les déclarations TypeScript (`.d.ts`), inutiles dans un projet Rust.

Dans un [workspace Cargo](https://doc.rust-lang.org/cargo/reference/workspaces.html), sélectionnez le paquet avec `-p` et utilisez le répertoire `target/` du workspace, comme le fait ce dépôt pour son exemple :

```sh
cargo build -p hello --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir examples/hello/pkg target/wasm32-unknown-unknown/wasm-release/hello.wasm
```

### Compilations plus rapides pendant le développement

Une compilation de débogage est plus rapide et donne de meilleurs messages de panique, au prix d'un fichier bien plus volumineux :

```sh
cargo build --lib --target wasm32-unknown-unknown
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/debug/my_site.wasm
```

Un petit script évite de tout retaper. Enregistrez-le sous le nom `build.sh` à la racine du projet, rendez-le exécutable avec `chmod +x build.sh`, et lancez `./build.sh && cargo run` après chaque modification :

```sh
#!/bin/sh
set -e
PROFILE=${PROFILE:-wasm-release}
DIR=$PROFILE
[ "$PROFILE" = dev ] && DIR=debug # the dev profile writes to target/…/debug
cargo build --lib --target wasm32-unknown-unknown --profile "$PROFILE"
wasm-bindgen --target web --no-typescript --out-dir pkg \
  "target/wasm32-unknown-unknown/$DIR/my_site.wasm"
```

`PROFILE=dev ./build.sh` produit une compilation de débogage.

Ajoutez `pkg` à votre `.gitignore` : c'est un résultat de compilation.

## Servir le bundle

Servez le dossier `pkg/` à une URL, généralement `/pkg/`, avec ces types de contenu :

| Fichier | Type de contenu |
| --- | --- |
| `.js` | `text/javascript` |
| `.wasm` | `application/wasm` |

Avec le bon type, le navigateur compile le module pendant son téléchargement. Avec un mauvais type, le chargeur affiche un avertissement dans la console et se rabat sur une méthode plus lente. Le `ServeDir` d'[Axum](../axum/) et la plupart des hébergeurs statiques définissent automatiquement ces deux types.

Compressez les deux fichiers : le WebAssembly se compresse bien avec Brotli ou gzip (voir [Performances](../performance/#tailles)).

## Le charger depuis la page

Passez les deux URL au document :

```rust
use active::prelude::*;

fn page(body: Element) -> String {
    Document::new(body)
        .client("/pkg/my_site.js", "/pkg/my_site_bg.wasm")
        .render()
}
```

La page précharge les deux fichiers et démarre le module. Votre fonction `#[wasm_bindgen(start)]` s'exécute alors, typiquement `active::hydrate(&ISLANDS)` pour des [îlots](../islands/) ou `active::mount_to("#app", app())` pour une [application côté client](../client-app/).

## En intégration continue

Ce dépôt compile son site à chaque push avec GitHub Actions ([`.github/workflows/ci.yml`](https://github.com/maxlestage/Active/blob/master/.github/workflows/ci.yml)). Les étapes concernées :

```yaml
- uses: dtolnay/rust-toolchain@stable
  with:
    targets: wasm32-unknown-unknown
- uses: Swatinem/rust-cache@v2
- name: Install wasm-bindgen-cli
  run: cargo install wasm-bindgen-cli --version 0.2.129 --locked
- name: Build the site
  run: |
    cargo build -p active-site --lib --target wasm32-unknown-unknown --profile wasm-release
    wasm-bindgen --target web --no-typescript --out-dir site/pkg target/wasm32-unknown-unknown/wasm-release/active_site.wasm
    cargo build -p active-site --bin active-site --release
```

Il analyse aussi le code du navigateur avec le linter, ce que la compilation normale ne vérifie pas :

```sh
cargo clippy --lib --target wasm32-unknown-unknown -- -D warnings
```

Pour Docker, le [`Dockerfile`](https://github.com/maxlestage/Active/blob/master/Dockerfile) exécute les mêmes commandes dans une étape de build ; voir [Déployer sur Heroku](../deploy-heroku/#comment-le-depot-est-deploye).

## Dépannage

### wasm-bindgen signale une incompatibilité de schéma ou de version

La CLI et le crate `wasm-bindgen` n'ont pas la même version. Trouvez la version du crate avec `cargo tree -i wasm-bindgen --target wasm32-unknown-unknown` et installez la même version de la CLI avec `cargo install wasm-bindgen-cli --version <version> --locked`. Épingler le crate avec `wasm-bindgen = "=0.2.129"` évite les surprises après un `cargo update`.

### Erreurs à propos de tokio, mio ou des sockets lors de la compilation pour wasm32

Une dépendance du serveur est compilée pour le navigateur. Déplacez-la sous `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]`, et vérifiez que vous passez bien `--lib`.

### `hydrate`, `mount_to` ou `location` introuvable

Ces fonctions n'existent qu'avec la fonctionnalité `client`. Vérifiez que `active = { …, features = ["client"] }` se trouve dans les dépendances `wasm32`, et que le code qui les appelle est derrière `#[cfg(target_arch = "wasm32")]`.

### `can't find crate for core` ou `std`

La cible n'est pas installée : `rustup target add wasm32-unknown-unknown`. Si vous avez plusieurs chaînes d'outils, installez-la pour celle qu'utilise le projet.

### La page se charge mais rien ne réagit

- Ouvrez la console de développement du navigateur et cherchez des erreurs ou des avertissements.
- Vérifiez que `/pkg/my_site.js` et `/pkg/my_site_bg.wasm` se chargent (onglet Réseau) et que leurs URL correspondent à celles passées à `Document::client`.
- Vérifiez que la fonction `start` porte `#[wasm_bindgen(start)]` et appelle `hydrate` avec les bons noms d'îlots.
- Après avoir modifié du code du navigateur, recompilez le bundle : `cargo run` seul ne recompile que le serveur.
- Un avertissement `active: re-rendering island (… mismatch)` signifie que le HTML du serveur et la vue du navigateur diffèrent ; voir [Incohérences d'hydratation](../islands/#incoherences-d-hydratation).
