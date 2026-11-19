# Wi24rd Huginn and Muninn – user guide

*for Cloudflare®*

Huginn and Muninn is a program for your own computer that shows what is actually happening against **your own** Cloudflare domains: attacks, robots and login attempts, on one screen and explained in plain language. The program talks to Cloudflare only with an API Token that can only read, and keeps the figures on your computer, so the history outlives Cloudflare's month.

- **Huginn (thought)** is what is happening now: the attacks, what was stopped and how.
- **Muninn (memory)** is what has happened: every finished day is kept locally.

> **Status:** version 1.1.0 for macOS (Apple silicon). The program is signed with Developer ID and notarised by Apple. This guide follows that version.

## How the guide is organised

| Chapter | Read it when you … |
|---|---|
| [Before you start](10-foer-du-starter.md) | want to know what the program does, does not do, and what it needs |
| [Installation](20-installation.md) | need the program on your computer and want to check what you downloaded |
| [Setup](30-opsaetning.md) | need to connect the program to Cloudflare and choose domains |
| [Create an API Token at Cloudflare](31-cloudflare.md) | want to see step by step, with screenshots, what to do in Cloudflare's dashboard |
| [The views](40-visninger/overblik.md) | use the program day to day: Overview, Since last time, Attacks, Robots, Login, your country, Visitors, Health, Technology |
| [Settings](50-indstillinger.md) | want to change language, appearance, notifications or the API Token, or choose domains |
| [Notifications and background](60-notifikationer.md) | want to understand when the program speaks up, and what happens when the window is closed |
| [Data and history](70-data.md) | want to know where the figures live, for how long, and how to reset |
| [Troubleshooting](80-fejlfinding.md) | see an error text in the program |
| [Updating](85-opdatering.md) | want a newer version |
| [Uninstalling](90-afinstallation.md) | want to remove the program completely, without leftovers |
| [Keyboard shortcuts](95-genveje.md) | want to drive the program from the keyboard |
| [Glossary](99-ordliste.md) | meet a word you do not know |

## The views

| View | Read it when you … |
|---|---|
| [Overview](40-visninger/overblik.md) | want the status of all your domains on one page and to know if anything needs action |
| [Since last time](40-visninger/siden-sidst.md) | want to see what happened yesterday and which domains need action or watching |
| [Attacks](40-visninger/vaernet.md) | want to see what Cloudflare stopped at the Edge, how, by which rules and from where |
| [Robots](40-visninger/robotter.md) | want to know which search engines, AI robots and other bots read your site |
| [Login](40-visninger/login.md) | want to see the attempts on your site's login pages and how many reached your server |
| [Your country now and the World](40-visninger/land.md) | want to compare the attacks on your domains with your country and the world |
| [Visitors](40-visninger/besoegende.md) | want to see how many visit your site, from where, and which pages they see |
| [Health](40-visninger/drift.md) | want to know whether your website answered and how much Cloudflare handled itself |
| [Drill-down](40-visninger/dyk.md) | want to dig into a figure, a country, a rule, a robot or a path on Attacks |
| [Technology by domain](40-visninger/teknologi.md) | want to tell the program what your site runs, so the status hits the right attacks |
| [Connect to Cloudflare (setup)](40-visninger/opsaetning.md) | need to connect the program to Cloudflare the first time with the guide |
| [Settings (the view)](40-visninger/indstillinger.md) | want to find language, appearance, domains, notifications, API Tokens or About |

## The short version

1. Install the program and open it. A guide with four steps opens by itself.
2. Create an API Token at Cloudflare with the two read permissions the program asks for, and paste it in step 2 of the guide.
3. In step 3, choose which domains Huginn should fetch and show. Later you change it on **Overview** with the button **Choose domains**.
4. Close the window. The program stays in the menu bar, fetches in the background and tells you when a domain needs action.

The rest is for those who want to understand more. Everything in the program that has figures behind it can be clicked, so you can see what the figure is made of.

## The window

At the top is a title bar with the program's name, back and forward, **Guide** (this guide on wi24rd.com in your browser), **?** (help: the explanations for the view and the shortcuts), the gear (**Settings**) and **i** (About). Below it are the tabs and a band with the buttons for the view you are in, with **Fetch now** first. On the left is the list **Domains**, where you choose one domain or **All domains**. The footer at the bottom shows the status, for example "Fetching 3 of 7" or "Updated 08:01", and the version.

![The help panel opened over the Overview](../img/hjaelp--light-en-1200x860.webp)
*The **?** button at the top right opens the help for the view you are in and lists the shortcuts. **Open the guide** at the top of the help opens this guide on wi24rd.com in your browser. Esc closes it again.*

## Licence and trademark

The program is open source under the Apache-2.0 licence. Cloudflare® is a registered trademark of Cloudflare, Inc. Wi24rd Huginn and Muninn is not affiliated with Cloudflare, Inc.
