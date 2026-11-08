# Glossary

**Act** · The status when there were unusually many attacks yesterday compared with the week before, or something got through. Notifies you by default.

**API Token** · A key you create in your Cloudflare account that lets a program do certain things. The program's API Token can only read: **Zone › Read** and **Analytics › Read**.

**Attacks** · Requests Cloudflare's rules stopped because they looked like an attack: known holes, your own rules, DDoS. Routine (bot checks) is not counted.

**Blocked** · Cloudflare refused the request outright.

**Bot Fight Mode** · Cloudflare's automatic protection against robots. Routine; never gives Watch or Act.

**Browser check (challenged)** · Cloudflare asked the visitor to prove it is a real browser. A robot cannot.

**Calm** · The status when nothing is unusual.

**CVE** · A number for a known security hole, for example CVE-2025-55182. A click opens the hole's page on cve.org.

**Data centre** · One of Cloudflare's many places in the world that your site answers from. Attacks are stopped in the data centre nearest the attacker.

**DMARC** · A rule for who may send mail in your domain's name. Mail that fails DMARC may be someone pretending to be you, or a service you use yourself but have not set up.

**DNS reconnaissance** · Someone asks for many names under your domain that do not exist, to find servers and services.

**Drill-down** · The old name for the filters in Attacks: the saved figures for exactly what you clicked.

**Edge – first line of defence** · Cloudflare's network of data centres. Your site answers from all of them (anycast), so an attack is stopped near the attacker, long before it reaches your server. Only applies to DNS records with the orange cloud.

**Gap** · A day the program never fetched, typically because the computer was off. Never drawn as 0.

**Huginn** · Thought. What is happening now: attacks, robots, login.

**In disguise** · A robot calling itself Googlebot, say, without being it.

**Keychain** · The place your operating system keeps passwords safely: macOS Keychain or Windows Credential Manager. This is where the API Token lives.

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

**Unknown** · The status when there are no figures yet.

**UTC** · Universal time. All days in the program are whole days in UTC, so "yesterday" ends at a different local hour depending on where you are.

**Watch** · The status when the pattern is unusual, but everything was stopped.

**Zone** · Cloudflare's word for a domain on your account.
