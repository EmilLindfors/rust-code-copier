# Code Copier for LLMs

A VS Code extension that copies your code files to the clipboard in a format optimized for sharing with Large Language Models (LLMs) like Claude.

## Features

- **Multi-language support**: Works with Rust and Python projects (easily extendable to more languages)
- **Smart project detection**: Automatically finds and extracts project metadata
  - For Rust: Extracts information from Cargo.toml
  - For Python: Extracts information from pyproject.toml, setup.py, or requirements.txt
- **Flexible selection**: Copy individual files, multiple files, or entire directories
- **Structured output**: Formats code with XML tags and includes directory structure
- **LLM optimization**: Formats output specifically for better comprehension by LLMs

## Supported Project Types

### Rust Projects
- Detects and extracts information from Cargo.toml
- Includes dependencies and dev-dependencies
- Works with standard Rust project structures

### Python Projects
- Supports multiple project formats:
  - Modern pyproject.toml (Poetry, PEP 621, Flit)
  - Traditional setup.py
  - Simple requirements.txt
- Extracts dependencies and project metadata
- Handles virtual environments appropriately

## Installation

### Prerequisites

- Visual Studio Code 1.60.0 or higher
- Rust and Cargo installed on your system (for building the extension)

## Installation

### For Development/Personal Use

1. Clone this repository:
   ```
   git clone https://github.com/yourusername/rust-code-copier.git
   cd rust-code-copier
   ```

2. Run the build script which handles everything automatically:
   
   On macOS/Linux:
   ```
   chmod +x build.sh
   ./build.sh
   ```
   
   On Windows:
   ```
   build.bat
   ```

   The script will:
   - Build the Rust binary
   - Set up the bin directory
   - Install JS dependencies (using Bun if available, falling back to npm)
   - Compile the TypeScript code
   - Package the extension

3. Install the extension in VS Code:
   - Go to the Extensions view (Ctrl+Shift+X)
   - Click on the "..." at the top of the Extensions view
   - Choose "Install from VSIX..."
   - Select the `.vsix` file created in the project directory

### Customizing the Publisher 

Before building, you should modify the `publisher` field in `package.json` if you plan to share this extension. For personal use, the default will work fine.

6. Install the extension in VS Code:
   - Go to the Extensions view (Ctrl+Shift+X)
   - Click on the "..." at the top of the Extensions view
   - Choose "Install from VSIX..."
   - Select the .vsix file you just created

## Usage

The extension supports multiple ways to copy files:

1. **Select multiple files/folders in the Explorer view**:
   - Select one or more files/folders in VS Code's Explorer view
   - Right-click and select "Copy Code for LLM" from the context menu
   - The selected files will be copied to your clipboard in LLM-friendly format

2. **Right-click on a single file in the editor**:
   - Right-click anywhere in an open editor
   - Select "Copy Code for LLM" from the context menu
   - The current file will be copied along with any detected project information

3. **Use the Command Palette**:
   - Press Ctrl+Shift+P (or Cmd+Shift+P on macOS)
   - Search for "Copy Code for LLM"
   - The command will use your current selection or active file, or prompt you to select a workspace folder

The extension will automatically:
- Find and include project metadata if available (Cargo.toml, pyproject.toml, etc.)
- Format all selected files with proper XML formatting
- Skip binary files, large files, and files in directories like target/, .git/, __pycache__/ etc.
- Include a directory structure visualization

## Output Format

The extension outputs your project in an XML-like format:

```xml
<project>
<cargo_info>
Project Name: my-rust-project
Version: 0.1.0
Description: A sample Rust project

Dependencies:
- serde = "1.0.188"
- tokio = "1.32.0"
...
</cargo_info>

<file_structure>
└── src/
  ├── main.rs
  ├── lib.rs
  └── utils/
    ├── mod.rs
    ├── helpers.rs
...
</file_structure>

<file path="src/main.rs">
fn main() {
    println!("Hello, world!");
}
</file>

<file path="src/lib.rs">
...
</file>

...
</project>
```

## Customization

You can configure which files are included/excluded by modifying the `excluded_dirs` and `excluded_ext` arrays in the `collect_files` function in `src/main.rs`.

## License

MIT

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.