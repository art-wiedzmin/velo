<h1 align="center">velo</h1>

<p align="center">
  VLESS desktop client for Windows.<br>
  Tauri 2 + Svelte 5 shell around a <a href="https://github.com/SagerNet/sing-box">sing-box</a> sidecar.
</p>

<p align="center">
  <a href="https://github.com/art-wiedzmin/velo/releases/latest"><img alt="Latest release" src="https://img.shields.io/github/v/release/art-wiedzmin/velo?sort=semver&color=0ea5e9&cacheSeconds=3600"></a>
  <a href="./LICENSE"><img alt="License: GPL-3.0-or-later" src="https://img.shields.io/github/license/art-wiedzmin/velo?color=6366f1&cacheSeconds=86400"></a>
  <a href="https://github.com/art-wiedzmin/velo/releases"><img alt="Downloads" src="https://img.shields.io/github/downloads/art-wiedzmin/velo/total?color=22c55e&cacheSeconds=3600"></a>
</p>

<!-- screenshot placeholder — drop in once the UI is stable -->

velo handles the GUI, profile storage, and config generation; sing-box owns
the networking stack (TLS/Reality, TUN, routing). Two processes talk over
stdio — velo never implements protocols it doesn't need to.

## Features
- VLESS, VMess, Trojan, Shadowsocks profiles via URL, subscription, or manual entry
- Reality / XTLS-Vision / uTLS fingerprinting, gRPC / WebSocket / HTTP/2 transports
- System proxy toggle with snapshot-based restore across unclean shutdowns
- Per-process routing rules (pick running processes from a picker)
- Scheduled-task autostart, no UAC on logon
- Portable and installed modes from the same binary
- Auto-connect on launch (off / last-used / pinned profile)

## Download

> [!NOTE]
> First public release is tagged. Grab the matching asset from [Releases](https://github.com/art-wiedzmin/velo/releases/latest).

| Artifact   | Format    | Use when                                        |
| ---------- | --------- | ----------------------------------------------- |
| Installer  | `.msi`    | You want Start Menu entry + uninstaller         |
| Installer  | `.exe`    | NSIS flavor — smaller, silent-install friendly  |
| Portable   | `.zip`    | No admin-at-install, runs from any folder       |

All three launch elevated (`requireAdministrator`). See [Platform notes](#platform-notes) for why.

## Stack

| Concern       | Technology                                                 |
| ------------- | ---------------------------------------------------------- |
| Frontend      | Svelte 5 (runes) + TypeScript + Vite                       |
| Shell         | Tauri 2 (Rust backend, WebView2 renderer)                  |
| Storage       | SQLite (`rusqlite`) with schema migrations                 |
| Proxy engine  | [sing-box](https://github.com/SagerNet/sing-box) v1.11.15 sidecar |
| Packaging     | MSI / NSIS via Tauri bundler; portable ZIP via custom script |

## Build

```sh
npm install          # postinstall fetches pinned sing-box.exe into tools/
npx tauri build      # MSI + NSIS installers
npm run build:portable   # portable .zip
```

The sing-box sidecar is not committed. `scripts/fetch-singbox.mjs` downloads
the pinned release from GitHub and verifies SHA-256 before use; it runs on
`npm install` (postinstall) and `npm run build` (prebuild). Respects
`HTTPS_PROXY` / `HTTP_PROXY` via `curl`.

## Platform notes

### Elevation

> [!IMPORTANT]
> velo's manifest declares `requireAdministrator`. Every launch triggers UAC
> and runs elevated. TUN mode needs admin to install a virtual NIC; a
> mixed-privilege model (user GUI + admin service over IPC) is a separate
> architecture not on the roadmap.

### Autostart

Enabled from *Settings → Startup*. Backed by a Windows scheduled task
registered with `/RL HIGHEST` — logon launches skip UAC because the task
itself is privileged. Toggling autostart costs one UAC prompt per change
(`schtasks.exe` needs admin).

### Portable mode

```sh
npm run build:portable
```

Produces `dist/velo-portable-<version>-x64.zip` containing `velo.exe`,
`sing-box.exe`, `LICENSE-sing-box.txt`, and a `velo.portable` marker. Unzip
anywhere; run the exe.

In portable mode velo:
- stores `velo.db`, `sing-box.log`, `last-config.json`, and
  `sysproxy-snapshot.json` in `<exe-dir>/data/` instead of `%APPDATA%`;
- hides the Startup section in Settings (scheduled tasks store absolute exe
  paths, which go stale the first time the user moves the folder).

> [!WARNING]
> The portable zip does **not** bundle the WebView2 runtime. Windows 11
> ships with it pre-installed. On Windows 10 LTSC or older without the
> Evergreen runtime, install it from
> <https://developer.microsoft.com/microsoft-edge/webview2/> before running velo.

### Data paths

| Mode      | Location                                    |
| --------- | ------------------------------------------- |
| Installed | `%APPDATA%\com.velo.app\`                   |
| Portable  | `<exe-dir>\data\`                           |

Both locations hold `velo.db`, `sing-box.log`, `last-config.json`, and
`sysproxy-snapshot.json`.

## Contributing

```sh
cd src-tauri && cargo test --lib    # 63 tests, offline
npm run check                       # svelte-check, 0 errors expected
```

Live integration tests (`src-tauri/tests/live_sub.rs`,
`src-tauri/tests/runner_e2e.rs`) need real endpoints. Copy `.env.example` to
`.env`, fill in `VELO_TEST_SUB_URL` and/or `VELO_TEST_VLESS_URL`, export
them, then:

```sh
cargo test --test live_sub -- --nocapture
cargo test --test runner_e2e -- --nocapture
```

Absent env vars → tests print a skip message and return cleanly. Safe to run
in CI.

## Credits

- **Core proxy engine**: [sing-box](https://github.com/SagerNet/sing-box) by nekohasekai and contributors
- **Desktop shell**: [Tauri 2](https://tauri.app/)
- **UI framework**: [Svelte 5](https://svelte.dev/)

## License

velo is licensed under **GPL-3.0-or-later** — see [LICENSE](./LICENSE). Any
fork or derivative must also be licensed under GPLv3 and ship its complete
corresponding source.

The bundled sing-box sidecar is also GPLv3
([upstream](https://github.com/SagerNet/sing-box),
pinned release: [v1.11.15](https://github.com/SagerNet/sing-box/releases/tag/v1.11.15)).
`scripts/fetch-singbox.mjs` downloads the official release artifact and
places its upstream `LICENSE` text next to the binary; the portable zip
ships both as `sing-box.exe` and `LICENSE-sing-box.txt`.
