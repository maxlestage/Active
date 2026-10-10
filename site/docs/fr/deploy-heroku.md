---
title: Déployer sur Heroku
description: Mettez ce site de documentation, ou votre propre app construite de la même façon, en ligne sur Heroku depuis un téléphone, avec mises à jour automatiques.
---

Ce guide met en ligne le site d'active-react (cette documentation et sa page de présentation) sur Heroku, avec seulement un téléphone et un navigateur web. Les mêmes étapes fonctionnent pour votre propre app si elle est construite de la même façon (voir [ce que fournit le dépôt](#ce-que-fournit-le-depot)).

Il existe deux options. L'**option A** crée l'app en quelques gestes. L'**option B** la connecte à GitHub pour que chaque modification fusionnée dans `master` soit mise en ligne automatiquement. Vous pouvez faire A, puis B.

## Avant de commencer

Il vous faut :

- **Un compte GitHub.** L'option A fonctionne directement avec le dépôt public. L'option B nécessite un dépôt que vous administrez : forkez d'abord `maxlestage/Active` (ouvrez le dépôt, touchez **Fork**), ou utilisez le vôtre.
- **Un compte Heroku** ([signup.heroku.com](https://signup.heroku.com)) avec un **moyen de paiement** ajouté dans Account settings, Billing. Heroku n'a plus d'offre gratuite. L'option la moins chère, les **dynos Eco, coûte environ 5 $ par mois** pour un ensemble d'heures partagé entre toutes vos apps Eco. Les apps Eco se mettent en veille après 30 minutes sans visite : la première visite après une pause prend donc quelques secondes. Consultez la page des tarifs de Heroku pour les prix actuels.

> **Astuce :** Le tableau de bord de Heroku fonctionne dans un navigateur mobile. Si un menu semble à l'étroit, passez votre navigateur en mode « Version pour ordinateur ».

## Option A : déploiement en un geste

1. Ouvrez ce lien sur votre téléphone : [Deploy to Heroku](https://heroku.com/deploy?template=https://github.com/maxlestage/Active). Pour votre fork, remplacez `maxlestage/Active` par votre dépôt dans l'adresse : `https://heroku.com/deploy?template=https://github.com/<you>/Active`.
2. Connectez-vous à Heroku si on vous le demande.
3. **App name** : choisissez un nom en minuscules, comme `my-active-docs`. Il fera partie de votre adresse, qui se termine par `.herokuapp.com`.
4. **Region** : choisissez United States ou Europe, selon la région la plus proche de vos visiteurs.
5. **SITE_URL** (facultatif) : laissez ce champ vide. Le site utilise alors l'adresse saisie par les visiteurs. Remplissez-le plus tard si vous ajoutez un domaine personnalisé.
6. Touchez **Deploy app**. Heroku télécharge le code, installe les dépendances et construit le site. Le journal défile ; cela prend quelques minutes.
7. Quand « Your app was successfully deployed » apparaît, touchez **View** pour ouvrir votre site, ou **Manage App** pour ouvrir son tableau de bord.

C'est fait. L'app ne se mettra toutefois pas à jour d'elle-même quand le code changera : c'est le rôle de l'option B.

## Option B : déploiements automatiques depuis GitHub

1. Ouvrez [dashboard.heroku.com](https://dashboard.heroku.com) dans votre navigateur mobile et touchez votre app. Pas encore d'app ? Touchez **New**, puis **Create new app**, et choisissez un nom et une région.
2. Ouvrez l'onglet **Deploy**.
3. Sous **Deployment method**, touchez **GitHub**, puis **Connect to GitHub**, et autorisez Heroku dans la fenêtre qui s'ouvre.
4. Dans **Search for a repository to connect to**, tapez `Active` (ou le nom de votre dépôt), touchez **Search**, puis **Connect** à côté du bon résultat.
5. Sous **Automatic deploys**, choisissez la branche **`master`**.
6. Cochez éventuellement **Wait for CI to pass before deploy**. Heroku attend alors que les vérifications GitHub Actions du dépôt réussissent : un build cassé n'est jamais mis en ligne.
7. Touchez **Enable Automatic Deploys**.
8. Pour le tout premier déploiement, faites défiler jusqu'à **Manual deploy** et touchez **Deploy Branch**.

Désormais, **chaque fusion dans `master` redéploie le site.** Vous pouvez suivre chaque build dans l'onglet **Activity**.

## Ce que fournit le dépôt

Heroku doit savoir comment construire et démarrer l'app. Le dépôt contient tout le nécessaire : il n'y a rien à configurer.

| Fichier | Rôle |
| --- | --- |
| `app.json` | Décrit l'app pour le bouton Deploy : nom, buildpack Node.js, variable `SITE_URL` facultative, un dyno web |
| `Procfile` | `web: npm start`, la commande que Heroku exécute pour démarrer le site |
| `package.json`, `heroku-postbuild` | Heroku exécute ce script après l'installation des dépendances. Il construit la bibliothèque, puis le site (`site/`) |
| `package.json`, `engines` | `"node": ">=20"` : la version de Node.js que Heroku installe |
| `.slugignore` | Exclut du bundle déployé les benchmarks, les exemples, les tests et les sources Rust |
| `site/server.js` | Le serveur de production, sans aucune dépendance |

`site/server.js` sert les pages pré-rendues et :

- redirige les visiteurs de `/` vers leur langue (`/en/`, `/fr/` ou `/es/`) d'après les réglages de leur navigateur ;
- renseigne l'adresse du site dans les URL canoniques et Open Graph : `SITE_URL` si elle est définie, sinon l'adresse de la requête ;
- compresse les réponses avec Brotli ou gzip ;
- met en cache pendant un an les fichiers dont le nom contient une empreinte (hash) et revalide les pages HTML ;
- écoute sur le port que Heroku fournit dans la variable d'environnement `PORT`.

> **Remarque :** Le build Node.js de Heroku ne dispose pas de compilateur Rust. Le module WebAssembly du site est compilé à l'avance et commité dans `site/src/wasm`. Si votre propre app utilise Rust, commitez de la même façon les fichiers `pkg` générés, ou construisez-les en CI.

## Domaine personnalisé

1. Dans votre app, ouvrez **Settings**, faites défiler jusqu'à **Domains** et touchez **Add domain**.
2. Saisissez votre domaine, par exemple `www.example.com`, et confirmez. Heroku affiche une **cible DNS** (DNS target) qui se termine par `herokudns.com`.
3. Chez votre registraire de domaine (la plupart ont un site adapté au mobile), ajoutez un enregistrement **CNAME** : nom `www`, valeur la cible DNS. Pour un domaine racine (`example.com`), utilisez un enregistrement ALIAS, ANAME ou « CNAME flattening » si votre fournisseur en propose un.
4. Dans **Settings**, **Config Vars**, définissez `SITE_URL` sur `https://www.example.com` pour que les URL canoniques utilisent toujours votre domaine.

Les modifications DNS peuvent prendre de quelques minutes à quelques heures. Pour le HTTPS sur votre propre domaine, utilisez **Configure SSL** dans la même section ; les certificats automatiques de Heroku ne sont pas proposés pour tous les types de dynos, vérifiez donc ce que comprend le vôtre.

## Logs

Dans votre app, touchez **More** (en haut à droite), puis **View logs**. Vous voyez en temps réel la sortie du serveur et chaque requête. Les logs de build se trouvent dans l'onglet **Activity** : touchez **View build log** à côté d'un build.

## Dépannage

### Le build échoue

Ouvrez **Activity**, puis le log du build en échec, et regardez les dernières lignes en rouge.

- `npm ci` signale que `package.json` et `package-lock.json` ne sont pas synchronisés : lancez `npm install` et commitez le fichier de verrouillage mis à jour.
- Une erreur TypeScript ou Vite : la même erreur se produit en local avec `npm run build` à la racine du dépôt et `npm --prefix site run build`.
- `cargo` ou `wasm-pack` introuvable : Heroku n'a pas Rust ; commitez les fichiers WebAssembly générés (voir la remarque ci-dessus).

### Application error (H10 App crashed)

La page affiche « Application error » et les logs contiennent `code=H10`. Le serveur a planté au démarrage. Cherchez l'erreur juste au-dessus dans les logs. Les causes habituelles :

- le serveur n'écoute pas sur `process.env.PORT` (Heroku signale alors `R10 Boot timeout` au bout de 60 secondes) ;
- le script `start` ou le `Procfile` pointe vers un fichier absent ;
- une dépendance nécessaire à l'exécution se trouve dans `devDependencies`, que Heroku supprime après le build.

`H14 No web processes running` signifie qu'aucun dyno ne tourne : ouvrez **Resources** et activez le dyno `web`.

### Mauvaise version de Node.js

Heroku choisit la version de Node.js d'après `engines.node` dans le `package.json` racine. Pour figer une version majeure, indiquez par exemple :

```json
{
  "engines": {
    "node": "22.x"
  }
}
```

Commitez, et Heroku l'utilisera au prochain déploiement. Le log de build affiche la version retenue vers le début.
