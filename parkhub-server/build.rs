//! Build script for ParkHub Server
//!
//! Compiles Slint UI files for the setup wizard (when gui feature is enabled).

fn main() {
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
