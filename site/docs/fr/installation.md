---
title: Installation
description: Installez Rust, ajoutez le crate active, configurez un seul crate pour le serveur et le navigateur, puis la cible WebAssembly et la CLI wasm-bindgen.
---

Cette page met en place tout ce dont vous avez besoin : la chaîne d'outils Rust, le crate `active`, une organisation de projet qui compile à la fois pour le serveur et pour le navigateur, et les deux outils qui produisent le bundle WebAssembly.

## Prérequis

| Outil | Version | Pourquoi |
| --- | --- | --- |
| Rust | 1.85 ou plus récent | `active` utilise l'édition 2024 |
| Cible `wasm32-unknown-unknown` | n'importe laquelle | compile le bundle du navigateur |
| `wasm-bindgen-cli` | la même que celle du crate `wasm-bindgen` dans `Cargo.lock` | génère le chargeur et le `.wasm` final |

Si vous vous contentez de rendre du HTML sur un serveur ou de générer un site statique sans îlots, Rust seul suffit.

## Installer Rust

Installez [rustup](https://rustup.rs), qui gère les versions de Rust, puis vérifiez la version :

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustc --version
```

Sous Windows, téléchargez plutôt `rustup-init.exe` depuis [rustup.rs](https://rustup.rs). Pour mettre à jour plus tard, lancez `rustup update`.

## Ajouter le crate

```sh
cargo add active
```

### Fonctionnalités

| Fonctionnalité | Par défaut | Ce qu'elle ajoute |
| --- | --- | --- |
| `client` | désactivée | Le runtime du navigateur : `active::hydrate`, `active::mount`, les écouteurs d'événements, les effets. Ajoute `wasm-bindgen` et `web-sys`. |

Sans `client`, `active` n'a aucune dépendance. Les gestionnaires d'événements et les effets sont acceptés (pour que les mêmes composants compilent partout), mais ne s'exécutent jamais.

## Un crate, deux cibles

L'organisation recommandée est un crate unique dont la bibliothèque contient les composants. La compilation native est votre serveur ; la compilation WebAssembly de cette même bibliothèque est le bundle du navigateur. C'est ainsi que sont organisés `examples/hello` et ce site web.

```text
my-site/
├── Cargo.toml
└── src/
    ├── lib.rs    composants, îlots et point d'entrée du navigateur
    └── main.rs   le serveur (natif uniquement)
```

```toml
[package]
name = "my-site"
version = "0.1.0"
edition = "2024"

[lib]
# cdylib : le .wasm pour le navigateur. rlib : la bibliothèque utilisée par src/main.rs.
crate-type = ["cdylib", "rlib"]

[dependencies]
active = "0.1"

# Dépendances propres au serveur : elles ne compilent pas pour wasm32.
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
vitesse = "0.1"

# Propres au navigateur : le runtime client et le générateur de liaisons.
[target.'cfg(target_arch = "wasm32")'.dependencies]
active = { version = "0.1", features = ["client"] }
wasm-bindgen = "=0.2.129"
```

Cargo fusionne les deux lignes `active` : sur `wasm32`, le crate reçoit la fonctionnalité `client`, sur le serveur non. Votre code peut donc appeler `active::hydrate` derrière `#[cfg(target_arch = "wasm32")]` :

```rust
// src/lib.rs
use active::prelude::*;

pub fn counter(props: &str) -> Node {
    let count = use_state(props.parse::<i32>().unwrap_or(0));
    button()
        .on_click(move |_| count.update(|n| *n += 1))
        .text_dyn(move || count.get().to_string())
        .into()
}

/// S'exécute dans le navigateur au démarrage du module WebAssembly.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    active::hydrate(&[("counter", counter)]);
}
```

> **Attention :** Gardez les crates serveur comme `vitesse` dans la table `cfg(not(target_arch = "wasm32"))`. Cargo compile toutes les dépendances du paquet pour la cible que vous construisez, même celles que seul `main.rs` utilise, et les crates réseau ne compilent pas pour `wasm32-unknown-unknown`.

## Installer les outils WebAssembly

Ajoutez la cible de compilation et l'outil en ligne de commande `wasm-bindgen` :

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
```

La version de la CLI doit être exactement celle du crate `wasm-bindgen` utilisé par votre projet, sinon `wasm-bindgen` s'arrête sur une erreur de version de schéma. Épingler le crate avec `=0.2.129` (comme ci-dessus) garde les deux synchronisés. Pour voir quelle version figure dans votre fichier de verrouillage :

```sh
cargo tree -i wasm-bindgen --target wasm32-unknown-unknown
```

## Ajouter les profils de compilation

Les profils se placent dans le `Cargo.toml` racine de votre projet (la racine du workspace si vous en avez un). Voici les réglages utilisés par ce dépôt : une compilation rapide pour le serveur et un bundle léger pour le navigateur.

```toml
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

# Bundle du navigateur : cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
[profile.wasm-release]
inherits = "release"
opt-level = "s"
panic = "abort"
```

La page [Performances](../performance/) explique chaque réglage et montre les tailles obtenues.

## Vérifier l'installation

Rendez une vue dans un test. S'il passe, le crate est correctement installé :

```rust
#[test]
fn renders() {
    use active::prelude::*;
    let html = Node::from(p().class("hello").text("Hello")).render();
    assert_eq!(html, r#"<p class="hello">Hello</p>"#);
}
```

Compilez ensuite une première fois le bundle du navigateur pour vérifier les outils WebAssembly :

```sh
cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_site.wasm
```

Cargo remplace `-` par `_` dans les noms de fichiers : le paquet `my-site` produit donc `my_site.wasm`. Le [Démarrage rapide](../quick-start/) reprend à partir d'ici, et [Compiler le client](../client-build/) détaille la compilation.

## Configuration de l'éditeur

Tout éditeur compatible avec [rust-analyzer](https://rust-analyzer.github.io) convient. Par défaut, rust-analyzer vérifie la compilation native : le code placé derrière `#[cfg(target_arch = "wasm32")]` apparaît donc grisé. Pour travailler sur le code propre au navigateur, faites pointer rust-analyzer vers la cible WebAssembly, par exemple dans le `settings.json` de VS Code :

```text
"rust-analyzer.cargo.target": "wasm32-unknown-unknown"
```

Revenez au réglage par défaut quand vous travaillez sur le serveur.
