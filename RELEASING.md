# Releasing

How a release of Wi24rd Huginn and Muninn is made. The app is built, signed and notarised on the maintainer's own Mac with `tools/signer.sh`. Signing keys and the notarisation profile never leave that Mac, and nothing is built or signed in GitHub Actions.

The order matters: the release is created as a **draft**, the finished file is attached, and only then is it published. A published release is not changed afterwards; a mistake is fixed with a new version.

## 1. Prepare

1. The version is the same in `package.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock` and `src-tauri/tauri.conf.json`, and `CHANGELOG.md` has a section for it.
2. The user guide and both READMEs name the new version, including the file name and the checksum in "Verify your download" (the checksum is filled in at step 4).
3. All tests pass locally (see [CONTRIBUTING.md](CONTRIBUTING.md)), the commit is pushed to `main`, and CI is green for that commit.

## 2. Build, sign and notarise (local)

```
sh tools/signer.sh udgiv
```

The script refuses to run with uncommitted changes or a commit that is not on `origin`. It builds, signs with the Developer ID, sends the app to Apple's notarisation, staples the answer and runs its own checks. The result is in the release folder outside the repository:

- `Huginn-Muninn-<version>.zip`: the file that is published,
- `Huginn-Muninn-<version>.txt`: the version and the commit it was built from.

## 3. Check the finished file

Check the zip that will be published, not the build folder. Unpack it to a temporary folder and run:

```
sh tools/signer.sh tjek "<temporary folder>/Huginn and Muninn.app"
```

It must end without errors: valid signature, `source=Notarized Developer ID`, a stapled ticket, and no user name or host name in the bundle.

## 4. Checksum

Compute the SHA-256 of the final, signed zip, and of nothing else:

```
shasum -a 256 Huginn-Muninn-<version>.zip
```

Write the value into the release notes and into "Verify your download" in `README.md` and `README.da.md`. After the upload, GitHub shows a `sha256:` value next to the file under **Assets**; it must be the same.

## 5. Tag and draft release

1. Tag the commit the file was built from (the commit in the `.txt` file) as `v<version>` and push the tag.
2. On GitHub: **Releases › Draft a new release**, choose the tag, write the notes (what is new, how to install, requirements, the SHA-256, and the trademark line), attach the zip, and choose **Save draft**. Do not publish yet.
3. Download the zip from the draft and run `shasum -a 256` on the downloaded file. It must match step 4.
4. Read the draft once more: the right tag, one file, the right checksum.

## 6. Publish

Choose **Publish release**. Then check the public page: the release is marked **Latest**, the download link in the README leads to it, and the `sha256:` under **Assets** matches.

## After a release

- A published release and its tag are never edited, moved or replaced. If something is wrong, publish a new version and say so in its notes.
- If a file must be withdrawn, mark the release as a pre-release with a warning at the top of the notes, and publish the fix as a new version.

## Considered, not in use

- **Immutable releases** (a repository setting): GitHub then refuses to change the files and the tag of a published release, and adds a signed release attestation that can be checked with `gh release verify`. It fits the draft-first order above. It is not turned on yet; when it is, a published file can no longer be replaced, and the tag name of a deleted release cannot be used again.
- **Artifact attestations** (build provenance from GitHub Actions): they say that a file was built by a given workflow. This app is built and signed locally, so such an attestation would not describe how the file was made. Apple's notarisation and the published checksum are what this project offers instead.
