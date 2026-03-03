// Ferrite Language Server extension for VS Code
// Launches the ferrite-lsp binary and connects it via stdio

const { LanguageClient, TransportKind } = require("vscode-languageclient/node");
const vscode = require("vscode");

let client;

function activate(context) {
    const config = vscode.workspace.getConfiguration("ferrite.lsp");
    const enabled = config.get("enabled", true);

    if (!enabled) {
        return;
    }

    const lspPath = config.get("path", "ferrite-lsp");

    const serverOptions = {
        command: lspPath,
        transport: TransportKind.stdio,
    };

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
