# iOS Screen Time Screenshot Processor

A desktop app for research teams that reads hourly usage out of iOS Screen Time
and Battery screenshots. Load a folder of screenshots, let the app detect the
device, crop the screenshot and read the 24 hourly bars with on-device OCR,
review and correct the values, then export CSV or JSON.

Everything happens on your computer. There is no account and no server, and no
screenshot or result is uploaded anywhere.

## Download

Get the latest version from the
[Releases page](https://github.com/uzaira0-internal-testing/ios-screen-time-desktop/releases/latest).

| System | File | Notes |
| --- | --- | --- |
| macOS 13.3 or later (Apple silicon and Intel) | `…_universal.dmg` | Open it and drag the app into Applications. Signed and notarized by Apple. |
| Windows 10 or 11 (64-bit) | `…_x64-setup.exe` | Installs for the current user. Signed. |
| Windows, managed installs | `…_x64_en-US.msi` | For IT deployment (Intune, Group Policy). Signed. |
| Linux (Debian, Ubuntu) | `…_amd64.deb` | `sudo apt install ./<file>.deb` |
| Linux (other distributions) | `…_amd64.AppImage` | `chmod +x <file>.AppImage`, then run it. |

The Windows installers download Microsoft Edge WebView2 if the computer does not
have it; Windows 10 and 11 normally already do.

## Updates

The app checks for a newer version at launch and every 15 minutes, and offers
to install it. Updates are signed, and the app refuses one whose signature does
not match. To stop the automatic checks, turn off **Settings → Privacy → Check
for Updates Automatically**; **Check for Updates…** in the app menu still works.
Each check asks GitHub for a small version file, so GitHub sees your IP address
and the app version. Nothing else leaves the computer.

## Where your work is kept

Screenshots, results and settings are stored by the app on this computer and
survive quitting and updates. **Settings → Workspace backup** exports everything to a
single file for backup or for moving to another computer. **Settings → Delete
all local data** erases it. Exports and workspace files can contain participant
identifiers; store them according to your study's data policy.

Uninstalling the app can leave this data behind. To remove it as well, delete
the app's data folder (`com.ios-screentime.desktop`): on macOS under
`~/Library/WebKit/` and `~/Library/Application Support/`, on Windows under
`%LOCALAPPDATA%`, and on Linux under `~/.local/share/`.

## Checking a download

Every installer is built by this repository's GitHub Actions workflow and has a
signed build provenance record. With the GitHub CLI:

```sh
gh attestation verify <file> --repo uzaira0-internal-testing/ios-screen-time-desktop
```

## Licenses

The app is licensed under the GNU Affero General Public License v3.0 or later.
Each release has its full source code attached as
`ios-screen-time-source-<version>.tar.gz` (Settings → About links to the one
for your version). Test screenshots taken from real devices, and data recorded
from them, are left out; the archive's `SOURCE-EXPORT.txt` lists what.

The app includes open-source software from other projects, among them
Tesseract, Leptonica, Tauri and React; their licenses are listed in the app
under **Settings → About → Third-party licenses**.

## Problems

Open an [issue](https://github.com/uzaira0-internal-testing/ios-screen-time-desktop/issues)
with your operating system, the app version (Settings → About) and what you
did. Do not attach screenshots or exports that contain participant data.
