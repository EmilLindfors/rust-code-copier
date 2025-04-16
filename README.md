Rust Code Copier for LLMs

A VS Code extension that copies your Rust project's files to the clipboard in a format optimized for sharing with Large Language Models (LLMs) like Claude.
Features

    Copies all relevant files from a Rust project to the clipboard
    Formats the files with XML tags for better LLM comprehension
    Extracts and includes key information from Cargo.toml (dependencies, project info)
    Includes a structured representation of your project directory
    Skips binary files, large files, and common non-relevant directories
    Available through the context menu or command palette

Installation
Prerequisites

    Visual Studio Code 1.60.0 or higher
    Rust and Cargo installed on your system (for building the extension)

Installation
For Development/Personal Use

    Clone this repository:

    git clone https://github.com/yourusername/rust-code-copier.git
    cd rust-code-copier

    Run the build script which handles everything automatically: On macOS/Linux:

    chmod +x build.sh
    ./build.sh

    On Windows:

    build.bat

    The script will:
        Build the Rust binary
        Set up the bin directory
        Install JS dependencies (using Bun if available, falling back to npm)
        Compile the TypeScript code
        Package the extension
    Install the extension in VS Code:
        Go to the Extensions view (Ctrl+Shift+X)
        Click on the "..." at the top of the Extensions view
        Choose "Install from VSIX..."
        Select the .vsix file created in the project directory

Customizing the Publisher

Before building, you should modify the publisher field in package.json if you plan to share this extension. For personal use, the default will work fine.

    Install the extension in VS Code:
        Go to the Extensions view (Ctrl+Shift+X)
        Click on the "..." at the top of the Extensions view
        Choose "Install from VSIX..."
        Select the .vsix file you just created

Usage

    Right-click on a Rust project folder in the VS Code Explorer
    Select "Copy Rust Project for LLM" from the context menu
    The project files will be copied to your clipboard in LLM-friendly format
    Paste the clipboard contents into your conversation with an LLM like Claude

Alternatively, use the Command Palette (Ctrl+Shift+P) and search for "Copy Rust Project for LLM".
Output Format

The extension outputs your project in an XML-like format:

xml

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

Customization

You can configure which files are included/excluded by modifying the excluded_dirs and excluded_ext arrays in the collect_files function in src/main.rs.
License

MIT
Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
