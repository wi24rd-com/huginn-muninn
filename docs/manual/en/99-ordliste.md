# Glossary

**Act** · The status when there were unusually many attacks yesterday compared with the week before, or something got through. Notifies you by default.

**API Token** · A key you create in your Cloudflare account that lets a program do certain things. The program's API Token can only read: **Zone › Read** and **Analytics › Read**.

**Attacks** · Requests Cloudflare's rules stopped because they looked like an attack: known holes, your own rules, DDoS. Routine (bot checks) is not counted.

**Blocked** · Cloudflare refused the request outright.

**Bot Fight Mode** · Cloudflare's automatic protection against robots. Routine; never gives Watch or Act.

**Browser check (challenged)** · Cloudflare asked the visitor to prove it is a real browser. A robot cannot.

**Cache (Cloudflare's store)** · Cloudflare stores copies of your files (images, CSS, JavaScript) and can answer with them without asking your website. The pages themselves are not stored by default. [Health](40-visninger/drift.md) shows how often the answer came from the store. The cache covers the whole domain.

**Calm** · The status when nothing is unusual.

**Connection** · One API Token and a name in the program, one per Cloudflare account. The name is the account's name at Cloudflare until you give the connection your own with **Rename…**. The domains from all connections are shown together.

**CVE** · A number for a known security hole, for example CVE-2025-55182. A click opens the hole's page on cve.org.

**Data centre** · One of Cloudflare's many places in the world that your site answers from. Attacks are stopped in the data centre nearest the attacker.

**DMARC** · A rule for who may send mail in your domain's name. Mail that fails DMARC may be someone pretending to be you, or a service you use yourself but have not set up.

**DNS reconnaissance** · Someone asks for many names under your domain that do not exist, to find servers and services.

**Drill-down** · The old name for the filters in Attacks: the saved figures for exactly what you clicked.

**Edge – first line of defence** · Cloudflare's network of data centres. Your site answers from all of them (anycast), so an attack is stopped near the attacker, long before it reaches your server. Only applies to DNS records with the orange cloud.

**Errors 520–527 and 530** · Cloudflare's own response codes when Cloudflare could not reach or find your server; the program calls them "Could not be reached". 520: the server answered, but the answer could not be used. 521: the server refused Cloudflare. 522: Cloudflare could not connect in time. 523: Cloudflare could not find a route to the server. 524: the server took too long to answer. 525: the secure connection to the server could not be set up. 526: the server's certificate is not valid. 527 is not on Cloudflare's page of the errors, but counts. 530: Cloudflare could not look up the server's name. If they are a large part of the responses on a day, the Overview says **Your server is not answering**. They need a server: if your website is hosted at Cloudflare (Workers or Pages), there is none to reach, and the figure is 0. Responses 500–599 from the server itself are something else: the server was up, but a program on it failed. Those, and errors from a website hosted at Cloudflare, are counted in [Health](40-visninger/drift.md) under **Your website answered with an error**.

**Gap** · A day the program never fetched, typically because the computer was off. Never drawn as 0.

**Huginn** · Thought. What is happening now: attacks, robots, login.

**IP class** · Cloudflare's rough grouping of the IP addresses the traffic came from, for example search engine, monitoring, scanner, Tor or known bad address. The program saves only the number of requests per class and day, never the addresses.

**In disguise** · A robot calling itself Googlebot, say, without being it.

**Keychain** · The place your operating system keeps passwords safely: macOS Keychain or Windows Credential Manager. This is where the API Token lives, one per connection.

**Known robots** · Robots Cloudflare has verified, for example Googlebot. Robots that pretend to be an ordinary browser are not included.

**Labyrinth** · Cloudflare shows the robot fake pages that waste its time. Does not stop the robot and does not count as a block.

**Looks like reconnaissance** · A label when one day has many different paths, as when a scanner looks for what your site runs.

**Looks targeted** · A label when the attacks rose for you but not on your other domains, and a large share came from one network. Needs at least 3 chosen domains and is decided only once the day after is fetched.

**Muninn** · Memory. What has happened: the history on your computer.

**Network** · The provider (an autonomous system) a request came from. The program sees the network, never the IP address.

**Path** · The part of an address after the domain, for example `/wp-login.php`. Stored without the query part.

**Radar** · Cloudflare Radar, Cloudflare's public figures for the whole internet. Used in your country's view. Needs no permission.

**Refused** · Everything Cloudflare stopped: blocked, challenged or lured into the labyrinth, including routine. The status counts only real attacks.

**Routine** · Cloudflare's automatic bot checks: the labyrinth, Bot Fight Mode and the browser check. Counts as refused, but never as an attack.

**Sampling** · Cloudflare does not keep every event but a sample, where each event states how many it stands for. The program adds that up, so the number refused is Cloudflare's estimate, not a number of rows. It can therefore be higher than the number of single events. The program fetches one day at a time, so the sample covers as short a time as possible.

**Snoopers** · Requests for files that do not exist: scanners looking for known holes.

**Subdomain** · A name below your domain, e.g. `www.wi24rd.com` or `shop.wi24rd.com` below `wi24rd.com`. The domain itself (with nothing in front) is also called the *apex*. The program shows traffic and attacks per subdomain in [Visitors](40-visninger/besoegende.md) and [Attacks](40-visninger/vaernet.md) when the traffic goes through Cloudflare (orange cloud), not when the name is “DNS only”.

**Unique visitors** · The number of different IP addresses that visited the site on a day. Robots count, and several people behind the same router count as one. A sum over several days counts the same visitor once per day.

**Unknown** · The status when there are no figures yet.

**UTC** · Universal time. All days in the program are whole days in UTC, so "yesterday" ends at a different local hour depending on where you are.

**Visits** · Cloudflare's own definition: a page view that came from another site or a direct link. The same figure as "Visits" in Cloudflare's dashboard, and Cloudflare's estimate from a sample. Shown per country on Visitors.

**Watch** · The status when the pattern is unusual, but everything was stopped.

**Zone** · Cloudflare's word for a domain on your account.
