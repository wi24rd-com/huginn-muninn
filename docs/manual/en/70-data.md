# Data and history

## Where the figures live

Everything the program keeps is in one SQLite file, `huginn-muninn.sqlite`, in the program's data folder on your computer:

| System | Data folder |
|---|---|
| macOS | `~/Library/Application Support/com.wi24rd.huginn-muninn/` |
| Windows | `%APPDATA%\com.wi24rd.huginn-muninn\` |
| Linux | `~/.local/share/com.wi24rd.huginn-muninn/` (or `$XDG_DATA_HOME`, if set) |

API Tokens are **not** in the file. They are in your computer's keychain (macOS Keychain or Windows Credential Manager) under the name `com.wi24rd.huginn-muninn`, one per connection. The names of the entries are in [Uninstalling](90-afinstallation.md).

The size is shown under **Settings** › **Data** › **Local database**.

## What is kept

- **Every finished day** per chosen domain as counts: refused requests by rule, country, network, path, data centre, action and robot; traffic figures; robots; login figures; DMARC figures; DNS figures; unique visitors (one figure per day, no IP addresses); requests per IP class; visits per country; visits per known robot (Cloudflare's category); the 50 most viewed pages per day (the path without query string, with status code, views and visits, and the subdomain when it is not the domain itself); requests, visits and attacks per subdomain.
- **Your settings**: language, date format, country, appearance, text size, notification choices, domain choice, login paths, technology choices.
- **Your connections**: the account's name at Cloudflare, your own name, how many accounts the API Token sees, when the connection was created, and which domains each connection sees. Never the API Token itself.
- **What has been reported**: which domains got a notification for which day, so the same thing is not reported twice. That list clears itself.
- **Radar figures** for your country and the world.

**The subdomains** are kept as the name below the domain, e.g. `www` or `shop`, in your own database on your computer, like the domain names. The name is what visitors typed, so the program cleans it first: at most 20 names per domain and day (the rest are added together as "other"), and a request with an IP address, a name outside the domain or an invalid name is kept only as one of those three kinds, never the address or the name itself. The subdomains never appear in notifications, in the menu bar or in Overview.

What is **not** kept: IP addresses (never fetched), the query part of paths, long codes in paths, full user agents (only the robot name), passwords and the time of day (everything is whole days in UTC).

## How far back

- Cloudflare keeps **security events for a month** and **traffic figures for a year**. That is what the program can fetch.
- The program keeps **everything it has fetched, for as long as you like**. Nothing is deleted by itself. So after a year you have a year of security events, even though Cloudflare only has a month.
- **What is available** under Settings › Data says how far back each kind of figure reaches, and **Oldest day** is the first day with figures.

## Gaps

A gap is a day the program never fetched, typically because the computer was off. Gaps are never drawn as 0, and sums say "N of M days".

- If the program was off for **less than a month**, it fetches the missing days itself when it starts again. The Overview says "Huginn was not on from … to …. Cloudflare still had the figures, so nothing is missing."
- If it was off for **longer**, the oldest days are gone at Cloudflare and cannot be fetched. The Overview says how many days are missing for good.
- **Settings** › **Data** › **Gaps** shows the number, and a click shows the list.

That is why **Start at login** is on by default. With it, the program also starts by itself once a day and fetches, so gaps do not appear even if you have chosen **Quit**. And that is why the program says you should open it at least every three weeks if you turn it off.

## Fetching

- The program asks for today and yesterday, one day at a time, so Cloudflare does not have to sample a long period. Older days are only fetched if they are missing.
- The number refused is Cloudflare's estimate: each event in the sample states how many it stands for, and the program adds that up. The first time the program starts after the update of 8 November 2026, it fetches the last 30 days again at the normal pace, so the figures can be compared. Older days cannot be delivered again by Cloudflare; they keep the counted rows.
- At most two calls at a time and at least 1.1 seconds apart, for all domains together. If Cloudflare asks it to wait, the program pauses and says "Trying again at …" in the footer at the bottom of the window.
- The figures for Login and "stopped at the Edge" per robot are fetched afterwards at a slow pace and only for finished days.
- For **Visitors**, unique visitors are fetched with the year's traffic figures, and visits per country last, one call per domain. Visits per country go back a month at Cloudflare; the program saves the days, so the history grows from there. The first time the program starts after the update of 9 November 2026, it fetches the year once more for the days it already has, so they get unique visitors. Only that figure is written; the other figures for those days do not change. The first start after the update with Known robots does the same once for the IP classes (kind `h_ipklasse`); only they are written. Known robots (kind `b_robot`) and the most visited pages (kinds `b_side`, `b_side_b` and `b_side_bytes`: views, visits and the total bytes of the answers per day and path) are each fetched with their own call for the same days as the visits, also for days whose visits were already fetched. The bytes are only used to tell paths that answer with the front page from real pages. The first start after the update of 10 November 2026 fetches the pages for the last month once more, so they get bytes; older days keep their figures without. If Cloudflare refuses one of the calls, it says "not available" at that section, and the rest works.
- **Subdomains**: requests and visits per subdomain are fetched last at the same slow pace, one call per domain for the finished days that are missing. Attacks per subdomain come with the call that already fetches the attacks, so they cost nothing extra. The first start after the update with subdomains fetches the last 30 days or so once (three calls per domain: traffic, attacks and the pages per subdomain), so the history per subdomain starts about 30 days before the update. Older days have no figures per subdomain. If Cloudflare refuses the name in the attacks, the program fetches the attacks without it until it starts again, and the domain’s figures are the same.
- **Quota at Cloudflare** under Settings › Data shows how many calls Cloudflare allows, and how many the program used in the last round.
- **Fetch now** (⌘R, Windows Ctrl+R or F5) starts a fetch at once.

## Export

The program has no export function yet. The database is an ordinary SQLite file that can be opened with any SQLite tool while the program is not running.

## Reset history

**Settings** › **Data** › **Reset history…** deletes all figures on your computer and fetches them again. The program asks first and says which dates are lost for good, and how many days that is: Cloudflare only has security events for the last month and traffic figures for the last year, so anything older cannot be fetched again.

What stays: your settings, the zone list, the domain choice, the notification choice per domain, the login paths and the technology choices. The file itself stays too; the space is released.

## Forget a connection

With one connection: **Settings** › **Connection** › **Forget API Token…** deletes the API Token from the keychain and your per-domain choices (domain choice, notifications per domain, login paths, technology choices). The history stays, so you can connect again later without losing it.

With more than one connection: **Settings** › **Connections** › **Forget…** in the connection's row deletes only its API Token from the keychain. Only domains that no other connection sees lose their choices. The other connections keep fetching, and the history stays. You forget the last connection with **Forget API Token…** as above.

## Newer database

If you open a database made by a newer version of the program, the program says "The local database is from a newer version of the program (schema …)". Update the program; the file is not touched.

The version with more than one connection and subdomains upgrades the database the first time it starts: first to schema 19 (the connections), then to schema 20 (two new tables of which days have been fetched per subdomain; no saved figures change). After that, version 1.0.0 cannot open it and says "(schema 20)". If you want to be able to go back, take a copy of the file first; see [Updating](85-opdatering.md).
