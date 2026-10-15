---
title: Venir d'autres frameworks
description: Le nom des fonctionnalités de React, Svelte, Solid, Angular, Qwik, Astro, Next.js et Nuxt dans active, de Suspense et des error boundaries aux transitions, à la validation de formulaires et à l'hydratation des îlots.
---

`active` emprunte les idées qui marchent le mieux dans chaque framework front-end et leur donne une API Rust cohérente. Si vous venez de Vue, lisez d'abord [Venir de Vue](../from-vue/) ; cette page fait la correspondance avec les autres frameworks.

## React

| React | active |
| --- | --- |
| `useState` | `use_state(value)` ; lire avec `get()`, écrire avec `set()` / `update()` |
| `useMemo` | `memo(move \|\| …)` |
| `useEffect` | `effect(move \|\| …)`, nettoyage avec `on_cleanup` |
| `useContext`, `<Provider>` | `use_context::<T>()`, `provide(value, \|\| view)` |
| `<Suspense fallback>` | `suspense(fallback, \|\| view)` avec des [ressources](../async-data/) |
| `useTransition` (garder l'ancienne interface pendant un chargement) | `Resource::loading()` : la valeur précédente reste affichée pendant un rechargement |
| error boundaries | `error_boundary(view, fallback)` ; les composants renvoient un `Result` |
| `createPortal` | `portal("body", view)` |
| batching automatique | `batch(\|\| …)` |
| `key` sur les éléments d'une liste | `keyed(items, key, row)` |
| Server Components | pages rendues par le serveur : seuls les [îlots](../islands/) envoient du code |

La grande différence : il n'y a pas de re-rendu. Une fonction de composant s'exécute **une fois** ; seules les closures qui lisent un état s'exécutent à nouveau. Les hooks n'ont pas de règle d'ordre, et peuvent être appelés dans des conditions et des boucles.

## Svelte et SvelteKit

| Svelte | active |
| --- | --- |
| `let count = $state(0)` | `let count = use_state(0)` |
| `$derived` | `memo(move \|\| …)` |
| `$effect` | `effect(move \|\| …)` |
| stores (`writable`) | un `State` (ou une structure d'états) partagé avec `provide` |
| `bind:value` | `bind_value(state)` |
| `transition:fade` | `.transition("fade", move \|\| visible.get())` avec des classes CSS |
| `in:fly` | `.transition_in("fly")` |
| actions (`use:tooltip`) | `.on_mount(\|element\| …)` |
| `{#await promise}` | `resource` et `suspense` |
| form actions avec amélioration progressive | un `<form action method>` traité par le serveur, amélioré avec `on_submit` et la [validation](../forms/) |
| `+layout.svelte` | un composant de mise en page qui reçoit la page en `Node` |

## Solid

Solid est le plus proche parent : réactivité à granularité fine, sans DOM virtuel.

| Solid | active |
| --- | --- |
| `createSignal` | `use_state` |
| `createMemo` | `memo` |
| `createEffect` | `effect` |
| `createResource` | `resource(source, fetch)` |
| `<Suspense>` | `suspense(fallback, view)` |
| `<ErrorBoundary>` | `error_boundary(view, fallback)` |
| `batch` | `batch` |
| `untrack` | `untrack` |
| `<Show when>` | `children_dyn(move \|\| if … { vec![…] } else { vec![] })` |
| `<For each>` | `keyed(items, key, row)` |
| `<Index each>` | `indexed(items, row)` |
| `<Switch>` / `<Match>` | `switch(value, kind, view)` |
| `<Portal>` | `portal(target, view)` |
| `onCleanup` | `on_cleanup` |

## Angular

| Angular | active |
| --- | --- |
| signals, `computed`, `effect` | `use_state`, `memo`, `effect` |
| injection de dépendances | `provide` / `use_context` (une structure d'états ou de services) |
| formulaires réactifs et validateurs | [`active::forms`](../forms/) : `Form`, `Field`, `required`, `email`, `rule`… |
| gardes de route (`canActivate`) | `before_navigate(\|to\| …)` qui renvoie `Navigation::Redirect` |
| pipes (`date`, `currency`) | de simples fonctions Rust appelées dans `text_dyn` |
| `@if`, `@for` | `children_dyn`, `keyed` |

## Qwik et Astro

Tous deux envoient d'abord du HTML, et aussi peu de JavaScript que possible. `active` fait de même avec les îlots : le serveur rend toute la page, et seuls les îlots sont hydratés. `island_with` choisit **quand** :

| Astro / Qwik | active |
| --- | --- |
| `client:load` | `island(name, props, component)` |
| `client:idle` | `island_with(name, props, component, Hydrate::Idle)` |
| `client:visible` | `island_with(…, Hydrate::Visible)` |
| `client:media="(max-width: 50em)"` | `island_with(…, Hydrate::Media("(max-width: 50em)".into()))` |
| la résumabilité de Qwik (code chargé à l'interaction) | `island_with(…, Hydrate::Interaction)` |
| content collections | du Markdown lu et rendu par le serveur ([Vitesse](../vitesse/)) |

Voir [Îlots](../islands/) pour les détails.

## Next.js et Nuxt

| Next / Nuxt | active |
| --- | --- |
| pages par fichiers | une table `Routes` : `.route("/posts/:id", post)` |
| layouts | un composant de mise en page autour de chaque page |
| `<Head>`, `useHead` | un `Seo` dans chaque page, fusionné avec celui de la mise en page |
| `router.push`, `router.replace` | `navigate(to)`, `navigate_replace(to)` |
| middleware, `definePageMeta({ middleware })` | `before_navigate(guard)` dans le navigateur, le middleware de votre serveur côté serveur |
| restauration du défilement | intégrée : le bouton retour rend la position de défilement |
| `getServerSideProps`, `useFetch` | le serveur charge les données et rend la page ; `resource` dans le navigateur |
| export statique | [sites statiques](../static-site/) |

## Lit et les Web Components

Un îlot est rendu comme un élément `<active-island>` avec ses props dans un attribut, un peu comme un custom element, et fonctionne dans n'importe quelle page, quel que soit ce qui l'a produite. Utilisez `el("my-widget")` pour rendre n'importe quel custom element, avec attributs et enfants, et `on("my-event", …)` pour écouter ses événements personnalisés.
