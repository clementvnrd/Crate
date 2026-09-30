# Signatures d'interface « générée par IA » — version app desktop dense

Source : [Leonxlnx/taste-skill](https://github.com/Leonxlnx/taste-skill) (skills `taste-skill` v2, `redesign-skill`, `minimalist-skill`, `image-to-code-skill`), licence MIT, © 2026 Leonxlnx. Taste-skill vise les landing pages et déclare lui-même les tableaux de bord et interfaces denses hors de son périmètre ; ce document n'en garde que ce qui s'applique à une app comme Crate, et le traduit dans ses règles.

Ces motifs sont les traces typiques d'une interface produite par un assistant sans direction artistique. Les builds 40 à 57 de Crate en contiennent beaucoup (Player, Pulse, Beatport, Upgrader, Duplicate Killer). Un audit les signale ; une refonte les retire.

## Surfaces et couleur

- **Verre dépoli** : `backdrop-blur-*` + fond semi-transparent (`bg-surface-1/70`). Dans Crate : surface opaque de l'échelle.
- **Halos et taches floues décoratives** : `absolute … rounded-full bg-emerald-500/10 blur-2xl`. À supprimer.
- **Glow** : `shadow-[0_0_8px_rgba(…)]`, `drop-shadow-[0_0_3px_…]`, `shadow-emerald-500/10`. À supprimer.
- **Une couleur par carte** (émeraude, violet, ambre, cyan pour quatre KPI). Une seule couleur d'accent, celle de l'utilisateur ; la distinction vient de l'icône et du libellé.
- **Dégradés** sur texte, boutons ou fonds (`bg-gradient-to-r from-cyan-400 to-sky-500`). À supprimer (sauf logo de marque officiel).
- **Noir pur `#000` et blanc pur sur fond sombre** en dur : utiliser l'échelle de surfaces et de texte.
- **Gris chauds et froids mélangés** (`stone` + `zinc`) : une seule famille, celle des tokens.
- **Accent sursaturé** et « violet IA » par défaut : l'accent est celui choisi par l'utilisateur, jamais imposé par une vue.

## Typographie

- **`font-black` / `font-extrabold` sur les chiffres** « pour faire premium » : `font-semibold` suffit, la hiérarchie vient de la taille et de la couleur.
- **Titres qui crient** (`text-3xl`+ dans une vue d'outil) : l'échelle de Crate s'arrête à `text-lg` pour les titres de vue ; un chiffre clé peut aller jusqu'à `text-2xl`.
- **Micro-textes arbitraires** (`text-[10px]`, `text-[9px]`) : illisibles et hors échelle ; `text-xs` minimum.
- **Étiquettes en capitales espacées au-dessus de chaque bloc** : une par zone au plus, via `Text variant="header-3|header-4"`.
- **Chiffres en chasse proportionnelle** dans des colonnes : `tabular-nums`.
- **Points de suspension ASCII** « ... » et guillemets droits : `…`, « », “ ”.

## Mise en page

- **Carte dans une carte dans une carte** : un seul niveau d'encadrement ; le reste par espacement et séparateurs.
- **Toutes les données dans des cartes** : à haute densité, les chiffres respirent dans une mise en page simple séparée par des filets de 1 px.
- **Rayons mélangés** (`rounded-2xl` pour les cartes, `rounded-md` pour les boutons, `rounded-xl` pour les icônes) : un seul système (voir DESIGN.md).
- **Hauteurs fixes** (`h-[225px]`) qui coupent le contenu à 1000×600, ou laissent de grands vides à 1920×1080.
- **Calculs de largeur** (`w-[calc(33.333%-2px)]`) là où une grille suffit.
- **`z-[9999]`** et z-index au hasard.
- **Éléments collés au bord ou mal alignés** : aligner les éléments partagés (titres, valeurs, actions) entre colonnes voisines ; correction optique de 1 à 2 px pour les icônes à côté du texte.

## Mouvement

- **Cartes qui se soulèvent au survol** (`hover:-translate-y-0.5`, `group-hover:scale-110`) : pas dans un outil ; survol de couleur.
- **`transition-all duration-300`** partout : lent et coûteux ; 150 à 200 ms sur des propriétés listées.
- **Animations infinies décoratives** (`animate-pulse` sur un logo, un badge, un point « live ») : réservées aux chargements, avec `motion-reduce`.
- **Animation sans raison** : chaque animation doit signaler un état (retour d'action, changement d'état, apparition). Sinon, la retirer.

## Contenu

- **Chaînes en dur** (souvent en français dans du code censé être anglais : « Temps d'Écoute ») : clé i18n.
- **Chiffres faussement précis ou inventés** dans des maquettes ou états vides : données réelles ou libellé clair d'exemple.
- **Clichés de rédaction** : « Découvrez », « Boostez », « Seamless », « Next-gen », exclamations, « Oups ! ». Texte simple et précis.
- **Libellés « PRO », badges de version, points colorés décoratifs** devant les éléments de navigation : à retirer (le point coloré n'est permis que pour un état réel, comme « connecté »).
- **Emojis** dans l'interface : remplacer par une icône de `Icon.svelte`.
- **Title Case sur tout en français** : casse de phrase en français.

## États oubliés

- **Seul l'état nominal est dessiné** : chargement (squelette à la forme du contenu), vide (message + action), erreur (message + comment s'en sortir) manquent.
- **Pas d'état actif dans la navigation** : l'élément courant doit se voir (`bg-brand-muted text-brand-primary`).
- **Pas de retour à l'appui** ni de focus visible.
- **Actions mortes** : boutons sans effet ou menant nulle part ; les masquer ou les désactiver avec une explication.

## Code

- **Soupe de `div`** : éléments sémantiques (`button`, `nav`, `header`, `main`, `section`, `table`).
- **Styles en ligne mêlés aux classes** (hors couleurs de données appliquées en `style=`).
- **Imports hallucinés** : vérifier `apps/desktop/package.json` avant d'importer ; ne pas ajouter de bibliothèque d'icônes, d'animation ou de composants.
- **Code mort commenté** laissé derrière soi.

## Licence de la source

```
MIT License

Copyright (c) 2026 Leonxlnx

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
