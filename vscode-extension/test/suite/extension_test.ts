// test/suite/extension.test.ts
import * as assert from 'assert';
import * as vscode from 'vscode';
import * as path from 'path';
import { spawn } from 'child_process';

suite('Txtx Extension Test Suite', () => {
    vscode.window.showInformationMessage('Start all tests.');

    test('Extension should activate', async () => {
        const ext = vscode.extensions.getExtension('cds-amal.txtx-lsp-extension');
        assert.ok(ext);
        
        // Activate the extension
        await ext!.activate();
        assert.strictEqual(ext!.isActive, true);
    });

    test('LSP server executable exists and responds', (done) => {
        // Test if the LSP server can be spawned
        const serverProcess = spawn('tyty', ['lsp'], {
            stdio: 'pipe'
        });

        let stdout = '';
        let stderr = '';

        serverProcess.stdout.on('data', (data) => {
            stdout += data.toString();
        });

        serverProcess.stderr.on('data', (data) => {
            stderr += data.toString();
        });

        serverProcess.on('close', (code) => {
            if (code === 0 || stdout.length > 0 || stderr.length > 0) {
                // Server exists and responded
                done();
            } else {
                done(new Error(`LSP server not found or failed to start. Exit code: ${code}`));
            }
        });

        serverProcess.on('error', (err) => {
            done(new Error(`Failed to spawn LSP server: ${err.message}`));
        });

        // Timeout after 5 seconds
        setTimeout(() => {
            serverProcess.kill();
            done(new Error('LSP server test timed out'));
        }, 5000);
    });

    test('Language configuration is registered', () => {
        const languages = vscode.languages.getLanguages();
        return languages.then(langs => {
            assert.ok(langs.includes('txtx'), 'txtx language should be registered');
        });
    });

    test('File association works', async () => {
        // Create a test .tx file
        const testUri = vscode.Uri.file(path.join(__dirname, '..', 'test.tx'));
        const document = await vscode.workspace.openTextDocument(testUri);
        
        assert.strictEqual(document.languageId, 'txtx', 'Should associate .tx files with txtx language');
    });
});
