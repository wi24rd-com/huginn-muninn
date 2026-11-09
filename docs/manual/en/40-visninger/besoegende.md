# Visitors

Shortcut: ⌘7 (Windows: Ctrl+7)

## What does it show?

Visitors shows how many people visit your site, and from where. You see one domain at a time or all of them together, for 7 days, 30 days, one year or a period you choose yourself (at most a year).

The tiles:

| Tile | Means |
|---|---|
| **Unique visitors yesterday** | The number of different IP addresses that visited the site yesterday. |
| **Unique visitors in 7 days** | The sum of the daily figures for the last 7 days. |
| **Unique visitors in 30 days** | The sum of the daily figures for the last 30 days. |
| **Unique visitors in a year** | The sum of the daily figures for the last year. |
| **Visits in 30 days** | Cloudflare's visits for the last 30 days. |

A sum over several days counts the same visitor once per day: if the same person comes back on five days, they count five times. If days are missing, it says, for example, "6 of 7 days" under the figure.

**Unique visitors per day** is a chart of the period. A day without figures is left empty; it is not 0, it just has not been fetched.

**Where the visits came from** is a table with country, visits and share in the period. The 10 largest are shown, the rest behind **Show all**.

## What should I do when …

- **… the figure jumps on a single day?** Click the day in the chart. It opens that day's traffic on [Attacks](vaernet.md), where you can see whether it was robots, an attack or real visitors.
- **… a country you do not expect is high?** Click the country. The chart then shows the visits from that country per day. Remember that the country is the one Cloudflare saw: a VPN counts in the VPN's country.
- **… it says "Visits per country are not available"?** Cloudflare did not answer for the visits on that domain. The rest of the page works. The program asks again the next time it starts.
- **… it says "No figures yet"?** The program fetches the year in the background after the other figures. The first time after the update it can take a while.

## How to

1. Click **Visitors** in the sidebar (or press ⌘7, Windows Ctrl+7).
2. Choose a domain in the selector at the top, or **Combined** for all of them.
3. Choose **7 days**, **30 days** or **One year**, or choose **From** and **To**. **Reset** shows a year.

## Click through

- A **tile** chooses the period: yesterday, 7 days, 30 days or a year.
- A **day in the chart** opens **Traffic on …** on Attacks for that day.
- A **country in the table** puts a filter on Visitors: the chart shows the visits from that country per day, and the country stands as a chip at the top. **×** on the chip removes the filter.
- A tile without figures, a day without figures or with 0, the figures in the table and the share lead nowhere and do not look clickable.

## Figures and terms

- **Unique visitors** are Cloudflare's unique IP addresses per day. Robots count, and several people behind the same router count as one. Cloudflare has the figure for a year back, and the program fetches the whole year.
- **Visits** are Cloudflare's own definition: a page view that came from another site or a direct link. It is the same figure as "Visits" in Cloudflare's dashboard. It is Cloudflare's estimate from a sample.
- Visits per country go back a month at Cloudflare. The program saves the days, so the history grows from there.
- **Combined** adds the domains together, so the same visitor on two domains counts twice.
- The figures are per day in UTC, from the figures saved on your computer. No IP addresses are saved, only the figure.
- The page gives no notifications and does not change the status in the Overview.
