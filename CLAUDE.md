# Crate — règles pour les assistants de code

Fork personnel de `blackboxaudio/crate` (remote `upstream`), poussé sur le dépôt privé `origin`. Le propriétaire écrit en français : documents, CHANGELOG et messages en français.

## Suivi obligatoire

- Toute correction référence un identifiant du registre (`suivi/REGISTRE-DEFAUTS.md`) : `fix(zone): description [B12]`.
- Dans le **même commit** : cocher la case dans `suivi/AVANCEMENT.md`, lancer `yarn suivi`, ajouter l'entrée au `CHANGELOG.md` (section « Fork personnel »).
- Mettre à jour le README et les autres `.md` si le comportement documenté change.

## Règles techniques

- Rust desktop : la feature `desktop` n'est pas par défaut. `cargo test --features desktop`, `cargo clippy --features desktop -- -D warnings`.
- Tout module desktop-only est protégé par `#[cfg(feature = "desktop")]` (le build mobile utilise `--no-default-features --features mobile`).
- IPC Tauri : paramètres Rust en snake_case, clés JS en camelCase ; un rejet IPC est une **chaîne**, jamais un `Error`.
- Migrations : uniquement ajoutées en fin de `schema::get_migrations()`, jamais renumérotées après publication.
- **Jamais** de test qui ouvre `~/Library/...` ou les vraies bases Crate/Mixed In Key : `tempfile`/`:memory:` uniquement.
- **Mixed In Key est en lecture seule** : aucune écriture dans `Collection11.mikdb`.
- **Aucun secret** dans le code (identifiants, jetons, mots de passe).
- Frontend : thème via `[data-theme]` (pas de `dark:`), couleurs par tokens, composants communs (`Button`, `Modal`, `Tooltip`…), aucune chaîne en dur (clé i18n `en.json` + `fr.json`).
- Tests : `yarn test` (Vitest), `yarn check:svelte`.
