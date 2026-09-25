# Suivi du projet

Ce dossier trace tout le travail de remise en état du fork personnel de Crate, depuis l'audit du 25 septembre 2026.

| Fichier | Rôle | Mis à jour |
| --- | --- | --- |
| [AVANCEMENT.md](AVANCEMENT.md) | Cases à cocher par défaut et par étape, actions du propriétaire, journal des sessions | À chaque correction |
| [../CHANGELOG.md](../CHANGELOG.md) | Journal détaillé de chaque modification (ce qui change pour toi, et pourquoi) | À chaque correction |
| [REGISTRE-DEFAUTS.md](REGISTRE-DEFAUTS.md) | Les ~180 défauts trouvés à l'audit, avec leur correctif prévu | Figé (référence) |
| [RAPPORT-AUDIT.md](RAPPORT-AUDIT.md) | Situation, points forts, vision, plan par phases, décisions | Figé (référence) |
| [historique/SYNTHESE_DISCUSSION.md](historique/SYNTHESE_DISCUSSION.md) | Synthèse laissée par l'assistant précédent (builds 36 à 57) | Figé (archive) |

## Méthode de travail

1. On suit l'**ordre de réparation** du registre (12 étapes) : d'abord ce qui peut détruire des données, puis ce qui est faux, puis la finition.
2. **Un défaut (ou un petit groupe lié) = un commit**, avec l'identifiant entre crochets dans le message : `fix(stats): dédoublonnage des écoutes Spotify [C10]`.
3. Le **même commit** met à jour le code, ses tests, la case dans `AVANCEMENT.md` (puis `yarn suivi`) et une entrée dans le `CHANGELOG.md`.
4. Chaque correction est **vérifiée** avant d'être cochée : tests Rust (`cargo test --features desktop`), Vitest (`yarn test`), et vérification manuelle ou dans le harnais navigateur quand c'est visuel.
5. Les tests ne touchent **jamais** les vraies bases (`~/Library/...`) : bases temporaires uniquement.
6. Poussé sur GitHub après chaque étape (ou plus souvent).

## Retrouver l'historique d'un défaut

```bash
git log --oneline --grep "\[C11\]"
```
