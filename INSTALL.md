# Installation & Sideloading Guide

> [!NOTE]
> **Windows System Requirements**: Windows 10 (64-bit) or Windows 11 (64-bit) is required for the Desktop GUI and pre-built binaries. (Windows 7 / 8.1 are no longer supported by modern Rust runtimes and WebView2).

This guide covers building the **omml-converter** CLI & Desktop Suite from source, running the application, and sideloading the **Microsoft Word** and **WPS Office** Add-ins on Windows, Linux, and macOS.

---

## 1. Prerequisites

- **Rust toolchain 1.75+** (`curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`)
- **Node.js** (Optional, only required if rebuilding web assets)
- **Platform Dependencies:**
  - **Linux (Ubuntu/Debian):**
    ```bash
    sudo apt-get update
    sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libgtk-3-dev
    ```
  - **Linux (Fedora/RHEL):**
    ```bash
    sudo dnf install -y webkit2gtk4.1-devel libappindicator-gtk3-devel librsvg2-devel gtk3-devel
    ```
  - **Windows:** Microsoft Visual Studio C++ Build Tools or Windows SDK.
  - **macOS:** Xcode Command Line Tools (`xcode-select --install`).

---

## 2. Building from Source

### CLI Binary (`omml-converter`)
```bash
cargo build --release
# Compiled executable located at: target/release/omml-converter
```

### Desktop GUI App (`omml-editor-desktop`)
```bash
cd src-tauri
cargo build --release
# Executable located at: target/release/omml-editor-desktop
```

---

## 3. One-Click Add-in Registration Scripts

The `scripts/` directory provides automated one-click registration for both Word and WPS Office:

### On Linux & macOS
Run the bash script:
```bash
# Install / Register Add-ins:
./scripts/install-addin.sh

# To remove / uninstall:
./scripts/install-addin.sh --uninstall
```

### On Windows
Run PowerShell or double-click the `.bat` file:
```powershell
# In PowerShell (Run as Administrator or Current User):
.\scripts\install-addin.ps1

# Or double-click:
scripts\install-addin.bat
```

---

## 4. Manual Sideloading Instructions

### A. Microsoft Word (Windows)
1. Ensure the desktop application or local bridge is running on port `17890`.
2. Open Windows Registry Editor (`regedit`) or run in PowerShell:
   ```powershell
   New-Item -Path "HKCU:\Software\Microsoft\Office\16.0\WEF\Developer" -Force
   Set-ItemProperty -Path "HKCU:\Software\Microsoft\Office\16.0\WEF\Developer" -Name "MathEditor" -Value "C:\path\to\omml-converter-rust\word-addin\manifest.xml"
   ```
3. Restart Microsoft Word.
4. Go to **Insert** ➔ **My Add-ins** ➔ **Developer Add-ins**, select **Math Equation Editor**. The **Math Editor** ribbon tab will appear with all 5 commands.

### B. Microsoft Word (macOS)
1. Copy the manifest file to Word's container WEF directory:
   ```bash
   mkdir -p ~/Library/Containers/com.microsoft.Word/Data/Documents/wef
   cp word-addin/manifest.xml ~/Library/Containers/com.microsoft.Word/Data/Documents/wef/manifest-matheditor.xml
   ```
2. Restart Microsoft Word for Mac.
3. In Word, navigate to **Insert** ➔ **Add-ins** ➔ **Developer Add-ins** to activate the Ribbon.

### C. Microsoft Word Online (Office on the Web)
1. Open any document in [Word Online](https://office.live.com/).
2. Click **Insert** ➔ **Add-ins** ➔ **More Add-ins**.
3. In the dialog, click **Manage My Add-ins** (or **Upload My Add-in**) ➔ **Upload**.
4. Browse and select `word-addin/manifest.xml`.

### D. WPS Office (Windows & Linux)
1. Copy the `wps-addin/` folder into your WPS jsaddons directory:
   - **Linux:** `~/.local/share/Kingsoft/wps/jsaddons/MathEditor/`
   - **Windows:** `%APPDATA%\Kingsoft\wps\jsaddons\MathEditor\`
2. Ensure `publish.xml` exists in the parent `jsaddons/` directory:
   ```xml
   <jsaddons>
     <jsaddon name="MathEditor" enable="true" />
   </jsaddons>
   ```
3. Restart WPS Office Writer. The **Math Editor** tab will automatically appear on the top Ribbon bar.

---

## 5. Troubleshooting & FAQ

- **Bridge connection shows "Bridge: Offline":**
  - Verify that the desktop app or local server is running.
  - Test via browser: Open `http://127.0.0.1:17890/health`. It should return `{"status":"ok","server":"omml-converter-bridge"}`.
- **Port 17890 is already occupied:**
  - You can run the bridge on a custom port via CLI: `omml-converter --bridge --port 17891`.
- **Linux WebKit GStreamer Warnings:**
  - The app automatically disables audio effects in MathLive to prevent GStreamer audio pipeline logs on Linux.
- **License & Security:**
  - 100% pure Rust and offline web components.
  - Zero external cloud calls, no telemetry, all conversions occur strictly on `127.0.0.1`.
