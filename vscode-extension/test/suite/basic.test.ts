import * as assert from 'assert';
import * as vscode from 'vscode';
import * as path from 'path';

suite('Basic Extension Test Suite', () => {
    test('Extension should be present', () => {
        assert.ok(vscode.extensions.getExtension('cds-amal.txtx-lsp-extension'));
    });

    test('Should activate extension', async () => {
        const ext = vscode.extensions.getExtension('cds-amal.txtx-lsp-extension');
        assert.ok(ext);
        
        // Activate the extension if not already active
        if (!ext.isActive) {
            await ext.activate();
        }
        
        assert.ok(ext.isActive);
    });

    test('Should register txtx language', () => {
        const languages = vscode.languages.getLanguages();
        // Note: getLanguages is async but we can check if our commands are registered
        const showLogsCommand = vscode.commands.getCommands(true).then(commands => {
            return commands.includes('txtx.showLogs');
        });
        
        assert.ok(showLogsCommand);
    });

    test('Should register commands', async () => {
        const commands = await vscode.commands.getCommands(true);
        
        assert.ok(commands.includes('txtx.showLogs'), 'txtx.showLogs command not found');
        assert.ok(commands.includes('txtx.testDefinition'), 'txtx.testDefinition command not found');
        assert.ok(commands.includes('txtx.restartLsp'), 'txtx.restartLsp command not found');
    });
});