# Troubleshooting

The program only shows its own error texts in plain language, never raw technical errors. Here is each text, what it means, and what you do.

## Error texts about the connection

| The text | Means | Do |
|---|---|---|
| **No connection to Cloudflare. Check your internet connection.** | The computer cannot reach `api.cloudflare.com`. | Check the network. The program tries again by itself and shows **Try again now**. |
| **No connection to Cloudflare, so … could not be fetched. Trying again by itself.** (in the band: **No connection · trying again**) | One or more domains could not be fetched because there was no network, for example right after the Mac has been asleep. | Nothing. The program starts a new round by itself after 2 minutes, and at once when it notices that the machine has been asleep. **Fetch now** tries straight away. |
| **Cloudflare is not answering right now.** | Cloudflare did not answer, or answered with a server error. | Wait. The program tries again by itself. |
| **Cloudflare asks us to wait a little (too many requests).** | Cloudflare has a limit on how many calls may come. | Nothing. The program pauses and says "Trying again at …" in the footer at the bottom of the window. |
| **Cloudflare Radar is not answering right now.** | Only the country view is affected. | Your own figures still work. The program tries again. |
| **Cloudflare answered with HTTP …** / **Cloudflare answered with an error: …** / **Unexpected answer from Cloudflare.** | An answer the program could not use. | Try **Fetch now**. If it keeps happening, it is worth reporting as a bug. |
| **Cloudflare could not deliver the figures for …** | One day could not be fetched. | The day becomes a gap, and the program tries again later, as long as Cloudflare still has the day. |
| **The figures from Cloudflare Radar are not up to date: … Your own figures still work.** | Radar did not answer. | Nothing. |
| **Fetching in the background has stopped: …** | Fetching stopped on an error. | Read the error in the text; typically an API Token that has expired. |

## Error texts about the API Token

| The text | Means | Do |
|---|---|---|
| **The field is empty. Paste your API Token.** | You clicked Save and connect with nothing in the field. | Paste the API Token. |
| **What was pasted does not look like a Cloudflare API Token (only letters, digits and symbols without spaces).** | Something else came along, such as a space or a line break. | Copy the API Token again from Cloudflare, with nothing around it. |
| **Cloudflare rejected the API Token. It may have expired or lack permissions.** | Cloudflare does not know the token, or it has expired. If Cloudflare rejects all your connections, it is the same as with one: fetching stops, and the status is red. | Create a new API Token following [Setup](30-opsaetning.md), and paste it under **Settings** (the gear) › **Connection** › **Change API Token…**. |
| **Cloudflare rejected the API Token “name”. Its domains are not fetched.** (red line on the Overview with the button **Change API Token…**; in the footer: **1 API Token rejected**) | You have more than one connection, and Cloudflare rejects the API Token on the connection named in the text. It has been deleted, has expired, or its permissions have been reduced. The other connections keep fetching. The menu at the raven icon names it too. | Create a new API Token at Cloudflare following [Setup](30-opsaetning.md). Click **Change API Token…** in the red line; it opens **Settings** at the connection's row. Paste the new API Token, and click **Save and connect**. |
| **This API Token sees the same domains as “name”. It was not added.** | You used **Add API Token…** (or the guide) with an API Token that sees exactly the same domains as the connection “name”, typically the same API Token once more. | Nothing. The connection already exists. If the API Token is to replace the old one, use **Change API Token…** in the connection's row. |
| **The API Token is not active (status: …). Create a new read-only API Token.** | The token is disabled or expired at Cloudflare. | Create a new one. |
| **The API Token cannot see any domains: it lacks the permission Zone › Read, or the domain is not under Include · All zones or Specific zone.** | The token may not see the zone list, or no zones are included. | Fix the token at Cloudflare: under **Policies**, **DNS & Zones** must have **Zone › Read**, and **Include** must cover the domain (**All zones** or **Specific zone**). |
| **The API Token lacks the permission Analytics › Read, so the program cannot read the figures.** | The token may see zones, but not figures. | Add **Analytics › Read** under **Analytics & Logs** at Cloudflare. |
| **No API Token is saved. Paste your API Token under Connect to Cloudflare.** | The keychain has no token (for example after **Forget API Token**). | Connect again. |
| **The zone does not exist, or the API Token has no access to it.** | A domain has been removed from the account, or the token no longer sees it. | Clear the domain with **Choose domains** on the Overview, or fix the token. |

## Error texts about the computer

| The text | Means | Do |
|---|---|---|
| **The keychain could not be used.** | The program could not read or write in macOS Keychain / Credential Manager. | Try again. If macOS asks for access to the keychain, allow it. |
| **The local database could not be used.** | The file in the data folder could not be opened or written. | Check that there is space on the disk, and that the data folder is not read-only. See [Data and history](70-data.md). |
| **The local database is from a newer version of the program (schema …).** | You opened an older version of the program with a newer database. | Update the program. The file is not touched. See [Updating](85-opdatering.md). |
| **Start at login could not be read or changed.** | The operating system refused to create or remove the autostart entry. | Try again. On macOS the entry is the file `~/Library/LaunchAgents/com.wi24rd.huginn-muninn.plist`; see [Uninstalling](90-afinstallation.md). |
| **An error occurred in the program.** | An error the program cannot explain itself. | Try again. If it happens again, it is worth reporting as a bug. |

## Other texts

| The text | Means |
|---|---|
| **Invalid zone ID.** / **Invalid date.** / **The filter in the address cannot be used.** | The address in the back/forward history was wrong. Go to the Overview and click your way forward again. |
| **You can have at most 20 login paths of your own per domain. Remove one before adding a new one.** | The limit for your own login paths is reached. |
| **“From” must be on or before “To”.** | The date picker got a period the wrong way round. |
| **not available** | Your Cloudflare plan lacks the fields for those figures (for example Login or "stopped at the Edge" per robot). The rest works. |

## My domains disappeared when I entered a new API Token

In version 1.0.0 the program could only have one API Token. If you used **Change API Token…** with an API Token for another Cloudflare account, it replaced the first one. Then the first account's domains and your choices for them disappeared. Figures already fetched were never deleted.

Now each account gets its own connection. For one more account, use **Add API Token…** under **Settings** › **Connection**. **Change API Token…** still replaces the API Token on one connection, and the form says so: "Do you have another account? Use Add API Token… instead."

If you have already changed it, add the first account again with **Add API Token…**. If you no longer have the API Token, create a new one at Cloudflare. Then choose the domains in the dialog **Domains**. Their figures are still there.

## A domain stands under two connections in Domains

If you have an API Token for the whole account and one for one of its domains, both connections see that domain. In the dialog **Domains** it stands under both, and under the newest one it says "Fetched with “name”": it is only fetched with the oldest connection's API Token. It is one choice; if you switch it off under one, it is off under both.

If the two connections have the same name (both have the account's name at Cloudflare), the program opens **Rename the connection** when the new one is added. Give it a name you can recognise it by, for example the customer's.

## Subdomains

| The text | Means | Do |
|---|---|---|
| **Cloudflare did not return subdomains for this domain.** (under **Subdomains** in Visitors or Attacks) | Cloudflare refused the query per subdomain for the domain, e.g. because the plan does not have the field. The domain’s own figures are fetched and shown as before, just not per subdomain. | Nothing. If Cloudflare refused the name in the main fetch of the attacks, the program asks without the name for the rest of the day (UTC) and tries with the name again the next day. If the period already has attacks per subdomain, they are shown. |
| **{n} days are not included: Cloudflare sent more rows than the program can fetch at once.** (below the table **Subdomains**) | Those days had so much traffic that Cloudflare’s answer hit the highest number of rows the program fetches at once. The program would rather save no figure per subdomain for those days than one that is too low. The domain’s own figures for the days are included as always. | Nothing. The sums in the table are without those days, so they can be lower than the domain’s figures for the period. |

**My subdomain is not there.** The section **Subdomains** only shows names with traffic in the chosen period, and only traffic that goes through Cloudflare. Check in Cloudflare’s DNS whether the name has the orange cloud. If it is “DNS only” (grey cloud), the traffic goes straight to the server, and Cloudflare has no figures for it. If the domain only has traffic on one name, the section is not shown at all. The program has figures per subdomain from about 30 days before the update; older days have no figures per subdomain, and then **Per subdomain from …** is shown.

## It is not an error

- **Status "Unknown"** or **"No figures yet – fetching"**: the domain was just chosen. Wait for the fetch.
- **"updating …"** next to the status at night (until 03:00 UTC): yesterday is not fully fetched yet.
- **The figures do not match Cloudflare's dashboard**: the dashboard shows sampled estimates and local time; the program shows whole days in UTC and only what has been fetched. Large differences over long periods are typically due to sampling.
- **A yellow line about a gap**: the program was off. See [Data and history](70-data.md).

## Report a bug

The program is open source. Bugs are reported in the source code's repository on GitHub. Never write your API Token, account ID or zone ID in a bug report.
