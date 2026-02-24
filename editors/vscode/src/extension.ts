import * as vscode from "vscode";
import { LanguageClient, LanguageClientOptions, ServerOptions } from "vscode-languageclient/node";
import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import { exec } from "child_process";

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  let serverCommand = "goodwrite-lsp";

  // Check if goodwrite-lsp is in PATH
  const isInstalled = await new Promise<boolean>((resolve) => {
    exec("goodwrite-lsp --version", (error) => resolve(!error));
  });

  if (!isInstalled) {
    const installDir = path.join(os.homedir(), ".goodwrite", "bin");
    const localBinary = path.join(installDir, "goodwrite-lsp");

    if (fs.existsSync(localBinary)) {
      serverCommand = localBinary;
    } else {
      const selection = await vscode.window.showWarningMessage(
        "goodwrite-lsp is not installed or not in PATH.",
        "Install Automatically",
        "Cancel"
      );

      if (selection === "Install Automatically") {
        await vscode.window.withProgress(
          {
            location: vscode.ProgressLocation.Notification,
            title: "Installing goodwrite...",
            cancellable: false,
          },
          async () => {
            return new Promise<void>((resolve, reject) => {
              exec(
                `curl -fsSL https://raw.githubusercontent.com/walkerbrown/goodwrite/main/scripts/install.sh | bash`,
                (error) => {
                  if (error) {
                    vscode.window.showErrorMessage(`Failed to install: ${error.message}`);
                    reject(error);
                  } else {
                    vscode.window.showInformationMessage("goodwrite installed successfully.");
                    resolve();
                  }
                }
              );
            });
          }
        );
        serverCommand = localBinary;
      } else {
        return; // Abort activation
      }
    }
  }

  const serverOptions: ServerOptions = {
    command: serverCommand,
    args: [],
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { language: "markdown" },
      { language: "typst" },
    ],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.{md,typ}"),
    },
  };

  client = new LanguageClient("goodwrite", "goodwrite", serverOptions, clientOptions);
  await client.start();
}

export async function deactivate(): Promise<void> {
  if (client) {
    await client.stop();
    client = undefined;
  }
}
