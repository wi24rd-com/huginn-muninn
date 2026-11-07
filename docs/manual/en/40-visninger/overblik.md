# Overview

Shortcut: ⌘1 (Windows: Ctrl+1)

## What does it show?

The Overview is the program's front page: one row per chosen domain with a status, an explanation and the figures for the last few days. At the top is a **verdict line** that sums it all up in one sentence, for example "1 domain should be watched" or "Nothing to do".

![Overview with three domains](../../img/overblik--light-en-1200x860.webp)
*The verdict line at the top, one row per domain, and the columns Refused and Attacks. The yellow line above the table tells of a gap in the history.*

The columns:

| Column | Means |
|---|---|
| **Domain** | The domain's name. |
| **Status** | **Act**, **Watch**, **Calm** or **Unknown** (no figures yet). The status compares yesterday's attacks with the average of the week before. |
| **Why** | The explanation in plain language: how many attacks, what they went for, and whether they were stopped. **more** unfolds the rest, and **What do I do?** opens advice. |
| **Refused** – yesterday · 7 days · 30 days | What Cloudflare blocked, challenged or lured into the labyrinth, including routine (Bot Fight Mode, browser check). |
| **Attacks** – this week · last week | Real attacks, without the routine. |

**Refused** and **Attacks** are not the same figure: Refused counts everything Cloudflare stopped, while the status and Attacks count only real attacks. Cloudflare's automatic bot checks (the labyrinth, Bot Fight Mode and the browser check) are routine and never give Watch or Act. Pages served with hidden trap links are not counted.

## What do I do when …

- **… the status is Calm?** Nothing. Cloudflare handles what comes.
- **… the status is Watch?** Read the explanation under Why. Typically everything was stopped, but the pattern is unusual. Click **What do I do?** for advice, such as "ask your web person to check that the site is up to date".
- **… the status is Act?** There were unusually many attacks yesterday compared with the week before, or something got through. Click the row to see the attacks, and follow the advice under **What do I do?**.
- **… the status is Unknown?** The program has no figures for the domain yet, typically because it was just chosen. Wait for the fetch; the status pill in the band shows how far it is.
- **… it says "updating …" next to the status?** At night (until 03:00 UTC), before yesterday is fully fetched, the status uses the day before.

## Labels on top of the status

Some domains get a label next to the status: **Looks targeted**, **Looks like reconnaissance** or **Looks like DNS reconnaissance**. The labels say something about the pattern, not the amount, and always as "looks", never as a certain verdict. What they mean is in the [Glossary](../99-ordliste.md) and in the advice behind **What does it mean? →**.

The Overview can also tell you that a domain is paused at Cloudflare, not active, suspected of phishing or in Development Mode, and that your server is not answering (errors 520–527). Under Why it may say "Follows the global trend" or "The world was normal yesterday, so the rise is yours"; that compares with the whole world and does not change the status.

## Gaps in the history

If the program has not been running for a while, a yellow line appears at the top: "Huginn was not on from … to …". Cloudflare keeps only a month, so days older than that cannot be fetched afterwards and are missing for good. The line says how many. **Close** hides it. If the gap was shorter than a month, the program fetches the days itself, and the line says that nothing is missing.

## New domains

If a new domain appears on your Cloudflare account, the Overview shows a discreet line with **Choose** and **Not now**. It is not fetched until you choose it.

## Click on

| Click | Opens |
|---|---|
| The rest of a row (or Enter on the row) | [Attacks](vaernet.md) for the domain |
| A figure under Refused or Attacks | [Attacks](vaernet.md) with that very figure as filter |
| **Attacks for all** (the button at the top right) | [Attacks](vaernet.md) for all chosen domains combined |
| **What do I do?** / **What does it mean? →** | Plain-language advice for that very situation |
| **more** | The rest of the explanation |
| A CVE number in the explanation | The CVE's page on cve.org in your browser |
| The line at the bottom about cleared domains | **Settings → Domains** |

Nothing without figures behind it looks clickable.

## Figures and terms

- All days are whole days in **UTC**. "Yesterday" is therefore yesterday in UTC, not in your local time.
- Figures over long periods may be **sampled** by Cloudflare and are therefore approximate.
- **Fetch now** in the band (⌘R) fetches new figures at once. Otherwise the program fetches by itself in the background.
