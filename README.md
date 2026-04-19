# velo

VLESS desktop client for Windows. Tauri 2 shell around a sing-box sidecar:
velo handles the GUI, profile storage, and config generation; sing-box owns
the networking stack (TLS/Reality, TUN, routing).

## Stack
- Tauri 2 + Svelte 5 (runes) + TypeScript
- Rust (backend) + sing-box 1.11 sidecar

## Build
```
npm install          # postinstall fetches pinned sing-box.exe into tools/
npx tauri build
```

The sing-box sidecar is not committed. `scripts/fetch-singbox.mjs` downloads
the pinned release zip from GitHub and verifies SHA-256 against a pin in the
script; runs on `npm install` (postinstall) and `npm run build` (prebuild).
It respects `HTTPS_PROXY` / `HTTP_PROXY` env vars via `curl`.

## Elevation
velo's exe manifest declares `requireAdministrator`, so every launch
triggers a UAC prompt and runs elevated. TUN mode needs admin to install
a virtual NIC; a mixed-privilege model (user GUI + admin service over IPC)
is a separate architecture not on the roadmap.

## Autostart
Enabled from Settings → Startup. Backed by a Windows scheduled task
registered with `/RL HIGHEST` — logon launches skip UAC because the task
itself is privileged. Creating/removing the task takes one UAC prompt per
toggle (the toggle triggers `schtasks.exe` which needs admin).

## Portable build
```
npm run build:portable
```
Produces `dist/velo-portable-<version>-x64.zip` containing `velo.exe`,
`sing-box.exe`, and a `velo.portable` marker. Unzip anywhere; run the exe.

In portable mode the app:
- stores `velo.db`, `sing-box.log`, and `last-config.json` in `<exe>/data/`
  instead of `%APPDATA%`;
- hides the Startup section in Settings (scheduled tasks store an absolute
  exe path which goes stale the first time the user moves the folder).

The portable zip does NOT bundle the WebView2 runtime. Windows 11 ships with
it pre-installed. On Windows 10 LTSC or older without the Evergreen runtime,
install it from <https://developer.microsoft.com/microsoft-edge/webview2/>
before running velo.

## Data paths
- Installed: `%APPDATA%\com.velo.app\` (velo.db, sing-box.log, last-config.json)
- Portable: `<exe-dir>\data\`


## Contributing
Rust tests: `cd src-tauri && cargo test --lib`. Frontend checks: `npm run check`.

Live integration tests (`tests/live_sub.rs`, `tests/runner_e2e.rs`) need real endpoints.
Copy `.env.example` to `.env`, fill in `VELO_TEST_SUB_URL` and/or `VELO_TEST_VLESS_URL`,
export them in your shell, then run `cargo test --test live_sub` or `cargo test --test runner_e2e`.
Absent env vars → the tests print a skip message and return cleanly.

## Licenses
velo is licensed under **GPL-3.0-or-later** — see [LICENSE](./LICENSE).
Any fork or derivative work must also be licensed under GPLv3 and ship its
complete corresponding source.

The bundled sing-box sidecar is also GPLv3 (upstream:
<https://github.com/SagerNet/sing-box>, pinned release:
<https://github.com/SagerNet/sing-box/releases/tag/v1.11.15>).
`scripts/fetch-singbox.mjs` downloads the official release artifact and
extracts its `LICENSE` text alongside the binary; the portable zip ships
both as `sing-box.exe` and `LICENSE-sing-box.txt` next to it.