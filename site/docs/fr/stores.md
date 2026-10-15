---
title: État persistant et débogage
description: Garder un état dans localStorage ou sessionStorage, synchronisé entre les onglets, avec persisted, et observer le runtime réactif avec active::debug (états et effets vivants, effets nommés qui comptent et journalisent leurs exécutions).
---

```rust
use active::prelude::*;
use active::{debug, persisted, persisted_in, Storage};
```

## persisted

`persisted(clé, initial)` est un état gardé dans `localStorage` : un thème, un panier, la dernière recherche, un brouillon. Il démarre avec la valeur enregistrée (ou `initial`), chaque changement est réécrit, et un changement fait dans un autre onglet du site le met à jour ici aussi (les stores persistants de Svelte, `useLocalStorage` de VueUse, `persist` de Zustand) :

```rust
let dark = persisted("theme-dark", false);
let toggle = button()
    .on_click(move |_| dark.update(|d| *d = !*d))
    .text_dyn(move || if dark.get() { "Clair".into() } else { "Sombre".into() });
```

`persisted_in(Storage::Session, clé, initial)` utilise `sessionStorage` à la place : gardé à travers les rechargements, un par onglet, non partagé.

Les valeurs sont écrites en JSON avec le [trait `Data`](../streaming/) (chaînes, nombres, `bool`, `Option`, `Vec`, maps, tuples, et vos propres types). Une valeur enregistrée illisible (un ancien format) est remplacée par `initial`.

Sur le serveur, et là où le stockage est bloqué, c'est un état ordinaire qui vaut `initial` : le serveur rend `initial`, et le navigateur affiche la valeur enregistrée une fois hydraté. Pour des valeurs qui changent beaucoup la mise en page, rendez-les dans une `transition` ou gardez serveur et navigateur d'accord avec un cookie.

Les valeurs dérivées ne demandent rien de nouveau : un [`memo`](../state/) d'un état persistant est un store dérivé.

## Débogage : active::debug

`debug::stats()` compte ce que contient le runtime réactif de ce thread :

```rust
let stats = debug::stats();
println!("{stats}"); // "1520 states, 310 effects, 98211 effect runs"
debug::log_stats("après fermeture du dialogue"); // dans la console (le terminal en natif)
```

Ouvrez et fermez un dialogue, changez de page quelques fois : des nombres qui ne cessent de grandir signalent quelque chose qui n'est jamais libéré — souvent un état créé hors de tout composant dans une région reconstruite souvent.

`debug::effect(nom, f)` est un `effect` qui compte ses exécutions sous `nom` ; `debug::runs(nom)` lit le compte, et après `debug::log(true)` chaque exécution est écrite dans la console (`active: effect "cart total" ran (3)`). On voit ce qui se ré-exécute quand un état change, et à quelle fréquence :

```rust
debug::log(true);
debug::effect("cart total", move || total.set(cart.with(|c| c.iter().map(|i| i.price).sum())));
```
