use std::{fs, path::PathBuf};

/// Windows application manifest. velo always runs elevated: TUN needs
/// admin, and splitting into user-GUI + admin-service is a separate
/// architecture. UAC prompts once per launch instead of per-feature-use.
/// Task Scheduler with `/RL HIGHEST` is the only autostart path
/// compatible with this — a plain HKCU\...\Run entry would UAC-prompt
/// every logon.
///
/// The Common-Controls dependency mirrors what tauri-build's default
/// manifest provides; dropping it would revert to Win95-era theming for
/// common dialogs.
#[cfg(windows)]
const VELO_MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
    <dependency>
        <dependentAssembly>
            <assemblyIdentity
                type="win32"
                name="Microsoft.Windows.Common-Controls"
                version="6.0.0.0"
                processorArchitecture="*"
                publicKeyToken="6595b64144ccf1df"
                language="*"
            />
        </dependentAssembly>
    </dependency>
    <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
        <security>
            <requestedPrivileges>
                <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
            </requestedPrivileges>
        </security>
    </trustInfo>
</assembly>"#;

fn main() {
	// Tauri's `externalBin` expects a binary named `<stem>-<target-triple>.<ext>`
	// sitting next to its manifest entry. We keep the canonical file at
	// `tools/sing-box.exe` (same name dev code uses) and mirror it to the
	// triple-suffixed path the bundler consumes. This avoids dual-naming the
	// source file and keeps dev/prod pointing at one authoritative binary.
	if let Err(e) = mirror_sidecar_for_bundle() {
		println!("cargo:warning=mirror_sidecar: {e}");
	}
	// Expose the target triple to sidecar.rs so its runtime resolver can
	// find the bundled `sing-box-<triple>.exe` next to velo.exe in installed
	// and portable builds.
	if let Ok(t) = std::env::var("TARGET") {
		println!("cargo:rustc-env=VELO_TARGET_TRIPLE={t}");
	}

	#[cfg(windows)]
	{
		let attrs = tauri_build::Attributes::new().windows_attributes(
			tauri_build::WindowsAttributes::new_without_app_manifest()
				.app_manifest(VELO_MANIFEST),
		);
		tauri_build::try_build(attrs).expect("tauri build failed");
	}
	#[cfg(not(windows))]
	tauri_build::build();
}

fn mirror_sidecar_for_bundle() -> std::io::Result<()> {
	let target = match std::env::var("TARGET") {
		Ok(t) if !t.is_empty() => t,
		_ => return Ok(()),
	};
	let manifest = std::env::var("CARGO_MANIFEST_DIR")
		.expect("CARGO_MANIFEST_DIR is always set during a build script run");
	let tools = PathBuf::from(&manifest).join("..").join("tools");
	let src = tools.join("sing-box.exe");
	if !src.is_file() {
		return Ok(());
	}
	let dst = tools.join(format!("sing-box-{target}.exe"));
	let needs_copy = match (fs::metadata(&dst), fs::metadata(&src)) {
		(Ok(d), Ok(s)) => s.modified().ok().zip(d.modified().ok())
			.map(|(sm, dm)| sm > dm)
			.unwrap_or(false),
		(Err(_), _) => true,
		_ => false,
	};
	if needs_copy {
		fs::copy(&src, &dst)?;
	}
	println!("cargo:rerun-if-changed={}", src.display());
	Ok(())
}
