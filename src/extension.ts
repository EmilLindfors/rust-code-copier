import * as vscode from 'vscode';
import { exec } from 'child_process';
import * as path from 'path';
import * as fs from 'fs';

export function activate(context: vscode.ExtensionContext) {
    console.log('Code Copier is now active!');

    let disposable = vscode.commands.registerCommand('llm-cocop.copyProject', async (uri?: vscode.Uri) => {
        try {
            let selectedPaths: string[] = [];
            
            // Check if we're getting this from the explorer context menu with a selection
            if (uri) {
                // If a single URI is provided directly (right-click on a single item)
                selectedPaths = [uri.fsPath];
            } else {
                // If command is invoked from command palette or without a selection
                
                // First, check if there are explorer selections
                const explorerSelection = getExplorerSelection();
                if (explorerSelection && explorerSelection.length > 0) {
                    selectedPaths = explorerSelection.map(u => u.fsPath);
                }
                // Then check if an editor is active
                else if (vscode.window.activeTextEditor) {
                    // An editor is active, so add the current file
                    selectedPaths = [vscode.window.activeTextEditor.document.uri.fsPath];
                } 
                else {
                    // No selection, offer to use the workspace folder
                    const workspaceFolders = vscode.workspace.workspaceFolders;
                    if (!workspaceFolders || workspaceFolders.length === 0) {
                        vscode.window.showErrorMessage('No workspace folder is open.');
                        return;
                    }
                    
                    // If multiple folders are open, ask the user to select one
                    if (workspaceFolders.length > 1) {
                        const folderOptions = workspaceFolders.map(folder => ({
                            label: folder.name,
                            uri: folder.uri
                        }));
                        
                        const selectedFolder = await vscode.window.showQuickPick(folderOptions, {
                            placeHolder: 'Select a workspace folder to copy'
                        });
                        
                        if (!selectedFolder) {
                            return; // User cancelled
                        }
                        
                        selectedPaths = [selectedFolder.uri.fsPath];
                    } else {
                        selectedPaths = [workspaceFolders[0].uri.fsPath];
                    }
                }
            }
            
            // If we still don't have paths, return
            if (selectedPaths.length === 0) {
                vscode.window.showErrorMessage('No files or directories selected.');
                return;
            }
            
            // Find the path to the binary
            const extensionPath = context.extensionPath;
            const binPath = path.join(extensionPath, 'bin', process.platform === 'win32' ? 'llm-cocop-rs.exe' : 'llm-cocop');
            
            // Check if the binary exists
            if (!fs.existsSync(binPath)) {
                vscode.window.showErrorMessage('Code Copier binary not found. Please check the installation.');
                return;
            }
            
            // Try to detect project type and find appropriate metadata files
            const projectFiles = await detectProjectFiles(selectedPaths);
            
            // Show progress notification
            vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: 'Copying selected files',
                cancellable: false
            }, async (progress) => {
                progress.report({ increment: 0, message: 'Starting...' });
                
                // Run the binary for each path, collecting the paths for a single execution
                return new Promise<void>((resolve, reject) => {
                    // Create a command string with all selected paths
                    let commandArgs = selectedPaths.map(p => `"${p}"`).join(' ');
                    
                    // Add cargo.toml path if found and not already included
                    if (projectFiles.cargoToml && !selectedPaths.includes(projectFiles.cargoToml)) {
                        commandArgs += ` --cargo-toml "${projectFiles.cargoToml}"`;
                    }
                    
                    // Add Python project file if found and not already included
                    if (projectFiles.pythonProject && !selectedPaths.includes(projectFiles.pythonProject)) {
                        commandArgs += ` --pyproject "${projectFiles.pythonProject}"`;
                    }
                    
                    const command = `"${binPath}" ${commandArgs}`;
                    console.log(`Executing command: ${command}`);
                    
                    exec(command, (error, stdout, stderr) => {
                        if (error) {
                            vscode.window.showErrorMessage(`Error copying files: ${error.message}`);
                            console.error(`Error: ${error.message}`);
                            console.error(`Stderr: ${stderr}`);
                            reject(error);
                            return;
                        }
                        
                        progress.report({ increment: 100, message: 'Complete!' });
                        
                        // Get the number of files processed from stdout if possible
                        const filesMatch = stdout.match(/Files processed: (\d+)/);
                        const sizeMatch = stdout.match(/Total size: (\d+) characters/);
                        const typeMatch = stdout.match(/Project type: (\w+)/);
                        
                        const filesCount = filesMatch ? filesMatch[1] : 'multiple';
                        const charCount = sizeMatch ? sizeMatch[1] : 'unknown';
                        const projectType = typeMatch ? typeMatch[1] : 'unknown';
                        
                        vscode.window.showInformationMessage(
                            `Files copied to clipboard! (${filesCount} files, ${charCount} characters, ${projectType} project)`
                        );
                        
                        resolve();
                    });
                });
            });
        } catch (err: any) {
            vscode.window.showErrorMessage(`Error: ${err.message}`);
        }
    });

    context.subscriptions.push(disposable);
}

// Helper function to get the current explorer selection
function getExplorerSelection(): vscode.Uri[] | undefined {
    // VS Code API doesn't provide direct access to Explorer selection,
    // but we can use this hack for multiple file selection
    return vscode.workspace.getConfiguration('explorer').get('experimentalFileNesting')
        ? undefined 
        : vscode.window.tabGroups.activeTabGroup.tabs
            .filter(tab => tab.input instanceof vscode.TabInputText)
            .map(tab => (tab.input as vscode.TabInputText).uri);
}

interface ProjectFiles {
    cargoToml: string | null;
    pythonProject: string | null;
}

// Function to detect project type and find appropriate project files
async function detectProjectFiles(paths: string[]): Promise<ProjectFiles> {
    const result: ProjectFiles = {
        cargoToml: null,
        pythonProject: null
    };
    
    for (const filePath of paths) {
        const stats = fs.statSync(filePath);
        let currentDir = stats.isFile() ? path.dirname(filePath) : filePath;
        
        // Keep going up directories until we find project files or reach the root
        while (currentDir) {
            // Check for Cargo.toml
            const cargoPath = path.join(currentDir, 'Cargo.toml');
            if (fs.existsSync(cargoPath) && !result.cargoToml) {
                result.cargoToml = cargoPath;
            }
            
            // Check for Python project files in priority order
            const pyprojectPath = path.join(currentDir, 'pyproject.toml');
            const setupPyPath = path.join(currentDir, 'setup.py');
            const requirementsPath = path.join(currentDir, 'requirements.txt');
            
            if (fs.existsSync(pyprojectPath) && !result.pythonProject) {
                result.pythonProject = pyprojectPath;
            } else if (fs.existsSync(setupPyPath) && !result.pythonProject) {
                result.pythonProject = setupPyPath;
            } else if (fs.existsSync(requirementsPath) && !result.pythonProject) {
                result.pythonProject = requirementsPath;
            }
            
            // If we found both types of project files, break early
            if (result.cargoToml && result.pythonProject) {
                break;
            }
            
            // Go up one directory
            const parentDir = path.dirname(currentDir);
            if (parentDir === currentDir) {
                // We've reached the root directory
                break;
            }
            currentDir = parentDir;
        }
        
        // If we found both project file types after checking one path, no need to check more
        if (result.cargoToml && result.pythonProject) {
            break;
        }
    }
    
    return result;
}

export function deactivate() {}