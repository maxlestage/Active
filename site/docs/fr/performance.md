---
title: Performances
description: Pourquoi active rend vite sur le serveur et reste léger dans le navigateur : réglages de compilation utiles, tailles mesurées, et comment alléger votre bundle.
---

`active` est conçu pour que le chemin rapide soit celui par défaut : les pages sont écrites directement dans une chaîne sur le serveur, le navigateur reçoit d'abord un HTML complet, et seules les parties interactives chargent du code. Cette page explique ce qui le rend rapide, présente des tailles mesurées et liste ce que vous pouvez ajuster. Pour des comparaisons avec d'autres frameworks Rust, voir [Benchmarks](../benchmarks/).

## Sur le serveur

- **Écriture directe dans une chaîne.** Le rendu parcourt la vue une seule fois et ajoute le HTML à une unique `String`. Il n'y a ni DOM virtuel, ni arbre intermédiaire à comparer, ni template à analyser à l'exécution. `Document::render` écrit d'abord le corps, en récupérant ses valeurs `Seo` au passage, puis la page autour : chaque région dynamique n'est ainsi évaluée qu'une fois ; `Seo` réserve la place dont ses balises ont besoin avant de les écrire.
- **Un échappement peu coûteux.** Le texte et les valeurs d'attributs sont parcourus octet par octet et copiés par tronçons ; seuls les cinq caractères spéciaux (`&`, `<`, `>`, `"`, `'`) sont remplacés.
- **Rien ne s'exécute sans produire de HTML.** Les gestionnaires d'événements et les effets ne sont même pas stockés sur le serveur.
- **Une mémoire bornée.** Les états vivent dans un registre propre à chaque thread, et ceux qu'aucun rendu n'utilise plus sont libérés à la fin des rendus suivants : un serveur qui rend des millions de pages ne garde ainsi qu'une poignée d'états vivants (voir [État, Côté serveur](../state/#cote-serveur)).
- **Aucune dépendance.** Sans la fonctionnalité `client`, le crate n'a aucune dépendance, ce qui limite les temps de compilation et la taille des binaires.

## Dans le navigateur

- **Le HTML d'abord.** Les visiteurs voient le contenu dès que le HTML arrive ; le bundle est préchargé en parallèle (`Document::client` écrit des liens `modulepreload` et `preload`) au lieu d'être découvert après l'exécution du chargeur.
- **Uniquement les îlots.** Les parties statiques d'une page n'envoient aucun code et ne sont jamais rendues une seconde fois. Une page sans îlot ne charge aucun WebAssembly.
- **L'hydratation réutilise le HTML.** Les îlots récupèrent les éléments rendus par le serveur au lieu d'en créer de nouveaux.
- **Des mises à jour à granularité fine.** Chaque `text_dyn`, `attr_dyn`, `class_if` ou `bool_attr` est lié à un seul nœud du DOM. Modifier un état ne réexécute que les liaisons qui le lisent : aucun composant n'est rendu de nouveau, aucun diff n'est calculé.
- **Des handles Copy.** Un `State` se résume à deux entiers. Le déplacer dans des closures ne coûte rien et ne demande aucun comptage de références.
- **Vous ne payez que ce que vous utilisez.** Le code qui tient le `<head>` à jour dans les applications côté client n'est inclus que si le bundle appelle `mount` ou `mount_to` ; un bundle qui ne fait qu'hydrater des îlots ne le contient pas.

Les régions dynamiques (`children_dyn`, `fragment_dyn`) font exception : elles reconstruisent leur contenu d'un bloc quand un état qu'elles lisent change. Pour les longues listes, gardez les lignes par clé avec `children_keyed` (voir [Longues listes : lignes par clé](../state/#longues-listes-lignes-par-cle)).

## Réglages de compilation

Ces profils, tirés du `Cargo.toml` racine du dépôt, donnent un serveur rapide et un bundle léger :

```toml
[profile.release]
opt-level = 3       # fastest code, for the server
lto = true          # optimize across crates, drop unused code
codegen-units = 1   # one unit: better optimization, slower build
strip = true        # no symbols in the binaries

[profile.wasm-release]
inherits = "release"
opt-level = "s"     # optimize for size: smaller downloads
panic = "abort"     # no unwinding machinery
```

| Réglage | Pourquoi |
| --- | --- |
| `opt-level = "s"` | Les navigateurs téléchargent le bundle : la taille compte davantage que les derniers pour cent de vitesse. `"z"` privilégie encore plus la taille ; mesurez les deux. |
| `lto = true` | L'optimisation à l'édition de liens retire les parties de `std`, de `web-sys` et de vos dépendances que vous n'utilisez pas. |
| `codegen-units = 1` | Permet au compilateur de voir tout le crate d'un coup. |
| `strip = true` | Supprime les noms de symboles. Désactivez-le temporairement pour analyser le bundle (voir plus bas). |
| `panic = "abort"` | Une panique arrête le module au lieu de dérouler la pile. C'est déjà le comportement par défaut pour `wasm32-unknown-unknown` ; le profil l'indique explicitement. |

La section `[profile.dev.package."*"]` de ce dépôt optimise aussi les dépendances dans les compilations de débogage (`opt-level = 2`) : le serveur de développement démarre vite, tandis que votre propre code se compile toujours rapidement.

## Tailles

Mesures effectuées sur ce dépôt avec Rust 1.97, `wasm-bindgen` 0.2.129 et le profil `wasm-release`, sans `wasm-opt`. Gzip au niveau 9, Brotli en qualité 11 (les niveaux qu'utiliserait un hébergeur statique ou une étape de build).

| Bundle | `_bg.wasm` brut | gzip | Brotli |
| --- | ---: | ---: | ---: |
| `examples/hello` : un îlot compteur | 56 668 o | 22 249 o | 18 935 o |
| `examples/spa` : liste de tâches avec routage, formulaires et stockage | 119 774 o | 46 562 o | 39 104 o |
| Ce site : quatre îlots, animations, recherche | 177 913 o | 64 747 o | 54 520 o |

Les chargeurs générés par `wasm-bindgen` ajoutent 4,2 ko (hello), 5,2 ko (spa) et 7,6 ko (site web) une fois compressés avec gzip.

Le même `examples/hello` compilé avec le profil `release` classique (`opt-level = 3`) pèse 61 100 o brut et 23 051 o avec gzip : le profil optimisé pour la taille économise environ 9 % brut et 5 % compressé.

L'essentiel d'un petit bundle est de la mécanique partagée (allocateur mémoire, formatage de chaînes, runtime d'hydratation et de réactivité, liaisons `web-sys`), payée une fois par page. Chaque îlot supplémentaire n'ajoute que son propre code : ce site, avec quatre îlots, ses animations et un client de recherche, fait environ trois fois la taille du simple compteur.

## Mesurer votre propre bundle

```sh
cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir pkg target/wasm32-unknown-unknown/wasm-release/my_site.wasm
ls -l pkg/my_site_bg.wasm                 # raw
gzip -9 -c pkg/my_site_bg.wasm | wc -c    # gzip
brotli -c -q 11 pkg/my_site_bg.wasm | wc -c   # Brotli, if the brotli tool is installed
```

Mesurez toujours le fichier `_bg.wasm` produit par `wasm-bindgen` : le fichier présent dans `target/` contient encore des métadonnées que `wasm-bindgen` supprime, et il est plusieurs fois plus volumineux.

Pour voir ce qui prend de la place, compilez une fois sans supprimer les symboles et inspectez le résultat avec [twiggy](https://github.com/rustwasm/twiggy) :

```sh
cargo install twiggy
CARGO_PROFILE_WASM_RELEASE_STRIP=false cargo build --lib --target wasm32-unknown-unknown --profile wasm-release
wasm-bindgen --target web --no-typescript --out-dir pkg-debug target/wasm32-unknown-unknown/wasm-release/my_site.wasm
twiggy top -n 20 pkg-debug/my_site_bg.wasm
```

## Réduire la taille

- **Servez des fichiers compressés.** Brotli économise environ 15 % par rapport à gzip sur ces bundles. Avec Axum, `tower_http::compression::CompressionLayer` compresse les réponses à la volée ; les hébergeurs statiques et les CDN le font généralement pour vous.
- **Mettez en cache pour toujours.** Placez un hachage du contenu dans l'URL du bundle et servez-le comme `immutable` (voir [Documents, Mise en cache longue durée](../documents/#mise-en-cache-longue-duree)). Les visiteurs qui reviennent ne téléchargent alors plus rien.
- **Gardez le travail lourd sur le serveur.** L'analyse du Markdown, la coloration du code ou le formatage des dates peuvent s'exécuter sur le serveur et parvenir à l'îlot sous forme de HTML ou de props. Ce site colore ses exemples de code sur le serveur et transmet le résultat sous forme de HTML brut : le bundle ne contient donc aucun outil de coloration syntaxique.
- **Surveillez les dépendances.** Chaque crate utilisé par le code du navigateur se retrouve dans le bundle. Vérifiez la taille après en avoir ajouté un, et n'activez que les fonctionnalités de `web-sys` que vous utilisez réellement.
- **Essayez `opt-level = "z"`** et gardez-le s'il donne un résultat plus petit pour votre code.
- **Lancez éventuellement `wasm-opt`** de [Binaryen](https://github.com/WebAssembly/binaryen) après `wasm-bindgen` : `wasm-opt -Oz -o out.wasm pkg/my_site_bg.wasm`. Il retire généralement quelques pour cent de plus. S'il rejette le module à cause d'une fonctionnalité WebAssembly, activez celle-ci avec l'option `--enable-…` correspondante. Ce dépôt ne l'utilise pas : les chiffres ci-dessus sont donc obtenus sans lui.

## Rendu côté serveur

Les temps de rendu côté serveur dépendent de vos pages et de votre machine. La page [Benchmarks](../benchmarks/) compare une page complète rendue par `active` avec du code `format!` écrit à la main et avec d'autres frameworks Rust, en détaillant la méthode et les chiffres.
