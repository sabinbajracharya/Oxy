// Ferrite Language Server extension for VS Code
// Launches the ferrite-lsp binary (directly or via Docker) and connects via stdio

const { LanguageClient, TransportKind } = require("vscode-languageclient/node");
const vscode = require("vscode");
const path = require("path");
const fs = require("fs");

let client;

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
    const config = vscode.workspace.getConfiguration("ferrite.lsp");
    const enabled = config.get("enabled", true);

    if (!enabled) {
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
                "Ferrite: Could not find project root (Cargo.toml). Set ferrite.lsp.path to your ferrite-lsp binary."
            );
            return;
        }

        serverOptions = {
            command: "docker",
            args: [
                "compose",
                "-f", path.join(projectRoot, "docker-compose.yml"),
                "run", "--rm", "-T",
                "dev",
                "cargo", "run", "--release", "-p", "ferrite-lsp", "--quiet",
            ],
            transport: TransportKind.stdio,
        };
    } else {
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
    };

    client = new LanguageClient(
        "ferrite-lsp",
        "Ferrite Language Server",
        serverOptions,
        clientOptions
    );

    client.start();
    context.subscriptions.push(client);
}

function deactivate() {
    if (client) {
        return client.stop();
    }
}

module.exports = { activate, deactivate };
