use anyhow::Context;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

pub fn search_folders_matching_regex<P: AsRef<Path>>(
    path: P,
    re: &Regex,
) -> anyhow::Result<Vec<PathBuf>> {
    let mut matching_folders = Vec::new();

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let folder_name = path
                .file_name()
                .context("Failed to geth the path. Make sure nothing ends in '..'")?
                .to_string_lossy();

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
