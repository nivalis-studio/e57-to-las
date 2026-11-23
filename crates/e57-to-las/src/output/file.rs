use std::ffi::OsString;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use super::{Output, Target};

/// Writes each output to a file whose path comes from a template.
///
/// The template is a path that may contain placeholders:
///
/// | Placeholder | Replaced by |
/// |---|---|
/// | `{key}` | the [group key](super::GroupKey) |
/// | `{index}` | index of the first point cloud of the output |
/// | `{name}` | name of the first point cloud (its index if it has none) |
/// | `{guid}` | GUID of the first point cloud (its index if it has none) |
/// | `{stem}` | file name of the input without extension (`input` for non-file inputs) |
///
/// Substituted values are made safe for file names: characters other than
/// letters, digits, `-`, `_` and `.` become `_`. Two outputs resolving to the
/// same path fail the conversion before anything is written.
///
/// Files are written next to their final path with a `.tmp` suffix and renamed
/// once complete, so a failed or cancelled conversion leaves no partial file.
/// Missing directories are created.
///
/// ```
/// use e57_to_las::output::FileOutput;
///
/// let merged = FileOutput::new("out/scan.laz");
/// let per_station = FileOutput::new("out/las/{index}.las");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileOutput {
    template: PathBuf,
}

impl FileOutput {
    /// Files at `template`, see [`FileOutput`].
    pub fn new(template: impl Into<PathBuf>) -> Self {
        FileOutput {
            template: template.into(),
        }
    }

    /// The path template.
    pub fn template(&self) -> &Path {
        &self.template
    }

    /// Whether the template has a placeholder, so it can name several outputs.
    pub fn has_placeholders(&self) -> bool {
        let template = self.template.to_string_lossy();
        PLACEHOLDERS.iter().any(|p| template.contains(p))
    }

    /// The path of `target`.
    pub fn path(&self, target: &Target<'_>) -> PathBuf {
        let Some(template) = self.template.to_str() else {
            // Not UTF-8, so it holds no placeholder we could have written.
            return self.template.clone();
        };
        if !self.has_placeholders() {
            return self.template.clone();
        }
        let first = &target.clouds[0];
        let index = first.index.to_string();
        let path = template
            .replace("{key}", &sanitize(target.key.as_str()))
            .replace("{index}", &index)
            .replace("{name}", &sanitize(first.name.as_deref().unwrap_or(&index)))
            .replace("{guid}", &sanitize(first.guid.as_deref().unwrap_or(&index)))
            .replace("{stem}", &sanitize(target.input_stem.unwrap_or("input")));
        PathBuf::from(path)
    }
}

const PLACEHOLDERS: [&str; 5] = ["{key}", "{index}", "{name}", "{guid}", "{stem}"];

/// Keeps letters, digits, `-`, `_` and `.`; everything else becomes `_`.
/// Leading dots are replaced too, so a value cannot be `..`.
fn sanitize(value: &str) -> String {
    let mut out: String = value
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.starts_with('.') {
        out.replace_range(..1, "_");
    }
    if out.is_empty() {
        out.push('_');
    }
    out
}

fn temp_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().map(OsString::from).unwrap_or_default();
    name.push(".tmp");
    path.with_file_name(name)
}

impl Output for FileOutput {
    type Writer = BufWriter<File>;

    fn location(&self, target: &Target<'_>) -> Option<PathBuf> {
        Some(self.path(target))
    }

    fn create(&self, target: &Target<'_>) -> std::io::Result<Self::Writer> {
        let path = self.path(target);
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        File::create(temp_path(&path)).map(BufWriter::new)
    }

    fn finish(&self, target: &Target<'_>, writer: Self::Writer) -> std::io::Result<()> {
        let path = self.path(target);
        let mut file = writer.into_inner().map_err(|err| err.into_error())?;
        file.flush()?;
        drop(file);
        std::fs::rename(temp_path(&path), &path)
    }

    fn abort(&self, target: &Target<'_>) {
        // Best effort: the temporary file may never have been created.
        let _ = std::fs::remove_file(temp_path(&self.path(target)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CloudInfo, GroupKey, output::Format};

    fn cloud(index: usize, name: Option<&str>) -> CloudInfo {
        let mut info = CloudInfo::for_tests(index);
        info.guid = Some(format!("{{guid-{index}}}"));
        info.name = name.map(str::to_owned);
        info
    }

    fn path_of(template: &str, key: &str, clouds: &[CloudInfo]) -> PathBuf {
        let key = GroupKey::from(key);
        FileOutput::new(template).path(&Target {
            key: &key,
            clouds,
            format: Format::Las,
            input_stem: Some("scan 01"),
        })
    }

    #[test]
    fn placeholders() {
        let clouds = [cloud(3, Some("Station/../7"))];
        assert_eq!(
            path_of("out/{stem}/{index}_{name}.las", "3", &clouds),
            PathBuf::from("out/scan_01/3_Station_.._7.las")
        );
        assert_eq!(
            path_of("{guid}-{key}.laz", "k", &clouds),
            PathBuf::from("_guid-3_-k.laz")
        );
        let unnamed = [cloud(5, None)];
        assert_eq!(path_of("{name}.las", "5", &unnamed), PathBuf::from("5.las"));
    }

    #[test]
    fn sanitize_never_climbs() {
        assert_eq!(sanitize(".."), "_.");
        assert_eq!(sanitize("a/b\\c"), "a_b_c");
        assert_eq!(sanitize(""), "_");
    }

    #[test]
    fn no_placeholder_keeps_the_path() {
        assert_eq!(
            path_of("a/b.las", "x", &[cloud(0, None)]),
            PathBuf::from("a/b.las")
        );
        assert!(!FileOutput::new("a/b.las").has_placeholders());
        assert!(FileOutput::new("a/{index}.las").has_placeholders());
    }

    #[test]
    fn temp_path_appends_a_suffix() {
        assert_eq!(
            temp_path(Path::new("a/0.las")),
            PathBuf::from("a/0.las.tmp")
        );
    }
}
