import { Socket } from 'net';
import { workspace, ExtensionContext, window, ExtensionMode } from "vscode";
import * as vscode from 'vscode';

import {
  LanguageClient,
  LanguageClientOptions,
  MessageReader,
  MessageTransports,
  MessageWriter,
  ServerOptions,
  SocketMessageReader,
  SocketMessageWriter,
  TransportKind,
} from "vscode-languageclient/node";


let client: LanguageClient;

export function activate(context: ExtensionContext) {
  // The server is implemented in node
  const config = workspace.getConfiguration("java_lsp");
  let serverModule = "";
  const configExecutablePath = config.get<string>("executablePath");
  if (configExecutablePath == undefined) {
    window.showErrorMessage("java_lsp please configure: java_lsp.executablePath");
    return;
  }
  serverModule = configExecutablePath;

  if (context.extensionMode == ExtensionMode.Development) {
    const serverOptions = async () => await createServerConnection("localhost", 4040);
    client = new LanguageClient('java_lsp', 'java_lsp', serverOptions, {});
    client.start();
    return;
  }

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
    documentSelector: [
      { scheme: "file", language: "java" },
      { scheme: "file", language: "xml" },
      { scheme: "file", language: 'gradle' },
      { scheme: "file", language: 'kotlin' }
    ],
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

async function createServerConnection(host: string, port: number): Promise<MessageTransports> {
  const uport = await askForPort("" + port);
  const socket: Socket = await new Promise((resolve, reject) => {
    const so = new Socket();
    so.connect(uport as number, host, () => resolve(so));
    so.once('error', reject);
    so.setTimeout(5000, () => {
      so.destroy();
      reject(new Error('TCP connect timeout'));
    });
  });
  return {
    reader: new SocketMessageReader(socket),
    writer: new SocketMessageWriter(socket)
  };
}
async function askForPort(defaultPort = '6009'): Promise<number | undefined> {
  const input = await vscode.window.showInputBox({
    prompt: 'Enter LSP TCP port',
    value: defaultPort,
    placeHolder: 'e.g. ' + defaultPort,
    validateInput: (v) => {
      const n = Number(v);
      if (!v) return 'Port required';
      if (!Number.isInteger(n) || n < 1 || n > 65535) return 'Enter valid port (1–65535)';
      return null;
    },
    ignoreFocusOut: true
  });
  if (!input) return undefined;
  return Number(input);
}