# Contributing

Thank you for wanting to help. Wi24rd Huginn and Muninn is a spare-time open source project with one maintainer, so this page is short and says honestly how things work here.

## Before you start

- **A bug or a wish:** open an [issue](https://github.com/wi24rd-com/huginn-muninn/issues/new/choose), or write through the [contact form](https://wi24rd.com/en/contact/) on wi24rd.com. For anything larger than a small fix, please open an issue before you write code, so you do not spend time on something that will not be taken in.
- **A security problem:** report it privately, see [SECURITY.md](SECURITY.md). Never in a public issue.
- **Never share** a Cloudflare API Token, zone ID, account ID or real domain names in issues, pull requests, tests or screenshots. Test data uses `wi24rd.com` and made-up subdomains only.

## What the app must stay

These are not up for discussion in a pull request:

- It only **reads** from Cloudflare.
- It talks only to `api.cloudflare.com`. No telemetry, no Wi24rd server.
- Everything the user sees exists in both Danish and English.
- No new dependency without a good reason. State its name, version and licence.

## Set-up

You need Rust (installed with [rustup](https://rustup.rs), 1.90 or newer), Node 20 or newer and, for the app itself, macOS on Apple silicon.

```
npm install
npm run tauri dev
```

On macOS, `sh tools/signer.sh proeve` makes a debug build signed with your own Developer ID, without notarisation.

## Tests

Run these before you open a pull request. The same commands run in CI (`.github/workflows/ci.yml`).

```
cd src-tauri && cargo test --locked && cd ..   # the Rust tests
npm run test:ui                                # the UI smoke test (jsdom; uses Chrome for the contrast checks if it is installed)
npm run test:signer                            # macOS only: the signing script with stubs; signs nothing
npm run kit:tjek                               # the copies in ui/assets/kit2/ match the Wi24rd UI Kit 2 package
cd src-tauri && cargo fmt --check && cd ..     # formatting
```

`npm run test:ui` reads a file from Cargo's registry, so run `cargo fetch --locked` in `src-tauri/` once first. On a slow machine, `HM_SOV=2 npm run test:ui` doubles the waits.

`npm run test:foto` takes the screenshots for the user guide (it needs Chrome). It is not part of CI.

The files in `ui/assets/kit2/` are copies of the Wi24rd UI Kit 2 package and are never edited by hand.

## Pull requests

- Keep a pull request to one thing, and say how you tested it.
- Pull requests are reviewed on GitHub but **not merged on github.com**. The maintainer brings the change into `main` locally and pushes it; your pull request is then closed with a reference to the commit, and you are credited in it.
- There is no promise about when, or whether, a pull request is looked at or taken in.
- By contributing you agree that your contribution is licensed under [Apache-2.0](LICENSE), like the rest of the project.

## Dependency updates

Dependabot opens grouped pull requests for Cargo, npm and GitHub Actions once a month. They are notices, not something to merge: the maintainer makes the update locally, runs the tests, and pushes. Dependabot closes its pull request when it sees the update on `main`.

Actions in `.github/workflows/` are pinned to a full commit SHA with the version in a comment. A new SHA is taken from the action's own repository (`git ls-remote https://github.com/<owner>/<action> refs/tags/<version>`), never from memory or from a fork.

## Releases

Releases are built, signed and notarised on the maintainer's own Mac, never in GitHub Actions. The steps are in [RELEASING.md](RELEASING.md).
