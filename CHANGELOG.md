# Changelog

## 0.1.3

### New
- **Profile page.** Every profile now has its own page with an overview, a mod manager and a screenshot gallery.
- **Mod manager.** See every mod in a profile, which ones XT keeps up to date and which you added, and switch any of them off without deleting it.
- **Modrinth search.** Find and install mods from inside a profile. Results are filtered to the profile's loader and Minecraft version, and required dependencies are added for you.
- **Screenshots.** The latest screenshots of a profile in a strip, with open, copy and delete (to the Recycle Bin), plus an Open folder button.
- **Profile icons and colours.** Give each profile a block icon or your own picture, and an accent colour, so they are easy to tell apart.
- **Storage.** Settings has a new Storage tab that shows where the disk space goes and cleans up Java runtimes and game versions no profile uses any more.
- **Repair profile.** Re-checks every file of a profile against its checksum and downloads anything broken. Use it when the game won't start after a crash.
- **What's new.** After an update the launcher shows what changed, once.

### Improved
- **Deleting a profile now deletes its files too** (worlds, mods, screenshots), so no space is left behind. Leftovers from profiles deleted earlier show up in Settings > Storage, where you can remove them.
- Fabric, Quilt, Forge and NeoForge profiles show their loader logo, and the profile editor picks the loader from logo cards. XT Optimized keeps the rocket.
- A redesigned Ely.by sign-in page in the browser, and a livelier skin preview on the account page that follows your pointer.
- Each profile shows when it was last played.
- Drag profiles and accounts to put them in the order you like (Alt+Up and Alt+Down work too).
- The memory slider tells you how much RAM is free right now.
- First run: on a PC with 8 GB of memory or less, XT Launcher suggests XT Optimized with a lower memory setting.

### Safer
- XT Launcher no longer starts helper programs in the background (PowerShell, reg, taskkill). Hardware detection, the graphics card setting, stopping the game and the Recycle Bin all use Windows directly, which also fixes Windows Defender wrongly flagging the installer.
- The game runs on javaw, so no console window is ever created.
- Every program the launcher downloads and runs is checked first: authlib-injector against its published SHA-256, Forge and NeoForge installers against the checksums on their Maven servers.
- Opening the game folder with no profile no longer creates an empty "default" folder.

## 0.1.2

### Improved
- After launch has three clear options: keep the launcher open, hide it until Minecraft closes, or close it.
- Hiding really hides: no taskbar button and almost no CPU use while you play.

## 0.1.1

### New
- Pick the exact Fabric, Quilt, Forge or NeoForge version for a profile, with version lists fetched from each loader.
- Caves & Cliffs is the default wallpaper.
