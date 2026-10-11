---
title: FAQ
description: Questions fréquentes sur active : autres frameworks Rust, JavaScript, SEO des applications monopages, hydratation, données asynchrones, routage, styles, taille.
---

Des réponses courtes aux questions les plus fréquentes, avec des liens vers les pages qui vont plus loin.

## Qu'est-ce qu'active, en une phrase ?

Une petite bibliothèque Rust pour construire le front-end de sites web : les composants sont des fonctions, l'état vit dans des handles `use_state`, chaque page reçoit des balises SEO complètes, et les parties interactives s'exécutent en WebAssembly, sur des pages rendues côté serveur, des sites statiques ou des applications côté client. Voir l'[Introduction](../introduction/).

## Pourquoi pas Leptos, Dioxus, Yew ou Sycamore ?

Ce sont d'excellents projets, plus vastes, et le meilleur choix dans certains cas :

- **Leptos** est un framework full-stack : réactivité à granularité fine, une macro `view!`, des fonctions serveur, un routeur et un mode îlots.
- **Dioxus** cible le web, le bureau et le mobile à partir d'une seule base de code, avec sa macro `rsx!`.
- **Yew** et **Sycamore** sont des frameworks de composants établis pour le navigateur, qui prennent en charge le rendu côté serveur.

`active` fait d'autres compromis :

- **Pas de macros.** Les vues sont de simples appels de fonctions : rust-analyzer, rustfmt et les erreurs du compilateur fonctionnent partout, et il n'y a aucune syntaxe à apprendre.
- **Le rendu côté serveur d'abord.** Du HTML complet sur chaque page, des îlots uniquement là où il en faut ; une page sans îlot n'envoie aucun code.
- **Le SEO intégré.** Titre, description, URL canonique, `hreflang`, Open Graph, Twitter et JSON-LD à partir d'un seul builder, avec une [fusion](../seo/#fusion) automatique entre mises en page et pages, sur le serveur comme dans les applications côté client.
- **Léger.** Une API courte, que vous pouvez lire en un après-midi ([Référence de l'API](../api/)), et aucune dépendance côté serveur.

Choisissez un framework plus vaste si vous avez besoin de fonctions serveur, de cibles bureau ou mobile, ou d'un large écosystème de composants prêts à l'emploi. La page [Benchmarks](../benchmarks/) compare les tailles et les temps de rendu.

## Dois-je écrire du JavaScript ?

Non. Composants, serveur, compilation et tests sont en Rust. Le navigateur reçoit deux morceaux de JavaScript que vous n'écrivez ni ne modifiez jamais : le chargeur généré par `wasm-bindgen`, et une ligne `<script type="module">` écrite par [`Document::client`](../documents/#charger-le-bundle-du-navigateur) pour le démarrer. Les API du navigateur sont accessibles depuis Rust grâce à [`web-sys`](https://docs.rs/web-sys) (voir [Événements](../events/#web-sys)).

## Une page fonctionne-t-elle avec JavaScript désactivé ?

Son contenu, oui : les pages rendues côté serveur et les pages statiques sont du HTML complet, si bien que le texte, les liens et les images fonctionnent sans aucun script. Les îlots restent tels que le serveur les a rendus, mais ne réagissent pas, puisque le démarrage du WebAssembly passe par le chargeur. Les formulaires dotés de vrais attributs `action` et `method` continuent de fonctionner comme des formulaires HTML classiques (voir [Événements](../events/#soumettre-un-formulaire)). Une application côté client, en revanche, a besoin des scripts pour afficher quoi que ce soit.

## Une application monopage est-elle bonne pour le SEO ?

Moins que des pages rendues côté serveur. Google sait exécuter des scripts et indexer du contenu construit dans le navigateur, mais plus tard et de façon moins fiable que du HTML simple, et la plupart des autres robots d'indexation et robots d'aperçu de liens (réseaux sociaux, messageries) ne lisent que le HTML qu'ils reçoivent. Une application côté client envoie une coquille : ils ne voient donc que le titre et la description de cette coquille.

Pour les pages publiques, utilisez des [pages rendues côté serveur avec des îlots](../islands/) ou un [site statique](../static-site/). Si vous avez besoin à la fois d'une application côté client et du SEO, rendez la première page de chaque URL sur le serveur (voir [Applications côté client](../client-app/#rendu-de-la-premiere-page-cote-serveur)) : le HTML contient alors le bon titre et les bonnes balises, et l'application prend le relais dans le navigateur.

## Qu'est-ce qu'une incohérence d'hydratation ?

Quand le navigateur hydrate un îlot, il s'attend à ce que le HTML du serveur corresponde à la vue qu'il construit lui-même. S'ils diffèrent, il affiche `active: re-rendering island (… mismatch)` dans la console et reconstruit l'îlot de zéro. La cause habituelle est une vue qui dépend de quelque chose dont un seul côté dispose (l'heure, un nombre aléatoire, des données d'une base de données) : transmettez à l'îlot tout ce dont il a besoin dans ses props. Détails et solutions : [Îlots, Incohérences d'hydratation](../islands/#incoherences-d-hydratation).

## Comment charger des données de manière asynchrone ?

- **Sur le serveur :** attendez d'abord vos appels à la base de données ou à l'API, puis construisez et rendez la vue en un seul appel synchrone, sans aucun `.await` entre les deux (le compilateur refuse un `State` conservé au-delà d'un `.await` dans un serveur multithread). Voir [Axum, Données asynchrones](../axum/#donnees-asynchrones).
- **Dans le navigateur :** lancez une future avec `wasm_bindgen_futures::spawn_local` depuis un gestionnaire ou un effet, et écrivez le résultat dans un état ; la vue se met à jour. Voir [Événements, Travail asynchrone dans les gestionnaires](../events/#travail-asynchrone-dans-les-gestionnaires).

Les composants eux-mêmes sont des fonctions synchrones.

## Y a-t-il un routeur ?

Sur le serveur, utilisez le routeur de votre framework web (les routes Axum dans le guide [Axum](../axum/)), avec une fonction de page par route. Dans une application côté client, `active::location()` donne le chemin courant sous forme d'état et gère les liens et le bouton Précédent, `active::query()` et `active::query_param("q")` donnent la chaîne de requête, et `active::navigate()` change de page depuis le code. Voir [Applications côté client, Routage](../client-app/#routage).

## Comment styliser les composants ?

Avec du CSS classique. Ajoutez des feuilles de style avec `Document::stylesheet("/static/app.css")`, définissez des classes avec `.class(...)`, activez-les ou désactivez-les avec `.class_if(...)`, et utilisez `.style(...)` pour des styles en ligne ponctuels ou des variables CSS (`.style(format!("--progress:{p}%"))`). Tout outil CSS qui produit un fichier `.css` fonctionne en parallèle. Les blocs `<style>` en ligne peuvent aller dans `Document::head(...)`.

## Quelle est la taille du WebAssembly ?

Environ 22 ko avec gzip (57 ko bruts) pour une page avec un îlot compteur, et 47 ko avec gzip pour l'application de liste de tâches complète de `examples/spa`, compilés avec le profil optimisé pour la taille et servis compressés. Le contenu statique n'ajoute rien, et chaque îlot supplémentaire n'ajoute que son propre code. Chiffres, réglages et conseils : [Performances](../performance/#tailles).

## Combien coûte l'hébergement sur Heroku ?

Heroku n'a pas d'offre gratuite. Au moment de la rédaction, l'option la moins chère, les dynos Eco, coûte environ 5 $ par mois (ils se mettent en veille après 30 minutes sans trafic), et un dyno Basic environ 7 $ par mois. Voir [Déployer sur Heroku, Coûts](../deploy-heroku/#couts). Un [site statique](../static-site/) peut être hébergé gratuitement sur GitHub Pages, Netlify ou Cloudflare Pages.

## Comment s'appelle le crate ?

`active`, sur [crates.io](https://crates.io/crates/active), avec la documentation sur [docs.rs/active](https://docs.rs/active). Tant que la première version n'y est pas publiée, dépendez du dépôt : `active = { git = "https://github.com/maxlestage/Active" }`. La page [Publier le crate](../publishing/) explique comment se font les publications.

## Quelle version de Rust faut-il ?

Rust 1.85 ou plus récent : le crate utilise l'édition 2024, déclarée avec `rust-version = "1.85"`. Mettez à jour avec `rustup update`.

## Puis-je l'utiliser avec Actix Web, Rocket ou un autre serveur ?

Oui. `Document::render()` renvoie une `String` : envoyez-la avec le type de contenu `text/html; charset=utf-8` depuis n'importe quel framework. Le guide [Axum](../axum/) montre une intégration, et `examples/hello` n'utilise que la bibliothèque standard. Un `Seo` peut aussi s'utiliser seul dans n'importe quel template.

## Comment voir les messages de panique dans le navigateur ?

Avec le profil `wasm-release`, une panique arrête le module et la console n'affiche qu'une erreur générique. Pendant le développement, installez un hook de panique qui affiche le message : ajoutez `console_error_panic_hook = "0.1"` aux dépendances `wasm32` et appelez `console_error_panic_hook::set_once()` au début de votre fonction `#[wasm_bindgen(start)]`. Une compilation de débogage donne aussi de meilleurs messages (voir [Compiler le client](../client-build/#compilations-plus-rapides-pendant-le-developpement)).

## Est-ce prêt pour la production ?

`active` est jeune (version 0.1). L'API est petite et testée, et ce site web tourne avec, mais elle peut changer avant la 1.0. Épinglez la version dans `Cargo.toml` et lisez les notes de version lors des mises à jour. Les tickets et les contributions sont les bienvenus sur [GitHub](https://github.com/maxlestage/Active).

## Sous quelle licence est-il distribué ?

`active` est un logiciel propriétaire : © Maxime Nathan Lestage, tous droits réservés. Son code source est publié sur crates.io et GitHub pour que vous puissiez le lire, l'auditer et le compiler, et vous pouvez l'utiliser librement comme dépendance de vos propres programmes, y compris commerciaux. Le modifier ou redistribuer son code source demande l'accord écrit de l'auteur. Les conditions complètes sont dans le fichier [LICENSE](https://github.com/maxlestage/Active/blob/master/LICENSE) du dépôt.
