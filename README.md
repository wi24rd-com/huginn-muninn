# Wi24rd Huginn and Muninn
<sub>for Cloudflare®</sub>

*Security isn't something you buy. It's something you understand.*

A desktop app (Tauri) that shows you what is actually happening against **your own** Cloudflare domains — in plain language, across all your zones, with a local history that outlives Cloudflare's 31 days.

- **Huginn (thought):** what is happening now — the attacks, what was stopped and how.
- **Muninn (memory):** what has happened — every finished day is kept locally.

**Privacy:** the app talks only to `api.cloudflare.com` with your own read-only API Token, stored in your OS keychain. Nothing is sent to Wi24rd. No telemetry.

**Status:** early prototype (macOS dev builds). Not yet released.

## The app
- **One window:** at the top a blue band with the app's name, a status pill ("Fetching 3 of 7", "Updated 08:01"), **Fetch now** and **?** (help: the explanations for the view, the shortcuts and About). On macOS the traffic lights sit in the band, and the band can be dragged like a title bar.
- **The sidebar:** **Overview** · **Since last time** · **Attacks** · **Robots** · **Login** · **Denmark now** (the name follows your country) · **Settings** at the bottom. Without an API Token the setup is shown in the window, and only Settings can be opened.
- **Domains** are something you choose. After you connect, no domains are chosen, unless the API Token can see exactly one, which is then chosen for you. Under **Settings → Domains** you choose the ones Huginn should fetch and show. A domain you clear is no longer fetched, is not shown anywhere and gives no notifications; its figures stay, and if you choose it again, what is missing is fetched, at most the last 30 days. A new domain on your account is not chosen: a discreet line in the Overview offers **Choose** or **Not now**.
- **Overview:** a verdict line at the top ("1 domain needs action" / "Nothing to do") and one row per domain with status (**Act**, **Watch**, **Calm**, **Unknown**), why, refused requests (yesterday, 7 and 30 days) and attacks this week and last week. Click a row (or press Enter) to see the domain's attacks; **Attacks for all** shows them combined.
- **Technology by domain** (under **Settings → Domains**, or **Edit technologies** in Attacks) is a table with a choice per domain and technology: Uses, Does not use or Don't know. Each column and each domain has its actions behind one **⋯** button. You can search, and under **All technologies** you choose what is shown and add technologies in advance.
- **Shortcuts:** macOS ⌘1 Overview · ⌘2 Since last time · ⌘3 Attacks · ⌘4 Robots · ⌘5 Login · ⌘6 Denmark now · ⌘, Settings · ⌘R Fetch now (also in the **View** menu). Windows: the same with Ctrl, and F5 = Fetch now.
- **CVE numbers** (e.g. CVE-2025-55182 in Attacks and in "What do I do?") can be clicked: they open the CVE's page on cve.org in your browser. The app itself fetches nothing from there.
- **Light and dark** follow the system, or you choose under **Settings → General**. The colours come from the Wi24rd UI kit (`design/wi24rd-tokens.json`).
- **Text size:** Small, Normal or Large under **Settings → General**, or ⌘+ ⌘− ⌘0 (Windows: Ctrl). The choice is remembered.
- **Reset history** under **Settings → Data** deletes all figures on your computer and fetches them again. Cloudflare only has security events for the last month and traffic figures for the last year — anything older is gone for good; the dialog tells you which dates, and how many days are lost.

## Attacks
- **One page per domain:** everything about the attacks is on one page, and the heading says, for example, "Attacks · wi24rd.com". The page shows 7 whole days by default. Choose **30 days**, **From** and **To** in the date picker, or **Reset** for all history.
- **Filters:** clicking a figure, a country, a rule or a path presets the period and the filter on the same page. The filters are chips under the heading: × removes one, and **Reset** removes them all. **Show** switches between **All**, **Attacks only** and **Routine only**. Back and forward go one step at a time.
- **All sections follow the period and the filters.** A section that cannot be filtered that way is left out, and a line says which. With a filter the saved figures are shown, and nothing is fetched from Cloudflare.
- **Traffic on …:** a day with unusually much traffic opens a section at the top of the page with the requests by country, status code and robot.
- **The world map** shows sea and land, and fills each country in five steps by the number of attacks, with the scale under the map. The country is the one Cloudflare saw the attack come from, which may be a VPN or proxy. An arc runs from the country to the data centre that stopped the attack, and Cloudflare's other data centres are faint blue dots. Click a country, ring or arc to see the attacks behind it. The smallest countries have no outline on the map and only get a ring.
- **Mail sent in your domain's name:** Attacks shows, for one domain, how much mail was sent in the domain's name, how much failed DMARC and from which senders. It needs DMARC Management to be turned on at Cloudflare. A new sender that fails, or a markedly larger share, gives Watch. IP addresses are never stored.
- **Looks like DNS reconnaissance:** a label when someone asks for many names under the domain that do not exist. It follows the notification for "Looks like reconnaissance".
- **Assessment:** for one day you can unfold **Why did the day (not) get a label?** Each of the three labels says yes, no or "yes, preliminary", with each condition and its figure. Limits are whole numbers, and a condition with nothing to compare with gets no ✓ or ✗.

## Your country now and the World
- **Denmark now** (⌘6; the name follows the country you choose) shows the attacks on your domains next to the world from Cloudflare Radar, as an index where 100 is the average of the period. The curves show rises and falls, not numbers.
- Attacks are shown from the target's side, because a source country can be due to a VPN or rented servers. **The largest sources of attacks on Denmark** stand as a share of the attacks on the country next to **Where attacks on your domains came from** for the last 7 days or 4 weeks, and countries that weigh much more for you are marked "Especially you".
- **Denmark as a target** shows the country's share of all attacks Cloudflare saw, as a figure and day by day. It is a measured share, not a number of attacks.
- **A year of traffic:** Huginn fetches a year of daily traffic figures for each chosen domain. **All history** and the date picker in Attacks go that far back for traffic. Cloudflare keeps security events for a month, and it says clearly where they begin.
- **The domain's state:** Overview and notifications say so if a domain is paused, not active, suspected of phishing or in Development Mode.
- **Internet disruptions** shows what Radar has recorded in the country in the last 30 days, and the World tab shows them for the whole world.
- **World** is a tab with the most targeted countries, most attacks from, what stopped them, robot traffic, quantum-safe connections and AI robots by purpose and name.
- The country is taken from your time zone the first time, then from your system language, and you can change it in the bar or under Settings. If at least half of your visitors clearly come from another country, Settings offers **Switch to** that country; robots and traffic spikes do not count.
- In the Overview, **Why** can say "Follows the global trend" or "The world was normal yesterday, so the rise is yours" (only when your day is at least three times your average). It compares WAF attacks in general, never a particular rule, and it does not change the status.
- Radar needs no extra permission, and an error at Radar only affects these views. Source: Cloudflare Radar, CC BY-NC 4.0.

## Robots and Login
- **Robots** (⌘4) shows who reads your site, in seven groups from Cloudflare's verified categories: AI training, AI search and assistants, Search engines, Social media and link previews, SEO tools, Monitoring and Unknown. Each group says what may happen if you block it. You do any blocking yourself at Cloudflare: the program only reads.
- You also see the most crawled pages, those in disguise (something calling itself Googlebot, say, without being it), which robots were stopped at the Edge, and which were lured into the labyrinth. The labyrinth does not stop the robot; it shows fake pages.
- **Login** (⌘5) shows what happened on the login paths you turn on per domain, in the view or under **Settings → Notifications → Login paths per domain**. There is a fixed starting list and suggestions from your own data, but the program never guesses what the site runs. A path is suggested only if its name has a login word as a whole word (`login`, `oauth`, `password` and the like) or Cloudflare's check saw a password on it. Paths that merely resemble a login, such as `admin-ajax.php`, do not count by name, and a path in events from a known rule is never suggested.
- **Passwords from known leaks:** Login shows per day and path how many login attempts used a password from a known leak. It gives at least Watch, and Act if your server answered 2xx or 302. Passwords are never stored, and the path is never in a notification. A request without a password stands as "Not a login attempt (no password sent)" and never counts as a leak.
- Login has three figures from separate sources that are never added up: **stopped at the Edge**, **got 404** and **reached your server**. Response 200 or 302 from your server may be a login, but may just as well be an error page. The program sees no IP addresses, and Cloudflare may use samples.
- The figures for Login and "stopped at the Edge" per robot are fetched separately, at a slow pace after the normal fetch, and only for finished days. The first time takes up to an hour for 30 days. If a plan lacks the fields, it says "not available", and the rest works.

## In the menu bar and at login
- **When you close the window, the app keeps running** in the menu bar (Windows: the notification area) and fetches your figures in the background. The Dock icon disappears until the window is opened again.
- **The menu at the raven icon:** Open Huginn · Since last time · Fetch now · a status line ("Fetching 3/16", "Fetched 12 min ago") · Quit.
- **Quit** in the menu (or ⌘Q while the window is open) closes the app completely.
- **Start at login** is turned on the first time a built app starts (not under `npm run tauri dev`). The app then starts without a window, only in the menu bar. Turn it off under **Settings** in the app. The app never turns it back on by itself.
- **Before you delete the app:** turn Start at login off in Settings. Otherwise the file `~/Library/LaunchAgents/com.wi24rd.huginn-muninn.plist` is left behind (Windows: the value under `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`), pointing to an app that no longer exists. It can also be deleted by hand.
- The app only runs once at a time; starting it again shows the window that is already there.

## "Since last time" and notifications
- **Since last time** is a view: yesterday's status for all your domains (Act, Watch, Calm) in one line each, with what is new since you last looked. It opens by itself from the menu bar (**Since last time**) or when you click a notification.
- **Notifications come from your own computer** (the system's local notification service), never from a server, e-mail or webhook. Choose under **Settings** (and when you connect the API Token): **Calm** (at most one message a day, only when a domain needs action), **Standard** (a message when a domain needs action, the default) or **Everything** (also for Watch). Each domain can also be set to Follow default, Serious and watch, Serious only or Never, and **Notify me when a domain needs action** can be turned off.
- **Looks targeted, Looks like reconnaissance and Looks like DNS reconnaissance** are labels on top of the status: they say something about the pattern, not the amount, and always as "looks", never as a certain verdict. "Looks targeted" notifies you by default; "Looks like reconnaissance", and with it the DNS label, can be turned on under **Settings → Notifications**. "Looks targeted" compares only with your chosen domains, needs at least 3, and notifies you only once the day after is fetched. Until then it stands as preliminary.
- **Date format** is chosen under **Settings → General**: Follow the language, Follow the system or ISO. It applies to all dates, including the notifications.
- Act means unusually many attacks yesterday compared with the week before. Cloudflare's routine (the labyrinth, Bot Fight Mode, the browser check) and old figures never notify you, and each domain is reported at most once a day.
- macOS asks for permission the first time a notification is shown. **To turn them off completely:** System Settings → Notifications → Huginn and Muninn.

## Build (development)
```
npm install
npm run tauri dev
npm run test:ui              # the UI smoke test (jsdom)
node design/build-tokens.mjs # tokens.css (and XAML) from design/wi24rd-tokens.json
```
Requires Rust (rustup) and Node 20+.

## Licence
Apache-2.0 — see [LICENSE](LICENSE) and [NOTICE](NOTICE). Third-party components: [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

Cloudflare is a registered trademark of Cloudflare, Inc. Huginn and Muninn is not affiliated with Cloudflare, Inc.

[Dansk](README.da.md)
