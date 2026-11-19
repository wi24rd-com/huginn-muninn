# Installation

> **Status today:** Version 1.1.0 is available for macOS on a Mac with Apple silicon. It is signed and notarised by Apple, and you can check what you downloaded yourself. There is no version for Windows or Linux yet.

## macOS

Requires macOS 11 or newer and a Mac with Apple silicon.

1. Download `Huginn-Muninn-1.1.0.zip` from the release on GitHub.
2. Open the zip file. It unpacks **Huginn and Muninn**.
3. Drag **Huginn and Muninn** to your Applications folder (`/Applications`).
4. Open the program.

The program is signed with Developer ID and notarised by Apple. macOS therefore opens it without the warning about an unknown developer. The first time, macOS may ask whether you want to open an application downloaded from the internet.

## Check what you downloaded

If you want to see for yourself that the program is the genuine one, run the four commands in Terminal when the program is in Applications and the zip file is in Downloads:

```
codesign --verify --deep --strict --verbose=2 "/Applications/Huginn and Muninn.app"
codesign -dv "/Applications/Huginn and Muninn.app" 2>&1 | grep TeamIdentifier      # TeamIdentifier=5DAVNFA355
spctl --assess --type execute --verbose "/Applications/Huginn and Muninn.app"      # accepted, source=Notarized Developer ID
shasum -a 256 ~/Downloads/Huginn-Muninn-1.1.0.zip
```

The first three lines check Apple's signature and notarisation. If the release notes list a SHA-256, the last line must give the same value. The app's name has spaces, so the path must be in quotation marks as above.

## Build the program yourself

You can also build the program from the source code. You need Rust (installed with rustup) and Node 20 or newer. In a copy of the source code:

```
npm install
npm run tauri build
```

The result is **Huginn and Muninn.app**. A version you build yourself is not signed unless you sign it with your own Developer ID.

If you only want to try the program, you can run `npm run tauri dev`. It then starts in development mode; in that mode the program does not turn **Start at login** on (see below).

## Windows

The program is built to run on Windows (the same shortcuts with Ctrl, the notification area instead of the menu bar, Credential Manager instead of the Keychain), but there is no Windows version yet, and it has not been tried.

## Linux

There is no Linux version.

## What happens the first time you open the program

1. A guide with four steps opens: **Welcome**, **Your API Token**, **Your domains** and **Notifications**. The tabs at the top cannot be used until an API Token is saved. See [Setup](30-opsaetning.md).
2. The first time a built version starts, the program turns **Start at login** on. That means the program starts without a window, only in the menu bar, when you log in to the computer, and fetches your figures in the background. You can turn it off in step 4 of the guide or under **Settings** (the gear at the top right) in the group **Start-up**; the program never turns it back on by itself.
3. The program creates its data folder and the local database. See [Data and history](70-data.md).
4. The first time the program shows a notification, macOS asks whether it may. See [Notifications and background](60-notifikationer.md).

The program only runs once at a time. If you start it again, the window that is already there is shown.

## What is put on your computer

| What | Where (macOS) | When |
|---|---|---|
| The application | `/Applications/Huginn and Muninn.app` (if you put it there) | At installation |
| Data folder with the database | `~/Library/Application Support/com.wi24rd.huginn-muninn/` | First start |
| API Tokens | macOS Keychain under `com.wi24rd.huginn-muninn`, one item per connection | When you connect or add an API Token |
| Start at login | `~/Library/LaunchAgents/com.wi24rd.huginn-muninn.plist` | First start of a built version |
| Daily start | `~/Library/LaunchAgents/com.wi24rd.huginn-muninn.dagligt.plist` | Together with Start at login |

How to remove all of it again is in [Uninstalling](90-afinstallation.md).
