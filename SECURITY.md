# Security policy

Wi24rd Huginn and Muninn is a spare-time open source project with one maintainer. This page says how to report a security problem and what you can expect.

## Reporting a vulnerability

Please report it privately. Do not open a public issue for a security problem.

1. Go to the repository's **Security** tab and choose **Report a vulnerability** ([direct link](https://github.com/wi24rd-com/huginn-muninn/security/advisories/new)). The report is visible only to you and the maintainer.
2. If you do not use GitHub, write through the contact form on wi24rd.com: [English](https://wi24rd.com/en/contact/) · [Dansk](https://wi24rd.com/kontakt/). The message is not public.

A useful report says:

- the version of the app (shown in the footer and under **About**) and your macOS version,
- what you did, what happened, and what you expected,
- why you think it is a security problem.

**Never include** your Cloudflare API Token, zone IDs, account IDs or the names of your domains, and remove them from screenshots and logs. If an API Token may have been exposed, delete it at Cloudflare first (My Profile › API Tokens) and create a new one.

## What to expect

- Reports are read and handled by one person in his spare time. There is no guaranteed response time and no bug bounty.
- You will get an answer when the report has been read, and again when there is a fix or a decision: in the advisory thread, or by e-mail if you used the contact form.
- Fixes are released as a new version. Only the latest release is maintained; older versions get no fixes.
- If you want credit in the release notes, say so in the report.

## Scope

In scope: the app in this repository and the files published under [Releases](https://github.com/wi24rd-com/huginn-muninn/releases).

Not in scope: Cloudflare's own services and API (report those to Cloudflare), the website wi24rd.com, and problems that require an attacker who already controls your user account on your computer.

## How the app handles your data

- The app sends requests only to `api.cloudflare.com`, with an API Token you create yourself. The token is stored in the operating system's keychain.
- Figures are kept in a local database on your computer. Nothing is sent to Wi24rd, and there is no telemetry.
- The app only reads. Give the API Token read permissions only; the user guide lists [the permissions it needs](docs/manual/en/31-cloudflare.md).

## Checking a download

Each release states the SHA-256 checksum of the published file, and the app is signed with a Developer ID and notarised by Apple. The README shows how to check both: [Verify your download](README.md#verify-your-download).
