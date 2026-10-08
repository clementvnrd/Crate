# Releasing Crate

How a version of Crate reaches the Mac, how the app updates itself, and how to undo a bad release.
Behind it: CRA-199 (own signing key, macOS-only pipeline) and CRA-198. Decided on CRA-198: the
fork's first version is **1.0.0** (Q2), releases are built by **GitHub Actions** on a `v*` tag with the
Mac as the backup (Q4), and upstream's tags are to be deleted locally and no longer fetched (Q5, the
owner's command in CRA-198). Where releases are published
(Q1) **awaits the owner's confirmation**: this page describes the public `clementvnrd/crate-releases`
repository; if another location is chosen, `RELEASES_REPO` in `scripts/release/manifest.mjs`, the two
`endpoints` in the Tauri configs and the publish step of the workflow change.

## In one minute

| What | Where |
| --- | --- |
| Built apps (`.dmg`, `.app.tar.gz`, `.sig`) | GitHub releases of the public repository `clementvnrd/crate-releases`. The source code stays in the private `clementvnrd/Crate`. |
| Update manifests read by the app | `channels/production/latest.json` and `channels/staging/latest.json` on the `main` branch of `clementvnrd/crate-releases`. Every change of what the app is offered is a commit there. |
| Who follows which channel | **Crate** (`com.bbx-audio.crate`) follows `production`. **Crate Staging** (`com.bbx-audio.crate-staging`, purple STG icon, its own empty library) follows `staging`. They are two different apps: a build never switches channel. |
| Platform | Apple Silicon only (`darwin-aarch64`), CRA-122. |
| Versions | `X.Y.Z` is stable, `X.Y.Z-staging.N` is a pre-release (`yarn bump`, see below). |

The app checks its channel's manifest, compares the version with its own, downloads the `.app.tar.gz`
and **refuses it unless its signature matches the public key built into the app**
(`plugins.updater.pubkey` in `src-tauri/tauri.conf.json`). That key is the fork's own; upstream's key
(`5E32E4C59470B97D`) and upstream's release server are rejected by a test
(`scripts/release/manifest.test.ts`).

## Signing key

| Item | Location |
| --- | --- |
| Private key (minisign, password-protected) | `~/.tauri/crate-updater.key` on the Mac, mode `600` |
| Its password | login Keychain, item `crate-updater-signing` |
| Public key | `src-tauri/tauri.conf.json`, key ID `B84B4C7F52EE0B2E` |
| Offline backup | password manager entry "Crate updater signing key" (key file attached + password), CRA-202 |
| GitHub Actions (default build) | secrets `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` of `clementvnrd/Crate`, set by the owner with `gh secret set` (commands in CRA-202, step 8). A secret can be used, never read back: it is not a backup. |

Never commit the key, never paste it into an issue, a chat or a log. The release scripts read it
without printing it.

- **Key lost** (no backup): installed copies can never verify an update again. Generate a new pair
  (`yarn tauri signer generate -w ~/.tauri/crate-updater.key`), put the new public key in
  `tauri.conf.json`, release, and reinstall every copy **once by hand** from the new `.dmg`.
- **Key leaked**: rotate at once. Release N+1 carries the *new* public key in `tauri.conf.json` but is
  signed with the *old* key, published with `yarn release:publish --version <N+1> --allow-key <old key ID>`;
  from N+2 on, sign with the new key only. Installed copies follow the rotation by themselves.

## Release a version (default: GitHub Actions)

All commands run **from the repository root** (`~/Coding Projects/crate`), on an up-to-date `develop`.

1. Bump the version (it updates `package.json`, `src-tauri/Cargo.toml` and the Tauri configs):
   - the fork's first pre-release: `yarn bump major staging` (0.2.9 → 1.0.0-staging.1);
   - first pre-release of a later version: `yarn bump minor staging` (1.0.0 → 1.1.0-staging.1);
   - next pre-release: `yarn bump prerelease` (→ 1.0.0-staging.2);
   - stable: `yarn bump stage` (→ 1.0.0), then `yarn changelog:graduate 1.0.0`.
2. For a pre-release, move the `[Unreleased]` notes under the new version:
   `yarn changelog:prepare 1.0.0-staging.1`. The script moves the text verbatim (the "Personal fork —
   change log" section included); `node scripts/changelog.js notes <version>` prints what users will read.
3. Commit (`chore(release): 1.0.0-staging.1`), open the pull request, and wait for the owner's merge.
4. After the merge, tag the merged `develop` and push **that one tag**:
   `git fetch origin && git tag v1.0.0-staging.1 origin/develop && git push origin v1.0.0-staging.1`.
   Never `git push --tags`: it pushes every local tag to `origin`, and any `v*` tag GitHub receives can
   start a release build.
5. `.github/workflows/cd.release.yml` checks that `package.json` matches the tag, builds Crate for
   Apple Silicon on a macOS runner, signs the updater bundle with the secrets above, archives the build
   as a release of `clementvnrd/Crate` and publishes it with `scripts/release/publish.mjs` when the
   secret `RELEASES_REPO_TOKEN` exists (a fine-grained token with *Contents: read and write* on
   `clementvnrd/crate-releases` only). Follow the run with `gh run watch --repo clementvnrd/Crate`.
6. Check the channel: `curl -s https://raw.githubusercontent.com/clementvnrd/crate-releases/main/channels/staging/latest.json`
   (GitHub may serve the previous file for up to 5 minutes).

*Actions → Release → Run workflow* starts the same build by hand, for a version that already has its tag:
choose that tag (`v<version>`) under *Use workflow from*, because the run builds whatever is picked
there, and the default, `develop`, may already be ahead of the release.

Cost: about 25 to 30 minutes of macOS runner. On a private repository they are billed ×10, so roughly
250 to 300 of the 2,000 monthly free minutes per release; on a public repository they are free.

A published version is **never replaced**: if something is wrong, bump and release again.

## Release from this Mac (backup path)

When Actions is unavailable or out of minutes, the same release is built on the Mac with one command:
free, about 8 minutes, and the key never leaves the Mac. After step 3 above, on the merged `develop`:
`yarn release:local --dry-run` (checks), then `yarn release:local --publish`. Nothing is uploaded unless
the tree is clean, the changelog has the version's section, the archive really contains that version
(read from its `Info.plist`), and its signature verifies against the public key the installed apps trust.

`yarn release:publish --version <v> --bundle-dir <dir>` publishes an existing build, for example the
artifacts of a GitHub Actions run that built but did not publish.

## First install on a Mac without an Apple Developer certificate

Crate is ad-hoc signed (`signingIdentity: "-"`), not notarised. The first time only:

1. Download the `.dmg` from the release page, drag the app to *Applications*.
2. Open it once: macOS refuses. Open **System Settings → Privacy & Security**, scroll down, click
   **Open Anyway** next to the app, confirm.
   Terminal alternative: `xattr -dr com.apple.quarantine "/Applications/Crate Staging.app"`.

Updates installed by the app itself are not quarantined and open directly. Because an ad-hoc
signature changes with every build, macOS may ask again, once per update, for the Keychain item of
the Beatport session ("Always Allow").

## Rollback

The updater only ever moves **forward**: it installs a version greater than the installed one and
never downgrades. A rollback therefore has two parts: stop offering the bad version, then move the
installed copies forward to a good one.

| Situation | What to do |
| --- | --- |
| A bad version is published but not installed yet | Re-point the channel to the previous good version: `yarn release:publish --version <good> --repoint`. Then mark the bad release on GitHub as withdrawn: `gh release edit v<bad> --repo clementvnrd/crate-releases --prerelease --notes "Withdrawn: <reason>"`. |
| A bad version is installed and the app works | Fix forward: revert the faulty change on `develop`, bump, release. The app updates itself to the fixed version. |
| A bad version is installed and the app does not start | Download the previous good `.dmg` from the releases page and install it by hand over the bad one. If the bad version had migrated the database, the previous build will refuse it (next row). |
| The bad version migrated the database | An older build **refuses** to open a library migrated by a newer one: a "Crate could not start" alert says the library "was last opened by a newer version of Crate", and nothing is touched. Either install a fixed newer build, or quit Crate and restore the copy of `~/Library/Application Support/com.bbx-audio.crate/` (`crate.db` **and** `db.key` together) taken before the update. |

Before installing a stable update that changes the database, keep a copy of that folder: an
automatic pre-migration snapshot is planned (roadmap, *Reliable core*) but not built yet.

## Never

- Replace the assets of a published release: bump instead.
- Put a private key or a GitHub token in the app, the repository or a manifest.
- Point the production app at the staging manifest, or the reverse: they are different apps with
  different libraries.
- Accept an update offered by a build that still points at upstream (any `0.2.9` built before CRA-199):
  that is upstream's app, not the fork.
