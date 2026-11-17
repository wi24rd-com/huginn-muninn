# Health

Shortcut: ⌘8 (Windows: Ctrl+8)

## What does it show?

Health answers two questions: did your website answer, and how much did Cloudflare handle without asking your server? You see one domain at a time or all of them together, for 7 days, 30 days, one year or a period you choose yourself (at most one year). You choose the period in the band (**7 days**, **30 days**, **One year**, or **From** and **To** under **Own dates**), and the heading **Selected period: …** shows the dates. The period runs up to and including yesterday: the figures only exist for whole days.

The tab has two sections: **Did your website answer?** and **How much Cloudflare handled itself**.

### Did your website answer?

These are the responses that were errors, as Cloudflare saw them. Only requests from visitors are counted.

The tiles:

| Tile | Means |
|---|---|
| **Could not be reached** | Responses where Cloudflare could not reach or find your server (Cloudflare's errors 520–527 and 530). The line below says how large a part of all responses it was. |
| **Your server answered with an error** | Your server was up, but answered with an error itself (response 500–599). It is usually a program on the server that fails, not an outage. |
| **Hours with errors** | The number of hours where at least 5 responses could not be reached, and they were at least 20% of the responses in that hour. |
| **The front page did not answer** | **Yes** if the front page (`/`) answered with an error at least once in the period; otherwise **No**. |

A tile with a figure shows its rows in the table, and one more click shows all errors again. A **0**, a **No** and a **–** cannot be clicked. If days are missing, it says e.g. "Figures for 6 of 7 days" below the figure.

Below the tiles there may be a yellow line: **Between 01.00 and 05.00, Cloudflare could not reach your server.** It is shown when three or more hours in a row had errors. If the front page also failed in those hours, the line says instead **Between 01.00 and 05.00, your website did not answer: Cloudflare could not reach your server.** The times are your local time. With several days or several domains, the day and the domain stand in front. If there were errors but no hours in a row, and the front page worked, it says **Some pages answered with errors, but the front page worked.**

**Responses with errors per day** is a chart of the period. The bars are yellow, so they do not look like attacks. A click on a day shows the 24 hours of that day in the same chart (**Responses with errors per hour on …**), with times in your local time. A click on an hour shows the errors in that hour in the table. **Back to the period** above the chart leads back, and **See the traffic that day under Attacks** opens **Traffic on …** under [Attacks](vaernet.md). For **One year** (and periods over 90 days) there is one bar per week, Monday to Sunday, and a click on a week shows the week. A day without figures is empty; it is not 0, it has just not been fetched.

The table has the columns **Domain**, **What happened**, **Responses** and **First and last hour**. Each row is one kind of error on one domain, most responses first, and the error is in plain language with the response code in brackets, e.g. "Cloudflare could not connect to your server in time (response 522)". With **All domains**, the domain opens Health for that domain. The 10 largest are shown, the rest behind **Show all … rows**.

At the bottom of the section it always says: **The program only sees traffic that goes through Cloudflare. If Cloudflare answers with an old copy, the visitor sees the page even though your server is down.**

### Subdomains with errors

If one domain has errors on at least two names in the period, e.g. `wi24rd.com` and `www.wi24rd.com`, the table **Subdomains with errors** is shown with the columns **Subdomain**, **Responses with errors** and **Could not be reached**. It is only shown when one domain is chosen. The rows **Called by IP address**, **Other name**, **Invalid name** and **Other subdomains** are not names and cannot be opened.

Click a subdomain, and Health shows only its errors. The chip **Subdomain: www.wi24rd.com** is shown at the top. Per subdomain there is only **Could not be reached** and the errors per day; the table shows **First and last day** instead of hours. The three other tiles show **–** with **Not available per subdomain**, and a line says: **Hours with errors, the front page and your server’s own errors are not available per subdomain. See them for all of wi24rd.com.** **×** on the chip or **all of wi24rd.com** removes the filter, and it is reset when you choose another domain.

### How much Cloudflare handled itself

Cloudflare stores copies of your files and can answer without asking your server. The section shows how often that happened.

The tiles:

| Tile | Means |
|---|---|
| **From Cloudflare’s store** | The share of the requests Cloudflare answered with a copy from its store. |
| **Data your server did not have to send** | The amount of data that came from the store, in readable units (kB, MB, GB). |
| **Answered by Cloudflare itself** | Requests Cloudflare answered without asking your server, e.g. a redirect. |
| **Fetched from your server** | Requests where the answer was fetched from your server. |

A tile shows its rows in the table. Below the tiles the shares stand on one line; they always add up to 100%.

The chart shows requests per day with two bars side by side: **From Cloudflare’s store** (green) and **Fetched from your server** (blue). A click on a day shows the day.

The table **What Cloudflare did** shows what happened to the requests, with the number of requests and the data:

| The text | Means |
|---|---|
| **Found in Cloudflare’s store** | Cloudflare had a copy and answered with it. |
| **Not in the store yet, so the answer was fetched from your server** | The file can be stored, but was not there yet. |
| **The copy in the store was too old, so the answer was fetched from your server** | The copy had expired. |
| **An old copy from the store, because your server did not answer** | Cloudflare could not get a new one and showed the old one. |
| **The answer could not be stored, so it was fetched from your server** | Your server said the answer must not be stored. |
| **Your server confirmed that the copy in the store is still valid** | Cloudflare asked briefly and used its copy. |
| **An old copy from the store while Cloudflare fetches a new one** | The visitor got the copy at once. |
| **Pages that are fetched from your server every time** | Cloudflare does not store them. That applies to most pages. |
| **Answered by Cloudflare itself, without asking your server** | E.g. a redirect to https. |
| **Other** | A value Cloudflare has not described. |

If there were old copies because the server did not answer, the line **Old copies shown because your server did not answer: …** is shown.

Below the table are two notes. The first: **Most pages on small websites are fetched from your server every time. That is Cloudflare’s default: images, CSS and JavaScript are stored, the pages themselves are not. It does not mean anything is broken.** The second: **Data is what Cloudflare sent to the visitor. It can be less than what your server would have sent.**

The cache always covers the whole domain. With a subdomain chosen, it only says **Cache covers the whole domain.**, never the domain's figures under the subdomain's name.

## What do I do when …

- **… there is a yellow line with a time span?** In those hours Cloudflare could not reach your server. Ask your provider whether there was a disruption, and see "What do I do?" on the [Overview](overblik.md) if the domain is also on Watch.
- **… Your server answered with an error has a figure?** The server was up, but a program on it failed. Show the rows in the table, and give your web person the times.
- **… it says "Some pages answered with errors, but the front page worked."?** It is often robots hitting an address that fails. Your website was not down.
- **… almost everything is Fetched from your server?** That is normal for small websites; see the note below the table. The program gives no advice that requires changes at Cloudflare.
- **… it says "–" or "The figures have not been fetched for the period yet."?** The program fetches in the background after the other figures. The first time after the update it takes a little while. See [Troubleshooting](../80-fejlfinding.md).
- **… it says "Errors per hour from …"?** The period goes further back than the figures exist. Errors per hour go about 30 days back at Cloudflare, and the program saves the days, so the history grows from there.

## How to

1. Click the tab **Health** (or press ⌘8, Windows Ctrl+8).
2. Choose a domain in the list **Domains** on the left, or **All domains** for all of them together.
3. Choose **7 days**, **30 days** or **One year** in the band, or choose **From** and **To**.

## Click further

- A **tile with a figure** shows its rows in the table; the chip **Only: …** says what is chosen, and **×** removes it.
- A **day in the chart** shows the hours of that day. An **hour** shows the errors in that hour in the table. A **week** shows the week.
- **Back to the period** leads back from a day, and **See the traffic that day under Attacks** opens Traffic on that day.
- A **domain in the table** (with All domains) opens Health for the domain with the same period.
- A **subdomain under Subdomains with errors** shows the errors for that subdomain.
- A **day in the cache chart** shows the day.
- The line **Your server is not answering** on the [Overview](overblik.md) and the link in [Since last time](siden-sidst.md) open Health for the domain and the day.
- Everything is in the address, so back and forward work.
- A tile with 0, No or –, a day without errors, the figures in the tables, the cache table, the lines and the notes lead nowhere and do not look clickable.

## Figures and terms

- **Could not be reached** is Cloudflare's errors 520–527 and 530: your server did not answer, refused the connection, took too long to answer, had a problem with the secure connection, or Cloudflare could not look up the server's name. The individual codes are in the [glossary](../99-ordliste.md).
- **Your server answered with an error** is responses 500–599 from your own server.
- **An hour with errors** has at least 5 responses that could not be reached, and they are at least 20% of the responses in that hour. Three or more hours in a row give the line with the time span.
- The figures are Cloudflare's estimate from a sample, and only requests from visitors count. What Cloudflare itself blocked is not an error.
- The program only sees traffic that goes through Cloudflare. A subdomain that is "DNS only", and visits directly to the server's IP address, are not seen. If Cloudflare shows an old copy while the server is down, the visitor sees the page, and it does not count as an error.
- Errors per hour and cache go about 30 days back at Cloudflare. The program saves the days, so the history grows from there. Errors per subdomain exist from about 30 days before the update.
- A figure that has not been fetched is shown as **–** or "Not fetched", never as 0.
- The days are whole days in UTC; the times are shown in your local time.
- **All domains** adds the domains together. A combined figure is only shown for the days where all chosen domains have been fetched.
- Health gives no notifications and does not change the status. Only **Your server is not answering** on the [Overview](overblik.md) can give Watch (never Act), and that rule is the same as before, now with error 530. Your server's own errors and "some pages answered with errors" never change the status and never give a message.
- No new permission is needed on your API Token.
