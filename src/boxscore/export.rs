//! Box score PDF and stats CSV export helpers: default file names and the atomic write.
//! No dialog or platform code lives here, so these compile without the desktop feature.
use crate::storage::MatchSummary;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Lowercase ASCII letters and digits; every other run becomes one dash, with no dashes at the ends.
fn slug(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut pending_dash = false;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(ch.to_ascii_lowercase());
        } else {
            pending_dash = true;
        }
    }
    out
}

/// `<date>-<home>-vs-<away>-boxscore.pdf`, safe for any file system.
pub fn default_file_name(summary: &MatchSummary) -> String {
    format!(
        "{}-{}-vs-{}-boxscore.pdf",
        slug(&summary.date),
        slug(&summary.home_name),
        slug(&summary.away_name)
    )
}

/// `<date>-<home>-vs-<away>-stats.csv`, safe for any file system.
pub fn default_csv_file_name(summary: &MatchSummary) -> String {
    format!(
        "{}-{}-vs-{}-stats.csv",
        slug(&summary.date),
        slug(&summary.home_name),
        slug(&summary.away_name)
    )
}

/// Appends `.<ext>` unless the path already ends in it (any case).
pub fn with_extension(path: PathBuf, ext: &str) -> PathBuf {
    let has_ext = path
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(ext));
    if has_ext {
        path
    } else {
        let mut os: OsString = path.into_os_string();
        os.push(".");
        os.push(ext);
        PathBuf::from(os)
    }
}

/// Appends `.pdf` unless the path already ends in it (any case).
pub fn with_pdf_extension(path: PathBuf) -> PathBuf {
    with_extension(path, "pdf")
}

/// Writes `<target>.tmp` then renames it over `<target>` (with `.pdf` ensured), so a reader
/// never sees a half-written file. Returns the final path.
pub fn write_atomic(target: &Path, bytes: &[u8]) -> std::io::Result<PathBuf> {
    write_atomic_as(target, "pdf", bytes)
}

/// [`write_atomic`] for any extension, which is appended when missing.
pub fn write_atomic_as(target: &Path, ext: &str, bytes: &[u8]) -> std::io::Result<PathBuf> {
    let target = with_extension(target.to_path_buf(), ext);
    let mut tmp_os: OsString = target.clone().into_os_string();
    tmp_os.push(".tmp");
    let tmp = PathBuf::from(tmp_os);
    std::fs::write(&tmp, bytes)?;
    if let Err(error) = std::fs::rename(&tmp, &target) {
        let _ = std::fs::remove_file(&tmp);
        return Err(error);
    }
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boxscore::fixtures;
    use crate::storage::MatchSummary;
    use std::path::PathBuf;

    #[test]
    fn default_file_name_is_date_home_vs_away_boxscore() {
        let s = MatchSummary {
            date: "2026-10-24".into(),
            home_name: "Batangas Blazers".into(),
            away_name: "Lipa City Hawks!".into(),
            ..fixtures::basketball_full().summary
        };
        assert_eq!(
            default_file_name(&s),
            "2026-10-24-batangas-blazers-vs-lipa-city-hawks-boxscore.pdf"
        );
    }

    #[test]
    fn csv_name_and_extension_follow_the_pdf_rules() {
        let s = MatchSummary {
            date: "2026-10-24".into(),
            home_name: "Batangas Blazers".into(),
            away_name: "Lipa City Hawks!".into(),
            ..fixtures::basketball_full().summary
        };
        assert_eq!(
            default_csv_file_name(&s),
            "2026-10-24-batangas-blazers-vs-lipa-city-hawks-stats.csv"
        );
        assert_eq!(
            with_extension("a/b".into(), "csv"),
            PathBuf::from("a/b.csv")
        );
        assert_eq!(
            with_extension("a/B.CSV".into(), "csv"),
            PathBuf::from("a/B.CSV")
        );
        let dir = tempfile::tempdir().unwrap();
        let written = write_atomic_as(&dir.path().join("x"), "csv", b"a,b").unwrap();
        assert_eq!(written, dir.path().join("x.csv"));
        assert!(!dir.path().join("x.csv.tmp").exists());
    }

    #[test]
    fn with_pdf_extension_appends_when_missing_and_keeps_existing() {
        assert_eq!(with_pdf_extension("a/b".into()), PathBuf::from("a/b.pdf"));
        assert_eq!(
            with_pdf_extension("a/B.PDF".into()),
            PathBuf::from("a/B.PDF")
        );
        assert_eq!(
            with_pdf_extension("a/b.txt".into()),
            PathBuf::from("a/b.txt.pdf")
        );
    }

    #[test]
    fn write_atomic_leaves_no_tmp_and_returns_pdf_path() {
        let dir = tempfile::tempdir().unwrap();
        let written = write_atomic(&dir.path().join("x"), b"%PDF-").unwrap();
        assert_eq!(written, dir.path().join("x.pdf"));
        assert_eq!(std::fs::read(&written).unwrap(), b"%PDF-");
        assert!(!dir.path().join("x.pdf.tmp").exists());
        assert!(!dir.path().join("x.tmp").exists());
    }

    #[test]
    fn write_atomic_reports_io_error_for_missing_folder() {
        let dir = tempfile::tempdir().unwrap();
        assert!(write_atomic(&dir.path().join("missing/x.pdf"), b"%PDF-").is_err());
    }
}
