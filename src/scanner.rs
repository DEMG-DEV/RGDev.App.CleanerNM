use std::collections::HashSet;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Clone, Debug)]
pub struct TargetCategory {
    #[allow(dead_code)]
    pub id: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub targets: &'static [&'static str],
    pub enabled: bool,
}

impl TargetCategory {
    pub fn default_categories() -> Vec<Self> {
        vec![
            TargetCategory {
                id: "node",
                label: "Node.js",
                icon: "NODE",
                targets: &["node_modules", "dist"],
                enabled: true,
            },
            TargetCategory {
                id: "dotnet",
                label: ".NET",
                icon: ".NET",
                targets: &["bin", "obj"],
                enabled: true,
            },
            TargetCategory {
                id: "rust",
                label: "Rust",
                icon: "RUST",
                targets: &["target"],
                enabled: true,
            },
            TargetCategory {
                id: "go",
                label: "Go",
                icon: "GO",
                targets: &["bin", "pkg"],
                enabled: true,
            },
            TargetCategory {
                id: "python",
                label: "Python",
                icon: "PYTHON",
                targets: &["__pycache__", "venv", ".venv", ".pytest_cache", ".tox"],
                enabled: true,
            },
            TargetCategory {
                id: "flutter",
                label: "Flutter / Dart",
                icon: "FLUTTER",
                targets: &["build", ".dart_tool"],
                enabled: true,
            },
        ]
    }
}

#[derive(Clone, Debug)]
pub struct ScanItem {
    pub path: PathBuf,
    pub folder_name: String,
    pub category: String,
    pub project_name: String,
    pub relative_path: String,
    pub size_bytes: u64,
    pub selected: bool,
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".to_string();
    }
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
    let k = 1024_f64;
    let i = ((bytes as f64).log(k).floor() as usize).min(UNITS.len() - 1);
    let val = (bytes as f64) / k.powi(i as i32);
    if i == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.2} {}", val, UNITS[i])
    }
}

pub fn get_dir_size(path: &Path) -> u64 {
    let mut total = 0;
    for entry in WalkDir::new(path).min_depth(1).into_iter().filter_map(|e| e.ok()) {
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                total += meta.len();
            }
        }
    }
    total
}

pub enum ScanMessage {
    Found(ScanItem),
    Progress { scanned_dirs: usize, current_path: String },
    Finished,
}

pub enum DeleteMessage {
    Progress { current: usize, total: usize, path: String },
    Done { deleted_count: usize, freed_bytes: u64, errors: Vec<String> },
}

pub fn start_scan(
    root: PathBuf,
    categories: Vec<TargetCategory>,
    sender: crossbeam_channel::Sender<ScanMessage>,
) {
    std::thread::spawn(move || {
        let mut target_to_category = std::collections::HashMap::new();
        let mut target_names = HashSet::new();

        for cat in &categories {
            if cat.enabled {
                for &target in cat.targets {
                    target_names.insert(target.to_string());
                    target_to_category.insert(target.to_string(), cat.label.to_string());
                }
            }
        }

        let mut it = WalkDir::new(&root).into_iter();
        let mut count = 0;

        while let Some(res) = it.next() {
            if let Ok(entry) = res {
                count += 1;
                if count % 100 == 0 {
                    let _ = sender.send(ScanMessage::Progress {
                        scanned_dirs: count,
                        current_path: entry.path().display().to_string(),
                    });
                }

                if entry.depth() > 0 && entry.file_type().is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if target_names.contains(&name) {
                        let path = entry.path().to_path_buf();
                        let size = get_dir_size(&path);
                        let category = target_to_category
                            .get(&name)
                            .cloned()
                            .unwrap_or_else(|| "Other".to_string());

                        let relative_path = path
                            .strip_prefix(&root)
                            .unwrap_or(&path)
                            .to_string_lossy()
                            .to_string();

                        let project_name = path
                            .parent()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "Root".to_string());

                        let _ = sender.send(ScanMessage::Found(ScanItem {
                            path: path.clone(),
                            folder_name: name,
                            category,
                            project_name,
                            relative_path,
                            size_bytes: size,
                            selected: true,
                        }));

                        it.skip_current_dir();
                    }
                }
            }
        }

        let _ = sender.send(ScanMessage::Finished);
    });
}

pub fn start_delete(
    items: Vec<ScanItem>,
    sender: crossbeam_channel::Sender<DeleteMessage>,
) {
    std::thread::spawn(move || {
        let total = items.len();
        let mut deleted = 0;
        let mut freed = 0;
        let mut errors = Vec::new();

        for (idx, item) in items.into_iter().enumerate() {
            let _ = sender.send(DeleteMessage::Progress {
                current: idx + 1,
                total,
                path: item.path.display().to_string(),
            });

            match std::fs::remove_dir_all(&item.path) {
                Ok(_) => {
                    deleted += 1;
                    freed += item.size_bytes;
                }
                Err(err) => {
                    errors.push(format!("{}: {}", item.path.display(), err));
                }
            }
        }

        let _ = sender.send(DeleteMessage::Done {
            deleted_count: deleted,
            freed_bytes: freed,
            errors,
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
    }

    #[test]
    fn test_target_categories_defaults() {
        let categories = TargetCategory::default_categories();
        assert_eq!(categories.len(), 6);
        assert!(categories.iter().any(|c| c.id == "node"));
        assert!(categories.iter().any(|c| c.id == "rust"));
    }
}
