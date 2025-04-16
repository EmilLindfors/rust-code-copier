// src/main.rs
use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use toml::Value;
use walkdir::WalkDir;

#[cfg(not(windows))]
use clipboard::{ClipboardContext, ClipboardProvider};

#[cfg(windows)]
use clipboard_win::{Clipboard, formats, Getter, Setter};

struct FileEntry {
    path: String,
    content: String,
}


#[derive(Debug, Clone, PartialEq)]
enum ProjectType {
    Rust,
    Python,
    Unknown,
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: code-copier <file_or_directory_paths> [--cargo-toml <cargo_toml_path>] [--pyproject <pyproject_path>]");
        return Ok(());
    }
    
    println!("Processing paths...");
    
    // Parse arguments
    let mut paths: Vec<String> = Vec::new();
    let mut cargo_toml_path: Option<String> = None;
    let mut pyproject_path: Option<String> = None;
    
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--cargo-toml" && i + 1 < args.len() {
            cargo_toml_path = Some(args[i + 1].clone());
            i += 2;
        } else if args[i] == "--pyproject" && i + 1 < args.len() {
            pyproject_path = Some(args[i + 1].clone());
            i += 2;
        } else {
            paths.push(args[i].clone());
            i += 1;
        }
    }
    
    // Collect all files from specified paths
    let mut files = Vec::new();
    for path_str in &paths {
        collect_files_from_path(path_str, &mut files)?;
    }
    
    // Detect project type and extract metadata
    let (project_type, project_info) = detect_project_type_and_extract_info(&paths, cargo_toml_path, pyproject_path);
    
    // Format the output
    let formatted_output = format_for_llm(files, project_type.clone(), project_info);
    
    // Copy to clipboard
    copy_to_clipboard(&formatted_output)?;
    
    println!("Files successfully copied to clipboard!");
    println!("Files processed: {}", formatted_output.matches("<file ").count());
    println!("Total size: {} characters", formatted_output.len());
    println!("Project type: {}", match project_type {
        ProjectType::Rust => "Rust",
        ProjectType::Python => "Python",
        ProjectType::Unknown => "Unknown",
    });
    
    Ok(())
}

fn detect_project_type_and_extract_info(
    paths: &[String], 
    cargo_toml_path: Option<String>, 
    pyproject_path: Option<String>
) -> (ProjectType, Option<String>) {
    // If cargo_toml_path is explicitly provided, it's a Rust project
    if let Some(ref path) = cargo_toml_path {
        if let Some(info) = extract_cargo_info(path) {
            return (ProjectType::Rust, Some(info));
        }
    }
    
    // If pyproject_path is explicitly provided, it's a Python project
    if let Some(ref path) = pyproject_path {
        if Path::new(path).exists() {
            if let Some(info) = extract_python_project_info(path) {
                return (ProjectType::Python, Some(info));
            }
        }
    }
    
    // Try to find project files in the given paths
    if !paths.is_empty() {
        let path = Path::new(&paths[0]);
        let dir_path = if path.is_file() {
            path.parent().map(|p| p.to_path_buf())
        } else {
            Some(path.to_path_buf())
        };
        
        if let Some(dir) = dir_path {
            // Check for Rust project first
            if let Some(info) = find_and_extract_cargo_info(&dir) {
                return (ProjectType::Rust, Some(info));
            }
            
            // Then check for Python project
            if let Some(info) = find_and_extract_python_info(&dir) {
                return (ProjectType::Python, Some(info));
            }
        }
    }
    
    // If no specific project info was found
    (ProjectType::Unknown, None)
}

fn collect_files_from_path(path_str: &str, files: &mut Vec<FileEntry>) -> io::Result<()> {
    let path = Path::new(path_str);
    
    if path.is_file() {
        // If path is a file, just add it
        process_file(path, path.to_string_lossy().to_string(), files)?;
    } else if path.is_dir() {
        // If path is a directory, walk through it
        let base_dir = path.to_string_lossy().to_string();
        for entry in WalkDir::new(path)
            .into_iter()
            .filter_entry(|e| !should_exclude_entry(e))
            .filter_map(|e| e.ok()) {
                
            let entry_path = entry.path();
            
            if entry_path.is_file() {
                process_file(entry_path, base_dir.clone(), files)?;
            }
        }
    }
    
    Ok(())
}

fn should_exclude_entry(entry: &walkdir::DirEntry) -> bool {
    let excluded_dirs = vec![
        ".git", "target", "node_modules", ".vscode", ".idea", 
        ".github", "dist", "build", "out", "__pycache__", 
        ".pytest_cache", ".mypy_cache", ".tox", ".eggs", 
        "*.egg-info", ".ipynb_checkpoints", "venv", "env", ".env"
    ];
    
    let path = entry.path();
    if path.is_dir() {
        return excluded_dirs.iter().any(|excluded| {
            let excluded = excluded.trim_start_matches('*');
            path.file_name()
                .and_then(|name| name.to_str())
                .map_or(false, |name| {
                    if excluded.starts_with('.') {
                        name == excluded
                    } else {
                        name == excluded || name.ends_with(excluded)
                    }
                })
        });
    }
    
    false
}

fn process_file(file_path: &Path, base_dir: String, files: &mut Vec<FileEntry>) -> io::Result<()> {
    let excluded_ext = vec![
        ".exe", ".dll", ".so", ".dylib", ".o", ".obj", ".a", 
        ".lib", ".bin", ".png", ".jpg", ".jpeg", ".gif", 
        ".svg", ".ico", ".woff", ".woff2", ".ttf", ".eot",
        ".pyc", ".pyd", ".pyo", ".class", ".jar"
    ];
    
    // Skip binary or image files
    if let Some(ext) = file_path.extension().and_then(|ext| ext.to_str()) {
        if excluded_ext.iter().any(|excluded| excluded.trim_start_matches(".") == ext) {
            return Ok(());
        }
    }
    
    // Skip large files (> 100KB)
    if let Ok(metadata) = fs::metadata(file_path) {
        if metadata.len() > 100 * 1024 {
            println!("Skipping large file: {}", file_path.display());
            return Ok(());
        }
    }
    
    // Read file content
    match read_file(file_path) {
        Ok(content) => {
            // Create a relative path that shows the structure well
            let relative_path = if file_path.starts_with(&base_dir) {
                if let Ok(rel_path) = file_path.strip_prefix(&base_dir) {
                    rel_path.to_string_lossy().to_string()
                } else {
                    file_path.to_string_lossy().to_string()
                }
            } else {
                file_path.to_string_lossy().to_string()
            };
            
            // Clean up path (remove leading / or \)
            let clean_path = relative_path.trim_start_matches('/').trim_start_matches('\\').to_string();
            
            files.push(FileEntry {
                path: clean_path,
                content,
            });
        }
        Err(e) => {
            eprintln!("Error reading file {}: {}", file_path.display(), e);
        }
    }
    
    Ok(())
}

fn read_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}

// Functions for Rust project detection and metadata extraction

fn find_and_extract_cargo_info(start_dir: &Path) -> Option<String> {
    let mut current_dir = start_dir.to_path_buf();
    
    loop {
        let cargo_path = current_dir.join("Cargo.toml");
        if cargo_path.exists() {
            return extract_cargo_info(&cargo_path.to_string_lossy());
        }
        
        // Go up one directory
        if !current_dir.pop() {
            break;
        }
    }
    
    None
}

fn extract_cargo_info(cargo_path: &str) -> Option<String> {
    let path = Path::new(cargo_path);
    
    if !path.exists() {
        return None;
    }
    
    let mut content = String::new();
    if let Ok(mut file) = File::open(path) {
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
                                if let Some(version) = table.get("version").and_then(|v| v.as_str()) {
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
                                if let Some(version) = table.get("version").and_then(|v| v.as_str()) {
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

// Functions for Python project detection and metadata extraction

fn find_and_extract_python_info(start_dir: &Path) -> Option<String> {
    let mut current_dir = start_dir.to_path_buf();
    
    loop {
        // Try pyproject.toml first (modern Python projects)
        let pyproject_path = current_dir.join("pyproject.toml");
        if pyproject_path.exists() {
            if let Some(info) = extract_python_project_info(&pyproject_path.to_string_lossy()) {
                return Some(info);
            }
        }
        
        // Then try setup.py
        let setup_py_path = current_dir.join("setup.py");
        if setup_py_path.exists() {
            if let Some(info) = extract_setup_py_info(&setup_py_path.to_string_lossy()) {
                return Some(info);
            }
        }
        
        // Then requirements.txt
        let requirements_path = current_dir.join("requirements.txt");
        if requirements_path.exists() {
            if let Some(info) = extract_requirements_info(&requirements_path.to_string_lossy()) {
                return Some(info);
            }
        }
        
        // Go up one directory
        if !current_dir.pop() {
            break;
        }
    }
    
    None
}

fn extract_python_project_info(pyproject_path: &str) -> Option<String> {
    let path = Path::new(pyproject_path);
    
    if !path.exists() {
        return None;
    }
    
    let mut content = String::new();
    if let Ok(mut file) = File::open(path) {
        if file.read_to_string(&mut content).is_err() {
            return None;
        }
    } else {
        return None;
    }
    
    match content.parse::<Value>() {
        Ok(pyproject_toml) => {
            let mut info = String::new();
            
            // Extract project metadata from pyproject.toml
            // Try both poetry and standard formats
            
            // Poetry format
            if let Some(tool) = pyproject_toml.get("tool") {
                if let Some(poetry) = tool.get("poetry") {
                    info.push_str("Project Type: Python (Poetry)\n");
                    
                    if let Some(name) = poetry.get("name").and_then(|v| v.as_str()) {
                        info.push_str(&format!("Project Name: {}\n", name));
                    }
                    
                    if let Some(version) = poetry.get("version").and_then(|v| v.as_str()) {
                        info.push_str(&format!("Version: {}\n", version));
                    }
                    
                    if let Some(description) = poetry.get("description").and_then(|v| v.as_str()) {
                        info.push_str(&format!("Description: {}\n", description));
                    }
                    
                    // Poetry dependencies
                    if let Some(deps) = poetry.get("dependencies") {
                        if let Some(deps_table) = deps.as_table() {
                            info.push_str("\nDependencies:\n");
                            
                            for (name, value) in deps_table {
                                if name == "python" {
                                    continue; // Skip python version constraint
                                }
                                
                                match value {
                                    Value::String(version) => {
                                        info.push_str(&format!("- {} = \"{}\"\n", name, version));
                                    }
                                    _ => {
                                        info.push_str(&format!("- {}\n", name));
                                    }
                                }
                            }
                        }
                    }
                    
                    // Poetry dev dependencies
                    if let Some(dev_deps) = poetry.get("dev-dependencies") {
                        if let Some(deps_table) = dev_deps.as_table() {
                            info.push_str("\nDev Dependencies:\n");
                            
                            for (name, value) in deps_table {
                                match value {
                                    Value::String(version) => {
                                        info.push_str(&format!("- {} = \"{}\"\n", name, version));
                                    }
                                    _ => {
                                        info.push_str(&format!("- {}\n", name));
                                    }
                                }
                            }
                        }
                    }
                    
                    return Some(info);
                }
            }
            
            // Standard PEP 621 format
            if let Some(project) = pyproject_toml.get("project") {
                info.push_str("Project Type: Python (PEP 621)\n");
                
                if let Some(name) = project.get("name").and_then(|v| v.as_str()) {
                    info.push_str(&format!("Project Name: {}\n", name));
                }
                
                if let Some(version) = project.get("version").and_then(|v| v.as_str()) {
                    info.push_str(&format!("Version: {}\n", version));
                }
                
                if let Some(description) = project.get("description").and_then(|v| v.as_str()) {
                    info.push_str(&format!("Description: {}\n", description));
                }
                
                // Dependencies from PEP 621
                if let Some(deps) = project.get("dependencies") {
                    if let Some(deps_array) = deps.as_array() {
                        info.push_str("\nDependencies:\n");
                        
                        for value in deps_array {
                            if let Some(dep_str) = value.as_str() {
                                info.push_str(&format!("- {}\n", dep_str));
                            }
                        }
                    }
                }
                
                // Optional dependencies
                if let Some(optional_deps) = project.get("optional-dependencies") {
                    if let Some(optional_table) = optional_deps.as_table() {
                        info.push_str("\nOptional Dependencies:\n");
                        
                        for (group, deps) in optional_table {
                            info.push_str(&format!("Group '{}':\n", group));
                            
                            if let Some(deps_array) = deps.as_array() {
                                for value in deps_array {
                                    if let Some(dep_str) = value.as_str() {
                                        info.push_str(&format!("  - {}\n", dep_str));
                                    }
                                }
                            }
                        }
                    }
                }
                
                return Some(info);
            }
            
            // Flit format
            if let Some(tool) = pyproject_toml.get("tool") {
                if let Some(flit) = tool.get("flit") {
                    info.push_str("Project Type: Python (Flit)\n");
                    
                    if let Some(metadata) = flit.get("metadata") {
                        if let Some(name) = metadata.get("module").and_then(|v| v.as_str()) {
                            info.push_str(&format!("Project Name: {}\n", name));
                        }
                        
                        if let Some(description) = metadata.get("description").and_then(|v| v.as_str()) {
                            info.push_str(&format!("Description: {}\n", description));
                        }
                        
                        if let Some(requires) = metadata.get("requires") {
                            if let Some(deps_array) = requires.as_array() {
                                info.push_str("\nDependencies:\n");
                                
                                for value in deps_array {
                                    if let Some(dep_str) = value.as_str() {
                                        info.push_str(&format!("- {}\n", dep_str));
                                    }
                                }
                            }
                        }
                        
                        if let Some(requires_extra) = metadata.get("requires-extra") {
                            if let Some(extras_table) = requires_extra.as_table() {
                                info.push_str("\nOptional Dependencies:\n");
                                
                                for (group, deps) in extras_table {
                                    info.push_str(&format!("Group '{}':\n", group));
                                    
                                    if let Some(deps_array) = deps.as_array() {
                                        for value in deps_array {
                                            if let Some(dep_str) = value.as_str() {
                                                info.push_str(&format!("  - {}\n", dep_str));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        
                        return Some(info);
                    }
                }
            }
            
            // If we found pyproject.toml but couldn't identify its format
            info.push_str("Project Type: Python (pyproject.toml format not recognized)\n");
            info.push_str("A pyproject.toml file was found but its format couldn't be parsed.\n");
            
            Some(info)
        }
        Err(_) => None,
    }
}

fn extract_setup_py_info(setup_py_path: &str) -> Option<String> {
    let path = Path::new(setup_py_path);
    
    if !path.exists() {
        return None;
    }
    
    let mut content = String::new();
    if let Ok(mut file) = File::open(path) {
        if file.read_to_string(&mut content).is_err() {
            return None;
        }
    } else {
        return None;
    }
    
    let mut info = String::new();
    info.push_str("Project Type: Python (setup.py)\n");
    
    // Try to extract the most common setup() parameters using basic parsing
    // This is not a full Python parser, just a simple regex-like approach
    
    // Extract name
    if let Some(name) = extract_setup_param(&content, "name") {
        info.push_str(&format!("Project Name: {}\n", name));
    }
    
    // Extract version
    if let Some(version) = extract_setup_param(&content, "version") {
        info.push_str(&format!("Version: {}\n", version));
    }
    
    // Extract description
    if let Some(description) = extract_setup_param(&content, "description") {
        info.push_str(&format!("Description: {}\n", description));
    }
    
    // Extract install_requires
    if let Some(requires) = extract_setup_list_param(&content, "install_requires") {
        if !requires.is_empty() {
            info.push_str("\nDependencies:\n");
            for dep in requires {
                info.push_str(&format!("- {}\n", dep));
            }
        }
    }
    
    // Extract extras_require
    if let Some(extras) = extract_setup_dict_param(&content, "extras_require") {
        if !extras.is_empty() {
            info.push_str("\nOptional Dependencies:\n");
            for (group, deps) in extras {
                info.push_str(&format!("Group '{}':\n", group));
                for dep in deps {
                    info.push_str(&format!("  - {}\n", dep));
                }
            }
        }
    }
    
    Some(info)
}

fn extract_setup_param(content: &str, param: &str) -> Option<String> {
    // Very simplified extraction of setup.py parameters
    // This won't work for complex setup.py files but covers simple cases
    
    let param_patterns = vec![
        format!("{}=['\"](.*?)['\"]", param),                   // name="value"
        format!("{} *= *['\"](.*?)['\"]", param),               // name = "value"
        format!("{} *: *['\"](.*?)['\"]", param),               // name: "value"
    ];
    
    for pattern in param_patterns {
        if let Some(captures) = regex_extract(content, &pattern) {
            if !captures.is_empty() {
                return Some(captures[0].clone());
            }
        }
    }
    
    None
}

fn extract_setup_list_param(content: &str, param: &str) -> Option<Vec<String>> {
    // Extracts list parameters like:
    // install_requires=["pkg1", "pkg2>=1.0"]
    // install_requires = [
    //     "pkg1",
    //     "pkg2>=1.0",
    // ]
    
    // Find the parameter
    let param_regex = format!(r"{}[ \t]*=[ \t]*\[", param);
    
    if let Some(start_pos) = content.find(&param_regex) {
        let mut result = Vec::new();
        let start_idx = content[start_pos..].find('[').map(|pos| start_pos + pos + 1)?;
        let mut depth = 1;
        let mut current_item = String::new();
        let mut in_string = false;
        let mut string_delim = ' ';
        
        for (i, ch) in content[start_idx..].chars().enumerate() {
            match ch {
                '[' if !in_string => depth += 1,
                ']' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        // End of the list
                        if !current_item.trim().is_empty() {
                            result.push(cleanup_string(&current_item));
                        }
                        break;
                    }
                },
                ',' if !in_string && depth == 1 => {
                    if !current_item.trim().is_empty() {
                        result.push(cleanup_string(&current_item));
                    }
                    current_item = String::new();
                },
                '"' | '\'' => {
                    if !in_string {
                        in_string = true;
                        string_delim = ch;
                    } else if ch == string_delim {
                        in_string = false;
                    } else {
                        // It's a quote inside another type of string
                        current_item.push(ch);
                    }
                },
                _ => {
                    current_item.push(ch);
                }
            }
        }
        
        return Some(result);
    }
    
    None
}

fn extract_setup_dict_param(content: &str, param: &str) -> Option<Vec<(String, Vec<String>)>> {
    // Extract dict parameters like extras_require={"dev": ["pkg1", "pkg2"]}
    let param_regex = format!(r"{}[ \t]*=[ \t]*{{", param);
    
    if let Some(start_pos) = content.find(&param_regex) {
        let mut result = Vec::new();
        let start_idx = content[start_pos..].find('{').map(|pos| start_pos + pos + 1)?;
        let mut depth = 1;
        let mut current_key = String::new();
        let mut current_value = String::new();
        let mut in_string = false;
        let mut string_delim = ' ';
        let mut in_key = true;
        
        for (i, ch) in content[start_idx..].chars().enumerate() {
            match ch {
                '{' if !in_string => depth += 1,
                '}' if !in_string => {
                    depth -= 1;
                    if depth == 0 {
                        // End of the dict
                        if !current_key.trim().is_empty() && !current_value.trim().is_empty() {
                            if let Some(list) = extract_list_from_str(&current_value) {
                                result.push((cleanup_string(&current_key), list));
                            }
                        }
                        break;
                    }
                },
                ':' if !in_string && in_key && depth == 1 => {
                    in_key = false;
                },
                ',' if !in_string && !in_key && depth == 1 => {
                    if !current_key.trim().is_empty() && !current_value.trim().is_empty() {
                        if let Some(list) = extract_list_from_str(&current_value) {
                            result.push((cleanup_string(&current_key), list));
                        }
                    }
                    current_key = String::new();
                    current_value = String::new();
                    in_key = true;
                },
                '"' | '\'' => {
                    if !in_string {
                        in_string = true;
                        string_delim = ch;
                    } else if ch == string_delim {
                        in_string = false;
                    } else {
                        // It's a quote inside another type of string
                        if in_key {
                            current_key.push(ch);
                        } else {
                            current_value.push(ch);
                        }
                    }
                },
                _ => {
                    if in_key {
                        current_key.push(ch);
                    } else {
                        current_value.push(ch);
                    }
                }
            }
        }
        
        return Some(result);
    }
    
    None
}

fn extract_list_from_str(list_str: &str) -> Option<Vec<String>> {
    // Extract a list from a string like "[item1, item2]"
    if let Some(start_idx) = list_str.find('[') {
        if let Some(end_idx) = list_str.rfind(']') {
            let list_content = &list_str[start_idx + 1..end_idx];
            let items: Vec<String> = list_content
                .split(',')
                .map(|s| cleanup_string(s))
                .filter(|s| !s.is_empty())
                .collect();
            return Some(items);
        }
    }
    None
}

fn cleanup_string(s: &str) -> String {
    let mut result = s.trim().to_string();
    if (result.starts_with('"') && result.ends_with('"')) || 
       (result.starts_with('\'') && result.ends_with('\'')) {
        result = result[1..result.len()-1].to_string();
    }
    result
}

fn extract_requirements_info(requirements_path: &str) -> Option<String> {
    let path = Path::new(requirements_path);
    
    if !path.exists() {
        return None;
    }
    
    let mut info = String::new();
    info.push_str("Project Type: Python (requirements.txt)\n");
    
    match File::open(path) {
        Ok(file) => {
            let reader = BufReader::new(file);
            let mut dependencies = Vec::new();
            
            for line_result in reader.lines() {
                if let Ok(line) = line_result {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() && !trimmed.starts_with('#') {
                        // Remove any comments at the end of the line
                        let dep = match trimmed.find('#') {
                            Some(idx) => trimmed[..idx].trim(),
                            None => trimmed
                        };
                        
                        if !dep.is_empty() {
                            dependencies.push(dep.to_string());
                        }
                    }
                }
            }
            
            if !dependencies.is_empty() {
                info.push_str("\nDependencies:\n");
                for dep in dependencies {
                    info.push_str(&format!("- {}\n", dep));
                }
            }
            
            Some(info)
        }
        Err(_) => None,
    }
}

// Very basic regex-like extractor
fn regex_extract(text: &str, pattern: &str) -> Option<Vec<String>> {
    let parts: Vec<&str> = pattern.split("(.*?)").collect();
    if parts.len() < 2 {
        return None;
    }
    
    let mut pos = 0;
    let mut captures = Vec::new();
    
    // Find the start position after the first part
    if let Some(start_pos) = text[pos..].find(parts[0]) {
        pos += start_pos + parts[0].len();
        
        // For each middle part, extract the text in between
        for i in 1..parts.len() {
            let part = parts[i];
            
            if let Some(end_pos) = text[pos..].find(part) {
                // Extract the captured text
                let captured = &text[pos..pos + end_pos];
                captures.push(captured.to_string());
                
                // Move position after this part
                pos += end_pos + part.len();
            } else {
                // If we can't find the next part, the pattern doesn't match
                return None;
            }
        }
        
        Some(captures)
    } else {
        None
    }
}

fn format_for_llm(files: Vec<FileEntry>, project_type: ProjectType, project_info: Option<String>) -> String {
    let mut output = String::new();
    
    // Add project metadata
    output.push_str("<project>\n");
    
    // Add project information based on type
    match project_type {
        ProjectType::Rust => {
            if let Some(info) = project_info {
                output.push_str("<cargo_info>\n");
                output.push_str(&info);
                output.push_str("</cargo_info>\n\n");
            }
        },
        ProjectType::Python => {
            if let Some(info) = project_info {
                output.push_str("<python_info>\n");
                output.push_str(&info);
                output.push_str("</python_info>\n\n");
            }
        },
        ProjectType::Unknown => {
            output.push_str("<project_info>\n");
            output.push_str("Project type could not be determined.\n");
            output.push_str("</project_info>\n\n");
        }
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
    let mut current_indent = 0;
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
        structure.push_str(&format!("{:indent$}├── {}\n", "", file_name, indent = indent));
    }
    
    structure
}

#[cfg(not(windows))]
fn copy_to_clipboard(text: &str) -> io::Result<()> {
    match ClipboardProvider::new() {
        Ok(mut ctx) => {
            ctx.set_contents(text.to_owned()).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("Clipboard error: {}", e))
            })
        }
        Err(e) => {
            Err(io::Error::new(io::ErrorKind::Other, format!("Clipboard error: {}", e)))
        }
    }
}

#[cfg(windows)]
fn copy_to_clipboard(text: &str) -> io::Result<()> {
    match Clipboard::new_attempts(10) {
        Ok(_clip) => {
            formats::Unicode.write_clipboard(&text).map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("Clipboard error: {:?}", e))
            })
        }
        Err(e) => {
            Err(io::Error::new(io::ErrorKind::Other, format!("Clipboard error: {:?}", e)))
        }
    }
}