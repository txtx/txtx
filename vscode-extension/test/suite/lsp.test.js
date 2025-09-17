"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const child_process_1 = require("child_process");
suite('LSP Server Tests', () => {
    let lspProcess;
    test('LSP server starts with --stdio', (done) => {
        lspProcess = (0, child_process_1.spawn)('txtx-lsp', ['--stdio'], {
            stdio: 'pipe'
        });
        let hasOutput = false;
        lspProcess.stdout.on('data', (data) => {
            hasOutput = true;
            console.log('LSP stdout:', data.toString());
        });
        lspProcess.stderr.on('data', (data) => {
            console.log('LSP stderr:', data.toString());
        });
        lspProcess.on('close', (code) => {
            if (code === 0 || hasOutput) {
                done();
            }
            else {
                done(new Error(`LSP server exited with code ${code}`));
            }
        });
        lspProcess.on('error', (err) => {
            done(new Error(`LSP server error: ${err.message}`));
        });
        // Send an LSP initialize request
        const initRequest = {
            jsonrpc: '2.0',
            id: 1,
            method: 'initialize',
            params: {
                processId: process.pid,
                clientInfo: { name: 'test-client', version: '1.0.0' },
                capabilities: {}
            }
        };
        const message = JSON.stringify(initRequest);
        const header = `Content-Length: ${Buffer.byteLength(message)}\r\n\r\n`;
        lspProcess.stdin.write(header + message);
        // Timeout after 3 seconds
        setTimeout(() => {
            lspProcess.kill();
            if (!hasOutput) {
                done(new Error('LSP server did not respond to initialize request'));
            }
        }, 3000);
    });
    teardown(() => {
        if (lspProcess && !lspProcess.killed) {
            lspProcess.kill();
        }
    });
});
//# sourceMappingURL=lsp.test.js.map