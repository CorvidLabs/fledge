# Release channels and lifecycle-hook support

Audited 2026-10-10 against `main` @ `85d7c4e` (`chore: release v1.8.1 (#536)`).
Every row below has a receipt; anything not reproduced is marked **unverified**.

## The hook changes

| Change | Commit | In tags (`git tag --contains`) | In crates.io 1.8.0 (`435b7c5`)? |
|---|---|---|---|
| Hook progress to stderr, `--json` clean (#467) | `fab8c95` | v1.7.0, v1.7.1, v1.7.2 | yes |
| Hooked command plugins keep granted `exec`; `FLEDGE_REPO_ROOT` set (#520) | `c4abcb2` | **none** | yes (compare `c4abcb2...435b7c5` = ahead 17, behind 0) |
| Regression test really guards #520 (#521) | `fbf51a9` | **none** | yes |
| hi habit (#519) | `4196958` | **none** | yes |

`435b7c5` (`chore: release v1.8.0`) is on branch `0xleif/chore/release-1-8-0-fresh`, never on `main` and never tagged
(see the CHANGELOG note under v1.8.1). `main` is at Cargo version 1.8.1, also **untagged and unpublished**.

## Supported-version matrix

| Channel | Version served today | Hook support (#520) | Verified by |
|---|---|---|---|
| GitHub Releases (latest) | v1.7.2 (2026-07-27) | **No** — hooks silently dead | `gh release list`; downloaded `fledge-linux-x86_64`, probe below: `capabilities: null`, hook did not fire |
| `install.sh` / `install.ps1` | follows GitHub `releases/latest` → v1.7.2 | **No** | `install.sh` `latest_version()` reads `releases/latest` |
| GitHub Action (`action.yml`) | whatever tag you pin; docs example `v1.7.2` | **No** for any existing tag | README.md:46, action.yml:10 |
| Homebrew `CorvidLabs/tap/fledge` | 1.7.2 (formula last updated 2026-07-29, `f6c75be`) | **No** | formula read, not installed (no brew on box); linux sha `bebbd9b9…65ec` matches the downloaded v1.7.2 asset |
| crates.io `cargo install fledge` | 1.8.0 (max_version) | **Yes** | `cargo install --locked fledge --version 1.8.0 --root <tmp>`; probe: `exec: true`, hook fired with `FLEDGE_REPO_ROOT` |
| crates.io pinned older | 1.7.0 (no 1.7.1/1.7.2 on crates.io) | No | same isolated install, probe: hook did not fire |
| Source `main` / Nix flake | 1.8.1 (`flake.nix` version 1.8.1) | Yes (by ancestry; not run here — **unverified** at runtime) | `git merge-base --is-ancestor c4abcb2 origin/main` |

Bottom line: **the only published channel that has working lifecycle hooks is crates.io 1.8.0.**
Releases, install scripts, the Action and Homebrew all still serve 1.7.2.

## Probe (reproducible, isolated)

Temp `HOME`, `XDG_CONFIG_HOME`, `GIT_CONFIG_GLOBAL`, `FLEDGE_NON_INTERACTIVE=1`, a local plugin with
`[hooks] post_work_start = "bin/nudge"` and `[capabilities] exec = true`, then `fledge plugins install <dir> --json`
and `fledge work start probe` in a fresh git repo:

```
=== fledge 1.7.0   install capabilities: null   audit exec: False   hook.log: NONE
=== fledge 1.7.2   install capabilities: null   audit exec: False   hook.log: NONE
=== fledge 1.8.0   install capabilities: {exec: true, ...}   audit exec: True
                   hook.log: HOOK FIRED root=/tmp/fledge-chan/h-v1.8.0_bin_fledge/repo
```

Note: in non-interactive mode install prints `WARN Permissions auto-granted via --force` and grants `exec` without a prompt.

## hi integration

`CLAUDE.md` documents `brew install corvidlabs/tap/hi` / `cargo install human-intent`. `hi check` on `main`:
`197 criteria · 10 families · 10 files` — matches CLAUDE.md's "197 criteria across 10 families".
The hi habit (#519) itself is only in crates.io 1.8.0 and `main`, not in any tag.

## Stale claims fixed in this PR

- `docs/HLD.md` said `main` is at 1.7.2; `Cargo.toml` is 1.8.1.

## Remaining gaps (need a maintainer decision; not done here)

- Tag/release v1.8.1 from `main` so Releases, install scripts, the Action and Homebrew (via
  `post-release-formula.yml`) pick up #520. Not done: this audit may not create tags or publish.
- crates.io 1.8.1 not published; crates.io 1.7.1/1.7.2 never published.
- README's Action example still pins `v1.7.2` — correct for "latest release" today, but it pins a version with dead hooks. Update after v1.8.1 is tagged.
- macOS and Windows assets and the Nix build were not run (**unverified**).
