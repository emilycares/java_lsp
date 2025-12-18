import { workspace, ExtensionContext, window } from "vscode";

import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient;

export function activate(context: ExtensionContext) {
  // The server is implemented in node
  const config = workspace.getConfiguration("java_lsp");
  let serverModule = "";
  const configExecutablePath = config.get<string>("executablePath");
  if (configExecutablePath.length == 0) {
    window.showErrorMessage("java_lsp please configure: java_lsp.executablePath");
    return;
  }
  serverModule = configExecutablePath;

  //window.showErrorMessage("module: " + serverModule);

  // If the extension is launched in debug mode then the debug server options are used
  // Otherwise the run options are used
  const serverOptions: ServerOptions = {
    run: { command: serverModule, transport: TransportKind.stdio },
    debug: {
      command: serverModule,
      transport: TransportKind.stdio,
    },
  };

  // Options to control the language client
  const clientOptions: LanguageClientOptions = {
    // Register the server for plain text documents
    documentSelector: [{ scheme: "file", language: "java" }],
    synchronize: {
      configurationSection: "java",
    },
  };

  // Create the language client and start the client.
  client = new LanguageClient(
    "java_lsp",
    "java_lsp",
    serverOptions,
    clientOptions,
  );

  // Start the client. This will also launch the server
  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
