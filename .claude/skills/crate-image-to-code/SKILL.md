---
name: crate-image-to-code
description: Transformer une image en interface Crate fidèle — capture d'écran d'un bug visuel, maquette, image générée, capture d'une autre app (Rekordbox, Serato, Spotify…) ou fichier DESIGN.md d'inspiration (awesome-design-md). Analyse systématique de l'image, traduction dans les tokens et composants de Crate, implémentation sans dérive, comparaison côte à côte. À utiliser dès qu'une image ou une référence visuelle accompagne la demande.
argument-hint: "[chemin de l'image ou référence] [vue cible]"
---

# De l'image au code, dans le système de Crate

Adapté de `image-to-code` ([Leonxlnx/taste-skill](https://github.com/Leonxlnx/taste-skill), MIT) : son idée centrale — l'image est une **spécification** qu'on analyse avant de coder, puis on implémente sans dériver vers un gabarit générique — vaut pour Crate. Sa partie « générer d'abord des maquettes de landing page » ne s'applique pas : Crate a déjà son système, l'image sert à **décider d'une disposition**, pas d'une palette.

## Sources d'image possibles

| Source | Ce qu'on en prend | Ce qu'on n'en prend jamais |
| --- | --- | --- |
| Capture de Crate (bug, avant/après) | L'état exact à corriger ou à atteindre | — |
| Maquette ou croquis du propriétaire | Disposition, hiérarchie, contenus, espacements | Couleurs et polices si elles contredisent le système (le signaler) |
| Capture d'une autre app DJ ou musicale | Idée d'organisation de l'information, densité, motif d'interaction | Palette, police, logo, style de marque |
| DESIGN.md d'inspiration | Principes (hiérarchie, densité, retenue) | Tokens bruts : on les traduit en tokens Crate |

## 1. Analyser l'image comme une spécification

Regarder l'image (outil `Read` sur le fichier) et remplir ce tableau **avant** de coder :

| Axe | À relever |
| --- | --- |
| Rôle | Quelle vue, quelle zone, quelle tâche du DJ |
| Priorité visuelle | Ce que l'œil voit en premier, deuxième, troisième |
| Structure | Grille, colonnes, alignements, zones fixes et défilantes |
| Texte | Tout le texte lisible, mot pour mot (il devient des clés i18n) |
| Typographie | Rapports de taille et de graisse, lignes, troncatures, chiffres alignés |
| Espacements | Entre titre et contenu, entre lignes, marges internes, gouttières ; logique, pas pixels |
| Composants | Boutons (plein, fantôme, icône), badges, champs, séparateurs, menus |
| Couleur | Où la couleur porte une information et où elle est décorative |
| États | Survol, sélection, lecture en cours, vide, chargement, erreur visibles ou suggérés |
| Densité | Nombre d'éléments par écran, comparé à la bibliothèque de Crate |
| Zones floues | Ce que l'image ne dit pas |

Pour une capture d'une autre app : noter aussi **pourquoi** elle fonctionne (ex. « le BPM et la tonalité sont en colonne fixe à droite, lisibles sans chercher »).

## 2. Traduire dans Crate

Écrire la table de correspondance :

| Dans l'image | Dans Crate |
| --- | --- |
| Fond, panneaux, contrôles | `surface-0` / `surface-1` / `surface-2` selon l'élévation |
| Couleur d'accent de la référence | `brand-primary` (l'accent choisi par l'utilisateur) |
| Couleurs d'information (tonalité, énergie, état) | palettes de données (`getCamelotColor`, `getEnergyInfo`) ou `danger/warning/success` |
| Police de la référence | police de l'utilisateur (`var(--font-family)`), échelle `Text` |
| Bouton, champ, menu, modale | `Button`, `Input`, `Select`, `ContextMenu`, `Modal`… |
| Rayons et ombres | système de DESIGN.md |
| Icônes | `Icon.svelte` (ajouter celles qui manquent) |

**Ordre de priorité en cas de conflit** : règles de Crate > fidélité à l'image > commodité d'implémentation. Tout écart imposé par le système est listé pour le propriétaire (« la maquette utilise un fond violet ; j'ai gardé l'accent utilisateur »).

## 3. Implémenter sans dériver

- Garder la disposition, l'ordre des informations, les rapports de taille et le rythme d'espacement de l'image.
- Ne pas « simplifier » en gabarit générique, ne pas resserrer un espacement généreux ni aérer une densité voulue, ne pas réintroduire des cartes imbriquées que l'image n'a pas.
- Zone floue : 1) garder le langage visible, 2) garder la logique d'espacement, 3) garder la famille de composants, 4) choisir la version la plus simple et fidèle — ne pas combler par un défaut générique.
- Tout le reste suit `crate-ui-build` (i18n, états, exécution complète, contrôle final).

## 4. Comparer

Si le harnais est disponible (skill `crate-visual-check`) : capturer le résultat dans la même taille de fenêtre que l'image, les regarder côte à côte, lister les écarts (structure, hiérarchie, espacements, texte) et itérer au plus trois fois. Sinon, décrire les écarts attendus et demander une capture au propriétaire.

## Bibliothèque d'inspiration

[awesome-design-md](https://github.com/VoltAgent/awesome-design-md) (MIT, © 2026 VoltAgent) rassemble 73 fichiers DESIGN.md extraits de sites réels. Ils décrivent des **marques**, pas des outils : à lire pour des principes, jamais pour copier des valeurs. Ceux qui parlent à une app comme Crate :

| Fichier | Intérêt pour Crate |
| --- | --- |
| `spotify` | Musique, sombre, pochettes, listes de pistes |
| `linear.app` | Produit dense et sombre, échelle de surfaces, un seul accent rare |
| `raycast` | App desktop macOS, clavier d'abord, listes compactes |
| `superhuman` | Densité et vitesse, raccourcis visibles |
| `warp` | Outil desktop sombre, lisibilité des données |
| `elevenlabs` | Audio, formes d'onde, sombre |
| `sentry`, `posthog` | Tableaux de bord de données, graphiques, statistiques |

Lecture à la demande (ne pas les copier dans le dépôt) :

```bash
curl -s https://raw.githubusercontent.com/VoltAgent/awesome-design-md/main/design-md/spotify/DESIGN.md
```

Pour un format de référence du fichier, voir la structure de [DESIGN.md](../../../DESIGN.md) de Crate, calquée sur ces fichiers (frontmatter de tokens, puis vue d'ensemble, couleurs, typographie, mise en page, élévation, formes, composants, à faire/à éviter, responsive, guide d'itération, lacunes).
