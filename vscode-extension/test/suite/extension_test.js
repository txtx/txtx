"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
// test/suite/extension.test.ts
const assert = __importStar(require("assert"));
const vscode = __importStar(require("vscode"));
const path = __importStar(require("path"));
const child_process_1 = require("child_process");
suite('Txtx Extension Test Suite', () => {
    vscode.window.showInformationMessage('Start all tests.');
    test('Extension should activate', async () => {
        const ext = vscode.extensions.getExtension('cds-amal.txtx-lsp-extension');
        assert.ok(ext);
        // Activate the extension
        await ext.activate();
        assert.strictEqual(ext.isActive, true);
    });
    test('LSP server executable exists and responds', (done) => {
        // Test if the LSP server can be spawned
        const serverProcess = (0, child_process_1.spawn)('tyty', ['lsp'], {
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
            }
            else {
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
//# sourceMappingURL=extension_test.js.map