# Uninstalling

The program puts five things on your computer: the application itself, two autostart entries, an API Token in the keychain and a data folder. Follow the steps in order so nothing is left behind.

## macOS

### 1. Turn Start at login off in the program

Open the program and turn **Settings → General → Start at login** off. That removes the files `~/Library/LaunchAgents/com.wi24rd.huginn-muninn.plist` and `~/Library/LaunchAgents/com.wi24rd.huginn-muninn.dagligt.plist`.

If you forget, the files are left behind and point to an application that no longer exists. They can also be deleted by hand:

```
rm ~/Library/LaunchAgents/com.wi24rd.huginn-muninn.plist
rm ~/Library/LaunchAgents/com.wi24rd.huginn-muninn.dagligt.plist
```

### 2. Forget the API Token

Choose **Settings → Connection → Forget API Token…** and confirm with **Forget**. That deletes the API Token from macOS Keychain.

To do it by hand: open macOS's keychain application (Keychain Access), search for `com.wi24rd.huginn-muninn`, and delete the item.

Also consider **revoking the API Token at Cloudflare**: log in, open **My Profile › API Tokens**, and delete the token with the name you gave it. Then it cannot be used by anyone, whatever happens.

### 3. Quit the program

Choose **Quit** in the menu at the raven icon in the menu bar, or ⌘Q while the window is open. The raven icon must be gone.

### 4. Delete the application

Drag **Huginn and Muninn.app** from the Applications folder to the Bin.

### 5. Delete the data folder

The history is in the data folder and is not deleted by itself. If you want it gone:

```
rm -r ~/Library/Application\ Support/com.wi24rd.huginn-muninn
```

If you want to keep the history for later, leave the folder. If you install the program again, it finds it.

### 6. Notifications

macOS remembers your notification permission under **System Settings → Notifications**. The entry disappears once the application is deleted, or can be removed there.

## Windows

There is no Windows version yet. When it comes, the places are:

| What | Where |
|---|---|
| Autostart | The value under `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run` (turn **Start at login** off in the program first) |
| Daily start | The scheduled task `Wi24rd Huginn and Muninn dagligt` (turn **Start at login** off in the program first) |
| API Token | Windows Credential Manager, the item `com.wi24rd.huginn-muninn` |
| Data folder | `%APPDATA%\com.wi24rd.huginn-muninn\` |

## Linux

There is no Linux version yet. The data folder will be in `~/.local/share/com.wi24rd.huginn-muninn/`.

## Checklist

- [ ] Start at login turned off (no `.plist` in `~/Library/LaunchAgents/`)
- [ ] API Token forgotten in the program (and revoked at Cloudflare)
- [ ] Program quit (no raven icon in the menu bar)
- [ ] Application deleted from Applications
- [ ] Data folder deleted, or deliberately kept
