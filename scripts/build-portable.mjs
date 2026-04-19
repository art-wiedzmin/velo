// Builds a portable Windows zip: velo.exe + sing-box.exe + portable marker.
// Requires the sidecar source at `tools/sing-box.exe` (the build.rs mirrors
// it to the triple-suffixed name tauri's externalBin consumes).
//
// Usage: `npm run build:portable`.
//
// WebView2 runtime is NOT bundled: the zip assumes Windows 11 (ships with
// the runtime). For Windows 10 LTSC / older, the user must install the
// Evergreen runtime manually. A future `webviewInstallMode: "fixedVersion"`
// would bundle the runtime at ~180 MB; skipped here to keep the zip small.

import { execSync } from "node:child_process";
import {
	cpSync,
	existsSync,
	mkdirSync,
	readFileSync,
	rmSync,
	writeFileSync,
} from "node:fs";
import { join, resolve } from "node:path";

const root = resolve(".");
const pkg = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const version = pkg.version;
const target = process.env.CARGO_TARGET ?? "x86_64-pc-windows-msvc";
const arch = target.includes("aarch64") ? "arm64" : "x64";

const stageName = `velo-portable-${version}-${arch}`;
const distDir = join(root, "dist");
const stageDir = join(distDir, stageName);
const zipPath = join(distDir, `${stageName}.zip`);
const releaseDir = join(root, "src-tauri", "target", "release");

console.log("==> tauri build (no bundle)");
execSync("npx tauri build --no-bundle", { stdio: "inherit" });

const veloExe = join(releaseDir, "velo.exe");
if (!existsSync(veloExe)) {
	throw new Error(`velo.exe not found at ${veloExe}`);
}
// Tauri's externalBin bundler places the sidecar next to the main exe with
// the triple stripped, so the filename in release/ is just `sing-box.exe`.
const singboxSrc = join(releaseDir, "sing-box.exe");
if (!existsSync(singboxSrc)) {
	throw new Error(
		`sing-box sidecar not found at ${singboxSrc}. Ensure tools/sing-box.exe exists so build.rs can mirror it for externalBin.`,
	);
}

console.log(`==> staging in ${stageDir}`);
rmSync(stageDir, { recursive: true, force: true });
mkdirSync(stageDir, { recursive: true });
cpSync(veloExe, join(stageDir, "velo.exe"));
cpSync(singboxSrc, join(stageDir, "sing-box.exe"));
// GPLv3 attribution: sing-box upstream LICENSE, fetched alongside the binary
// by scripts/fetch-singbox.mjs.
const singboxLicense = join(root, "tools", "sing-box-LICENSE.txt");
if (!existsSync(singboxLicense)) {
	throw new Error(
		`sing-box LICENSE not found at ${singboxLicense}. Run 'node scripts/fetch-singbox.mjs' to fetch it.`,
	);
}
cpSync(singboxLicense, join(stageDir, "LICENSE-sing-box.txt"));
// Marker flips velo into portable mode: data dir becomes `<exe>/data/`,
// autostart plugin skips registration, SettingsDrawer hides the section.
writeFileSync(join(stageDir, "velo.portable"), "");

console.log(`==> zipping to ${zipPath}`);
rmSync(zipPath, { force: true });
execSync(
	`powershell -NoProfile -Command "Compress-Archive -Path '${stageDir}/*' -DestinationPath '${zipPath}' -CompressionLevel Optimal"`,
	{ stdio: "inherit" },
);

console.log(`\nPortable bundle ready: ${zipPath}`);
