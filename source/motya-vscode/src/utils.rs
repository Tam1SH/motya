use path_clean::PathClean;
use std::path::{Path, PathBuf};

pub fn normalize_path(base: &Path, relative: &str) -> PathBuf {
    base.join(relative).clean()
}
