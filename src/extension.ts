import * as vscode from 'vscode';
import { exec } from 'child_process';
import * as path from 'path';
import * as fs from 'fs';

export function activate(context: vscode.ExtensionContext) {
    console.log('Rust Code Copier is now active!');

    let disposable = vscode.commands.registerCommand('rust-code-copier.copyProject', async (uri?: vscode.Uri) => {
        try {
            // If no URI is provided (command executed from command palette)
            if (!uri) {
                // Get the current workspace folders
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
                    
                    uri = selectedFolder.uri;
                } else {
                    uri = workspaceFolders[0].uri;
                }
            }
            
            // Get the path of the selected directory
            const dirPath = uri.fsPath;
            
            // Find the path to the rust-code-copier binary
            const extensionPath = context.extensionPath;
            const binPath = path.join(extensionPath, 'bin', process.platform === 'win32' ? 'rust-code-copier.exe' : 'rust-code-copier');
            
            // Check if the binary exists
            if (!fs.existsSync(binPath)) {
                vscode.window.showErrorMessage('Rust Code Copier binary not found. Please check the installation.');
                return;
            }
            
            // Show progress notification
            vscode.window.withProgress({
                location: vscode.ProgressLocation.Notification,
                title: 'Copying Rust project files',
                cancellable: false
            }, async (progress) => {
                progress.report({ increment: 0, message: 'Starting...' });
                
                // Run the Rust binary
                return new Promise<void>((resolve, reject) => {
                    exec(`"${binPath}" "${dirPath}"`, (error, stdout, stderr) => {
                        if (error) {
                            vscode.window.showErrorMessage(`Error copying project: ${error.message}`);
                            console.error(`Error: ${error.message}`);
                            console.error(`Stderr: ${stderr}`);
                            reject(error);
                            return;
                        }
                        
                        progress.report({ increment: 100, message: 'Complete!' });
                        
                        // Get the number of files processed from stdout if possible
                        const filesMatch = stdout.match(/Files processed: (\d+)/);
                        const sizeMatch = stdout.match(/Total size: (\d+) characters/);
                        
                        const filesCount = filesMatch ? filesMatch[1] : 'multiple';
                        const charCount = sizeMatch ? sizeMatch[1] : 'unknown';
                        
                        vscode.window.showInformationMessage(
                            `Project copied to clipboard! (${filesCount} files, ${charCount} characters)`
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

export function deactivate() {}