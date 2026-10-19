use super::*;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "hustlx-release-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn fixture() -> Temp {
    let t = Temp::new();
    fs::write(t.0.join("Cargo.toml"),"[package]\nname = \"hustlx-oss\"\nversion = \"0.1.0\"\n\n[dependencies]\nother = \"3.0.0\"\n").unwrap();
    fs::write(t.0.join("Cargo.lock"),"version = 4\n\n[[package]]\nname = \"other\"\nversion = \"3.0.0\"\n\n[[package]]\nname = \"hustlx-oss\"\nversion = \"0.1.0\"\n\n[[package]]\nname = \"xtask\"\nversion = \"0.1.0\"\n").unwrap();
    fs::create_dir_all(t.0.join("assets/fonts")).unwrap();
    for file in [
        "THIRD_PARTY_NOTICES.md",
        "assets/fonts/EXO-OFL.txt",
        "assets/fonts/INTER-OFL.txt",
    ] {
        fs::write(t.0.join(file), file).unwrap();
    }
    t
}
#[test]
fn versions_follow_semver_and_keep_numeric_bundle_version() {
    for valid in ["0.1.0", "1.2.3-beta.1", "1.2.3-rc.0+build.9"] {
        let v = Version::parse(valid).unwrap();
        assert_eq!(v.full, valid);
    }
    assert_eq!(
        Version::parse("1.2.3-beta.1+sha.9").unwrap().numeric,
        "1.2.3"
    );
    for invalid in [
        "v1.0.0",
        "1.0",
        "01.0.0",
        "1.0.0-01",
        "1.0.0-",
        "1.0.0+",
        "1.0.0-foo..bar",
        "../escape",
        "1.0.0/evil",
        "1.0.0\n",
        "1.0.0+$()",
    ] {
        assert!(Version::parse(invalid).is_err(), "{invalid}");
    }
}
#[test]
fn version_preparation_keeps_dependency_and_tool_versions() {
    let t = fixture();
    prepare_version(&t.0, "1.2.3-beta.1").unwrap();
    assert!(
        fs::read_to_string(t.0.join("Cargo.toml"))
            .unwrap()
            .contains("other = \"3.0.0\"")
    );
    let lock = fs::read_to_string(t.0.join("Cargo.lock")).unwrap();
    assert!(lock.contains("name = \"xtask\"\nversion = \"0.1.0\""));
    assert_eq!(current_version(&t.0).unwrap().full, "1.2.3-beta.1");
    prepare_version(&t.0, "1.2.3-beta.1").unwrap();
}
#[test]
fn invalid_or_divergent_versions_never_mutate_files() {
    let t = fixture();
    let before = fs::read(t.0.join("Cargo.toml")).unwrap();
    assert!(prepare_version(&t.0, "bad").is_err());
    fs::write(
        t.0.join("Cargo.lock"),
        "[[package]]\nname = \"hustlx-oss\"\nversion = \"9.0.0\"\n",
    )
    .unwrap();
    assert!(prepare_version(&t.0, "0.2.0").is_err());
    assert_eq!(fs::read(t.0.join("Cargo.toml")).unwrap(), before);
}
#[test]
fn plans_reject_version_mismatch_cross_os_and_unsupported_targets() {
    let t = fixture();
    assert!(plan(&t.0, "0.2.0", "aarch64-apple-darwin", Os::Mac).is_err());
    assert!(plan(&t.0, "0.1.0", "x86_64-pc-windows-msvc", Os::Mac).is_err());
    assert!(plan(&t.0, "0.1.0", "x86_64-unknown-linux-gnu", Os::Mac).is_err());
    let p = plan(&t.0, "0.1.0", "aarch64-apple-darwin", Os::Mac).unwrap();
    assert!(!p.output.exists());
    assert!(!t.0.join("dist").exists());
}
#[test]
fn package_tree_contains_only_binary_metadata_and_required_notices() {
    let t = fixture();
    fs::write(t.0.join("synthetic-binary"), "fixture, not executable").unwrap();
    fs::write(t.0.join("private.sqlite3"), "never package").unwrap();
    for (os, target) in [
        (Os::Mac, "aarch64-apple-darwin"),
        (Os::Windows, "x86_64-pc-windows-msvc"),
    ] {
        let p = plan(&t.0, "0.1.0", target, os).unwrap();
        let payload = stage(&p, &t.0.join("synthetic-binary")).unwrap();
        let resources = if os == Os::Mac {
            payload.join("Contents/Resources")
        } else {
            payload.clone()
        };
        assert!(resources.join("THIRD_PARTY_NOTICES.md").exists());
        assert!(resources.join("licenses/EXO-OFL.txt").exists());
        assert!(resources.join("VERSION.txt").exists());
        assert!(!payload.join("private.sqlite3").exists());
        if os == Os::Mac {
            let plist = fs::read_to_string(payload.join("Contents/Info.plist")).unwrap();
            assert!(plist.contains("<string>0.1.0</string>"));
            assert!(plist.contains("com.hustlx.oss.desktop"));
            assert!(payload.join("Contents/MacOS/hustlx-oss").exists());
        } else {
            assert!(payload.join("hustlx-oss.exe").exists());
        }
        assert!(stage(&p, &t.0.join("synthetic-binary")).is_err());
    }
}
#[test]
fn native_archive_and_checksum_work_with_spaces_in_paths() {
    let t = fixture();
    fs::write(t.0.join("synthetic-binary"), "fixture, not executable").unwrap();
    let os = native_os().unwrap();
    let target = if os == Os::Mac {
        "aarch64-apple-darwin"
    } else {
        "x86_64-pc-windows-msvc"
    };
    let p = plan(&t.0, "0.1.0", target, os).unwrap();
    let payload = stage(&p, &t.0.join("synthetic-binary")).unwrap();
    archive(&p, &payload).unwrap();
    write_checksum(&p).unwrap();
    assert!(p.archive.exists());
    let checksum = fs::read_to_string(p.output.join("SHA256SUMS")).unwrap();
    assert_eq!(checksum.split_whitespace().next().unwrap().len(), 64);
    assert!(checksum.contains(p.archive.file_name().unwrap().to_str().unwrap()));
    extract_fixture(&p).unwrap();
}

fn extract_fixture(p: &Plan) -> Result<()> {
    let destination = p.output.join("extraction check");
    match p.os {
        Os::Mac => {
            output(
                Command::new("/usr/bin/ditto")
                    .args(["-x", "-k"])
                    .arg(&p.archive)
                    .arg(&destination),
            )?;
        }
        Os::Windows => {
            output(powershell("$ErrorActionPreference='Stop'; Expand-Archive -LiteralPath $env:HUSTLX_PACKAGE_ARCHIVE -DestinationPath $env:HUSTLX_PACKAGE_EXTRACT").env("HUSTLX_PACKAGE_ARCHIVE",&p.archive).env("HUSTLX_PACKAGE_EXTRACT",&destination))?;
        }
    }
    let relative = if p.os == Os::Mac {
        "HustlX OSS.app/Contents/MacOS/hustlx-oss"
    } else {
        "HustlX OSS/hustlx-oss.exe"
    };
    assert_eq!(
        fs::read_to_string(destination.join(relative)).unwrap(),
        "fixture, not executable"
    );
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn synthetic_macos_app_has_valid_plist_and_ad_hoc_signature() {
    let t = fixture();
    let source = t.0.join("fixture.rs");
    let executable = t.0.join("fixture executable");
    fs::write(&source, "fn main() {}\n").unwrap();
    output(
        Command::new("rustc")
            .arg(&source)
            .arg("-o")
            .arg(&executable),
    )
    .unwrap();
    let p = plan(&t.0, "0.1.0", &host().unwrap(), Os::Mac).unwrap();
    let payload = stage(&p, &executable).unwrap();
    output(
        Command::new("/usr/bin/plutil")
            .arg("-lint")
            .arg(payload.join("Contents/Info.plist")),
    )
    .unwrap();
    sign(&p, &payload).unwrap();
}
