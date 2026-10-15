---
title: Coming from other frameworks
description: What React, Svelte, Solid, Angular, Qwik, Astro, Next.js and Nuxt features are called in active, from Suspense and error boundaries to transitions, form validation and island hydration.
---

`active` borrows the ideas that work best in each front-end framework and gives them one consistent Rust API. If you come from Vue, read [Coming from Vue](../from-vue/) first; this page maps the other frameworks.

## React

| React | active |
| --- | --- |
| `useState` | `use_state(value)`; read with `get()`, write with `set()` / `update()` |
| `useMemo` | `memo(move \|\| …)` |
| `useEffect` | `effect(move \|\| …)`, cleanup with `on_cleanup` |
| `useContext`, `<Provider>` | `use_context::<T>()`, `provide(value, \|\| view)` |
| `<Suspense fallback>` | `suspense(fallback, \|\| view)` with [resources](../async-data/) |
| `useTransition` (keep the old UI while loading) | `Resource::loading()`: the previous value stays shown during a reload |
| error boundaries | `error_boundary(view, fallback)`; components return `Result` |
| `createPortal` | `portal("body", view)` |
| automatic batching | `batch(\|\| …)` |
| `key` on list items | `keyed(items, key, row)` |
| Server Components | server-rendered pages: only [islands](../islands/) ship code |

The main difference: there is no re-render. A component function runs **once**; only the closures that read a state run again. Hooks have no rules of order, and can be called in conditions and loops.

## Svelte and SvelteKit

| Svelte | active |
| --- | --- |
| `let count = $state(0)` | `let count = use_state(0)` |
| `$derived` | `memo(move \|\| …)` |
| `$effect` | `effect(move \|\| …)` |
| stores (`writable`) | a `State` (or a struct of states) shared with `provide` |
| `bind:value` | `bind_value(state)` |
| `transition:fade` | `.transition("fade", move \|\| visible.get())` with CSS classes |
| `in:fly` | `.transition_in("fly")` |
| actions (`use:tooltip`) | `.on_mount(\|element\| …)` |
| `{#await promise}` | `resource` and `suspense` |
| form actions with progressive enhancement | a `<form action method>` the server handles, enhanced with `on_submit` and [validation](../forms/) |
| `+layout.svelte` | a layout component that takes the page as a `Node` |

## Solid

Solid is the closest relative: fine-grained reactivity without a virtual DOM.

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
| dependency injection | `provide` / `use_context` (a struct of states or of services) |
| reactive forms and validators | [`active::forms`](../forms/): `Form`, `Field`, `required`, `email`, `rule`… |
| route guards (`canActivate`) | `before_navigate(\|to\| …)` returning `Navigation::Redirect` |
| pipes (`date`, `currency`) | plain Rust functions called in `text_dyn` |
| `@if`, `@for` | `children_dyn`, `keyed` |

## Qwik and Astro

Both send HTML first and as little JavaScript as possible. `active` does the same with islands: the server renders the whole page, and only the islands are hydrated. `island_with` chooses **when**:

| Astro / Qwik | active |
| --- | --- |
| `client:load` | `island(name, props, component)` |
| `client:idle` | `island_with(name, props, component, Hydrate::Idle)` |
| `client:visible` | `island_with(…, Hydrate::Visible)` |
| `client:media="(max-width: 50em)"` | `island_with(…, Hydrate::Media("(max-width: 50em)".into()))` |
| Qwik's resumability (code loaded on interaction) | `island_with(…, Hydrate::Interaction)` |
| content collections | Markdown read and rendered by the server ([Vitesse](../vitesse/)) |

See [Islands](../islands/) for the details.

## Next.js and Nuxt

| Next / Nuxt | active |
| --- | --- |
| file-based pages | a `Routes` table: `.route("/posts/:id", post)` |
| layouts | a layout component wrapping each page |
| `<Head>`, `useHead` | a `Seo` in each page, merged with the layout's |
| `router.push`, `router.replace` | `navigate(to)`, `navigate_replace(to)` |
| middleware, `definePageMeta({ middleware })` | `before_navigate(guard)` in the browser, your server's middleware on the server |
| scroll restoration | built in: the back button brings the scroll position back |
| `getServerSideProps`, `useFetch` | the server loads the data and renders the page; `resource` in the browser |
| static export | [static sites](../static-site/) |

## Lit and Web Components

An island renders as an `<active-island>` element with its props in an attribute, much like a custom element, and works in any page, whatever produced it. Use `el("my-widget")` to render any custom element, with attributes and children, and `on("my-event", …)` to listen to its custom events.
