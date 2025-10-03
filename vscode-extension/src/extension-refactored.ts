import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
  RevealOutputChannelOn,
  State
} from 'vscode-languageclient/node';

// Constants
const EXTENSION_NAME = 'Txtx Language Server';
const DEFAULT_ENVIRONMENT = 'global';
const LSP_COMMAND = 'txtx';
const LSP_ARGS = ['lsp'];

// Type definitions
interface ExtensionContext {
  client: LanguageClient;
  outputChannel: vscode.OutputChannel;
  statusBar: vscode.StatusBarItem;
  envStatusBar: vscode.StatusBarItem;
}

class LspPathResolver {
  constructor(
    private outputChannel: vscode.OutputChannel,
    private workspaceFolders: readonly vscode.WorkspaceFolder[] | undefined
  ) {}

  resolve(): string {
    // Priority order: config -> env -> relative paths -> workspace -> system
    return (
      this.resolveFromConfig() ||
      this.resolveFromEnvironment() ||
      this.resolveFromProjectBinaries() ||
      this.resolveFromWorkspace() ||
      this.resolveFromSystem()
    );
  }

  private resolveFromConfig(): string | null {
    const config = vscode.workspace.getConfiguration('txtx');
    const configuredPath = config.get<string>('lspPath');

    if (!configuredPath?.length) {
      return null;
    }

    const resolvedPath = this.substituteVariables(configuredPath);

    if (fs.existsSync(resolvedPath)) {
      this.log(`Using configured LSP path: ${resolvedPath}`);
      return resolvedPath;
    }

    this.log(`Configured path not found: ${resolvedPath}, falling back to auto-detection`);
    return null;
  }

  private resolveFromEnvironment(): string | null {
    const envPath = process.env.TXTX_LSP_PATH;

    if (envPath && fs.existsSync(envPath)) {
      this.log(`Using TXTX_LSP_PATH: ${envPath}`);
      return envPath;
    }

    return null;
  }

  private resolveFromProjectBinaries(): string | null {
    const extensionRoot = path.join(__dirname, '..');
    const projectRoot = path.join(extensionRoot, '..');

    return this.findFirstExisting([
      path.join(projectRoot, 'target', 'release', LSP_COMMAND),
      path.join(projectRoot, 'target', 'debug', LSP_COMMAND),
    ], 'project binary');
  }

  private resolveFromWorkspace(): string | null {
    if (!this.workspaceFolders?.length) {
      return null;
    }

    const workspaceRoot = this.workspaceFolders[0].uri.fsPath;

    return this.findFirstExisting([
      path.join(workspaceRoot, 'target', 'release', LSP_COMMAND),
      path.join(workspaceRoot, 'target', 'debug', LSP_COMMAND),
    ], 'workspace binary');
  }

  private resolveFromSystem(): string {
    this.log('Using system txtx from PATH');
    return LSP_COMMAND;
  }

  private findFirstExisting(paths: string[], description: string): string | null {
    for (const binaryPath of paths) {
      if (fs.existsSync(binaryPath)) {
        this.log(`Using ${description}: ${binaryPath}`);
        return binaryPath;
      }
    }
    return null;
  }

  private substituteVariables(configuredPath: string): string {
    if (!this.workspaceFolders?.length) {
      return configuredPath;
    }

    const workspaceFolder = this.workspaceFolders[0].uri.fsPath;
    return configuredPath.replace('${workspaceFolder}', workspaceFolder);
  }

  private log(message: string): void {
    this.outputChannel.appendLine(message);
  }
}

class EnvironmentManager {
  constructor(
    private context: vscode.ExtensionContext,
    private client: LanguageClient,
    private outputChannel: vscode.OutputChannel,
    private envStatusBar: vscode.StatusBarItem
  ) {}

  async initialize(): Promise<void> {
    const savedEnv = this.getSavedEnvironment();
    this.updateStatusBar(savedEnv);
    await this.sendEnvironmentToLsp(savedEnv);
  }

  async handleReconnection(): Promise<void> {
    const savedEnv = this.getSavedEnvironment();
    this.outputChannel.appendLine(`LSP reconnected, restoring environment: ${savedEnv}`);
    await this.sendEnvironmentToLsp(savedEnv);
  }

  async selectEnvironment(): Promise<void> {
    this.outputChannel.appendLine('Requesting available environments from LSP...');

    try {
      const environments = await this.client.sendRequest<string[]>('workspace/environments');

      if (!environments?.length) {
        vscode.window.showInformationMessage('No environments found in the workspace');
        return;
      }

      const selected = await vscode.window.showQuickPick(environments, {
        placeHolder: 'Select environment for Txtx validation',
        title: 'Txtx Environment Selector'
      });

      if (selected) {
        await this.setEnvironment(selected);
        vscode.window.showInformationMessage(`Switched to environment: ${selected}`);
      }
    } catch (error) {
      this.handleError('Failed to get environments', error);
    }
  }

  private async setEnvironment(environment: string): Promise<void> {
    this.outputChannel.appendLine(`Setting environment: ${environment}`);

    await this.client.sendNotification('workspace/setEnvironment', { environment });
    this.updateStatusBar(environment);
    await this.context.workspaceState.update('selectedEnvironment', environment);
  }

  private async sendEnvironmentToLsp(environment: string): Promise<void> {
    try {
      this.outputChannel.appendLine(`Sending environment to LSP: ${environment}`);
      await this.client.sendNotification('workspace/setEnvironment', { environment });
      this.outputChannel.appendLine('Environment successfully sent to LSP');
    } catch (error) {
      this.handleError('Failed to send environment to LSP', error);
    }
  }

  private getSavedEnvironment(): string {
    return this.context.workspaceState.get<string>('selectedEnvironment') || DEFAULT_ENVIRONMENT;
  }

  private updateStatusBar(environment: string): void {
    this.envStatusBar.text = `$(globe) Txtx Env: ${environment}`;
    this.envStatusBar.tooltip = `Current Txtx environment: ${environment}\nClick to change`;
  }

  private handleError(message: string, error: unknown): void {
    const errorMessage = error instanceof Error ? error.message : String(error);
    this.outputChannel.appendLine(`${message}: ${errorMessage}`);
    vscode.window.showErrorMessage(`${message}: ${errorMessage}`);
  }
}

class StatusBarManager {
  private readonly icons = {
    starting: '$(sync~spin)',
    ready: '$(check)',
    stopped: '$(x)',
    failed: '$(x)'
  } as const;

  constructor(private statusBar: vscode.StatusBarItem) {}

  updateState(state: State): void {
    switch (state) {
      case State.Starting:
        this.setStatus('starting', 'Starting...');
        break;
      case State.Running:
        this.setStatus('ready', 'Ready', 'Txtx Language Server is running');
        break;
      case State.Stopped:
        this.setStatus('stopped', 'Stopped', 'Txtx Language Server is not running');
        break;
    }
  }

  setError(error: unknown): void {
    const errorMessage = error instanceof Error ? error.message : String(error);
    this.setStatus('failed', 'Failed', `Failed to start: ${errorMessage}`);
  }

  private setStatus(
    icon: keyof typeof this.icons,
    text: string,
    tooltip?: string
  ): void {
    this.statusBar.text = `${this.icons[icon]} Txtx LSP: ${text}`;
    if (tooltip) {
      this.statusBar.tooltip = tooltip;
    }
  }
}

function createMiddleware(outputChannel: vscode.OutputChannel): LanguageClientOptions['middleware'] {
  const logRequest = (type: string, document: vscode.TextDocument, position: vscode.Position) => {
    outputChannel.appendLine(
      `[${type} Request] File: ${document.uri.fsPath}, Position: ${position.line}:${position.character}`
    );
  };

  const logResponse = (type: string, result: any) => {
    if (result) {
      outputChannel.appendLine(`[${type} Response] ${JSON.stringify(result) ?? 'Has content'}`);
    } else {
      outputChannel.appendLine(`[${type} Response] No content`);
    }
  };

  const logError = (type: string, error: unknown) => {
    outputChannel.appendLine(`[${type} Error] ${error}`);
  };

  return {
    provideDefinition: async (document, position, token, next) => {
      logRequest('Definition', document, position);
      try {
        const result = await next(document, position, token);
        logResponse('Definition', result);
        return result;
      } catch (error) {
        logError('Definition', error);
        throw error;
      }
    },
    provideHover: async (document, position, token, next) => {
      logRequest('Hover', document, position);
      try {
        const result = await next(document, position, token);
        logResponse('Hover', result);
        return result;
      } catch (error) {
        logError('Hover', error);
        throw error;
      }
    }
  };
}

function registerCommands(
  context: vscode.ExtensionContext,
  ctx: ExtensionContext,
  envManager: EnvironmentManager
): void {
  const commands = [
    {
      name: 'txtx.showLogs',
      handler: () => ctx.outputChannel.show()
    },
    {
      name: 'txtx.restartLsp',
      handler: async () => {
        ctx.outputChannel.appendLine('Restarting LSP client...');
        if (ctx.client) {
          await ctx.client.stop();
          await ctx.client.start();
        }
      }
    },
    {
      name: 'txtx.selectEnvironment',
      handler: () => envManager.selectEnvironment()
    },
    {
      name: 'txtx.testDefinition',
      handler: async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
          vscode.window.showWarningMessage('No active editor');
          return;
        }

        const position = editor.selection.active;
        const wordRange = editor.document.getWordRangeAtPosition(position);
        const word = wordRange ? editor.document.getText(wordRange) : '';

        ctx.outputChannel.appendLine(`Testing go-to-definition at ${position.line}:${position.character}`);
        ctx.outputChannel.appendLine(`Word at cursor: "${word}"`);
        ctx.outputChannel.appendLine(`Current line: "${editor.document.lineAt(position.line).text}"`);

        try {
          const definitions = await vscode.commands.executeCommand<vscode.Location[]>(
            'vscode.executeDefinitionProvider',
            editor.document.uri,
            position
          );

          if (definitions?.length) {
            ctx.outputChannel.appendLine(`Found ${definitions.length} definition(s):`);
            definitions.forEach((def, i) => {
              ctx.outputChannel.appendLine(
                `  ${i + 1}. ${def.uri.fsPath} at ${def.range.start.line}:${def.range.start.character}`
              );
            });
          } else {
            ctx.outputChannel.appendLine('No definitions found');
          }
        } catch (error) {
          ctx.outputChannel.appendLine(`Error getting definitions: ${error}`);
        }
      }
    }
  ];

  commands.forEach(({ name, handler }) => {
    const disposable = vscode.commands.registerCommand(name, handler);
    context.subscriptions.push(disposable);
  });
}

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const outputChannel = vscode.window.createOutputChannel(EXTENSION_NAME);
  outputChannel.appendLine('Txtx extension activating...');

  // Resolve LSP path
  const pathResolver = new LspPathResolver(outputChannel, vscode.workspace.workspaceFolders);
  const serverCommand = pathResolver.resolve();

  outputChannel.appendLine(`LSP command: ${serverCommand} ${LSP_ARGS.join(' ')}`);
  outputChannel.appendLine(
    `Workspace folders: ${vscode.workspace.workspaceFolders?.map(f => f.uri.fsPath).join(', ')}`
  );

  // Create server options
  const serverOptions: ServerOptions = {
    run: {
      command: serverCommand,
      args: LSP_ARGS,
      transport: TransportKind.stdio
    },
    debug: {
      command: serverCommand,
      args: LSP_ARGS,
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

  // Create client options
  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: 'file', language: 'txtx' },
      { scheme: 'file', pattern: '**/txtx.{yml,yaml}' }
    ],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher('**/{*.tx,txtx.yml,txtx.yaml}')
    },
    outputChannel,
    revealOutputChannelOn: RevealOutputChannelOn.Info,
    middleware: createMiddleware(outputChannel)
  };

  // Create client
  const client = new LanguageClient(
    'txtxLanguageServer',
    EXTENSION_NAME,
    serverOptions,
    clientOptions
  );

  // Create status bars
  const statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 100);
  statusBar.show();
  context.subscriptions.push(statusBar);

  const envStatusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Right, 99);
  envStatusBar.command = 'txtx.selectEnvironment';
  envStatusBar.show();
  context.subscriptions.push(envStatusBar);

  // Create managers
  const statusBarManager = new StatusBarManager(statusBar);
  const envManager = new EnvironmentManager(context, client, outputChannel, envStatusBar);

  // Extension context
  const ctx: ExtensionContext = { client, outputChannel, statusBar, envStatusBar };

  // Handle client state changes
  client.onDidChangeState(async (event) => {
    outputChannel.appendLine(`[State Change] Old: ${State[event.oldState]}, New: ${State[event.newState]}`);
    statusBarManager.updateState(event.newState);

    // Restore environment on reconnection
    if (event.newState === State.Running && event.oldState === State.Stopped) {
      await envManager.handleReconnection();
    }
  });

  // Register commands
  registerCommands(context, ctx, envManager);

  // Start the client
  try {
    outputChannel.appendLine('Starting LSP client...');

    // Start the client and wait for it to be fully ready
    await client.start();

    outputChannel.appendLine('LSP client started and ready!');

    // Client is now fully ready - initialize environment immediately
    await envManager.initialize();
  } catch (error) {
    outputChannel.appendLine(`Failed to start LSP client: ${error}`);
    vscode.window.showErrorMessage(`Failed to start ${EXTENSION_NAME}: ${error}`);
    statusBarManager.setError(error);
  }
}

export function deactivate(): Thenable<void> | undefined {
  // Note: We don't have access to the client here in the refactored version
  // You might want to store it globally or in a different pattern
  return undefined;
}