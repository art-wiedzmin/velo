// Fetches the pinned sing-box sidecar into `tools/sing-box.exe`.
//
// Invoked automatically by `npm install` (postinstall) and `npm run build`
// (prebuild). Idempotent: if the binary already matches the pinned SHA-256,
// exits immediately. Any mismatch triggers a re-download.
//
// Uses `curl` (ships with Windows 11) so HTTPS_PROXY / HTTP_PROXY env vars
// work out of the box — matches how cargo is invoked through velo's own
// mixed inbound on this workstation.

import { execSync } from "node:child_process";
import { createHash } from "node:crypto";
import { copyFileSync, existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";

const VERSION = "1.11.15";
// SHA-256 of the extracted `sing-box.exe` (not the zip). Decouples the pin
// from upstream re-zipping and matches what we actually ship.
const SHA256 = "6fe4af11bf4fd485e0e454d7c060cb5ece29a2839a78ffb53759ce4dd6645161";
const URL = `https://github.com/SagerNet/sing-box/releases/download/v${VERSION}/sing-box-${VERSION}-windows-amd64.zip`;

const root = resolve(".");
const outPath = join(root, "tools", "sing-box.exe");
// Upstream GPLv3 license text. Bundled into portable zip and MSI/NSIS for
// redistribution compliance.
const licenseOut = join(root, "tools", "sing-box-LICENSE.txt");

const sha = (path) => createHash("sha256").update(readFileSync(path)).digest("hex");

if (existsSync(outPath) && existsSync(licenseOut)) {
	const got = sha(outPath);
	if (got === SHA256) {
		console.log(`sing-box ${VERSION} already present (sha256 ok).`);
		process.exit(0);
	}
	console.log(`sing-box present but sha mismatch (got ${got}); refetching.`);
} else if (existsSync(outPath)) {
	console.log("sing-box present but LICENSE missing; refetching to restore it.");
}

const work = join(tmpdir(), `velo-singbox-${process.pid}-${Date.now()}`);
mkdirSync(work, { recursive: true });
const zipPath = join(work, "sb.zip");

try {
	console.log(`==> downloading ${URL}`);
	execSync(
		`curl -fL --retry 3 --retry-delay 2 --progress-bar -o "${zipPath}" "${URL}"`,
		{ stdio: "inherit" },
	);

	console.log("==> extracting");
	execSync(
		`powershell -NoProfile -Command "Expand-Archive -Force -Path '${zipPath}' -DestinationPath '${work}'"`,
		{ stdio: "inherit" },
	);

	const extracted = join(work, `sing-box-${VERSION}-windows-amd64`, "sing-box.exe");
	const extractedLicense = join(work, `sing-box-${VERSION}-windows-amd64`, "LICENSE");
	if (!existsSync(extracted)) {
		throw new Error(`expected binary missing: ${extracted}`);
	}
	if (!existsSync(extractedLicense)) {
		throw new Error(`upstream LICENSE missing: ${extractedLicense}`);
	}

	const got = sha(extracted);
	if (got !== SHA256) {
		throw new Error(
			`sha256 mismatch for sing-box.exe\n  expected ${SHA256}\n  got      ${got}\nAborting to avoid shipping an unverified binary.`,
		);
	}

	mkdirSync(dirname(outPath), { recursive: true });
	rmSync(outPath, { force: true });
	copyFileSync(extracted, outPath);
	copyFileSync(extractedLicense, licenseOut);
	console.log(`==> installed ${outPath}`);
	console.log(`==> installed ${licenseOut}`);
} finally {
	rmSync(work, { recursive: true, force: true });
}
