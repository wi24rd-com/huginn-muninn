# Installation

> **Status today:** The program is not released. There are no finished installation packages to download yet, and it has only been built and tried on macOS. This chapter describes what applies now and what is planned. It will be updated when the first version is released.

## macOS

### Today: build the program yourself

You need Rust (installed with rustup) and Node 20 or newer. Then, in a copy of the source code:

```
npm install
npm run tauri build
```

The result is the application **Huginn and Muninn.app**. Drag it to your Applications folder (`/Applications`) and open it like any other application.

If you only want to try the program without building a finished version, you can run `npm run tauri dev`. It then starts in development mode; in that mode the program does not turn **Start at login** on (see below).

### Planned: a finished package

When the program is released, it will come as a package you download and open. Signing with Apple (notarisation) is planned but not in place yet. Until then, macOS will warn if you open a package downloaded from the internet, because it is not signed.

## Windows

The program is built to run on Windows (the same shortcuts with Ctrl, the notification area instead of the menu bar, Credential Manager instead of the Keychain), but there is no Windows version yet, and it has not been tried. Signing on Windows is planned.

## Linux

A Linux version is planned but does not exist yet.

## What happens the first time you open the program

1. The program shows **Connect to Cloudflare**, because no API Token is saved. Only **Settings** can be opened in the sidebar until you have connected. See [Setup](30-opsaetning.md).
2. The first time a built version starts, the program turns **Start at login** on. That means the program starts without a window, only in the menu bar, when you log in to the computer, and fetches your figures in the background. You can turn it off under **Settings → General → Start at login**; the program never turns it back on by itself.
3. The program creates its data folder and the local database. See [Data and history](70-data.md).
4. The first time the program shows a notification, macOS asks whether it may. See [Notifications and background](60-notifikationer.md).

The program only runs once at a time. If you start it again, the window that is already there is shown.

## What is put on your computer

| What | Where (macOS) | When |
|---|---|---|
| The application | `/Applications/Huginn and Muninn.app` (if you put it there) | At installation |
| Data folder with the database | `~/Library/Application Support/com.wi24rd.huginn-muninn/` | First start |
| API Token | macOS Keychain, the item `com.wi24rd.huginn-muninn` | When you connect |
| Start at login | `~/Library/LaunchAgents/com.wi24rd.huginn-muninn.plist` | First start of a built version |

How to remove all of it again is in [Uninstalling](90-afinstallation.md).
