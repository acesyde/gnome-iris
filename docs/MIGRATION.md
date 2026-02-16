# Documentation - ReShade Linux Installer

## Overview

This Bash script installs and manages ReShade for Windows games running under Wine or Proton on Linux. It automates downloading ReShade, managing shaders, and the configuration needed to integrate ReShade into games.

## Prerequisites

The script requires the following programs to be installed on the system:

- `grep`: Used for text processing and pattern matching
- `7z`: Used to extract exe archives
- `curl`: Used to download files from the Internet
- `git`: Used to clone and update shader repositories
- `wine`: Required only for Vulkan games (to modify the Windows registry)

## Script Architecture

### Directory Structure

By default, all files are stored under `$HOME/.local/share/reshade` (or `$XDG_DATA_HOME/reshade`).

```
$MAIN_PATH/
├── reshade/                    # ReShade versions
│   ├── latest/                 # Symbolic link to the latest version
│   ├── 5.0.2/                  # Specific versions
│   └── ...
├── ReShade_shaders/            # Cloned shader repositories
│   ├── Merged/                 # Merged shaders (if MERGE_SHADERS=1)
│   │   ├── Shaders/
│   │   └── Textures/
│   ├── sweetfx-shaders/
│   ├── martymc-shaders/
│   └── ...
├── External_shaders/           # Custom shaders added manually
├── d3dcompiler_47.dll.32       # 32-bit DirectX DLL
├── d3dcompiler_47.dll.64       # 64-bit DirectX DLL
├── ReShade.ini                 # Global configuration (if GLOBAL_INI=1)
├── LVERS                       # Version tracking file
└── LASTUPDATED                 # Last update timestamp
```

### Main Functions

#### 1. `printErr()`
**Parameters:**
- `$1`: Error message
- `$2`: Exit code (optional, default: 1)

**Purpose:** Prints an error message in red, cleans the temporary directory, and exits the script.

#### 2. `checkStdin()`
**Parameters:**
- `$1`: Message to display to the user
- `$2`: Regular expression to validate input

**Purpose:** Prompts for user input and validates it against a regex. Loops until valid input is received.

**Returns:** Validated user input

#### 3. `getGamePath()`
**Parameters:** None

**Purpose:** Asks the user for the path of the directory containing the game executable. Validates that:
- The path exists
- It contains at least one .exe file (with confirmation if absent)
- The user confirms the path

**Effect:** Sets the global variable `$gamePath`

#### 4. `createTempDir()` / `removeTempDir()`
**Purpose:** Manages a temporary directory for download and extraction operations.

#### 5. `downloadD3dcompiler_47()`
**Parameters:**
- `$1`: Architecture (32 or 64)

**Purpose:** Downloads the d3dcompiler_47.dll from the Firefox installer (which includes this DLL). Verifies integrity with SHA256.

**File created:** `$MAIN_PATH/d3dcompiler_47.dll.$1`

#### 6. `downloadReshade()`
**Parameters:**
- `$1`: ReShade version
- `$2`: Full URL of the ReShade exe file

**Purpose:** Downloads and extracts a specific ReShade version into `$RESHADE_PATH/$1/`

#### 7. `linkShaderFiles()`
**Parameters:**
- `$1`: Source directory (full path)
- `$2`: Destination directory (Shaders/Textures with optional subdirectory)

**Purpose:** Creates symbolic links of shader files into the Merged directory, avoiding duplicates.

#### 8. `mergeShaderDirs()`
**Parameters:**
- `$1`: ReShade_shaders or External_shaders
- `$2`: Repository name (optional)

**Purpose:** Walks Shaders and Textures directories and creates symbolic links into the Merged directory.

## Environment Variables

### UPDATE_RESHADE
**Default:** 1  
**Allowed values:** 0 or 1

**Description:** Controls whether the script checks for ReShade and shader updates. The script automatically checks every 4 hours.

**Example:**
```bash
UPDATE_RESHADE=0 ./reshade-linux.sh
```

### MAIN_PATH
**Default:** `$XDG_DATA_HOME/reshade` (typically `$HOME/.local/share/reshade`)

**Description:** Root directory where all ReShade and shader files are stored.

**Example:**
```bash
MAIN_PATH=~/Documents/reshade ./reshade-linux.sh
```

### SHADER_REPOS
**Default:**
```
https://github.com/CeeJayDK/SweetFX|sweetfx-shaders;
https://github.com/martymcmodding/qUINT|martymc-shaders;
https://github.com/BlueSkyDefender/AstrayFX|astrayfx-shaders;
https://github.com/prod80/prod80-ReShade-Repository|prod80-shaders;
https://github.com/crosire/reshade-shaders|reshade-shaders|slim
```

**Format:** `URI|local_name|branch` (branch optional)  
**Separator:** `;` between repositories

**Description:** List of Git repositories containing ReShade shaders to clone or update.

**Example:**
```bash
SHADER_REPOS="https://github.com/martymcmodding/qUINT|martymc-shaders" ./reshade-linux.sh
```

### MERGE_SHADERS
**Default:** 1  
**Allowed values:** 0 or 1

**Description:** When enabled, merges all unique shaders from multiple repositories into a single "Merged" directory. Priority order is defined by SHADER_REPOS.

**Example:**
```bash
MERGE_SHADERS=0 ./reshade-linux.sh
```

### REBUILD_MERGE
**Default:** 0  
**Allowed values:** 0 or 1

**Description:** Forces rebuilding of the Merged directory. Useful after changing SHADER_REPOS.

**Example:**
```bash
REBUILD_MERGE=1 SHADER_REPOS="https://github.com/martymcmodding/qUINT|martymc-shaders" ./reshade-linux.sh
```

### GLOBAL_INI
**Default:** "ReShade.ini"  
**Allowed values:** 0, filename, or "ReShade.ini"

**Description:**
- If set to 1 or "ReShade.ini": creates and links a global ReShade.ini file
- If set to a filename: uses that file as configuration
- If set to 0: ReShade will create its own ini file when the game starts

**Example:**
```bash
GLOBAL_INI="ReShade2.ini" ./reshade-linux.sh
```

### LINK_PRESET
**Default:** Undefined

**Description:** Links a ReShade preset file to the game directory. The file must be placed in MAIN_PATH.

**Example:**
```bash
LINK_PRESET=ReShadePreset.ini ./reshade-linux.sh
```

### RESHADE_VERSION
**Default:** "latest"  
**Allowed values:** "latest" or a specific version number (e.g. "4.9.1")

**Description:** Specifies the ReShade version to use. If the version does not exist, the script exits with an error.

**Example:**
```bash
RESHADE_VERSION="4.9.1" ./reshade-linux.sh
```

### FORCE_RESHADE_UPDATE_CHECK
**Default:** 0  
**Allowed values:** 0 or 1

**Description:** Bypasses the 4-hour limit to force an update check.

**Example:**
```bash
FORCE_RESHADE_UPDATE_CHECK=1 ./reshade-linux.sh
```

### RESHADE_ADDON_SUPPORT
**Default:** 0  
**Allowed values:** 0 or 1

**Description:** Downloads ReShade with addon support. Intended for single-player games only, as anti-cheat may flag it as malicious.

**Example:**
```bash
RESHADE_ADDON_SUPPORT=1 ./reshade-linux.sh
```

### DELETE_RESHADE_FILES
**Default:** 0  
**Allowed values:** 0 or 1

**Description:** On uninstall, also removes ReShade.log and ReShadePreset.ini.

**Example:**
```bash
DELETE_RESHADE_FILES=1 ./reshade-linux.sh
```

### VULKAN_SUPPORT
**Default:** 0  
**Allowed values:** 0 or 1

**Description:** Enables the Vulkan installation feature (currently non-functional under Wine).

**Example:**
```bash
VULKAN_SUPPORT=1 ./reshade-linux.sh
```

## Execution Flow

### Initialization phase (Z0000–Z0005)

1. **Dependency check:** The script verifies that all required programs are installed.
2. **MAIN_PATH creation:** Creates the main directory if it does not exist.
3. **Update cache check:** Reads the LASTUPDATED file to decide if an update is needed (< 4 hours).

### Shader update phase (Z0010)

**Condition:** When `SHADER_REPOS` is set.

**Process:**
1. For each repository in SHADER_REPOS:
   - Clone the repository if it does not exist.
   - Update the repository if it exists and UPDATE_RESHADE=1.
2. If MERGE_SHADERS=1:
   - Create the Merged directory.
   - Link all unique shaders to Merged/Shaders.
   - Link all unique textures to Merged/Textures.
3. Process external shaders in External_shaders.

### ReShade update phase (Z0015–Z0016)

**Z0015 – "latest" version:**
1. Read current version from the LVERS file.
2. Check if an addon mode change is requested.
3. Connect to reshade.me (or static.reshade.me as fallback).
4. Parse the HTML to find the download link.
5. Extract the version number.
6. If a new version is found:
   - Download and extract ReShade.
   - Create a "latest" symbolic link.
   - Update LVERS.

**Z0016 – Specific version:**
1. Append "_Addon" to the name if RESHADE_ADDON_SUPPORT=1.
2. Check if the DLLs exist.
3. Download the version if needed.

### Global configuration phase (Z0020)

**Condition:** When GLOBAL_INI != 0 and the file does not exist.

**Process:**
1. Download the ReShade.ini file from the GitHub repository.
2. Replace placeholders:
   - `_USERSED_` → username
   - `_SHADSED_` → Windows path to Merged/Shaders
   - `_TEXSED_` → Windows path to Merged/Textures

### Vulkan phase (Z0025)

**Condition:** When VULKAN_SUPPORT=1 and the user confirms using Vulkan.

**Process:**
1. Prompt for WINEPREFIX.
2. Prompt for architecture (32/64-bit).
3. Install: Add an entry to the Wine registry:
   ```
   HKLM\SOFTWARE\Khronos\Vulkan\ImplicitLayers
   ```
4. Uninstall: Remove the registry entry.

**Note:** Currently non-functional under Wine.

### DirectX/OpenGL uninstall phase (Z0030)

**Condition:** When the user selects 'u' (uninstall).

**Process:**
1. Prompt for game path.
2. Remove symbolic links:
   - All common DLLs (d3d8, d3d9, d3d11, ddraw, dinput8, dxgi, opengl32)
   - ReShade.ini
   - ReShade32.json, ReShade64.json
   - d3dcompiler_47.dll
   - Shaders and Textures directories
   - Preset file if defined
3. If DELETE_RESHADE_FILES=1: remove ReShade.log and ReShadePreset.ini.
4. Remind the user to adjust WINEDLLOVERRIDES.

### DLL detection phase (Z0035)

**Automatic detection:**
1. Scan all .exe files in the game directory.
2. Use the `file` command to detect architecture.
3. If x86-64: exeArch=64, wantedDll="dxgi".
4. Otherwise: exeArch=32, wantedDll="d3d9".
5. Ask the user to confirm.

**Manual process:**
1. Prompt for architecture (32/64).
2. Prompt for the DLL name to override.
3. Common values: d3d8, d3d9, d3d11, ddraw, dinput8, dxgi, opengl32.

### d3dcompiler download phase (Z0040)

**Process:**
1. Check if d3dcompiler_47.dll.$exeArch exists.
2. If not:
   - Download Firefox Setup (which contains the DLL).
   - Verify SHA256 integrity.
   - Extract with 7z.
   - Copy to MAIN_PATH.

### File linking phase (Z0045)

**Process:**
1. Link ReShade32.dll or ReShade64.dll → $wantedDll.dll.
2. Link d3dcompiler_47.dll.$exeArch → d3dcompiler_47.dll.
3. Link the ReShade_shaders directory.
4. If GLOBAL_INI != 0: link the ini file.
5. If LINK_PRESET is set: link the preset file.
6. Display instructions for configuring WINEDLLOVERRIDES.

## Detailed Use Cases

### Installing for a DirectX game

**Example: Back To The Future Episode 1**

1. Locate the game directory:
   ```bash
   find ~/.local/share/Steam/steamapps/common -iname "*Back to the future*.exe"
   ```

2. Run the script:
   ```bash
   ./reshade-linux.sh
   ```

3. Answer the prompts:
   - Vulkan? → `n`
   - Install/Uninstall? → `i`
   - Game path → `/home/user/.local/share/Steam/steamapps/common/Back to the Future Ep 1`
   - Automatic detection? → `y` (or `n` for manual)

4. Configure Steam:
   - Right-click the game → Properties → Launch options
   - Add: `WINEDLLOVERRIDES="d3dcompiler_47=n;d3d9=n,b" %command%`

5. Launch the game and configure ReShade.

### Installing for an OpenGL game

**Example: Wolfenstein: The New Order**

1. Run the script:
   ```bash
   ./reshade-linux.sh
   ```

2. Answer the prompts:
   - Vulkan? → `n`
   - Install/Uninstall? → `i`
   - Game path → [path to game]
   - Automatic detection? → `n`
   - Manual override → `opengl32`

3. Configure WINEDLLOVERRIDES with `opengl32` instead of d3d9/dxgi.

### Installing for a Vulkan game

**Example: DOOM (2016)**

1. Find the App ID on SteamDB: 379720.

2. Find the WINEPREFIX:
   ```bash
   find ~/.local/share/Steam -wholename "*compatdata/379720"
   ```

3. Find the architecture:
   ```bash
   file ~/.local/share/Steam/steamapps/common/DOOM/DOOMx64vk.exe
   ```
   (x86-64 = 64-bit, Intel 80386 = 32-bit)

4. Run the script:
   ```bash
   VULKAN_SUPPORT=1 ./reshade-linux.sh
   ```

5. Answer the prompts:
   - Vulkan? → `y`
   - WINEPREFIX → `/home/user/.local/share/Steam/steamapps/compatdata/379720`
   - Architecture → `64`
   - Install/Uninstall? → `i`

**Note:** Currently non-functional under Wine.

### Adding custom shaders

1. Create the External_shaders directory:
   ```bash
   mkdir -p "$HOME/.local/share/reshade/External_shaders"
   ```

2. Add the shader:
   ```bash
   cd "$HOME/.local/share/reshade/External_shaders"
   curl -LO https://gist.github.com/kevinlekiller/cbb663e14b0f6ad6391a0062351a31a2/raw/CMAA2.fx
   ```

3. Run the script again to create the links.

### Using a custom preset

1. Place the preset file in MAIN_PATH:
   ```bash
   cp my_preset.ini "$HOME/.local/share/reshade/ReShadePreset.ini"
   ```

2. Run the script with LINK_PRESET:
   ```bash
   LINK_PRESET=ReShadePreset.ini ./reshade-linux.sh
   ```

## Recommended shader order

When enabling shaders in ReShade, this order is recommended:

1. Color (color correction)
2. Contrast/Brightness/Gamma
3. Anti-aliasing
4. Sharpening
5. Film grain

## Limitations and important notes

### Vulkan
- ReShade under Vulkan is currently not functional with Wine.
- Support is included in the script for potential future use.
- See: https://github.com/kevinlekiller/reshade-steam-proton/issues/6

### 32-bit games with Direct3D 11
Some 32-bit games use Direct3D 11 (e.g. Leisure Suit Larry: Wet Dreams Don't Dry).  
You must specify manually:
- Architecture: 32
- DLL: dxgi

### Configuring paths in ReShade
On first launch of the game with ReShade:
1. Open ReShade settings.
2. Go to the "Settings" tab.
3. Add paths if missing:
   - "Effect Search Paths" → Shaders directory
   - "Texture Search Paths" → Textures directory
4. Go to the "Home" tab.
5. Click "Reload".

## Troubleshooting

### Script cannot find ReShade
- Check Internet connectivity.
- Try with FORCE_RESHADE_UPDATE_CHECK=1.
- The script will automatically try the alternative URL.

### Shaders do not appear
- Ensure MERGE_SHADERS=1.
- Check paths in ReShade.ini.
- Run again with REBUILD_MERGE=1.

### SHA256 integrity error
- Firefox download failed.
- Run the script again.
- Check available disk space.

### Game does not start
- Verify WINEDLLOVERRIDES.
- Try a different DLL (d3d9, dxgi, opengl32).
- Check Proton/Wine logs.

## Complete removal

To remove all ReShade and shader files:

```bash
rm -rf "$HOME/.local/share/reshade"
```

Remember to:
1. Uninstall ReShade for each game using the script.
2. Remove WINEDLLOVERRIDES from Steam launch options.

## License

Copyright (C) 2021–2022 kevinlekiller

This program is free software; you may redistribute it under the terms of the GNU General Public License v2.
