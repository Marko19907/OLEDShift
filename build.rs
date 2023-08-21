extern crate winresource;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() == "windows" {
        let mut res = winresource::WindowsResource::new();

        // Needed due to an NWG and windows-sys bug, the libloading crate depends on windows-sys
        // For more info, see this issue on GitHub: https://github.com/gabdube/native-windows-gui/issues/251
        res.set_manifest(r#"
            <assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
            <assemblyIdentity
                version="1.0.0.0"
                processorArchitecture="*"
                name="app"
                type="win32"
            />
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
            </assembly>
        "#);

        res.set_icon("icon.ico");
        res.compile().unwrap();
    }
}
