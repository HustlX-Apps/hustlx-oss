//! Local version preparation and desktop packaging. No remote publishing operations.
use std::{
    fs,
    ops::Range,
    path::{Path, PathBuf},
    process::{Command, Output},
};
type Result<T> = std::result::Result<T, String>;
#[derive(Debug, Clone, PartialEq, Eq)]
struct Version {
    full: String,
    numeric: String,
}
impl Version {
    fn parse(value: &str) -> Result<Self> {
        let (without_build, build) = value
            .split_once('+')
            .map_or((value, None), |(a, b)| (a, Some(b)));
        let (numeric, pre) = without_build
            .split_once('-')
            .map_or((without_build, None), |(a, b)| (a, Some(b)));
        let number = |v: &str| {
            !v.is_empty()
                && v.bytes().all(|c| c.is_ascii_digit())
                && (v.len() == 1 || !v.starts_with('0'))
        };
        let parts: Vec<_> = numeric.split('.').collect();
        let identifiers = |v: &str, prerelease: bool| {
            v.split('.').all(|id| {
                !id.is_empty()
                    && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
                    && (!prerelease || !id.bytes().all(|c| c.is_ascii_digit()) || number(id))
            })
        };
        if parts.len() != 3
            || !parts.iter().all(|p| number(p) && p.parse::<u32>().is_ok())
            || pre.is_some_and(|p| !identifiers(p, true))
            || build.is_some_and(|b| !identifiers(b, false))
        {
            return Err(
                "Use a SemVer version such as 0.1.0 or 0.2.0-beta.1, without a v prefix.".into(),
            );
        }
        Ok(Self {
            full: value.into(),
            numeric: numeric.into(),
        })
    }
}
fn version_range(text: &str, lock: bool) -> Result<Range<usize>> {
    let mut active = false;
    let mut app = false;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            active = trimmed == if lock { "[[package]]" } else { "[package]" };
            app = !lock;
        }
        if active && let Some((key, value)) = trimmed.split_once('=') {
            if key.trim() == "name" {
                app = value.trim() == "\"hustlx-oss\"";
            }
            if key.trim() == "version" && app {
                let start = line.find('"').ok_or("Expected a quoted package version.")? + 1;
                let end = start
                    + line[start..]
                        .find('"')
                        .ok_or("Invalid quoted package version.")?;
                return Ok(offset + start..offset + end);
            }
        }
        offset += line.len();
    }
    Err("Cannot locate the hustlx-oss package version.".into())
}
fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| format!("Read {}: {e}", path.display()))
}
fn current_version(root: &Path) -> Result<Version> {
    let manifest = read(&root.join("Cargo.toml"))?;
    let lock = read(&root.join("Cargo.lock"))?;
    let version = Version::parse(&manifest[version_range(&manifest, false)?])?;
    if lock[version_range(&lock, true)?] != version.full {
        return Err(
            "Cargo.toml and Cargo.lock versions differ. Resolve that before preparing a package."
                .into(),
        );
    }
    Ok(version)
}
fn prepare_version(root: &Path, value: &str) -> Result<()> {
    let version = Version::parse(value)?;
    if current_version(root)? == version {
        return Ok(());
    }
    let manifest_path = root.join("Cargo.toml");
    let lock_path = root.join("Cargo.lock");
    let original_manifest = read(&manifest_path)?;
    let mut manifest = original_manifest.clone();
    let original_lock = read(&lock_path)?;
    let mut lock = original_lock.clone();
    manifest.replace_range(version_range(&manifest, false)?, &version.full);
    lock.replace_range(version_range(&lock, true)?, &version.full);
    fs::write(&manifest_path, manifest).map_err(|e| format!("Write manifest: {e}"))?;
    if let Err(error) = fs::write(&lock_path, lock) {
        let manifest_restore = fs::write(&manifest_path, original_manifest);
        let lock_restore = fs::write(&lock_path, original_lock);
        if manifest_restore.is_err() || lock_restore.is_err() {
            return Err(format!(
                "Version update failed ({error}); rollback was incomplete. Restore both version files from Git."
            ));
        }
        return Err(format!(
            "Version update failed; both version files restored: {error}"
        ));
    }
    Ok(())
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Os {
    Mac,
    Windows,
}
fn native_os() -> Result<Os> {
    if cfg!(target_os = "macos") {
        Ok(Os::Mac)
    } else if cfg!(target_os = "windows") {
        Ok(Os::Windows)
    } else {
        Err("Run packaging on macOS or Windows. Cross-OS packaging is not supported.".into())
    }
}
struct Plan {
    root: PathBuf,
    version: Version,
    target: String,
    os: Os,
    output: PathBuf,
    archive: PathBuf,
}
fn plan(root: &Path, value: &str, target: &str, os: Os) -> Result<Plan> {
    let version = Version::parse(value)?;
    if current_version(root)? != version {
        return Err("Package version must match Cargo.toml/Cargo.lock. Use cargo xtask version VERSION first.".into());
    }
    let allowed = match os {
        Os::Mac => ["aarch64-apple-darwin", "x86_64-apple-darwin"],
        Os::Windows => ["x86_64-pc-windows-msvc", "aarch64-pc-windows-msvc"],
    };
    if !allowed.contains(&target) {
        return Err(format!(
            "Unsupported target on this OS. Choose one of: {}",
            allowed.join(", ")
        ));
    }
    let name = format!("hustlx-oss-{}-{target}", version.full);
    let output = root.join("dist").join(&name);
    if output.exists() {
        return Err(format!(
            "Output already exists: {}. Preserve it or move it before rebuilding.",
            output.display()
        ));
    }
    for file in [
        "THIRD_PARTY_NOTICES.md",
        "assets/fonts/EXO-OFL.txt",
        "assets/fonts/INTER-OFL.txt",
    ] {
        if !root.join(file).is_file() {
            return Err(format!("Required notice is missing: {file}"));
        }
    }
    let archive = output.join(format!("{name}.zip"));
    Ok(Plan {
        root: root.into(),
        version,
        target: target.into(),
        os,
        output,
        archive,
    })
}
fn write(path: &Path, content: impl AsRef<[u8]>) -> Result<()> {
    fs::write(path, content).map_err(|e| format!("Write {}: {e}", path.display()))
}
fn copy(from: &Path, to: &Path) -> Result<()> {
    fs::copy(from, to)
        .map(|_| ())
        .map_err(|e| format!("Copy {}: {e}", from.display()))
}
fn stage(p: &Plan, binary: &Path) -> Result<PathBuf> {
    if !binary.is_file() {
        return Err(format!("Built executable is missing: {}", binary.display()));
    }
    fs::create_dir_all(p.output.parent().ok_or("Invalid output directory")?)
        .map_err(|e| e.to_string())?;
    fs::create_dir(&p.output).map_err(|e| format!("Create new package directory: {e}"))?;
    let payload = p.output.join(if p.os == Os::Mac {
        "HustlX OSS.app"
    } else {
        "HustlX OSS"
    });
    let resources = if p.os == Os::Mac {
        payload.join("Contents/Resources")
    } else {
        payload.clone()
    };
    fs::create_dir_all(resources.join("licenses")).map_err(|e| e.to_string())?;
    let executable = if p.os == Os::Mac {
        let macos = payload.join("Contents/MacOS");
        fs::create_dir_all(&macos).map_err(|e| e.to_string())?;
        write(
            &payload.join("Contents/Info.plist"),
            format!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleExecutable</key><string>hustlx-oss</string>
<key>CFBundleIdentifier</key><string>com.hustlx.oss.desktop</string>
<key>CFBundleName</key><string>HustlX OSS</string>
<key>CFBundleDisplayName</key><string>HustlX OSS</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>{}</string>
<key>CFBundleVersion</key><string>{}</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
"#,
                p.version.numeric, p.version.numeric
            ),
        )?;
        macos.join("hustlx-oss")
    } else {
        payload.join("hustlx-oss.exe")
    };
    copy(binary, &executable)?;
    copy(
        &p.root.join("THIRD_PARTY_NOTICES.md"),
        &resources.join("THIRD_PARTY_NOTICES.md"),
    )?;
    for name in ["EXO-OFL.txt", "INTER-OFL.txt"] {
        copy(
            &p.root.join("assets/fonts").join(name),
            &resources.join("licenses").join(name),
        )?;
    }
    write(
        &resources.join("VERSION.txt"),
        format!(
            "HustlX OSS {}\nTarget: {}\nProfile: release\n",
            p.version.full, p.target
        ),
    )?;
    write(
        &resources.join("README.txt"),
        "HustlX OSS portable development build\n\nmacOS: move HustlX OSS.app to Applications. This bundle is ad-hoc signed, not Developer ID signed or notarized.\nWindows: extract the complete folder, then open hustlx-oss.exe. It is unsigned.\n\nYour local SQLite library is stored in your user data directory, separately from this package. Preserve a closed-library backup before upgrading. No runtime Rust, Node, database server or network connection is needed.\nThis project has no selected open-source license yet. This archive does not grant reuse rights beyond the included third-party notices.\n",
    )?;
    Ok(payload)
}
fn output(command: &mut Command) -> Result<Output> {
    let result = command
        .output()
        .map_err(|e| format!("Run {}: {e}", command.get_program().to_string_lossy()))?;
    if !result.status.success() {
        return Err(format!(
            "{} failed: {}",
            command.get_program().to_string_lossy(),
            String::from_utf8_lossy(&result.stderr)
        ));
    }
    Ok(result)
}
fn powershell(script: &str) -> Command {
    let mut c = Command::new("powershell.exe");
    c.args(["-NoProfile", "-NonInteractive", "-Command", script]);
    c
}
fn archive(p: &Plan, payload: &Path) -> Result<()> {
    match p.os {
        Os::Mac => {
            output(
                Command::new("/usr/bin/ditto")
                    .args(["-c", "-k", "--sequesterRsrc", "--keepParent"])
                    .arg(payload)
                    .arg(&p.archive),
            )?;
        }
        Os::Windows => {
            output(powershell("$ErrorActionPreference='Stop'; Compress-Archive -LiteralPath $env:HUSTLX_PACKAGE_PAYLOAD -DestinationPath $env:HUSTLX_PACKAGE_ARCHIVE -CompressionLevel Optimal").env("HUSTLX_PACKAGE_PAYLOAD",payload).env("HUSTLX_PACKAGE_ARCHIVE",&p.archive))?;
        }
    }
    Ok(())
}
fn checksum(os: Os, path: &Path) -> Result<String> {
    let result = match os {
        Os::Mac => output(Command::new("/usr/bin/shasum").args(["-a", "256"]).arg(path))?,
        // A Windows PowerShell child can inherit a module path from pwsh that
        // cannot resolve Get-FileHash. .NET hashing does not need that cmdlet.
        Os::Windows => output(powershell("$ErrorActionPreference='Stop'; $sha=[System.Security.Cryptography.SHA256]::Create(); $stream=[System.IO.File]::OpenRead($env:HUSTLX_PACKAGE_ARCHIVE); try { [BitConverter]::ToString($sha.ComputeHash($stream)).Replace('-','') } finally { $stream.Dispose(); $sha.Dispose() }").env("HUSTLX_PACKAGE_ARCHIVE", path))?,
    };
    let text = String::from_utf8_lossy(&result.stdout);
    let hash = text
        .split_whitespace()
        .next()
        .ok_or("No checksum returned")?;
    if hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("Invalid SHA-256 output".into());
    }
    Ok(hash.to_ascii_lowercase())
}
fn write_checksum(p: &Plan) -> Result<()> {
    write(
        &p.output.join("SHA256SUMS"),
        format!(
            "{}  {}\n",
            checksum(p.os, &p.archive)?,
            p.archive
                .file_name()
                .ok_or("No archive name")?
                .to_string_lossy()
        ),
    )
}
fn host() -> Result<String> {
    let result = output(Command::new("rustc").arg("-vV"))?;
    String::from_utf8_lossy(&result.stdout)
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .ok_or("Cannot identify the Rust host target".into())
}
fn sign(p: &Plan, payload: &Path) -> Result<()> {
    if p.os == Os::Mac {
        output(
            Command::new("/usr/bin/codesign")
                .args(["--force", "--sign", "-", "--timestamp=none"])
                .arg(payload),
        )?;
        output(
            Command::new("/usr/bin/codesign")
                .args(["--verify", "--strict"])
                .arg(payload),
        )?;
    }
    Ok(())
}
fn release(root: &Path, version: &str, target: Option<&str>, dry: bool) -> Result<()> {
    let target = target.map(str::to_owned).map_or_else(host, Ok)?;
    let p = plan(root, version, &target, native_os()?)?;
    println!(
        "Version: {}\nTarget: {}\nArchive: {}",
        p.version.full,
        p.target,
        p.archive.display()
    );
    if dry {
        println!("DRY RUN: validate only. No app build, package, tag, push or upload.");
        return Ok(());
    }
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let status = Command::new(cargo)
        .current_dir(root)
        .args([
            "build",
            "--locked",
            "--release",
            "--package",
            "hustlx-oss",
            "--target",
            &p.target,
            "--target-dir",
        ])
        .arg(root.join("target"))
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("Release build failed; no package was staged.".into());
    }
    let binary = root
        .join("target")
        .join(&p.target)
        .join("release")
        .join(if p.os == Os::Windows {
            "hustlx-oss.exe"
        } else {
            "hustlx-oss"
        });
    let payload = stage(&p, &binary)?;
    sign(&p, &payload)?;
    archive(&p, &payload)?;
    write_checksum(&p)?;
    println!(
        "Local package ready: {}\nNo release was published.",
        p.output.display()
    );
    Ok(())
}
pub fn run(args: Vec<String>) -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("Cannot find repository root")?;
    if args.is_empty() || args == ["--help"] || args == ["help"] {
        println!(
            "cargo xtask version [VERSION]\ncargo xtask release --version VERSION [--target TARGET] [--dry-run]\n\nVersion changes Cargo.toml/Cargo.lock only. Release builds local ZIPs/checksums only. Nothing publishes or tags."
        );
        return Ok(());
    }
    if args == ["version"] {
        println!("{}", current_version(root)?.full);
        return Ok(());
    }
    if args.first().is_some_and(|a| a == "version") && args.len() == 2 {
        prepare_version(root, &args[1])?;
        println!(
            "Prepared version {}. Review the Cargo.toml/Cargo.lock diff; nothing committed or tagged.",
            args[1]
        );
        return Ok(());
    }
    if args.first().is_none_or(|a| a != "release") {
        return Err("Unknown command. Use cargo xtask --help.".into());
    }
    let mut version = None;
    let mut target = None;
    let mut dry = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--version" | "--target" => {
                let flag = &args[i];
                i += 1;
                let value = args
                    .get(i)
                    .ok_or_else(|| format!("Missing value after {flag}"))?;
                let entry = if flag == "--version" {
                    &mut version
                } else {
                    &mut target
                };
                if entry.replace(value.as_str()).is_some() {
                    return Err(format!("Duplicate option {flag}"));
                }
            }
            "--dry-run" => dry = true,
            other => return Err(format!("Unknown release option: {other}")),
        }
        i += 1;
    }
    release(
        root,
        version.ok_or("Use --version VERSION; it must match Cargo.toml/Cargo.lock.")?,
        target,
        dry,
    )
}
#[cfg(test)]
mod tests;
