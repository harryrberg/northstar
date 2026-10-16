# Windows MSI

`NorthStar.wxs` packages the 64-bit desktop prototype for the current Windows user. It installs the executable, project license, font notices, IBM Plex license, prototype guide, UTF-8 recovery guide, and WordStar navigation, deletion, and line-erasure guides under the user's local application data. It creates a Start menu shortcut and a normal Windows uninstall entry. It does not associate document extensions or touch user documents. Recovery copies live in a separate directory and are retained by uninstall.

The package is an unsigned prototype. It includes only the desktop application; no Rust, .NET, or WiX runtime is required on the target machine. The desktop can now save an existing file with a backup and an external-change check; many WordStar commands remain unimplemented. Read `WINDOWS_PROTOTYPE.md` before editing important files.

Build on Windows with Rust 1.99.0, the MSVC C++ build tools, Windows SDK, and .NET SDK 10.0.100:

```powershell
./packaging/windows/build.ps1
```

The script builds `northstar-desktop` in release mode, installs WiX 5.0.2 locally under ignored `target/tools/wix` if needed, and writes `releases/windows/NorthStar-Prototype-<version>-x64.msi`. The version comes from the desktop crate in `Cargo.toml`. To repackage an existing release executable without rebuilding it, pass `-SkipRustBuild`.

Validate the MSI with `target/tools/wix/wix.exe msi validate <path-to-msi> -sice ICE91`, then run `./packaging/windows/test.ps1 -MsiPath <path-to-msi>` on a machine where NorthStar is not already installed. ICE91 concerns per-machine deployment to a user directory; this package is fixed to per-user scope. The smoke test checks installed files, executable hash, Start menu and uninstall entry, then removes the test installation.

For a quiet per-user install or uninstall, use `msiexec /i <path-to-msi> /qn` or `msiexec /x <path-to-msi> /qn`. Use `/l*v <path-to-log>` when diagnosing an installer problem. The MSI keeps the upgrade code stable and replaces an earlier NorthStar MSI of a lower version. Bump the desktop crate version when publishing a new build. The checked-in MSI is an inspectable prototype artifact, not a signed public release.
