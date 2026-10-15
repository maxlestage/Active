---
title: Estado persistente y depuración
description: Guardar un estado en localStorage o sessionStorage, sincronizado entre pestañas, con persisted, y observar el runtime reactivo con active::debug (estados y efectos vivos, efectos con nombre que cuentan y registran sus ejecuciones).
---

```rust
use active::prelude::*;
use active::{debug, persisted, persisted_in, Storage};
```

## persisted

`persisted(clave, inicial)` es un estado guardado en `localStorage`: un tema, un carrito, la última búsqueda, un borrador. Empieza con el valor guardado (o `inicial`), cada cambio se vuelve a escribir, y un cambio hecho en otra pestaña del sitio también lo actualiza aquí (los stores persistentes de Svelte, `useLocalStorage` de VueUse, `persist` de Zustand):

```rust
let dark = persisted("theme-dark", false);
let toggle = button()
    .on_click(move |_| dark.update(|d| *d = !*d))
    .text_dyn(move || if dark.get() { "Claro".into() } else { "Oscuro".into() });
```

`persisted_in(Storage::Session, clave, inicial)` usa `sessionStorage`: se conserva al recargar, uno por pestaña, no compartido.

Los valores se escriben en JSON con el [trait `Data`](../streaming/) (cadenas, números, `bool`, `Option`, `Vec`, mapas, tuplas y tus propios tipos). Un valor guardado que no se puede leer (un formato antiguo) se sustituye por `inicial`.

En el servidor, y donde el almacenamiento está bloqueado, es un estado normal que vale `inicial`: el servidor renderiza `inicial` y el navegador muestra el valor guardado al hidratar. Para valores que cambian mucho el diseño, renderízalos dentro de una `transition` o mantén de acuerdo servidor y navegador con una cookie.

Los valores derivados no necesitan nada nuevo: un [`memo`](../state/) de un estado persistente es un store derivado.

## Depuración: active::debug

`debug::stats()` cuenta lo que contiene el runtime reactivo de este hilo:

```rust
let stats = debug::stats();
println!("{stats}"); // "1520 states, 310 effects, 98211 effect runs"
debug::log_stats("tras cerrar el diálogo"); // en la consola (la terminal en nativo)
```

Abre y cierra un diálogo, cambia de página unas cuantas veces: números que no paran de crecer indican algo que nunca se libera — normalmente un estado creado fuera de cualquier componente dentro de una región que se reconstruye a menudo.

`debug::effect(nombre, f)` es un `effect` que cuenta sus ejecuciones bajo `nombre`; `debug::runs(nombre)` lee la cuenta, y tras `debug::log(true)` cada ejecución se escribe en la consola (`active: effect "cart total" ran (3)`). Muestra qué se vuelve a ejecutar cuando cambia un estado, y con qué frecuencia:

```rust
debug::log(true);
debug::effect("cart total", move || total.set(cart.with(|c| c.iter().map(|i| i.price).sum())));
```
