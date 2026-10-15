---
title: Si vienes de otros frameworks
description: Cómo se llaman en active las funciones de React, Svelte, Solid, Angular, Qwik, Astro, Next.js y Nuxt, de Suspense y las error boundaries a las transiciones, la validación de formularios y la hidratación de islas.
---

`active` toma las ideas que mejor funcionan en cada framework front-end y les da una API de Rust coherente. Si vienes de Vue, lee primero [Si vienes de Vue](../from-vue/); esta página hace la correspondencia con los demás frameworks.

## React

| React | active |
| --- | --- |
| `useState` | `use_state(value)`; leer con `get()`, escribir con `set()` / `update()` |
| `useMemo` | `memo(move \|\| …)` |
| `useEffect` | `effect(move \|\| …)`, limpieza con `on_cleanup` |
| `useContext`, `<Provider>` | `use_context::<T>()`, `provide(value, \|\| view)` |
| `<Suspense fallback>` | `suspense(fallback, \|\| view)` con [recursos](../async-data/) |
| `useTransition` (mantener la interfaz anterior mientras carga) | `Resource::loading()`: el valor anterior sigue visible durante una recarga |
| error boundaries | `error_boundary(view, fallback)`; los componentes devuelven `Result` |
| `createPortal` | `portal("body", view)` |
| batching automático | `batch(\|\| …)` |
| `key` en los elementos de una lista | `keyed(items, key, row)` |
| Server Components | páginas renderizadas por el servidor: solo las [islas](../islands/) envían código |

La gran diferencia: no hay re-renderizado. Una función de componente se ejecuta **una vez**; solo vuelven a ejecutarse los closures que leen un estado. Los hooks no tienen reglas de orden, y se pueden llamar dentro de condiciones y bucles.

## Svelte y SvelteKit

| Svelte | active |
| --- | --- |
| `let count = $state(0)` | `let count = use_state(0)` |
| `$derived` | `memo(move \|\| …)` |
| `$effect` | `effect(move \|\| …)` |
| stores (`writable`) | un `State` (o un struct de estados) compartido con `provide` |
| `bind:value` | `bind_value(state)` |
| `transition:fade` | `.transition("fade", move \|\| visible.get())` con clases CSS |
| `in:fly` | `.transition_in("fly")` |
| actions (`use:tooltip`) | `.on_mount(\|element\| …)` |
| `{#await promise}` | `resource` y `suspense` |
| form actions con mejora progresiva | un `<form action method>` que procesa el servidor, mejorado con `on_submit` y la [validación](../forms/) |
| `+layout.svelte` | un componente de layout que recibe la página como `Node` |

## Solid

Solid es el pariente más cercano: reactividad de grano fino, sin DOM virtual.

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
| inyección de dependencias | `provide` / `use_context` (un struct de estados o de servicios) |
| formularios reactivos y validadores | [`active::forms`](../forms/): `Form`, `Field`, `required`, `email`, `rule`… |
| guardas de ruta (`canActivate`) | `before_navigate(\|to\| …)` que devuelve `Navigation::Redirect` |
| pipes (`date`, `currency`) | funciones de Rust normales llamadas en `text_dyn` |
| `@if`, `@for` | `children_dyn`, `keyed` |

## Qwik y Astro

Los dos envían primero HTML y el mínimo JavaScript posible. `active` hace lo mismo con las islas: el servidor renderiza toda la página, y solo se hidratan las islas. `island_with` elige **cuándo**:

| Astro / Qwik | active |
| --- | --- |
| `client:load` | `island(name, props, component)` |
| `client:idle` | `island_with(name, props, component, Hydrate::Idle)` |
| `client:visible` | `island_with(…, Hydrate::Visible)` |
| `client:media="(max-width: 50em)"` | `island_with(…, Hydrate::Media("(max-width: 50em)".into()))` |
| la resumabilidad de Qwik (código cargado al interactuar) | `island_with(…, Hydrate::Interaction)` |
| content collections | Markdown leído y renderizado por el servidor ([Vitesse](../vitesse/)) |

Ver [Islas](../islands/) para los detalles.

## Next.js y Nuxt

| Next / Nuxt | active |
| --- | --- |
| páginas por archivos | una tabla `Routes`: `.route("/posts/:id", post)` |
| layouts | un componente de layout alrededor de cada página |
| `<Head>`, `useHead` | un `Seo` en cada página, fusionado con el del layout |
| `router.push`, `router.replace` | `navigate(to)`, `navigate_replace(to)` |
| middleware, `definePageMeta({ middleware })` | `before_navigate(guard)` en el navegador, el middleware de tu servidor en el servidor |
| restauración del desplazamiento | incluida: el botón atrás recupera la posición de desplazamiento |
| `getServerSideProps`, `useFetch` | el servidor carga los datos y renderiza la página; `resource` en el navegador |
| exportación estática | [sitios estáticos](../static-site/) |

## Lit y los Web Components

Una isla se renderiza como un elemento `<active-island>` con sus props en un atributo, parecido a un custom element, y funciona en cualquier página, la produzca lo que la produzca. Usa `el("my-widget")` para renderizar cualquier custom element, con atributos e hijos, y `on("my-event", …)` para escuchar sus eventos personalizados.
