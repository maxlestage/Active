---
title: Introducción
description: active-react lleva las etiquetas SEO y Rust/WebAssembly a React 19 en unos 1,3 kB gzip, sin dependencias y con soporte para SSR, SSG y RSC.
---

active-react es una pequeña biblioteca para React 19 que hace bien dos cosas: renderiza las etiquetas SEO que necesita cada página y permite que tus componentes llamen a código Rust compilado a WebAssembly (WASM).

La biblioteca completa pesa unos **1,3 kB gzip**. Tiene **cero dependencias** (React es una peer dependency), ejecuta **cero efectos** y funciona con renderizado en servidor (SSR), generación estática de sitios (SSG) y React Server Components (RSC).

```tsx
import { createSeo, wasm, useWasm } from "active-react";

const Seo = createSeo({ siteName: "Acme", titleTemplate: "%s · Acme", baseUrl: "https://acme.com" });
const rust = wasm(() => import("./pkg/my_crate.js"));

export function Page() {
  const lib = useWasm(rust); // undefined while loading
  return (
    <>
      <Seo title="Home" description="The Acme website." canonical="/" image="/og.png" />
      <p>{lib ? lib.greet("you") : "Loading…"}</p>
    </>
  );
}
```

## Lo que obtienes

- **`<Seo>` y `createSeo()`**: título, descripción, URL canónica, Open Graph, tarjetas de Twitter, robots, alternativas hreflang y datos estructurados JSON-LD, desde un solo componente.
- **`wasm()` y `useWasm()`**: declara un módulo Rust una vez, cárgalo una vez y úsalo desde cualquier componente. El binario se precarga durante el renderizado en servidor.
- **`renderWithHead()`**: un helper de servidor para plantillas HTML (Vite SSR, generación estática) que devuelve por separado las etiquetas del head y el body.
- **El crate `active`**: el gemelo en Rust de `<Seo>`, que produce las mismas etiquetas desde servidores Rust, generadores estáticos o WebAssembly.

## Cómo funciona

### SEO: React se encarga del hoisting

Desde React 19, los elementos `<title>`, `<meta>` y `<link>` renderizados en cualquier parte de tu árbol de componentes son trasladados ("hoisting") al `<head>` del documento por el propio React, tanto en el servidor como en el navegador.

A una biblioteca de SEO le queda muy poco por hacer. `<Seo>` simplemente renderiza las etiquetas correctas. No hay provider, ni contexto, ni store, ni efecto, ni segunda pasada de renderizado. Cuando el usuario navega, React elimina las etiquetas de la página anterior e inserta las nuevas, de modo que el `<head>` siempre corresponde a la página que está en pantalla.

### WASM: declarar, cargar una vez, precargar

- `wasm()` **declara** un módulo. Todavía no se descarga nada.
- `useWasm()` lo **carga** la primera vez que un componente lo necesita, con una única petición compartida, sin importar cuántos componentes lo usen. Devuelve las exportaciones del módulo, o `undefined` mientras se carga.
- Cuando se conoce la URL del binario, `useWasm()` le pide a React que lo **precargue**. Durante el SSR, esto coloca un `<link rel="preload">` en el `<head>`, de modo que el navegador descarga el archivo `.wasm` en paralelo con tu JavaScript en lugar de después.

## Cifras clave

Según [los benchmarks](../benchmarks/) (la misma página para cada biblioteca, builds de producción):

| Medida | active-react | Para comparar |
| --- | ---: | --- |
| Código SEO en el bundle del cliente | 0,74 kB gzip | react-helmet 6,76 kB, react-helmet-async 6,29 kB, @unhead/react 6,07 kB |
| Tiempo de SSR por página | 42,4 µs | 39,7 µs con etiquetas escritas a mano, 66,5 µs con react-helmet |
| Actualización del head tras un cambio de ruta | 0,3 ms de mediana | 16,7 ms con react-helmet |
| Cargador WASM en el bundle del cliente | 0,60 kB gzip | react-wasm 2,07 kB |
| Primer resultado de Rust en pantalla, RTT de 100 ms | 224 ms con preload en SSR | 330 ms sin preload |

En resumen: el SEO cuesta apenas un pequeño porcentaje más que escribir las etiquetas a mano, el renderizado en servidor es 1,5× más rápido que con react-helmet y el preload automático reduce un 32 % el tiempo hasta el primer resultado de Rust.

## Cuándo usarlo

active-react encaja bien cuando:

- desarrollas con **React 19** y quieres etiquetas SEO correctas y completas con muy poco código: Vite (SPA, SSR o SSG), React Router v7, TanStack Start, Waku o tu propio servidor;
- quieres trasladar el trabajo pesado (parsing, cálculos matemáticos, procesamiento de imágenes o de datos) a **Rust** sin escribir código repetitivo de carga;
- te importa el **tamaño del bundle**.

## Cuándo no usarlo

- **React 18 o anterior** no está soportado. Trasladar las etiquetas al `<head>` es una funcionalidad de React 19, y la biblioteca se basa en ella.
- **Next.js App Router** tiene su propia Metadata API (`metadata` y `generateMetadata`), integrada con su enrutamiento y su streaming. Puede que la prefieras para el SEO. `wasm()` y `useWasm()` siguen funcionando en los componentes cliente.
- **Los cálculos diminutos** no necesitan Rust: cada llamada a WebAssembly tiene un pequeño costo fijo. Consulta la [guía de rendimiento](../performance/).

## Mapa de la documentación

**Primeros pasos**

- [Instalación](../installation/): requisitos, comandos de instalación y puntos de entrada.
- [Inicio rápido](../quick-start/): una app de Vite con SEO y Rust en cinco minutos.

**SEO**

- [El componente `<Seo>`](../seo-component/): cada prop y el HTML que produce.
- [Valores por defecto del sitio](../site-defaults/): `createSeo`, plantillas de título, URLs, robots y hreflang.
- [Datos estructurados](../structured-data/): recetas JSON-LD para resultados enriquecidos.
- [Renderizado en servidor](../server-rendering/): SSR, generación estática y `renderWithHead`.

**Rust y WebAssembly**

- [Configuración de Rust](../rust-setup/): toolchain, perfil de Cargo y primer build.
- [Cargar WebAssembly](../loading-wasm/): `wasm()`, `useWasm()`, preload, errores y Suspense.
- [WebAssembly puro](../raw-wasm/): módulos diminutos sin wasm-bindgen.
- [Guía de rendimiento](../performance/): lo que enseñan los benchmarks.
- [El crate de Rust](../rust-crate/): las mismas etiquetas SEO, generadas desde Rust.

**Puesta en producción**

- [Desplegar en Heroku](../deploy-heroku/): desde tu teléfono, sin necesidad de computadora.
- [Publicación](../publishing/): publica en npm y crates.io a partir de una GitHub Release.

**Referencia**

- [Referencia de la API](../api/), [Benchmarks](../benchmarks/) y [Preguntas frecuentes](../faq/).
