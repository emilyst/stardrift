//! Path utilities

use std::path::PathBuf;

/// Expands a leading `~` (alone or followed by a separator) to the user's
/// home directory. `~user` forms and non-leading tildes are returned
/// unchanged, as is everything on platforms without a resolvable home
/// directory (e.g. WebAssembly).
pub fn expand_tilde(path: &str) -> PathBuf {
    #[cfg(not(target_arch = "wasm32"))]
    if (path == "~" || path.starts_with("~/") || path.starts_with("~\\"))
        && let Some(user_dirs) = directories::UserDirs::new()
    {
        let home = user_dirs.home_dir();
        return if path == "~" {
            home.to_path_buf()
        } else {
            home.join(&path[2..])
        };
    }
    PathBuf::from(path)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    fn expands_leading_tilde() {
        let home = directories::UserDirs::new()
            .unwrap()
            .home_dir()
            .to_path_buf();
        assert_eq!(expand_tilde("~"), home);
        assert_eq!(expand_tilde("~/Pictures"), home.join("Pictures"));
    }

    #[test]
    fn leaves_other_paths_alone() {
        assert_eq!(expand_tilde("/abs/path"), PathBuf::from("/abs/path"));
        assert_eq!(expand_tilde("rel/path"), PathBuf::from("rel/path"));
        assert_eq!(expand_tilde("~user/x"), PathBuf::from("~user/x"));
        assert_eq!(expand_tilde("a/~/b"), PathBuf::from("a/~/b"));
    }
}
