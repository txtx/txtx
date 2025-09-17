import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
  RevealOutputChannelOn
} from 'vscode-languageclient/node';

let client: LanguageClient;
let outputChannel: vscode.OutputChannel;

export async function activate(context: vscode.ExtensionContext) {
  outputChannel = vscode.window.createOutputChannel('Txtx Language Server');
  outputChannel.appendLine('Txtx extension activating...');
  
  // Get the LSP path from configuration or use defaults
  const config = vscode.workspace.getConfiguration('txtx');
  let configuredPath = config.get<string>('lspPath');
  
  let serverCommand: string = 'txtx'; // Default to system txtx
  
  // Handle VSCode variable substitution
  if (configuredPath && configuredPath.length > 0) {
    // Replace ${workspaceFolder} with actual path
    if (vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0) {
      const workspaceFolder = vscode.workspace.workspaceFolders[0].uri.fsPath;
      configuredPath = configuredPath.replace('${workspaceFolder}', workspaceFolder);
    }
    
    if (fs.existsSync(configuredPath)) {
      serverCommand = configuredPath;
      outputChannel.appendLine(`Using configured LSP path: ${serverCommand}`);
    } else {
      outputChannel.appendLine(`Configured path not found: ${configuredPath}, falling back to auto-detection`);
      configuredPath = ''; // Clear to trigger auto-detection
    }
  }
  
  if (!configuredPath || configuredPath.length === 0) {
    // Check environment variable first
    const envPath = process.env.TXTX_LSP_PATH;
    if (envPath && fs.existsSync(envPath)) {
      serverCommand = envPath;
      outputChannel.appendLine(`Using TXTX_LSP_PATH: ${serverCommand}`);
    } else {
      // Try relative paths (when running from source with F5 in VSCode)
      const extensionRoot = path.join(__dirname, '..');
      const projectRoot = path.join(extensionRoot, '..');
      const relativePaths = [
        path.join(projectRoot, 'target', 'release', 'txtx'),
        path.join(projectRoot, 'target', 'debug', 'txtx'),
      ];
      
      let found = false;
      for (const relBinary of relativePaths) {
        if (fs.existsSync(relBinary)) {
          serverCommand = relBinary;
          outputChannel.appendLine(`Using project binary: ${serverCommand}`);
          found = true;
          break;
        }
      }
      
      if (!found) {
        // Try workspace folder binary (if workspace contains txtx project)
        if (vscode.workspace.workspaceFolders && vscode.workspace.workspaceFolders.length > 0) {
          const workspaceRoot = vscode.workspace.workspaceFolders[0].uri.fsPath;
          const workspacePaths = [
            path.join(workspaceRoot, 'target', 'release', 'txtx'),
            path.join(workspaceRoot, 'target', 'debug', 'txtx'),
          ];
          
          for (const wsBinary of workspacePaths) {
            if (fs.existsSync(wsBinary)) {
              serverCommand = wsBinary;
              outputChannel.appendLine(`Using workspace binary: ${serverCommand}`);
              found = true;
              break;
            }
          }
        }
      }
      
      if (!found) {
        outputChannel.appendLine(`Using system txtx from PATH`);
      }
    }
  }
  const serverArgs = ['lsp'];
  
  outputChannel.appendLine(`LSP command: ${serverCommand} ${serverArgs.join(' ')}`);
  outputChannel.appendLine(`Workspace folders: ${vscode.workspace.workspaceFolders?.map(f => f.uri.fsPath).join(', ')}`);

  const serverOptions: ServerOptions = {
    run: { 
      command: serverCommand, 
      args: serverArgs, 
      transport: TransportKind.stdio
    },
    debug: { 
      command: serverCommand, 
      args: serverArgs, 
      transport: TransportKind.stdio,
      options: {
        env: {
          ...process.env,
          RUST_LOG: 'debug',
          RUST_BACKTRACE: '1'
        }
      }
    }
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: 'file', language: 'txtx' },
      { scheme: 'file', pattern: '**/txtx.{yml,yaml}' }
    ],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher('**/{*.tx,txtx.yml,txtx.yaml}')
    },
    outputChannel: outputChannel,
    revealOutputChannelOn: RevealOutputChannelOn.Info,
    middleware: {
      // Log all requests and responses for debugging
      provideDefinition: async (document, position, token, next) => {
        outputChannel.appendLine(`[Definition Request] File: ${document.uri.fsPath}, Position: ${position.line}:${position.character}`);
        try {
          const result = await next(document, position, token);
          outputChannel.appendLine(`[Definition Response] Result: ${JSON.stringify(result)}`);
          return result;
        } catch (error) {
          outputChannel.appendLine(`[Definition Error] ${error}`);
          throw error;
        }
      },
      provideHover: async (document, position, token, next) => {
        outputChannel.appendLine(`[Hover Request] File: ${document.uri.fsPath}, Position: ${position.line}:${position.character}`);
        try {
          const result = await next(document, position, token);
          if (result) {
            outputChannel.appendLine(`[Hover Response] Has content`);
          } else {
            outputChannel.appendLine(`[Hover Response] No content`);
          }
          return result;
        } catch (error) {
          outputChannel.appendLine(`[Hover Error] ${error}`);
          throw error;
        }
      }
    }
  };

  client = new LanguageClient(
    'txtxLanguageServer',
    'Txtx Language Server',
    serverOptions,
    clientOptions
  );

  // Add status bar item to show LSP status
  const statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
  statusBarItem.text = '$(sync~spin) Txtx LSP: Starting...';
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);

  // Add environment selector status bar item
  const envStatusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 99);
  // Use workspace state instead of configuration for persistence
  const savedEnv = context.workspaceState.get<string>('selectedEnvironment') || 'global';
  envStatusBarItem.text = `$(globe) Txtx Env: ${savedEnv}`;
  envStatusBarItem.tooltip = `Current Txtx environment: ${savedEnv}\nClick to change`;
  envStatusBarItem.command = 'txtx.selectEnvironment';
  envStatusBarItem.show();
  context.subscriptions.push(envStatusBarItem);


  // Handle client state changes
  client.onDidChangeState((event) => {
    outputChannel.appendLine(`[State Change] Old: ${event.oldState}, New: ${event.newState}`);
    
    switch (event.newState) {
      case 1: // Starting
        statusBarItem.text = '$(sync~spin) Txtx LSP: Starting...';
        break;
      case 2: // Running
        statusBarItem.text = '$(check) Txtx LSP: Ready';
        statusBarItem.tooltip = 'Txtx Language Server is running';
        break;
      case 3: // Stopped
        statusBarItem.text = '$(x) Txtx LSP: Stopped';
        statusBarItem.tooltip = 'Txtx Language Server is not running';
        break;
    }
  });

  // Register commands before starting client
  const showLogsCommand = vscode.commands.registerCommand('txtx.showLogs', () => {
    outputChannel.show();
  });
  
  const testDefinitionCommand = vscode.commands.registerCommand('txtx.testDefinition', async () => {
    const editor = vscode.window.activeTextEditor;
    if (!editor) {
      vscode.window.showWarningMessage('No active editor');
      return;
    }
    
    const position = editor.selection.active;
    outputChannel.appendLine(`Testing go-to-definition at ${position.line}:${position.character}`);
    
    // Get word at cursor
    const wordRange = editor.document.getWordRangeAtPosition(position);
    const word = wordRange ? editor.document.getText(wordRange) : '';
    outputChannel.appendLine(`Word at cursor: "${word}"`);
    
    // Check current line content
    const line = editor.document.lineAt(position.line);
    outputChannel.appendLine(`Current line: "${line.text}"`);
    
    try {
      // Manually trigger go-to-definition
      const definitions = await vscode.commands.executeCommand<vscode.Location[]>(
        'vscode.executeDefinitionProvider',
        editor.document.uri,
        position
      );
      
      if (definitions && definitions.length > 0) {
        outputChannel.appendLine(`Found ${definitions.length} definition(s):`);
        definitions.forEach((def, i) => {
          outputChannel.appendLine(`  ${i + 1}. ${def.uri.fsPath} at ${def.range.start.line}:${def.range.start.character}`);
        });
      } else {
        outputChannel.appendLine('No definitions found');
      }
    } catch (error) {
      outputChannel.appendLine(`Error getting definitions: ${error}`);
    }
  });
  
  const restartLspCommand = vscode.commands.registerCommand('txtx.restartLsp', async () => {
    outputChannel.appendLine('Restarting LSP client...');
    if (client) {
      await client.stop();
      await client.start();
    }
  });

  const selectEnvironmentCommand = vscode.commands.registerCommand('txtx.selectEnvironment', async () => {
    outputChannel.appendLine('Requesting available environments from LSP...');

    try {
      // Request available environments from the LSP
      const environments = await client.sendRequest<string[]>('workspace/environments');

      if (!environments || environments.length === 0) {
        vscode.window.showInformationMessage('No environments found in the workspace');
        return;
      }

      // Show quick pick to user
      const selected = await vscode.window.showQuickPick(environments, {
        placeHolder: 'Select environment for Txtx validation',
        title: 'Txtx Environment Selector'
      });

      if (selected) {
        outputChannel.appendLine(`User selected environment: ${selected}`);

        // Notify LSP of the environment change
        await client.sendNotification('workspace/setEnvironment', { environment: selected });

        // Update status bar
        envStatusBarItem.text = `$(globe) Txtx Env: ${selected}`;
        envStatusBarItem.tooltip = `Current Txtx environment: ${selected}\nClick to change`;

        // Store in workspace state instead of configuration
        await context.workspaceState.update('selectedEnvironment', selected);

        vscode.window.showInformationMessage(`Switched to environment: ${selected}`);
      }
    } catch (error) {
      outputChannel.appendLine(`Error selecting environment: ${error}`);
      vscode.window.showErrorMessage(`Failed to get environments: ${error}`);
    }
  });

  context.subscriptions.push(showLogsCommand, testDefinitionCommand, restartLspCommand, selectEnvironmentCommand);

  // Start the client
  try {
    outputChannel.appendLine('Starting LSP client...');
    
    // Start the client - this returns void
    await client.start();
    
    outputChannel.appendLine('LSP client started!');
    
    // The client should now be ready - check capabilities after a short delay
    setTimeout(() => {
      outputChannel.appendLine('Checking server capabilities...');
      
      // Try to get server info through the client's internal state
      if ((client as any)._serverProcess) {
        outputChannel.appendLine('Server process is running');
      }
      
      // Try manual capability check
      if (client.protocol2CodeConverter) {
        outputChannel.appendLine('Protocol converter available - server should be functional');
      }

      // Always send the saved environment to the LSP (even if it's 'global')
      // This ensures the LSP knows what environment the user had selected previously
      outputChannel.appendLine(`Sending saved environment to LSP: ${savedEnv}`);
      client.sendNotification('workspace/setEnvironment', { environment: savedEnv }).catch(err => {
        outputChannel.appendLine(`Failed to set environment: ${err}`);
      });

    }, 2000);
    
  } catch (error) {
    outputChannel.appendLine(`Failed to start LSP client: ${error}`);
    vscode.window.showErrorMessage(`Failed to start Txtx Language Server: ${error}`);
    statusBarItem.text = '$(x) Txtx LSP: Failed';
    statusBarItem.tooltip = `Failed to start: ${error}`;
  }

  // Language configuration is now defined in language-configuration.json
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  outputChannel.appendLine('Deactivating Txtx extension...');
  return client.stop();
}