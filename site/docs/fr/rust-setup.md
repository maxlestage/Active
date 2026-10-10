---
title: Configurer Rust
description: Installez Rust, la cible wasm32 et wasm-pack ou wasm-bindgen-cli, configurez Cargo pour des binaires petits et rapides, et compilez votre premier module.
---

Cette page met en place tout ce qu'il faut pour compiler du Rust en WebAssembly pour active-react. Elle ne concerne que la partie WASM : les fonctionnalités SEO marchent sans Rust.

## Installer Rust

Rust s'installe avec **rustup**, le gestionnaire de toolchain officiel. Sur macOS et Linux :

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Sous Windows, téléchargez et lancez `rustup-init.exe` depuis [rustup.rs](https://rustup.rs). Ouvrez ensuite un nouveau terminal et vérifiez :

```sh
rustc --version
cargo --version
```

Tenez Rust à jour avec `rustup update`.

## Ajouter la cible WebAssembly

Par défaut, Rust compile pour votre machine. Ajoutez une fois pour toutes la cible du navigateur :

```sh
rustup target add wasm32-unknown-unknown
```

## Choisir un outil de build

Rust produit un fichier `.wasm` ; **wasm-bindgen** génère ensuite le code JavaScript de liaison et les types TypeScript qui vous permettent de passer des chaînes, des tableaux et plus encore. Vous pouvez le piloter de deux façons.

### wasm-pack (le plus simple)

```sh
cargo install wasm-pack
```

wasm-pack lance Cargo, télécharge automatiquement la bonne version de wasm-bindgen pour votre crate et optimise le résultat. Une seule commande fait tout.

### wasm-bindgen-cli (plus de contrôle)

```sh
cargo install wasm-bindgen-cli --version 0.2.129
```

La version du CLI **doit correspondre exactement** à la version du crate `wasm-bindgen` présente dans votre `Cargo.lock`. Figez la version du crate avec `=` pour que les deux ne puissent jamais diverger :

```toml
[dependencies]
wasm-bindgen = "=0.2.129"
```

Si elles diffèrent, le build s'arrête avec une erreur comme celle-ci :

```text
it looks like the Rust project used to create this Wasm file was linked against
version of wasm-bindgen that uses a different bindgen format than this binary:

  rust Wasm file schema version: 0.2.129
     this binary schema version: 0.2.100
```

Corrigez-la en installant le CLI correspondant (`cargo install -f wasm-bindgen-cli --version 0.2.129`) ou en changeant la version du crate. C'est la configuration utilisée par l'exemple et les benchmarks du dépôt.

## Cargo.toml

Une bibliothèque WebAssembly a besoin de `crate-type = ["cdylib"]`. Voici le fichier complet, avec le profil release validé par les [benchmarks](../benchmarks/) :

```toml
[package]
name = "hello"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
wasm-bindgen = "0.2"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

Le rôle de chaque réglage du profil :

| Réglage | Effet |
| --- | --- |
| `opt-level = 3` | Optimise pour la vitesse. `"s"` et `"z"` optimisent pour la taille ; d'après les benchmarks, ils ne sont que légèrement plus petits et un peu plus lents |
| `lto = true` | Optimisation à l'édition de liens sur tous les crates : supprime le code inutilisé des dépendances, pour un résultat plus petit et plus rapide |
| `codegen-units = 1` | Compile le crate en une seule unité pour que l'optimiseur voie tout. Builds plus lents, meilleur code |
| `panic = "abort"` | Aucun mécanisme de déroulement de pile dans le binaire. Un panic provoque un trap et lève une `RuntimeError` en JavaScript |
| `strip = true` | Retire du binaire les noms de symboles et les informations de débogage |

Ces réglages ne concernent que les builds `--release` : les builds de debug restent rapides à compiler.

## Écrire des fonctions Rust

Annotez chaque fonction que vous voulez appeler depuis JavaScript avec `#[wasm_bindgen]` :

```rust
use wasm_bindgen::prelude::*;

/// Numbers: i32, u32, f64… become JavaScript numbers.
#[wasm_bindgen]
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

/// Strings: &str in, String out.
#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello {name}, from Rust!")
}

/// Typed arrays: &[f64] receives a Float64Array.
#[wasm_bindgen]
pub fn mean(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    values.iter().sum::<f64>() / values.len() as f64
}

/// Vec<f64> comes back as a new Float64Array.
#[wasm_bindgen]
pub fn normalize(values: &[f64]) -> Vec<f64> {
    let max = values.iter().cloned().fold(f64::MIN, f64::max);
    if max <= 0.0 {
        return values.to_vec();
    }
    values.iter().map(|v| v / max).collect()
}
```

Comment les types courants traversent la frontière :

| Rust | TypeScript |
| --- | --- |
| `i32`, `u32`, `f32`, `f64` | `number` |
| `i64`, `u64` | `bigint` |
| `bool` | `boolean` |
| `&str`, `String` | `string` |
| `&[f64]`, `Vec<f64>` | `Float64Array` |
| `&[u8]`, `Vec<u8>` | `Uint8Array` |
| `Option<T>` | `T \| undefined` (les arguments acceptent aussi `null`) |

Les chaînes et les tableaux sont copiés entre la mémoire JavaScript et celle de WebAssembly, ce qui prend du temps : voir le [guide des performances](../performance/).

## Compiler

Avec wasm-pack, depuis la racine de votre projet :

```sh
wasm-pack build hello --target web --out-dir ../src/pkg
```

`--out-dir` est relatif au dossier du crate. Avec wasm-bindgen-cli, il faut deux commandes :

```sh
cargo build --manifest-path hello/Cargo.toml --release --target wasm32-unknown-unknown
wasm-bindgen --target web --out-dir src/pkg hello/target/wasm32-unknown-unknown/release/hello.wasm
```

Utilisez `--target web` (recommandé) : active-react l'initialise pour vous et peut précharger le binaire. `--target bundler` fonctionne aussi si votre bundler prend en charge l'intégration ESM de WebAssembly.

## Fichiers générés

| Fichier | Contenu |
| --- | --- |
| `hello.js` | Code JavaScript de liaison : vos fonctions, plus un export `init` par défaut |
| `hello_bg.wasm` | Le Rust compilé |
| `hello.d.ts` | Les types TypeScript de vos fonctions |
| `hello_bg.wasm.d.ts` | Les types des exports bas niveau (vous en aurez rarement besoin) |
| `package.json`, `.gitignore` | Uniquement avec wasm-pack |

Les types générés sont précis. Pour les fonctions ci-dessus :

```ts
export function add(a: number, b: number): number;
export function greet(name: string): string;
export function mean(values: Float64Array): number;
export function normalize(values: Float64Array): Float64Array;
```

> **Remarque :** Les noms de fichiers reprennent le nom du crate, en remplaçant les tirets par des underscores : un crate nommé `my-crate` produit `my_crate.js` et `my_crate_bg.wasm`.

## Organisation du projet

Une organisation qui fonctionne bien avec Vite :

```text
my-app/
├── hello/                 Rust crate
│   ├── Cargo.toml
│   └── src/lib.rs
├── src/
│   ├── pkg/               generated, not committed
│   ├── App.tsx
│   ├── seo.ts             createSeo defaults
│   └── wasm.ts            wasm() declarations
├── index.html
└── package.json
```

Ajoutez `hello/target` et `src/pkg` au `.gitignore` (wasm-pack écrit déjà un `.gitignore` dans `src/pkg`).

## Scripts npm

Placez le build dans un script pour que personne n'ait à retenir les options :

```json
{
  "scripts": {
    "wasm": "wasm-pack build hello --target web --out-dir ../src/pkg",
    "dev": "npm run wasm && vite",
    "build": "npm run wasm && tsc -b && vite build"
  }
}
```

Le même script avec wasm-bindgen-cli, comme dans l'[exemple vite-ssg](https://github.com/maxlestage/Active/tree/master/examples/vite-ssg) :

```json
{
  "scripts": {
    "wasm": "cargo build --manifest-path hello/Cargo.toml --release --target wasm32-unknown-unknown && wasm-bindgen --target web --out-dir src/pkg hello/target/wasm32-unknown-unknown/release/hello.wasm"
  }
}
```

Après avoir modifié le code Rust, relancez `npm run wasm` : Vite détecte les nouveaux fichiers et recharge la page.

Étape suivante : [charger le module dans React](../loading-wasm/).
