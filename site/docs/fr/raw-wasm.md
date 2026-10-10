---
title: WebAssembly brut
description: Livrez de petites fonctions Rust numériques dans un simple fichier .wasm aux exports extern C, sans code JavaScript de liaison, chargé avec wasm(url).
---

wasm-bindgen est pratique, mais il ajoute du code JavaScript de liaison et des utilitaires de gestion mémoire à chaque module. Pour du calcul purement numérique, vous pouvez vous en passer : exportez de simples fonctions depuis Rust et chargez directement le fichier `.wasm`. Dans les benchmarks, un module de deux fonctions pesait **144 octets**, contre 15,87 kB pour la version wasm-bindgen (qui gérait aussi une chaîne).

## Quand l'utiliser

Les modules bruts sont parfaits quand chaque fonction ne prend et ne renvoie que des nombres : mathématiques, physique, sommes de contrôle, interpolation, conversion de couleurs, calcul de scores.

Dès que vous avez besoin de chaînes, de tableaux ou d'objets, utilisez plutôt [wasm-bindgen](../rust-setup/) (voir les [limites](#limites)).

## Écrire le Rust

Créez un crate de bibliothèque sans dépendance :

```toml
[package]
name = "math"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

Exportez les fonctions avec `#[unsafe(no_mangle)]` et `extern "C"` :

```rust
// src/lib.rs
#[unsafe(no_mangle)]
pub extern "C" fn add(a: i32, b: i32) -> i32 {
    a.wrapping_add(b)
}

#[unsafe(no_mangle)]
pub extern "C" fn fib(n: u32) -> u32 {
    if n < 2 { n } else { fib(n - 1).wrapping_add(fib(n - 2)) }
}
```

- `#[unsafe(no_mangle)]` conserve le nom de la fonction tel quel, pour que JavaScript puisse la trouver. C'est la syntaxe de l'édition 2024 de Rust ; les éditions plus anciennes écrivent `#[no_mangle]`.
- `extern "C"` donne à la fonction une convention d'appel stable que WebAssembly comprend.

## Compiler

```sh
cargo build --release --target wasm32-unknown-unknown
```

Le binaire se trouve dans `target/wasm32-unknown-unknown/release/math.wasm`. Aucun autre outil n'est nécessaire. Copiez-le là où votre app sert ses fichiers statiques, par exemple dans le dossier `public/` de Vite :

```sh
cp target/wasm32-unknown-unknown/release/math.wasm ../public/math.wasm
```

## Le charger

Passez l'URL du fichier à `wasm()`, avec le type de ses exports :

```ts
// src/wasm.ts
import { wasm } from "active-react";

export const math = wasm<{ add(a: number, b: number): number; fib(n: number): number }>("/math.wasm");
```

Utilisez-le ensuite comme n'importe quel autre module :

```tsx
import { useWasm } from "active-react";
import { math } from "./wasm";

export function Fibonacci({ n }: { n: number }) {
  const m = useWasm(math);
  return <p>fib({n}) = {m ? m.fib(n) : "…"}</p>;
}
```

Comme l'URL est la source elle-même, les modules bruts sont toujours préchargés pendant le rendu serveur. Au lieu d'utiliser `public/`, vous pouvez aussi laisser Vite ajouter une empreinte (hash) au nom du fichier :

```ts
import mathUrl from "./math.wasm?url";

export const math = wasm<{ add(a: number, b: number): number }>(mathUrl);
```

> **Remarque :** Sans argument de type, les exports sont typés `WebAssembly.Exports`. Écrivez l'interface à la main : c'est votre contrat avec le code Rust.

## Le type MIME

active-react télécharge le fichier et le compile **pendant son téléchargement** avec `WebAssembly.instantiateStreaming`, la méthode la plus rapide. Les navigateurs ne l'autorisent que si le serveur envoie l'en-tête `Content-Type: application/wasm`.

Si l'en-tête est absent ou différent, active-react se rabat sur le téléchargement complet du fichier (`arrayBuffer()`), puis sur sa compilation. Votre module fonctionne toujours, juste un peu plus tard. Les serveurs de développement et de preview de Vite, ainsi que la plupart des hébergeurs statiques, envoient le bon en-tête ; vérifiez le vôtre dans l'onglet réseau du navigateur, et consultez la [FAQ](../faq/) pour le corriger.

Une réponse hors de la plage 2xx est une erreur, par exemple `wasm: /math.wasm responded 404`, levée vers votre error boundary comme n'importe quelle [erreur de chargement](../loading-wasm/#gerer-les-erreurs).

## Passer des imports

Un module brut peut aussi appeler JavaScript. Déclarez les fonctions en Rust, dans un module d'import nommé `env` :

```rust
#[link(wasm_import_module = "env")]
unsafe extern "C" {
    fn now() -> f64;
    fn report(ms: f64);
}

#[unsafe(no_mangle)]
pub extern "C" fn timed_fib(n: u32) -> u32 {
    let start = unsafe { now() };
    let result = fib(n);
    unsafe { report(now() - start) };
    result
}
```

Fournissez-les ensuite avec l'option `imports` :

```ts
export const math = wasm<{ timed_fib(n: number): number }>("/math.wasm", {
  imports: {
    env: {
      now: () => performance.now(),
      report: (ms: number) => console.log(`fib took ${ms} ms`),
    },
  },
});
```

> **Astuce :** Les versions récentes de Rust refusent de lier des fonctions déclarées mais non définies. L'attribut `#[link(wasm_import_module = "env")]` les marque explicitement comme des imports.

## Les nombres à la frontière

Les fonctions WebAssembly n'échangent que des nombres, et quelques conversions sont bonnes à connaître :

| Rust | JavaScript |
| --- | --- |
| `i32`, `f32`, `f64` | `number` |
| `u32` | `number`, mais lu comme **signé** : les valeurs supérieures à 2 147 483 647 reviennent négatives. Utilisez `result >>> 0` |
| `i64`, `u64` | `bigint` |
| `bool` | `number` : `1` pour true, `0` pour false |
| `usize`, pointeurs | `number` (32 bits sur `wasm32`) |

## Limites

- **Des nombres uniquement.** Les chaînes, tableaux et structs vivent dans la mémoire linéaire du module. Pour les transmettre, vous devez exporter un allocateur, copier des octets dans `memory.buffer` depuis JavaScript, passer un pointeur et une longueur, puis libérer la mémoire. wasm-bindgen génère exactement ce code pour vous, et correctement.
- **Les types sont écrits à la main.** Rien ne vérifie que votre interface TypeScript correspond aux signatures Rust.
- **Pas de message de panic.** Un panic provoque un trap et lève une `RuntimeError` générique en JavaScript, sans le message Rust.

Si l'une de ces limites vous gêne, passez à [wasm-bindgen](../rust-setup/). Le code JavaScript de liaison n'ajoute que 0,93 kB gzip pour trois fonctions.
