use std::path::{Component, Path, PathBuf};

const HARNESS_FILES: &[&str] = &["conftest.py", "Directory.Build.props"];

pub fn detect(patch_paths: &[PathBuf]) -> Vec<PathBuf> {
    patch_paths
        .iter()
        .filter(|p| is_protected(p))
        .cloned()
        .collect()
}

fn is_protected(path: &Path) -> bool {
    // Check each component of the path
    let components: Vec<_> = path.components().collect();
    
    for (idx, component) in components.iter().enumerate() {
        if let Component::Normal(name) = component {
            let name_str = name.to_string_lossy();
            
            // Check if this is a "tests" or "test" directory component
            // It must be a directory (not the final component) to be protected
            if (name_str == "tests" || name_str == "test") && idx < components.len() - 1 {
                return true;
            }
            
            // Check if this is a harness file
            if HARNESS_FILES.contains(&name_str.as_ref()) {
                return true;
            }
        }
    }
    
    false
}
