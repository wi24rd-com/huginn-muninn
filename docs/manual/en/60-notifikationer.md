# Notifications and background

## The program keeps running when the window is closed

When you close the window, the program keeps running in the **menu bar** (Windows: the notification area) and fetches your figures in the background. On macOS the Dock icon disappears until the window is opened again. That is how the history stays without gaps: the program only needs to be running.

The menu at the raven icon in the menu bar:

| Item | Does |
|---|---|
| **Open Huginn** | Opens the window. |
| **Since last time** | Opens the window on [Since last time](40-visninger/siden-sidst.md). |
| **Fetch now** | Fetches new figures at once. |
| The status line | For example "Fetching 3/16" or "Fetched 12 min ago". Cannot be clicked. |
| **Quit** | Closes the program completely. Nothing is fetched until you start it again. |

**⌘Q** while the window is open also closes the program completely. Clicking the red close icon only closes the window.

**Start at login** (under **Settings** in the group **Start-up**) makes the program start without a window, only in the menu bar, when you log in to the computer. It is turned on the first time a built version starts, and the program never turns it back on by itself if you turn it off. While it is on, the program also starts by itself once a day, at 12:00, without a window, fetches and closes again if it was not already running. If it is off, the program only fetches while it is open; open it at least every three weeks, or gaps will appear in the history, because Cloudflare keeps only a month.

## Where the notifications come from

The notifications come from your own computer's notification service. Never from a server, an e-mail or a webhook, because there is no server. That also means notifications only come while the computer is on and the program is running.

macOS asks for permission the first time a notification is shown. If you say no, you can turn them on again under **System Settings → Notifications → Huginn and Muninn**. The same place turns them off completely.

Clicking a notification opens **Since last time**.

## When you are told

You choose the overall level in setup and under **Settings** in the group **Notifications**:

| Level | When |
|---|---|
| **Only when serious, at most once a day** (Calm) | One combined message a day, only at **Act**, and not before 03:00 UTC, when the night's figures are in place. |
| **When serious (recommended)** (Standard) | A message after every fetch where a domain has gone to **Act**. |
| **Also when a domain should be watched** (Everything) | As Standard, and also at **Watch**. If a domain goes from Watch to Act the same day, one more message comes. |

Per domain you can choose **Follow default**, **Serious and watch** (as Everything), **Serious only** (as Standard) or **Never**. **Notify me when a domain needs action** can be turned off if you only want to see Act in the program.

The rules behind it:

- **Act** means unusually many attacks yesterday compared with the week before, or that something got through. Cloudflare's routine (the labyrinth, Bot Fight Mode, the browser check) and old figures never notify you.
- Each domain is reported **at most once per status day**, so the night (when the status uses the day before) never reports the same thing again.
- A domain without figures (Unknown) never notifies you.

## What the notifications can say

The text is the same as under **Why** in the Overview, in the program's language and your date format. Paths and passwords are never in a notification, and IP addresses do not exist in the program.

| Message about | When | Level |
|---|---|---|
| **Unusually many attacks** | Yesterday's attacks against the week before, with what they went for. | Act / Watch |
| **Looks targeted** | The attacks rose here but not on your other domains, and a large share came from one network. Compares only with your chosen domains, needs at least 3, and notifies you only once the day after is fetched. Until then it is preliminary. Also comes on a calm domain. | Standard and Everything |
| **Looks like reconnaissance** and **Looks like DNS reconnaissance** | Many different paths, or many lookups of names that do not exist, in one day. Only the first day of an episode. | Only if **Notify me of “Looks like reconnaissance”** is on |
| **Your server is not answering** | A large part of yesterday's responses were errors because Cloudflare could not reach or find your server (errors 520–527 and 530). If three or more hours in a row had errors, the message names the time span in your local time: "between 01.00 and 05.00". | Follows the status |
| **Unusually much traffic** | Far more requests than usual in one day. | Follows the status |
| **Mail in your domain's name** | A new sender failed DMARC, or the share rose markedly. Needs DMARC Management at Cloudflare. | Watch |
| **Passwords from known leaks** | Login attempts with leaked passwords. At least Watch; Act if the server answered 2xx or 302. | Follows the status |
| **The domain's state** | The domain is paused, not active, suspected of phishing or in Development Mode. | Follows the status |
| **More attacks on a technology** | Only if you have chosen **Notify me** on the technology under [Technology by domain](40-visninger/teknologi.md). Reports whatever the level, but not for a domain set to Never. | Whatever the level |

## What do I do when …

- **… you get too many messages?** Choose **Only when serious, at most once a day**, or set individual domains to **Serious only** or **Never**.
- **… you get no messages?** Check that the program is running (the raven icon in the menu bar), that **Start at login** is on, that macOS allows notifications for the program, and that the domain is not set to **Never**. Calm never notifies you; that is the point.
- **… you would rather have an e-mail?** The program cannot do that. It has no server, and that is on purpose: nothing leaves your computer.
