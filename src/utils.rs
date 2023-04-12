use regex::Regex;
use std::fs;
use std::io::Error;
use std::path::{Path, PathBuf};

pub fn search_folders_matching_regex<P: AsRef<Path>>(
    path: P,
    re: &Regex,
) -> Result<Vec<PathBuf>, Error> {
    let mut matching_folders = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let folder_name = path.file_name().unwrap().to_string_lossy();

            if re.is_match(&folder_name) {
                matching_folders.push(path.clone());
            }

            // Recursively search subfolders
            let mut subfolder_matches = search_folders_matching_regex(path, re)?;
            matching_folders.append(&mut subfolder_matches);
        }
    }

    Ok(matching_folders)
}
