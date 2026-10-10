---
title: Desplegar en Heroku
description: Publica este sitio de documentación, o tu app construida igual, en Heroku desde un teléfono, con despliegue en un toque y actualizaciones automáticas.
---

Esta guía publica el sitio web de active-react (esta documentación y su página de presentación) en Heroku usando solo un teléfono y un navegador web. Los mismos pasos funcionan para tu propia app si está construida de la misma manera (consulta [lo que incluye el repositorio](#lo-que-incluye-el-repositorio)).

Hay dos opciones. La **opción A** crea la app en unos pocos toques. La **opción B** la conecta a GitHub para que cada cambio fusionado en `master` se publique automáticamente. Puedes hacer la A y después la B.

## Antes de empezar

Necesitas:

- **Una cuenta de GitHub.** La opción A funciona directamente con el repositorio público. La opción B necesita un repositorio que administres: primero haz un fork de `maxlestage/Active` (abre el repositorio y toca **Fork**) o usa el tuyo.
- **Una cuenta de Heroku** ([signup.heroku.com](https://signup.heroku.com)) con un **método de pago** añadido en Account settings, Billing. Heroku ya no tiene plan gratuito. La opción más barata, los **dynos Eco, cuesta unos 5 USD al mes** por una bolsa de horas compartida entre todas tus apps Eco. Las apps Eco se duermen tras 30 minutos sin visitas, así que la primera visita después de una pausa tarda unos segundos. Consulta la página de precios de Heroku para ver los precios actuales.

> **Consejo:** El panel de Heroku funciona en un navegador móvil. Si un menú se ve apretado, cambia tu navegador al modo "Sitio de escritorio".

## Opción A: despliegue en un toque

1. Abre este enlace en tu teléfono: [Deploy to Heroku](https://heroku.com/deploy?template=https://github.com/maxlestage/Active). Para tu fork, reemplaza `maxlestage/Active` por tu repositorio en la dirección: `https://heroku.com/deploy?template=https://github.com/<you>/Active`.
2. Inicia sesión en Heroku si te lo pide.
3. **App name:** elige un nombre en minúsculas, como `my-active-docs`. Pasa a formar parte de tu dirección, que termina en `.herokuapp.com`.
4. **Region:** elige United States o Europe, la que esté más cerca de tus visitantes.
5. **SITE_URL** (opcional): déjalo vacío. El sitio usará entonces la dirección que escribieron los visitantes. Rellénalo más adelante si añades un dominio personalizado.
6. Toca **Deploy app**. Heroku descarga el código, instala las dependencias y construye el sitio. El log va desfilando; tarda unos minutos.
7. Cuando aparezca "Your app was successfully deployed", toca **View** para abrir tu sitio, o **Manage App** para abrir su panel.

Listo. Eso sí, la app no se actualizará sola cuando cambie el código: para eso está la opción B.

## Opción B: despliegues automáticos desde GitHub

1. Abre [dashboard.heroku.com](https://dashboard.heroku.com) en tu navegador móvil y toca tu app. ¿Todavía no tienes ninguna? Toca **New**, después **Create new app**, y elige un nombre y una región.
2. Abre la pestaña **Deploy**.
3. En **Deployment method**, toca **GitHub**, después **Connect to GitHub**, y autoriza a Heroku en la ventana que se abre.
4. En **Search for a repository to connect to**, escribe `Active` (o el nombre de tu repositorio), toca **Search** y después **Connect** junto al resultado correcto.
5. En **Automatic deploys**, elige la rama **`master`**.
6. Opcionalmente, marca **Wait for CI to pass before deploy**. Heroku esperará entonces a que las comprobaciones de GitHub Actions del repositorio terminen con éxito, así que un build roto nunca llega a producción.
7. Toca **Enable Automatic Deploys**.
8. Para el primer despliegue, desplázate hasta **Manual deploy** y toca **Deploy Branch**.

A partir de ahora, **cada fusión en `master` vuelve a desplegar el sitio.** Puedes seguir cada build en la pestaña **Activity**.

## Lo que incluye el repositorio

Heroku necesita saber cómo construir e iniciar la app. El repositorio lo incluye todo, así que no hay nada que configurar:

| Archivo | Función |
| --- | --- |
| `app.json` | Describe la app para el botón Deploy: nombre, buildpack de Node.js, variable opcional `SITE_URL`, un dyno web |
| `Procfile` | `web: npm start`, el comando que Heroku ejecuta para iniciar el sitio |
| `package.json`, `heroku-postbuild` | Heroku ejecuta este script después de instalar las dependencias. Compila la biblioteca y después el sitio (`site/`) |
| `package.json`, `engines` | `"node": ">=20"`: la versión de Node.js que instala Heroku |
| `.slugignore` | Deja los benchmarks, los ejemplos, los tests y el código fuente Rust fuera del bundle desplegado |
| `site/server.js` | El servidor de producción, sin dependencias |

`site/server.js` sirve las páginas prerrenderizadas y además:

- envía a los visitantes de `/` a su idioma (`/en/`, `/fr/` o `/es/`) según la configuración de su navegador;
- completa la dirección del sitio en las URLs canónicas y de Open Graph: `SITE_URL` si está definida, o si no la dirección de la petición;
- comprime las respuestas con Brotli o gzip;
- guarda en caché durante un año los recursos con hash en el nombre y revalida las páginas HTML;
- escucha en el puerto que Heroku proporciona en la variable de entorno `PORT`.

> **Nota:** El build de Node.js de Heroku no tiene compilador de Rust. El módulo WebAssembly del sitio se compila de antemano y se incluye en el repositorio, en `site/src/wasm`. Si tu propia app usa Rust, haz commit de los archivos `pkg` generados de la misma manera, o genéralos en CI.

## Dominio personalizado

1. En tu app, abre **Settings**, desplázate hasta **Domains** y toca **Add domain**.
2. Escribe tu dominio, por ejemplo `www.example.com`, y confirma. Heroku muestra un **DNS target** que termina en `herokudns.com`.
3. En tu registrador de dominios (la mayoría tiene un sitio adaptado a móviles), añade un registro **CNAME**: nombre `www`, valor el DNS target. Para un dominio raíz (`example.com`), usa un registro ALIAS, ANAME o "CNAME flattening" si tu proveedor lo ofrece.
4. En **Settings**, **Config Vars**, define `SITE_URL` como `https://www.example.com` para que las URLs canónicas usen siempre tu dominio.

Los cambios de DNS pueden tardar desde unos minutos hasta unas horas. Para tener HTTPS en tu propio dominio, usa **Configure SSL** en la misma sección; los certificados automáticos de Heroku no están disponibles en todos los tipos de dyno, así que comprueba qué incluye el tuyo.

## Logs

En tu app, toca **More** (arriba a la derecha) y después **View logs**. Verás la salida del servidor y cada petición en tiempo real. Los logs de build están en la pestaña **Activity**: toca **View build log** junto a un build.

## Solución de problemas

### El build falla

Abre **Activity**, después el log del build fallido, y mira las últimas líneas en rojo.

- `npm ci` se queja de que `package.json` y `package-lock.json` no están sincronizados: ejecuta `npm install` y haz commit del lock file actualizado.
- Un error de TypeScript o de Vite: el mismo error se produce en local con `npm run build` en la raíz del repositorio y `npm --prefix site run build`.
- No se encuentra `cargo` o `wasm-pack`: Heroku no tiene Rust; haz commit de los archivos WebAssembly generados (consulta la nota anterior).

### Application error (H10 App crashed)

La página muestra "Application error" y los logs contienen `code=H10`. El servidor falló al arrancar. Busca el error justo encima en los logs. Las causas habituales:

- el servidor no escucha en `process.env.PORT` (Heroku informa entonces de `R10 Boot timeout` tras 60 segundos);
- el script `start` o el `Procfile` apunta a un archivo que no existe;
- una dependencia necesaria en tiempo de ejecución está en `devDependencies`, que Heroku elimina después del build.

`H14 No web processes running` significa que no hay ningún dyno en marcha: abre **Resources** y activa el dyno `web`.

### Versión de Node.js incorrecta

Heroku elige la versión de Node.js a partir de `engines.node` en el `package.json` raíz. Para fijar una versión mayor, define por ejemplo:

```json
{
  "engines": {
    "node": "22.x"
  }
}
```

Haz commit y Heroku la usará en el siguiente despliegue. El log del build muestra la versión resuelta cerca del principio.
