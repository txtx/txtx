import * as assert from 'assert';
import * as vscode from 'vscode';
import * as path from 'path';

suite('Hover Functionality Tests', () => {
    test('Extension should be present', () => {
        const ext = vscode.extensions.getExtension('cds-amal.txtx-lsp-extension');
        assert.ok(ext, 'Extension cds-amal.txtx-lsp-extension should be present');
    });

    test('Should activate', async () => {
        const ext = vscode.extensions.getExtension('cds-amal.txtx-lsp-extension');
        if (ext) {
            await ext.activate();
            assert.ok(ext.isActive, 'Extension should be active');
        } else {
            assert.fail('Extension not found');
        }
    });
});