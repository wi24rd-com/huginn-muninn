# Updating

> **Status today:** The program has no built-in updater yet. It does not check for new versions by itself and fetches nothing from Wi24rd. Automatic updates from a public repository, where everything downloaded is signed and checked, are planned.

## How to update now

1. Choose **Quit** in the menu at the raven icon (or ⌘Q while the window is open), so the program is closed completely.
2. Download the new version, and replace **Huginn and Muninn** in your Applications folder.
3. Open the program again.

Your history, your settings and your API Token stay where they are. Replacing the application does not touch the database or the keychain.

## The database is upgraded on first start

Version 1.1.0 upgrades the local database by itself the first time it starts. Nothing is lost: your API Token becomes connection 1 with all your choices.

The figures per subdomain in Visitors and Attacks and the errors and cache in Health arrive during the first while after the update: they are fetched at the slow pace after the other figures, about 30 days back. Until then **–** is shown, which means "not fetched yet", not 0.

Afterwards version 1.0.0 cannot open the database. It says "The local database is from a newer version of the program (schema 21)." and leaves the file alone.

**Might you want to go back to 1.0.0?** Then take a copy of the database file `huginn-muninn.sqlite` before you open the new version for the first time. The program must be closed while you copy. Where the file is, is in [Data and history](70-data.md).

## If the database is newer than the program

If you open an older version of the program after having run a newer one, the program says "The local database is from a newer version of the program". Then you need to go back to the newer version; the file is not touched.

## Which version do I have?

The **i** button at the top right (**About Huginn and Muninn**) shows the version. It is also shown at the bottom right of the window. Releases are listed in `CHANGELOG.md` in the source code.
