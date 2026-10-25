==============================================================================
  Flip 3D (Vista Style) - Standalone Portable Edition (Windows 10 & 11)
==============================================================================

Original Windhawk mod by caliberda (MIT License)
Ported to standalone portable Rust by Vigi Riyantanu, jundaee & Ahmad Baihagi

------------------------------------------------------------------------------
QUICK START
------------------------------------------------------------------------------
1. Double-click `flip3d.exe` to launch the background window switcher.
2. An icon will appear in your system notification tray.
3. Press Alt+Tab (or Win+Tab, Ctrl+Alt+Tab depending on your configuration)
   to switch windows in fluid 3D / 2D layouts.
4. Double-click the tray icon or right-click -> Settings to open the settings panel.
5. Right-click the tray icon and choose "Exit" to close Flip 3D anytime.

------------------------------------------------------------------------------
PORTABLE MODE & DATA STORAGE
------------------------------------------------------------------------------
This package runs in self-contained Portable Mode because `flip3d.portable`
is present in this directory.

All application data is isolated inside the local `data\` folder:
  - `data\config.json`: Your customized switcher settings
  - `data\logs\flip3d.log`: Application log file
  - `data\webview2\`: Isolated WebView2 browser data for the settings UI

No registry keys or external files are modified, unless you explicitly enable
"Start with Windows" in the Settings UI or tray menu.

------------------------------------------------------------------------------
WEBVIEW2 RUNTIME
------------------------------------------------------------------------------
The settings panel uses Microsoft Edge WebView2, which is pre-installed on
Windows 11 and Windows 10 (version 2004+).
If your system lacks WebView2, you can install it using the installer in:
  `redist\MicrosoftEdgeWebview2Setup.exe`

------------------------------------------------------------------------------
UPGRADING
------------------------------------------------------------------------------
To upgrade to a newer version of Flip 3D:
1. Exit the running `flip3d.exe`.
2. Replace `flip3d.exe` and `settings-ui.exe` with the new executables.
3. Your configuration and data in `data\` will remain intact.

------------------------------------------------------------------------------
UNINSTALLATION
------------------------------------------------------------------------------
Simply delete this folder. Nothing is left behind on your system.
==============================================================================
