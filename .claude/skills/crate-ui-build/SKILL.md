---
name: crate-ui-build
description: Protocole pour créer un composant ou une vue Crate, ou refondre une vue existante (Player, Pulse, Beatport, Upgrader…) — lecture du besoin, mode préserver/refondre, leviers de modernisation dans l'ordre, quatre états, exécution complète sans raccourci, contrôle avant livraison. À utiliser pour toute tâche qui écrit ou réécrit de l'interface.
argument-hint: "[vue ou composant] [objectif]"
---

# Construire ou refondre une interface Crate

Protocole adapté de taste-skill (lecture du besoin, protocole de refonte, contrôle final), de son skill `redesign-existing-projects` et de `full-output-enforcement` ([Leonxlnx/taste-skill](https://github.com/Leonxlnx/taste-skill), MIT). Les règles de fond sont dans `crate-design-system` et [DESIGN.md](../../../DESIGN.md).

## 1. Lire le besoin avant de coder

Écrire une ligne, en tête de travail :

> **Lecture :** <vue/composant> pour <usage DJ concret>, mode <création | refonte-préserver | refonte-structurelle>, cadrans <V/M/D> (référence 2/2/8), composants communs mobilisés : <liste>.

Exemple : *« Lecture : cartes KPI de Pulse pour lire d'un coup d'œil son temps d'écoute de la semaine, mode refonte-préserver, cadrans 2/2/8, composants : Text, Icon, Tooltip. »*

S'il y a une vraie ambiguïté qui change le résultat, poser **une seule** question. Sinon, déclarer la lecture et avancer.

## 2. Choisir le mode

- **Création** : nouveau composant ou vue. Partir du composant commun le plus proche et des motifs existants (ligne de piste, panneau, modale).
- **Refonte-préserver** (par défaut pour toute vue existante) : on garde la structure, les contenus, les libellés, les raccourcis ; on ramène l'apparence dans le système.
- **Refonte-structurelle** : la mise en page elle-même est cassée (débordements, hauteurs fixes, composants réinventés imbriqués). On restructure, en conservant le contenu et le comportement.

Si le mode n'est pas évident : « Faut-il garder la disposition actuelle de <vue>, ou la repenser ? »

## 3. Auditer avant de toucher

Pour une refonte, faire d'abord un audit rapide (skill `crate-ui-audit`, au minimum `yarn design:scan <fichiers> --details`) et noter :

- ce qui marche et doit rester (interactions signatures, ordre des informations, raccourcis) ;
- ce qui est hors système (tokens, rayons, verre, glow, tailles arbitraires, composants maison) ;
- les états manquants (chargement, vide, erreur) ;
- la lecture actuelle des cadrans : les vues des builds 40 à 57 sont souvent à « variance 6 / mouvement 6 / densité 4 », loin de la cible.

## 4. Ne jamais changer en silence

Sans accord explicite du propriétaire :

- libellés de navigation, noms de vues, raccourcis clavier ;
- commandes IPC, stores, clés de réglages, clés `localStorage` (`crate-theme`, `crate-accent`, `crate-font`, `crate-language`) ;
- clés i18n existantes (on en ajoute, on ne renomme pas sans migrer toutes les locales) ;
- l'ordre ou le sens des colonnes de la bibliothèque, le comportement du transport du Player ;
- le logo et les marques tierces.

## 5. Leviers, dans cet ordre

S'arrêter dès que l'objectif est atteint (environ 70 % de la valeur pour 40 % du risque avec les trois premiers) :

1. **Couleurs → tokens** : palette, hex et `dark:` remplacés (table de conversion de `crate-design-system`) ; couleurs propres à une vue ramenées à l'accent.
2. **Typographie** : `Text` et échelle Tailwind, fin des tailles arbitraires et des graisses extrêmes, `tabular-nums`.
3. **Surfaces et formes** : verre, halos, glow et dégradés retirés ; rayons et ombres du système ; un seul niveau d'encadrement.
4. **Composants communs** : remplacer les contrôles maison (`Button`, `IconButton`, `Select`, `Checkbox`, `Tooltip`, `Spinner`, `Modal`) ; extraire un composant partagé quand un motif est copié (D11 : `SegmentedControl`, `KeyBadge`).
5. **Mise en page** : hauteurs intrinsèques, `flex-1 min-h-0`, `min-w-0`, grilles `minmax(0, …)`, tenue à 1000×600.
6. **États** : chargement, vide, erreur, focus, survol, appui, désactivé.
7. **Mouvement** : retirer l'inutile, garder 150 à 200 ms sur des propriétés listées, `motion-reduce`.
8. **Remplacement complet d'un bloc** : seulement s'il est irrécupérable.

## 6. Écrire

- Svelte 5 (runes `$props`, `$state`, `$derived`, `$effect`), TypeScript, Tailwind 4 : suivre le style des fichiers voisins (tabulations, pas de point-virgule, imports `$lib/…` et `$shared/…`).
- Vérifier `apps/desktop/package.json` avant tout import ; **aucune nouvelle dépendance** (icônes, animation, composants) sans accord.
- Toute chaîne visible : `{$translate('…')}` et la clé dans `en.json` **et** `fr.json` (les 13 autres locales sont traitées à l'étape 12 du suivi, ne pas y inventer de traductions).
- Nouvelle couleur sémantique : token dans les deux thèmes de `style.css` (et `app.html` si l'écran de démarrage l'utilise).
- Commentaires en anglais dans le code, comme le reste du dépôt ; documents et CHANGELOG en français.

### Exécution complète

Un livrable partiel est un livrable cassé. Interdits : `// ...`, `// reste inchangé`, `// TODO`, « même principe pour les autres », squelette à la place d'une implémentation, un exemple suivi d'une description. Avant de rendre la main : recompter les éléments demandés (fichiers, composants, états, clés i18n) et vérifier qu'ils sont tous livrés. Si le travail doit être coupé, s'arrêter à une frontière propre (fin de fichier) et écrire exactement ce qui reste.

## 7. Contrôle avant livraison

Chaque case doit pouvoir être cochée honnêtement ; sinon ce n'est pas terminé.

- [ ] Ligne de lecture écrite, mode déclaré.
- [ ] `yarn design:scan <fichiers touchés>` : aucune nouvelle occurrence (comparer avant/après), les restantes justifiées.
- [ ] Aucune classe de palette, hex, `dark:`, `text-[Npx]`, `rounded-xl+`, `backdrop-blur`, glow, dégradé, `transition-all` introduits.
- [ ] Une seule couleur d'accent (`brand-*`), aucune couleur propre à la vue ; palettes de données via `shared/utils`.
- [ ] Composants communs utilisés ; aucun contrôle réinventé.
- [ ] Tout élément cliquable est un bouton avec nom accessible traduit ; focus visible ; clavier possible.
- [ ] Chargement, vide et erreur traités.
- [ ] Toutes les chaînes traduites en `en` et `fr` (« … », « », vouvoiement, casse de phrase en français).
- [ ] Chiffres en `tabular-nums`, nombres et dates via `Intl`.
- [ ] Tenue à 1000×600 et 1920×1080 ; pas de hauteur fixe sur une zone de contenu.
- [ ] Animations motivées, 150 à 200 ms, `motion-reduce` sur toute boucle.
- [ ] Vérification visuelle faite (skill `crate-visual-check`) ou explicitement signalée comme non faite.
- [ ] `yarn check:svelte` et `yarn test` verts ; `yarn format:check` et `yarn lint:check` sur les fichiers touchés.
- [ ] Suivi à jour : entrée CHANGELOG (section « Fork personnel », en français, identifiant entre crochets), case cochée dans `suivi/AVANCEMENT.md` avec note si partiel, `yarn suivi` relancé, DESIGN.md mis à jour si une règle a changé.

Ne pas commiter ni pousser : rendre la main avec la liste des fichiers modifiés, les identifiants traités et le message de commit proposé (`fix(ui): … [D11]`). La session principale relit et commite.
