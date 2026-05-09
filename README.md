# Stream Pad

Stream Pad is a Windows desktop app for building your own quick-action panel.
Create profiles, add buttons, drag in apps, folders, sounds, and websites, then
launch them from a clean grid or list view.

It is built with Tauri, React, and Rust.

## Project Documents

- [MIT License](LICENSE)
- [Privacy Policy](PRIVACY.md)
- [Third-Party Notices](THIRD_PARTY_NOTICES.md)

## What It Does

- Create up to 15 profiles.
- Add up to 20 buttons per profile.
- Drag and drop `.exe` apps, folders, sound files, Windows shortcuts, and website URLs.
- Store button names, icons, and actions automatically when possible.
- Open apps, folders, URLs, and play assigned sound files.
- Switch between grid and list views.
- Select, duplicate, and delete multiple buttons.
- Minimize to the Windows tray.
- Check GitHub Releases for updates.

## Download And Install

Go to the latest GitHub Release and download one of these Windows files:

- `Stream.Pad_<version>_x64-installer.exe`

Use `Stream.Pad_<version>_x64-installer.exe` for the normal installation. It
shows the branded installer window and installs the latest version.

The release also includes `latest.json` and `Stream.Pad_<version>_x64-setup.exe`
for the built-in updater. Most users do not need to download those manually.

## How To Use

1. Open Stream Pad.
2. Create a profile.
3. Press `Add Button` or drag a supported item onto an empty button.
4. Edit the button name, icon, and action in the inspector panel.
5. Press a button to run its action.

Supported dropped items:

- Applications: `.exe`
- Folders
- Audio files: `.mp3`, `.wav`, `.ogg`, `.m4a`, `.flac`, `.aac`
- Website URLs
- Windows shortcuts: `.lnk`, `.url`, `.website`

## Tray Behavior

When Stream Pad is minimized to the tray:

- Double-click the tray icon to open the app.
- Right-click the tray icon for `Launch Stream Pad` and `Quit`.

## Updates

Stream Pad checks the GitHub Releases updater endpoint. When a newer version is
available, the app can download and install it through the built-in updater.

## Development

Install dependencies:

```powershell
npm install
```

Run the app in development:

```powershell
npm run tauri dev
```

Build the frontend:

```powershell
npm run build
```

Build the Windows app:

```powershell
npm run tauri build
```

Build the branded installer bootstrapper:

```powershell
cd installer-bootstrapper
cargo build --release
```

## Legal

Stream Pad is released under the MIT License. See `LICENSE`.

Third-party dependency information is listed in `THIRD_PARTY_NOTICES.md`.

Privacy notes are listed in `PRIVACY.md`.

Stream Pad is an independent project and is not affiliated with, endorsed by,
or sponsored by Elgato, CORSAIR, or any other stream controller product maker.
