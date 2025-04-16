// src/main.rs
use std::env;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use toml::Value;
use walkdir::WalkDir;

#[cfg(not(windows))]
use clipboard::{ClipboardContext, ClipboardProvider};

#[cfg(windows)]
use clipboard_win::{formats, Clipboard, Setter};

struct FileEntry {
    path: String,
    content: String,
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: rust-code-copier <directory_path>");
        return Ok(());
    }

    let dir_path = &args[1];
    println!("Processing directory: {}", dir_path);

    // Collect all files
    let files = collect_files(dir_path)?;

    // Extract Cargo.toml info if it exists
    let cargo_info = extract_cargo_info(dir_path);

    // Format the output
    let formatted_output = format_for_llm(files, cargo_info);

    // Copy to clipboard
    copy_to_clipboard(&formatted_output)?;

    println!("Project successfully copied to clipboard!");
    println!(
        "Files processed: {}",
        formatted_output.matches("<file ").count()
    );
    println!("Total size: {} characters", formatted_output.len());

    Ok(())
}

fn collect_files(dir_path: &str) -> io::Result<Vec<FileEntry>> {
    let mut files = Vec::new();
    let excluded_dirs = vec![
        ".git",
        "target",
        "node_modules",
        ".vscode",
        ".idea",
        ".github",
        "dist",
        "build",
        "out",
    ];

    let excluded_ext = vec![
        ".exe", ".dll", ".so", ".dylib", ".o", ".obj", ".a", ".lib", ".bin", ".png", ".jpg",
        ".jpeg", ".gif", ".svg", ".ico", ".woff", ".woff2", ".ttf", ".eot",
    ];

    for entry in WalkDir::new(dir_path)
        .into_iter()
        .filter_entry(|e| {
            let path = e.path();
            let is_excluded_dir = path.is_dir()
                && excluded_dirs.iter().any(|excluded| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .map_or(false, |name| name == *excluded)
                });

            !is_excluded_dir
        })
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        if path.is_file() {
            // Skip binary or image files
            if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                if excluded_ext
                    .iter()
                    .any(|excluded| excluded.trim_start_matches(".") == ext)
                {
                    continue;
                }
            }

            // Skip large files (> 100KB)
            if let Ok(metadata) = fs::metadata(path) {
                if metadata.len() > 100 * 1024 {
                    println!("Skipping large file: {}", path.display());
                    continue;
                }
            }

            // Read file content
            match read_file(path) {
                Ok(content) => {
                    let relative_path = path
                        .strip_prefix(dir_path)
                        .unwrap_or(path)
                        .to_string_lossy()
                        .to_string();

                    files.push(FileEntry {
                        path: relative_path,
                        content,
                    });
                }
                Err(e) => {
                    eprintln!("Error reading file {}: {}", path.display(), e);
                }
            }
        }
    }

    Ok(files)
}

fn read_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

fn extract_cargo_info(dir_path: &str) -> Option<String> {
    let cargo_path = PathBuf::from(dir_path).join("Cargo.toml");

    if !cargo_path.exists() {
        return None;
    }

    let mut content = String::new();
    if let Ok(mut file) = File::open(&cargo_path) {
        if file.read_to_string(&mut content).is_err() {
            return None;
        }
    } else {
        return None;
    }

    match content.parse::<Value>() {
        Ok(cargo_toml) => {
            let mut info = String::new();

            // Extract project name and version
            if let Some(package) = cargo_toml.get("package") {
                if let Some(name) = package.get("name").and_then(|v| v.as_str()) {
                    info.push_str(&format!("Project Name: {}\n", name));
                }

                if let Some(version) = package.get("version").and_then(|v| v.as_str()) {
                    info.push_str(&format!("Version: {}\n", version));
                }

                if let Some(description) = package.get("description").and_then(|v| v.as_str()) {
                    info.push_str(&format!("Description: {}\n", description));
                }
            }

            // Extract dependencies
            if let Some(deps) = cargo_toml.get("dependencies") {
                if let Some(deps_table) = deps.as_table() {
                    info.push_str("\nDependencies:\n");

                    for (name, value) in deps_table {
                        match value {
                            Value::String(version) => {
                                info.push_str(&format!("- {} = \"{}\"\n", name, version));
                            }
                            Value::Table(table) => {
                                if let Some(version) = table.get("version").and_then(|v| v.as_str())
                                {
                                    info.push_str(&format!("- {} = \"{}\"\n", name, version));
                                } else {
                                    info.push_str(&format!("- {}\n", name));
                                }
                            }
                            _ => {
                                info.push_str(&format!("- {}\n", name));
                            }
                        }
                    }
                }
            }

            // Extract dev-dependencies
            if let Some(deps) = cargo_toml.get("dev-dependencies") {
                if let Some(deps_table) = deps.as_table() {
                    info.push_str("\nDev Dependencies:\n");

                    for (name, value) in deps_table {
                        match value {
                            Value::String(version) => {
                                info.push_str(&format!("- {} = \"{}\"\n", name, version));
                            }
                            Value::Table(table) => {
                                if let Some(version) = table.get("version").and_then(|v| v.as_str())
                                {
                                    info.push_str(&format!("- {} = \"{}\"\n", name, version));
                                } else {
                                    info.push_str(&format!("- {}\n", name));
                                }
                            }
                            _ => {
                                info.push_str(&format!("- {}\n", name));
                            }
                        }
                    }
                }
            }

            Some(info)
        }
        Err(_) => None,
    }
}

fn format_for_llm(files: Vec<FileEntry>, cargo_info: Option<String>) -> String {
    let mut output = String::new();

    // Add project metadata
    output.push_str("<project>\n");

    // Add Cargo.toml information if available
    if let Some(info) = cargo_info {
        output.push_str("<cargo_info>\n");
        output.push_str(&info);
        output.push_str("</cargo_info>\n\n");
    }

    // Add file structure information
    output.push_str("<file_structure>\n");

    // Get directory structure and format it nicely
    let dir_structure = get_directory_structure(files.iter().map(|f| &f.path).collect());
    output.push_str(&dir_structure);

    output.push_str("</file_structure>\n\n");

    // Add each file with content
    for file in files {
        output.push_str(&format!("<file path=\"{}\">\n", file.path));
        output.push_str(&file.content);
        output.push_str("\n</file>\n\n");
    }

    output.push_str("</project>");

    output
}

fn get_directory_structure(paths: Vec<&String>) -> String {
    let mut structure = String::new();
    let current_indent = 0;
    let mut current_dirs: Vec<String> = Vec::new();

    // Sort paths to ensure directories are processed in order
    let mut sorted_paths = paths.clone();
    sorted_paths.sort();

    for path in sorted_paths {
        let parts: Vec<&str> = path.split('/').collect();

        // Handle directory structure
        for (i, part) in parts.iter().enumerate().take(parts.len() - 1) {
            let dir_path = parts[0..=i].join("/");

            if i >= current_dirs.len() {
                // New directory level
                structure.push_str(&format!("{:indent$}└── {}/\n", "", part, indent = i * 2));
                current_dirs.push(dir_path);
            } else if current_dirs[i] != dir_path {
                // New directory at existing level
                structure.push_str(&format!("{:indent$}└── {}/\n", "", part, indent = i * 2));
                current_dirs[i] = dir_path;

                // Clear deeper levels
                current_dirs.truncate(i + 1);
            }
        }

        // Add file with appropriate indentation
        let file_name = parts.last().unwrap_or(&"");
        let indent = (parts.len() - 1) * 2;
        structure.push_str(&format!(
            "{:indent$}├── {}\n",
            "",
            file_name,
            indent = indent
        ));
    }

    structure
}

#[cfg(not(windows))]
fn copy_to_clipboard(text: &str) -> io::Result<()> {
    match ClipboardProvider::new() {
        Ok(mut ctx) => ctx
            .set_contents(text.to_owned())
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Clipboard error: {}", e))),
        Err(e) => Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Clipboard error: {}", e),
        )),
    }
}

#[cfg(windows)]
fn copy_to_clipboard(text: &str) -> io::Result<()> {
    match Clipboard::new_attempts(10) {
        Ok(_clip) => formats::Unicode
            .write_clipboard(&text)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Clipboard error: {:?}", e))),
        Err(e) => Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Clipboard error: {:?}", e),
        )),
    }
}
