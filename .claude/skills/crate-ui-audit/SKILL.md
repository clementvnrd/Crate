---
name: crate-ui-audit
description: Audit de design et d'accessibilité d'une vue, d'un dossier ou de composants Svelte de Crate — tokens, composants communs, Web Interface Guidelines (Vercel), signatures « IA » (taste-skill), i18n, contraste. Produit des constats file:line reliés au registre des défauts. À utiliser pour « audite / relis / vérifie le design de… », avant une refonte, ou pour relire un diff d'interface.
argument-hint: "[fichier | dossier | nom de vue]"
---

# Audit d'interface Crate

Un audit **constate**, il ne corrige pas : aucune modification de fichier sauf demande explicite. Cible : `$ARGUMENTS` (si vide, demander quelle vue ou quel dossier ; pour un diff, `git diff --name-only -- apps/desktop/src`).

## Déroulé

1. **Périmètre.** Lister les fichiers `.svelte` concernés et, pour une vue, les composants enfants qu'elle rend. Noter la vue dans l'app (Player, Bibliothèque, Pulse, Beatport, Réglages…).
2. **Mesure automatique.** `yarn design:scan <chemins> --details`. Chaque ligne est un constat candidat ; vérifier dans le code avant de la reporter (la règle `hardcoded-text` est heuristique, les logos de marque sont des exceptions).
3. **Lecture.** Lire chaque fichier en entier. Contrôler dans l'ordre :
   - règles de [DESIGN.md](../../../DESIGN.md) et du skill `crate-design-system` (tokens, rayons, élévation, composants communs, typographie, états) ;
   - [Web Interface Guidelines adaptées](references/web-interface-guidelines.md) (accessibilité, focus, formulaires, animation, typographie, contenus, performance, langue, rédaction) ;
   - [signatures « IA » en app dense](references/anti-slop-produit.md) ;
   - i18n : toute chaîne visible passe par `{$translate('clé')}` (import `translate` depuis `$shared/i18n`) et existe dans `en.json` et `fr.json` ;
   - robustesse de mise en page : `min-w-0`, `minmax(0, …)`, pas de hauteur fixe, 1000×600.
4. **Vérification visuelle** si le harnais est disponible (skill `crate-visual-check`) : clair et sombre, deux accents, 1000×600 et 1400×900, contraste mesuré. Sinon, le dire dans le rapport (« non vérifié visuellement »).
5. **Rapport** au format ci-dessous, puis **plan de correction** ordonné.

## Gravité

- **Bloquant** : invisible ou inutilisable (contraste < 3:1, contenu coupé à 1000×600, action inaccessible au clavier, élément cliquable sans nom, chaîne non traduite visible en anglais).
- **Majeur** : hors système (palette, hex, `dark:`, composant réinventé, rayon ou élévation hors système, couleur propre à une vue, état manquant).
- **Mineur** : finition (`…`, `tabular-nums`, `transition-all`, graisse, taille arbitraire isolée, rédaction).

## Rattachement au registre

Relier chaque constat à un identifiant de `suivi/REGISTRE-DEFAUTS.md` quand il existe :

| Identifiant | Sujet |
| --- | --- |
| D3 | Thème clair cassé, contrastes insuffisants |
| D7 | Tableau Beatport trop étroit à 1000 px |
| D10 | Accessibilité (noms, `div` cliquables, clavier, focus, `motion-reduce`) |
| D11 | Composants réinventés au lieu des communs |
| L1 | Chaînes non traduites |
| L3 | Nombres et unités mal formatés en français |

Un défaut sans identifiant est marqué `[nouveau]` et proposé pour le registre (catégorie, fichier, correctif, effort XS/S/M) : c'est au propriétaire de l'y ajouter ou à la session principale de le faire avec son accord.

## Format du rapport

Constats groupés par fichier, une ligne chacun, chemin cliquable, sans préambule :

```text
## apps/desktop/src/lib/components/stats/StatsKpiCards.svelte

StatsKpiCards.svelte:40 — majeur [D11] carte en verre dépoli (bg-surface-1/70 + backdrop-blur-xl + halo) → bg-surface-1, rounded-lg, sans halo
StatsKpiCards.svelte:47 — bloquant [L1] « Temps d'Écoute » en dur → clé stats.kpi.listeningTime (en + fr)
StatsKpiCards.svelte:48 — mineur font-black → font-semibold, tabular-nums

## apps/desktop/src/lib/components/common/Button.svelte

✓ conforme
```

Puis :

1. **Synthèse** : nombre de constats par gravité, compteurs `design:scan` avant correction.
2. **Plan de correction** : lots ordonnés (d'abord bloquants, puis tokens et composants communs, puis finition), chacun avec les identifiants concernés et une estimation.
3. **Ce qui n'a pas pu être vérifié** (rendu visuel, interactions clavier réelles).

Écrire le rapport en français. Ne pas reformuler les règles dans le rapport : citer la correction attendue.
