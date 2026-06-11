//! Resolve dropped paths into candidate video files (B3).
//!
//! [`scan`] takes the raw paths from a drop (files and/or folders) and returns
//! the candidate files to probe. A dropped folder is walked **flat** — its
//! immediate files only, no subdirectory recursion (ADR-0009). Candidates are
//! filtered by extension (ADR-0011):
//!
//! - known video extension → candidate;
//! - no extension → candidate, deferred to the probe fallback (ADR-0011);
//! - any other (known non-media or unknown) extension → skipped.
//!
//! The scanner does filesystem IO (it reads directories) but never probes,
//! so the flat-walk + filter rules are table-tested over a temp tree.
//! Probing each candidate for duration / video-stream confirmation stays in
//! pre-flight (`commands.rs`), per the ADR-0010 separation.

use std::path::{Path, PathBuf};

/// Broad allowlist of known video container extensions (lowercased, no dot).
/// Deliberately wide to match the "any video file" goal (ADR-0011, ADR-0005).
const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "m4v", "mov", "mkv", "webm", "avi", "wmv", "flv", "f4v", "mpg", "mpeg", "m2v", "mts",
    "m2ts", "ts", "3gp", "3g2", "ogv", "vob", "asf", "divx", "dv", "mxf",
];

/// Resolve dropped paths into a deduplicated list of candidate files.
///
/// Top-level dropped order is preserved; each folder's immediate files are
/// sorted for deterministic output. A file reachable twice (dropped directly
/// and via its folder) appears once.
pub fn scan<I, S>(dropped: I) -> Vec<PathBuf>
where
    I: IntoIterator<Item = S>,
    S: AsRef<Path>,
{
    let mut out: Vec<PathBuf> = Vec::new();
    let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

    let mut push = |path: PathBuf, out: &mut Vec<PathBuf>| {
        if seen.insert(path.clone()) {
            out.push(path);
        }
    };

    for entry in dropped {
        let path = entry.as_ref();
        if path.is_dir() {
            // Flat walk: immediate files only, no recursion (ADR-0009).
            let mut files: Vec<PathBuf> = match std::fs::read_dir(path) {
                Ok(rd) => rd
                    .filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|p| p.is_file() && is_video_candidate(p))
                    .collect(),
                Err(_) => continue,
            };
            files.sort();
            for f in files {
                push(f, &mut out);
            }
        } else if path.is_file() && is_video_candidate(path) {
            push(path.to_path_buf(), &mut out);
        }
    }

    out
}

/// Whether a file's extension marks it as a video candidate (ADR-0011).
/// Extensionless files are candidates too — the probe fallback decides.
fn is_video_candidate(path: &Path) -> bool {
    match path.extension() {
        None => true,
        Some(ext) => {
            let ext = ext.to_string_lossy().to_ascii_lowercase();
            VIDEO_EXTENSIONS.contains(&ext.as_str())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// A unique scratch dir under the OS temp dir, cleaned up on drop.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new(tag: &str) -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let dir = std::env::temp_dir().join(format!("ruckel-scan-{tag}-{nanos}"));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
        fn touch(&self, rel: &str) -> PathBuf {
            let p = self.0.join(rel);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&p, b"x").unwrap();
            p
        }
        fn mkdir(&self, rel: &str) -> PathBuf {
            let p = self.0.join(rel);
            std::fs::create_dir_all(&p).unwrap();
            p
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn names(paths: &[PathBuf]) -> HashSet<String> {
        paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn pure_extension_filter_keeps_video_and_extensionless_only() {
        for keep in ["a.mp4", "B.MOV", "clip.mkv", "no_extension"] {
            assert!(is_video_candidate(Path::new(keep)), "should keep {keep:?}");
        }
        for skip in ["notes.txt", "deck.pptx", "image.png", "archive.zip"] {
            assert!(!is_video_candidate(Path::new(skip)), "should skip {skip:?}");
        }
    }

    #[test]
    fn folder_walk_is_flat_and_filters_non_video() {
        let s = Scratch::new("flat");
        s.touch("keep.mp4");
        s.touch("keep2.mov");
        s.touch("skip.txt");
        s.touch("skip.pdf");
        s.touch("nested/deep.mp4"); // a subfolder file — must NOT be recursed into

        let got = scan([s.0.clone()]);
        assert_eq!(names(&got), HashSet::from(["keep.mp4".into(), "keep2.mov".into()]));
    }

    #[test]
    fn dropped_files_are_filtered_by_extension() {
        let s = Scratch::new("files");
        let mp4 = s.touch("movie.mp4");
        let txt = s.touch("readme.txt");

        let got = scan([mp4.clone(), txt]);
        assert_eq!(got, vec![mp4]);
    }

    #[test]
    fn mix_of_files_and_folders_resolves_the_union() {
        let s = Scratch::new("mix");
        let loose = s.touch("loose.webm");
        s.mkdir("bundle");
        s.touch("bundle/one.mp4");
        s.touch("bundle/two.avi");
        s.touch("bundle/ignore.docx");
        let folder = s.0.join("bundle");

        let got = scan([loose.clone(), folder]);
        assert_eq!(
            names(&got),
            HashSet::from(["loose.webm".into(), "one.mp4".into(), "two.avi".into()])
        );
    }

    #[test]
    fn a_file_dropped_directly_and_via_its_folder_appears_once() {
        let s = Scratch::new("dedup");
        let direct = s.touch("clip.mp4");
        let folder = s.0.clone();

        let got = scan([folder, s.0.clone()]); // same folder twice + ...
        let got2 = scan([direct.clone(), s.0.clone()]); // direct file + its folder
        assert_eq!(got.iter().filter(|p| **p == direct).count(), 1);
        assert_eq!(got2.iter().filter(|p| **p == direct).count(), 1);
    }

    #[test]
    fn nonexistent_paths_are_ignored() {
        let got = scan([PathBuf::from("does-not-exist-xyz.mp4")]);
        assert!(got.is_empty());
    }
}
