# Visitors

Shortcut: ⌘7 (Windows: Ctrl+7)

## What does it show?

Visitors shows how many people visit your site, where from, how many of the visits are known robots, and which pages are viewed. You see one domain at a time or all of them together, for 7 days, 30 days, one year or a period you choose yourself (at most one year).

At the top are the tiles under the heading **The last days**. They are fixed: they always show yesterday, the last 7 and 30 days and the last year, whatever period you have chosen. Below them is the selector (7 days, 30 days, One year, From and To), and below that the heading **Selected period: 11 Oct – 9 Nov 2026**. Everything under that heading follows the period: the chart, the countries, the robots and the pages.

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

## What do I do when …

- **… the figure jumps on a single day?** Click the day in the chart. It opens that day's traffic on [Attacks](vaernet.md), where you can see whether it was robots, an attack or real visitors.
- **… a country you do not expect is high?** Click the country. The chart then shows the visits from that country per day. Remember that the country is the one Cloudflare saw: a VPN counts in the VPN's country.
- **… the visits are many more than the unique visitors?** See the line under **Visits in 30 days** and the **Known robots** section. Robots that pretend to be an ordinary browser cannot be seen and count as visitors.
- **… /admin or /wp-login.php is at the top of Pages that do not exist?** That is almost always scanners trying well-known addresses. Cloudflare answered 404; you do not need to do anything.
- **… `.env` files or `wp-config.php` are under Answered with the site’s ordinary page?** Those are scanners trying well-known addresses, and the site answered with its front page, not the file. To be sure, open the address in your browser: if you see the front page, the file is not there.
- **… it says "not available"?** Cloudflare did not answer for those figures for that domain. The rest of the page works. The program asks again the next time it starts.
- **… it says "No figures yet" or "not been fetched yet"?** The program fetches in the background after the other figures. The first time after the update it can take a while.

## How to

1. Click **Visitors** in the sidebar (or press ⌘7, Windows Ctrl+7).
2. Choose a domain in the selector at the top, or **All** for all of them.
3. Choose **7 days**, **30 days** or **One year** below the tiles, or choose **From** and **To**. **Reset** shows one year. The heading **Selected period** shows the dates.

## Click further

- A **tile** chooses the period: yesterday, 7 days, 30 days or one year. The tiles do not follow the period, so none of them is marked.
- A **day in the chart** opens **Traffic on …** on Attacks for that day. A **week in the chart** shows the week (From and To are set to Monday and Sunday).
- A **country in the table** puts a filter on Visitors: the chart shows visits from that country per day, and the country stands as a chip at the top.
- A **group under Known robots** puts a filter: the chart shows visits from that group per day.
- A **path under Most visited pages**, **Pages that do not exist** or **Answered with the site’s ordinary page** puts a filter: the chart shows views of the page per day, and the Visits tile follows the page.
- There is at most one filter at a time. **×** on the chip removes it.
- A tile without figures, a day without figures or with 0, the figures and shares in the tables, the percentage lines and the notes lead nowhere and do not look clickable.

## Figures and terms

- **Unique visitors** are Cloudflare's unique IP addresses per day. Robots count, and several people behind the same router count as one. The figure exists a year back at Cloudflare, and the program fetches the whole year.
- **Visits** is Cloudflare's own definition: a page view that came from another site or a direct link. It is the same figure as "Visits" in Cloudflare's dashboard. It is Cloudflare's estimate from a sample.
- **Known robots** are robots Cloudflare has verified (for example Googlebot). The line "of which … from known robots" counts their visits in the last 30 days on the days where both visits and robots have been fetched.
- **Known robots and scanners** (the share of the traffic in the last year) is based on Cloudflare's [IP classes](../99-ordliste.md): search engines, monitoring, scanners, Tor and known bad addresses. It is requests, not visits.
- **Views** is the number of html responses for a path. **Visits from outside** is Cloudflare's visits to the path. Robots count. The path is shown without a query string, and long codes are hidden. The 50 most viewed pages are saved per day.
- Visits per country, known robots and pages exist a month back at Cloudflare. The program saves the days, so the history grows from there.
- **All** adds the domains together, so the same visitor on two domains counts twice.
- The figures are per day in UTC, from the figures saved on your computer. No IP addresses or user agents are saved, only the figures.
- The page gives no notifications and does not change the status in the Overview.
