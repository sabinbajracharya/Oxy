// Ferrite Language Server extension for VS Code
// Launches the ferrite-lsp binary (directly or via Docker) and connects via stdio

const { LanguageClient, TransportKind } = require("vscode-languageclient/node");
const vscode = require("vscode");
const path = require("path");
const fs = require("fs");

let client;
let outputChannel;

function findProjectRoot() {
    // Walk up from the extension directory to find the project root (has Cargo.toml)
    let dir = __dirname;
    for (let i = 0; i < 5; i++) {
        if (fs.existsSync(path.join(dir, "Cargo.toml"))) {
            return dir;
        }
        dir = path.dirname(dir);
    }
    // Also check workspace folders
    const folders = vscode.workspace.workspaceFolders;
    if (folders) {
        for (const folder of folders) {
            if (fs.existsSync(path.join(folder.uri.fsPath, "Cargo.toml"))) {
                return folder.uri.fsPath;
            }
        }
    }
    return null;
}

function activate(context) {
    outputChannel = vscode.window.createOutputChannel("Ferrite LSP");

    const config = vscode.workspace.getConfiguration("ferrite.lsp");
    const enabled = config.get("enabled", true);

    if (!enabled) {
        outputChannel.appendLine("Ferrite LSP is disabled via settings.");
        return;
    }

    const mode = config.get("mode", "auto");
    const lspPath = config.get("path", "ferrite-lsp");

    let serverOptions;

    const useDocker = mode === "docker" || (mode === "auto" && lspPath === "ferrite-lsp");

    if (useDocker) {
        const projectRoot = findProjectRoot();
        if (!projectRoot) {
            vscode.window.showErrorMessage(
                "Ferrite: Could not find project root (Cargo.toml). Set ferrite.lsp.mode to 'native' and ferrite.lsp.path to your ferrite-lsp binary."
            );
            return;
        }

        outputChannel.appendLine(`Project root: ${projectRoot}`);
        outputChannel.appendLine("Starting Ferrite LSP via Docker...");

        serverOptions = {
            command: "docker",
            args: [
                "compose", "run", "--rm", "-T",
                "dev",
                "cargo", "run", "--release", "-p", "ferrite-lsp", "--quiet", "--",
            ],
            options: { cwd: projectRoot },
            transport: TransportKind.stdio,
        };
    } else {
        outputChannel.appendLine(`Starting Ferrite LSP native binary: ${lspPath}`);

        serverOptions = {
            command: lspPath,
            transport: TransportKind.stdio,
        };
    }

    const clientOptions = {
        documentSelector: [{ scheme: "file", language: "ferrite" }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher("**/*.fe"),
        },
        outputChannel: outputChannel,
    };

    client = new LanguageClient(
        "ferrite-lsp",
        "Ferrite Language Server",
        serverOptions,
        clientOptions
    );

    client.start().catch((err) => {
        outputChannel.appendLine(`Failed to start Ferrite LSP: ${err.message}`);
        vscode.window.showErrorMessage(
            `Ferrite LSP failed to start: ${err.message}. Check "Ferrite LSP" output channel for details.`
        );
    });

    context.subscriptions.push(client);
    context.subscriptions.push(outputChannel);
}

function deactivate() {
    if (client) {
        return client.stop();
    }
}

module.exports = { activate, deactivate };
