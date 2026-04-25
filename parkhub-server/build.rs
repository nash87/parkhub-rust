//! Build script for `ParkHub` Server
//!
//! Compiles Slint UI files for the setup wizard (when gui feature is enabled).

fn main() {
    // Local-first CI hint: nudge contributors to install lefthook on
    // the very first build. Strictly informational — the build never
    // depends on or fails when hooks are absent. Suppressed in CI and
    // when hooks are already installed (.git/hooks/pre-commit exists).
    print_local_ci_hint();

    // Only compile Slint UI when GUI feature is enabled
    #[cfg(feature = "gui")]
    {
        let config = slint_build::CompilerConfiguration::new()
            .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer);

        slint_build::compile_with_config("ui/main.slint", config)
            .expect("Slint compilation failed");
    }

    // Windows-specific: embed icon and manifest
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();

        res.set_manifest(
            r#"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity
    version="0.1.0.0"
    processorArchitecture="*"
    name="ParkHub.Server"
    type="win32"
  />
  <description>ParkHub Server - Parking Management Backend</description>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>
"#,
        );

        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resources: {}", e);
        }
    }
}

/// Print a one-time-per-build hint reminding contributors to install
/// the local-first CI git hooks (lefthook). No-op in CI environments
/// and when the pre-push hook is already installed.
///
/// Cargo's `cargo:warning=…` prefix surfaces the message in a yellow
/// banner without affecting build status. Kept dependency-free and
/// fail-closed: any `io::Error` is silently swallowed.
fn print_local_ci_hint() {
    use std::path::PathBuf;

    // Don't nag in CI — most CI shells already manage their own hooks
    // (or explicitly skip them via --no-verify).
    if std::env::var_os("CI").is_some() || std::env::var_os("GITHUB_ACTIONS").is_some() {
        return;
    }

    // Find workspace root (parent of parkhub-server/).
    let manifest_dir = match std::env::var_os("CARGO_MANIFEST_DIR") {
        Some(v) => PathBuf::from(v),
        None => return,
    };
    let workspace_root = match manifest_dir.parent() {
        Some(p) => p.to_path_buf(),
        None => return,
    };

    let lefthook_yml = workspace_root.join("lefthook.yml");
    let pre_push_hook = workspace_root.join(".git").join("hooks").join("pre-push");

    if lefthook_yml.is_file() && !pre_push_hook.is_file() {
        println!(
            "cargo:warning=lefthook hooks not installed — run `npx lefthook install` to enable local-first CI gates (see CONTRIBUTING.md#local-ci)."
        );
    }
}
