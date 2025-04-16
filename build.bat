@echo off
REM Build script for the Rust Code Copier VS Code extension on Windows

echo === Building Rust Code Copier VS Code Extension ===

REM Build the Rust binary
echo Building Rust binary...
cargo build --release
if %ERRORLEVEL% neq 0 (
    echo Failed to build Rust binary
    exit /b %ERRORLEVEL%
)

REM Create bin directory if it doesn't exist
if not exist bin mkdir bin

REM Copy the binary to the bin directory
echo Copying binary to extension folder...
copy target\release\llm-cocop-rs.exe bin\
if %ERRORLEVEL% neq 0 (
    echo Failed to copy binary
    exit /b %ERRORLEVEL%
)

REM Check if Bun is available
where bun >nul 2>nul
if %ERRORLEVEL% equ 0 (
    echo Bun found, using it for JavaScript operations...
    
    REM Install dependencies with Bun
    echo Installing dependencies with Bun...
    call bun install
    if %ERRORLEVEL% neq 0 (
        echo Failed to install dependencies with Bun
        exit /b %ERRORLEVEL%
    )
    
    REM Compile TypeScript with Bun
    echo Compiling TypeScript with Bun...
    call bun run compile
    if %ERRORLEVEL% neq 0 (
        echo Failed to compile TypeScript with Bun
        exit /b %ERRORLEVEL%
    )
    
    REM Package the extension with Bun
    echo Packaging VS Code extension with Bun...
    call bun x vsce package --no-dependencies
    if %ERRORLEVEL% neq 0 (
        echo Failed to package extension with Bun
        exit /b %ERRORLEVEL%
    )
) else (
    echo Bun not found, falling back to npm...
    
    REM Install npm dependencies
    echo Installing npm dependencies...
    call npm install
    if %ERRORLEVEL% neq 0 (
        echo Failed to install npm dependencies
        exit /b %ERRORLEVEL%
    )
    
    REM Compile TypeScript
    echo Compiling TypeScript...
    call npm run compile
    if %ERRORLEVEL% neq 0 (
        echo Failed to compile TypeScript
        exit /b %ERRORLEVEL%
    )
    
    REM Package the extension
    echo Packaging VS Code extension...
    call npx vsce package --no-dependencies
    if %ERRORLEVEL% neq 0 (
        echo Failed to package extension
        exit /b %ERRORLEVEL%
    )
)

echo === Build complete! ===
echo You can now install the .vsix file in VS Code:
echo 1. Go to Extensions view (Ctrl+Shift+X)
echo 2. Click on '...' at the top of the Extensions view
echo 3. Select 'Install from VSIX...'
echo 4. Choose the .vsix file created in this directory
