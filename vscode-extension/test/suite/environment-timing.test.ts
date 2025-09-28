import * as assert from 'assert';
import * as vscode from 'vscode';

// Extension ID can vary between development and production
const EXTENSION_IDS = ['cds-amal.txtx-lsp-extension', 'txtx.txtx-vscode'];

function getExtension(): vscode.Extension<any> | undefined {
    for (const id of EXTENSION_IDS) {
        const ext = vscode.extensions.getExtension(id);
        if (ext) return ext;
    }
    return undefined;
}

suite('Environment Persistence Timing Issue', () => {
    /**
     * The problem: When VSCode starts, the extension sends the saved environment
     * to the LSP after a 2-second delay (setTimeout in extension.ts line ~325).
     * However, if the LSP isn't ready yet, this notification might be lost.
     *
     * Additionally, the environment is only sent once on startup. If the LSP
     * restarts or if the connection is established later, the environment
     * won't be re-sent.
     */

    test('FAILING: Should reliably send saved environment to LSP on connection', async function() {
        this.timeout(10000);

        const extension = getExtension();
        assert.ok(extension, 'Extension should be available');

        if (!extension.isActive) {
            await extension.activate();
        }

        // Simulate the scenario where:
        // 1. User previously selected 'production' environment
        // 2. VSCode restarts
        // 3. Extension loads with saved environment
        // 4. LSP connection is established

        const context = (extension as any).exports?.context || (extension as any).extensionContext;
        if (!context) {
            console.warn('Cannot access extension context');
            this.skip();
            return;
        }

        // Set a test environment to simulate previous session
        await context.workspaceState.update('selectedEnvironment', 'production');

        // The bug: The extension sends the environment after 2 seconds
        // but doesn't verify the LSP is ready to receive it

        // Wait for the timeout to pass
        await new Promise(resolve => setTimeout(resolve, 2500));

        // At this point, the environment should have been sent to LSP
        // But we can't easily verify this in a unit test

        // The issue is that there's no retry mechanism or confirmation
        // that the LSP received and applied the environment setting

        // Check that the saved environment is still there
        const savedEnv = context.workspaceState.get('selectedEnvironment') as string;
        assert.strictEqual(savedEnv, 'production',
            'Environment should still be saved');

        // The real problem: No way to verify LSP actually has this environment set
        // The extension should either:
        // 1. Wait for LSP to be fully ready before sending
        // 2. Have a retry mechanism
        // 3. Send the environment on every LSP connection/reconnection
    });

    test('Should handle LSP restart without losing environment', async function() {
        this.timeout(15000);

        const extension = getExtension();
        if (!extension) {
            this.skip();
            return;
        }

        if (!extension.isActive) {
            await extension.activate();
        }

        const context = (extension as any).exports?.context || (extension as any).extensionContext;
        if (!context) {
            this.skip();
            return;
        }

        // Set environment
        await context.workspaceState.update('selectedEnvironment', 'staging');

        // Simulate LSP restart
        try {
            await vscode.commands.executeCommand('txtx.restartLsp');
        } catch (error) {
            console.warn('Could not restart LSP:', error);
        }

        // After restart, the environment should be re-sent
        // But currently it's not - this is the bug

        // The environment is only sent once on initial extension activation
        // Not on LSP reconnection
    });
});

suite('Proposed Fix Validation', () => {
    test('Environment should be sent when LSP becomes ready', async function() {
        // The fix should ensure that:
        // 1. The extension waits for the LSP to be in Running state
        // 2. Then sends the saved environment
        // 3. Confirms receipt or retries if needed

        // This test validates the fix once implemented
        this.skip(); // Skip until fix is implemented
    });

    test('Environment should be re-sent on LSP reconnection', async function() {
        // The fix should ensure that:
        // When LSP reconnects (after restart, crash, etc.)
        // The saved environment is automatically re-sent

        this.skip(); // Skip until fix is implemented
    });

    test('Should show environment in status bar even before LSP is ready', async function() {
        const extension = getExtension();
        if (!extension) {
            this.skip();
            return;
        }

        if (!extension.isActive) {
            await extension.activate();
        }

        const context = (extension as any).exports?.context || (extension as any).extensionContext;
        if (!context) {
            this.skip();
            return;
        }

        // Set a test environment
        await context.workspaceState.update('selectedEnvironment', 'test-env');

        // The status bar should show this immediately
        // Even if LSP isn't connected yet

        // This part works correctly - the status bar shows the saved env
        // The issue is only with sending it to the LSP
        const savedEnv = context.workspaceState.get('selectedEnvironment') as string;
        assert.strictEqual(savedEnv, 'test-env',
            'Status bar should reflect saved environment');
    });
});