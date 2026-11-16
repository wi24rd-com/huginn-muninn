# Visitors

Shortcut: ⌘7 (Windows: Ctrl+7)

## What does it show?

Visitors shows how many people visit your site, where from, how many of the visits are known robots, and which pages are viewed. You see one domain at a time or all of them together, for 7 days, 30 days, one year or a period you choose yourself (at most one year).

At the top are the tiles under the heading **The last days**. They are fixed: they always show yesterday, the last 7 and 30 days and the last year, whatever period you have chosen. You choose the period in the band (**7 days**, **30 days**, **One year**, or **From** and **To** under **Own dates**), and below the tiles is the heading **Selected period: 11 Oct – 9 Nov 2026**. Everything under that heading follows the period: the chart, the countries, the robots and the pages.

The tiles:

| Tile | Means |
|---|---|
| **Unique visitors yesterday** | The number of different IP addresses that visited the site yesterday. |
| **Unique visitors in 7 days** | The sum of the daily figures for the last 7 days. |
| **Unique visitors in 30 days** | The sum of the daily figures for the last 30 days. |
| **Unique visitors in a year** | The sum of the daily figures for the last year. |
| **Visits in 30 days** | Cloudflare's visits for the last 30 days. The line below says how many of them came from known robots. |

A sum over several days counts the same visitor once per day: if the same person comes back on five days, they count five times. If days are missing, it says for example "6 of 7 days" under the figure.

With a country as a filter the tile is called **Visits from Denmark in 30 days** (and with a page **Visits to /products in 30 days**) and counts only those. Unique visitors cannot be split by country or page; those tiles get the line **all countries** or **all pages**.

**Unique visitors per day** is a chart of the period. The bars are blue, so they do not look like attacks. For **One year** (and periods over 90 days) there is one bar per week, Monday to Sunday, and the whole year fits at once with the months on the axis; if days are missing in a week, the tooltip says for example "3 of 7 days". A day or week without figures is empty; it is not 0, it just has not been fetched.

**Where the visits came from** is a table of country, visits and share in the period. The 10 largest are shown, the rest behind **Show all**.

**Known robots** shows visits from robots Cloudflare has verified, in the same groups as [Robots](robotter.md) (search engines, AI training, monitoring …). Above the table it says how large a part of the traffic in the last year came from known robots and scanners.

**Most visited pages** shows the pages (html) in the period: path, views, visits from outside and share. The 10 largest are shown, the rest behind **Show all**. Below it is **Pages that do not exist**: paths that answered 404. Not every attempt is there: many sites answer with their ordinary page at addresses they do not know.

That is why **Answered with the site’s ordinary page** is below the 404 list: paths like `//crm/.env` where the answer was as large as the front page (within 25 %) and nobody came from outside. The page probably does not exist; the site just answers with its front page. It is an estimate from the size of the answers, the same rule as Sensitive files on [Attacks](vaernet.md). The front page itself is never there. If the front page is not among the pages in the period, the list is not split, and a line says so. A site that answers 404 at unknown addresses gets no such list.

### Subdomains

If the domain has traffic on at least two names in the period, e.g. `wi24rd.com`, `www.wi24rd.com` and `shop.wi24rd.com`, the section **Subdomains** is shown below the graph. It is only shown when one domain is selected, and only without a filter. A domain without subdomains looks as before.

The table has the columns **Subdomain**, **Visits**, **Requests**, **Attacks** and **Share** (the share of the requests). The domain itself is shown with its own name, `www` as `www.<domain>`. If the domain has more than 20 names, the rest are in a last row **Other (n)**, and the names can be opened under **Show n more subdomains**. Three rows are not names and cannot be opened: **Called by IP address** (someone called the site by its IP address instead of its name), **Other name** (a name outside the domain) and **Invalid name**.

The table follows the selected period. The program has figures per subdomain from about 30 days before the update to this version. When the period goes further back, **Per subdomain from {date}.** is shown below the table, and the figures only cover the days from then. Below the table it always says: **Only traffic through Cloudflare. A subdomain set to “DNS only” has no numbers here.** A name with a grey cloud in Cloudflare’s DNS does not go through Cloudflare, so Cloudflare has no figures for it.

Click a subdomain (the name or the figures), and Visitors shows only that one. The chip **Subdomain: shop.wi24rd.com** is shown at the top. The tiles and graphs show visits and requests per day on the subdomain, and **Most visited pages** shows only the subdomain’s pages. Unique visitors, countries, known robots and the IP classes do not exist per subdomain at Cloudflare. They are shown with **Not per subdomain**, never with the whole domain’s figures.

In **Most visited pages**, the same path on two subdomains is two rows. The row for a subdomain has the name next to the path, e.g. `/` with `shop.wi24rd.com`. Days saved before the update that are older than about 30 days cannot be told apart and count as the domain itself. If Cloudflare sent the highest number of rows for a day, the note **Only the most visited pages are included: Cloudflare sent at most 10,000 rows.** is shown.

## What do I do when …

- **… the figure jumps on a single day?** Click the day in the chart. It opens that day's traffic on [Attacks](vaernet.md), where you can see whether it was robots, an attack or real visitors.
- **… a country you do not expect is high?** Click the country. The chart then shows the visits from that country per day. Remember that the country is the one Cloudflare saw: a VPN counts in the VPN's country.
- **… the visits are many more than the unique visitors?** See the line under **Visits in 30 days** and the **Known robots** section. Robots that pretend to be an ordinary browser cannot be seen and count as visitors.
- **… /admin or /wp-login.php is at the top of Pages that do not exist?** That is almost always scanners trying well-known addresses. Cloudflare answered 404; you do not need to do anything.
- **… `.env` files or `wp-config.php` are under Answered with the site’s ordinary page?** Those are scanners trying well-known addresses, and the site answered with its front page, not the file. To be sure, open the address in your browser: if you see the front page, the file is not there.
- **… it says "not available"?** Cloudflare did not answer for those figures for that domain. The rest of the page works. The program asks again the next time it starts.
- **… a subdomain has many attacks or requests?** Click it under **Subdomains**. Then you see it per day and which pages were viewed. The attacks per day are on [Attacks](vaernet.md) for the same subdomain.
- **… my subdomain is not there?** If it is “DNS only” (grey cloud) in Cloudflare’s DNS, Cloudflare has no figures for it. Otherwise it had no traffic in the period. See [Troubleshooting](../80-fejlfinding.md).
- **… it says "No figures yet" or "not been fetched yet"?** The program fetches in the background after the other figures. The first time after the update it can take a while.

## How to

1. Click the tab **Visitors** (or press ⌘7, Windows Ctrl+7).
2. Choose a domain in the list **Domains** on the left, or **All domains** for all of them combined.
3. Choose **7 days**, **30 days** or **One year** in the band, or choose **From** and **To**. **Reset** shows one year. The heading **Selected period** shows the dates.

## Click further

- A **tile** chooses the period: yesterday, 7 days, 30 days or one year. The tiles do not follow the period, so none of them is marked.
- A **day in the chart** opens **Traffic on …** on Attacks for that day. A **week in the chart** shows the week (From and To are set to Monday and Sunday).
- A **country in the table** puts a filter on Visitors: the chart shows visits from that country per day, and the country stands as a chip at the top.
- A **group under Known robots** puts a filter: the chart shows visits from that group per day.
- A **path under Most visited pages**, **Pages that do not exist** or **Answered with the site’s ordinary page** puts a filter: the chart shows views of the page per day, and the Visits tile follows the page.
- A **subdomain under Subdomains** (the name or Visits, Requests and Attacks) shows Visitors for that subdomain, with the chip **Subdomain: …**. The filter is in the address, so back and forward work, and it is reset when you choose another domain.
- A **path on a subdomain** under Most visited pages shows the views of the path on that subdomain. It is the only filter that can be combined with a subdomain.
- Otherwise there is at most one filter at a time. **×** on the chip removes it.
- A tile without figures, a day without figures or with 0, the figures and shares in the tables, the percentage lines and the notes lead nowhere and do not look clickable. This also applies to **Share**, **Other (n)**, the rows **Called by IP address**, **Other name** and **Invalid name**, and the notes under Subdomains.

## Figures and terms

- **Unique visitors** are Cloudflare's unique IP addresses per day. Robots count, and several people behind the same router count as one. The figure exists a year back at Cloudflare, and the program fetches the whole year.
- **Visits** is Cloudflare's own definition: a page view that came from another site or a direct link. It is the same figure as "Visits" in Cloudflare's dashboard. It is Cloudflare's estimate from a sample.
- **Known robots** are robots Cloudflare has verified (for example Googlebot). The line "of which … from known robots" counts their visits in the last 30 days on the days where both visits and robots have been fetched.
- **Known robots and scanners** (the share of the traffic in the last year) is based on Cloudflare's [IP classes](../99-ordliste.md): search engines, monitoring, scanners, Tor and known bad addresses. It is requests, not visits.
- **Views** is the number of html responses for a path. **Visits from outside** is Cloudflare's visits to the path. Robots count. The path is shown without a query string, and long codes are hidden. The 50 most viewed pages are saved per day.
- Visits per country, known robots and pages exist a month back at Cloudflare. The program saves the days, so the history grows from there.
- **Subdomains**: visits and requests per subdomain are Cloudflare’s figures for the name as the visitor typed it. The sum over all rows is the domain’s figure. **Attacks** are counted as on Attacks. A subdomain set to “DNS only” has no figures.
- **All domains** adds the domains together, so the same visitor on two domains counts twice.
- The figures are per day in UTC, from the figures saved on your computer. No IP addresses or user agents are saved, only the figures.
- The page gives no notifications and does not change the status in the Overview.
