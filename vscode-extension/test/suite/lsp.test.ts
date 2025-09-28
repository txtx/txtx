
import * as assert from 'assert';
import { spawn, ChildProcess } from 'child_process';

suite('LSP Server Tests', () => {
    let lspProcess: ChildProcess;

    test('LSP server starts with --stdio', (done) => {
        lspProcess = spawn('txtx', ['lsp'], {
            stdio: 'pipe'
        });

        let hasResponse = false;
        let testComplete = false;

        lspProcess.stdout!.on('data', (data) => {
            const dataStr = data.toString();
            console.log('LSP stdout:', dataStr);
            
            // Check if we received an LSP response
            if (dataStr.includes('"jsonrpc":"2.0"') && dataStr.includes('"id":1')) {
                hasResponse = true;
                if (!testComplete) {
                    testComplete = true;
                    lspProcess.kill();
                    done();
                }
            }
        });

        lspProcess.stderr!.on('data', (data) => {
            console.log('LSP stderr:', data.toString());
        });

        lspProcess.on('close', (code) => {
            // Only call done if test hasn't completed yet
            if (!testComplete) {
                testComplete = true;
                if (hasResponse) {
                    done();
                } else {
                    done(new Error(`LSP server exited with code ${code} without responding`));
                }
            }
        });

        lspProcess.on('error', (err) => {
            if (!testComplete) {
                testComplete = true;
                done(new Error(`LSP server error: ${err.message}`));
            }
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
        
        lspProcess.stdin!.write(header + message);

        // Timeout after 3 seconds
        setTimeout(() => {
            if (!testComplete) {
                testComplete = true;
                lspProcess.kill();
                if (!hasResponse) {
                    done(new Error('LSP server did not respond to initialize request'));
                }
            }
        }, 3000);
    });

    teardown(() => {
        if (lspProcess && !lspProcess.killed) {
            lspProcess.kill();
        }
    });
});

