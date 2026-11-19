# Before you start

## What the program does

Huginn and Muninn fetches figures from your own Cloudflare account and shows them as an overview of all your domains: how many attacks Cloudflare stopped, which robots read your site, what happened on your login pages, who visits your site, whether your website answered, and how it looks in the rest of your country and in the world. Each domain gets a status in plain language: **Calm**, **Watch**, **Act** or **Unknown** (no figures yet).

Every finished day is kept on your computer. Cloudflare only keeps security events for the last month, errors per hour and cache for about 30 days and traffic figures for the last year; the program keeps them for as long as you like.

## What the program does not do

- It **never changes anything** at Cloudflare. The API Token can only read. If you want to block a robot or make a rule, you do it yourself in Cloudflare's dashboard.
- It **sends nothing to Wi24rd**. There is no telemetry, no Wi24rd account and no server in between. The program talks only to `api.cloudflare.com`.
- It **neither fetches nor stores IP addresses**. Paths are stored without the query part, long codes are hidden, and user agents are only stored as a robot name.
- It gives no guarantee that everything is caught. The figures are Cloudflare's figures, and Cloudflare may use samples over long periods, so figures can be approximate.

## Who it is for

Anyone who has one or more domains at Cloudflare and wants to know whether they need to do something, without clicking through Cloudflare's dashboard domain by domain. You do not need to be technical: the default views answer "do I need to do anything?" on their own. If you want to know more, everything with figures behind it can be clicked.

An IT company can create one API Token per customer that only sees the customer's domain, and give it to the customer. The customer then sees only their own domains.

## What it needs

| | |
|---|---|
| **Computer** | macOS 11 or newer on a Mac with Apple silicon. Version 1.1.0 is signed and notarised by Apple. The Windows version is not released yet, and there is no Linux version; see [Installation](20-installation.md). |
| **Cloudflare** | A Cloudflare account with at least one domain (a zone). All plans, including Free, can be used. |
| **Permissions** | An API Token with two required permissions that can only read: **Zone › Read** and **Analytics › Read**. One more permission is optional: **Account Security Center Insights › Read**, if you want to see Cloudflare's own findings under Attacks. The program asks for nothing more. |
| **Internet** | The program fetches from Cloudflare while it runs. It only runs while the computer is on. |
| **Space** | The local database is small (a few megabytes after a month with a few domains). You can see the size under **Settings** › **Data**. |

Some figures need more at Cloudflare: the figures for **Mail sent in your domain's name** need DMARC Management turned on at Cloudflare, and some fields do not exist on every plan. Where they are missing it says "not available", and the rest works.

## Privacy in one table

| What | Where | Who sees it |
|---|---|---|
| API Tokens (one per connection) | Your computer's keychain (macOS Keychain or Windows Credential Manager), one entry per connection | Only the program's Rust part. The window cannot read them back. |
| The figures (the history) | An SQLite file in the program's data folder on your computer, see [Data and history](70-data.md) | Only you |
| Your choices (domains, notifications, language) | The same database | Only you |
| Notifications | Your own computer's notification service | Only you. No e-mail, no webhook, no server. |
| Traffic leaving the computer | Only to `api.cloudflare.com` (and Cloudflare Radar for the country view) | Cloudflare |

Clicking a CVE number opens cve.org in your browser; the program itself fetches nothing from there.
