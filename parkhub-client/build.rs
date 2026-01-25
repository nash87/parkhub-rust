//! Build script for ParkHub Client
//!
//! Compiles Slint UI files and embeds Windows resources.

fn main() {
    // Compile Slint UI files with Phosphor icon font
    // Include the fonts directory so Slint can find and embed Phosphor.ttf
    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer)
        .with_include_paths(vec![std::path::PathBuf::from("fonts")]);

    slint_build::compile_with_config("ui/main.slint", config).expect("Slint compilation failed");

    // Tell Cargo to rerun if font changes
    println!("cargo:rerun-if-changed=fonts/Phosphor.ttf");

    // Windows-specific: embed icon and manifest
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();

        // Set application icon (if exists)
        if std::path::Path::new("assets/app.ico").exists() {
            res.set_icon("assets/app.ico");
        }

        // Set application manifest for high DPI and visual styles
        res.set_manifest(r#"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity
    version="0.1.0.0"
    processorArchitecture="*"
    name="ParkHub.Desktop"
    type="win32"
  />
  <description>ParkHub - Open Source Parking Management</description>
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
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
</assembly>
"#);

        // Compile Windows resources
        if let Err(e) = res.compile() {
            eprintln!("Warning: Failed to compile Windows resources: {}", e);
        }
    }
}
