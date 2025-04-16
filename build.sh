#!/bin/bash
# Build script for the Rust Code Copier VS Code extension

# Exit on any error
set -e

echo "=== Building Rust Code Copier VS Code Extension ==="

# Build the Rust binary
echo "Building Rust binary..."
cargo build --release

# Create bin directory if it doesn't exist
mkdir -p bin

# Copy the binary to the bin directory
echo "Copying binary to extension folder..."
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    cp target/release/llm-cocop-rs bin/
    chmod +x bin/llm-cocop-rs
elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # Linux
    cp target/release/llm-cocop-rs bin/
    chmod +x bin/llm-cocop-rs
elif [[ "$OSTYPE" == "msys"* || "$OSTYPE" == "win32" ]]; then
    # Windows
    cp target/release/llm-cocop-rs.exe bin/
else
    echo "Unsupported OS: $OSTYPE"
    exit 1
fi

# Check if Bun is available
if command -v bun &> /dev/null; then
    echo "Bun found, using it for JavaScript operations..."
    
    # Install dependencies
    echo "Installing dependencies with Bun..."
    bun install
    
    # Compile TypeScript
    echo "Compiling TypeScript with Bun..."
    bun run compile
    
    # Package the extension
    echo "Packaging VS Code extension with Bun..."
    # Add --no-dependencies flag to avoid punycode deprecation warning
    bun x vsce package --no-dependencies
else
    echo "Bun not found, falling back to npm..."
    
    # Install npm dependencies
    echo "Installing npm dependencies..."
    npm install
    
    # Compile TypeScript
    echo "Compiling TypeScript..."
    npm run compile
    
    # Package the extension
    echo "Packaging VS Code extension..."
    npx vsce package --no-dependencies
fi

echo "=== Build complete! ==="
echo "You can now install the .vsix file in VS Code:"
echo "1. Go to Extensions view (Ctrl+Shift+X)"
echo "2. Click on '...' at the top of the Extensions view"
echo "3. Select 'Install from VSIX...'"
echo "4. Choose the .vsix file created in this directory"