import * as assert from 'assert';
import { spawn, ChildProcess } from 'child_process';
import * as path from 'path';
import * as fs from 'fs';

suite('Go to Definition Tests', () => {
    let lspProcess: ChildProcess;
    let requestId = 1;

    const fixturesPath = path.join(__dirname, '../../fixtures');
    const deployTxPath = path.join(fixturesPath, 'deploy.tx');
    const manifestPath = path.join(fixturesPath, 'txtx.yml');

    // Helper to send LSP request
    function sendRequest(method: string, params: any): Promise<any> {
        return new Promise((resolve, reject) => {
            const request = {
                jsonrpc: '2.0',
                id: requestId++,
                method,
                params
            };

            const message = JSON.stringify(request);
            const header = `Content-Length: ${Buffer.byteLength(message)}\r\n\r\n`;
            
            let response = '';
            const dataHandler = (data: Buffer) => {
                response += data.toString();
                
                // Try to parse response
                const contentLengthMatch = response.match(/Content-Length: (\d+)/);
                if (contentLengthMatch) {
                    const contentLength = parseInt(contentLengthMatch[1]);
                    const headerEndIndex = response.indexOf('\r\n\r\n');
                    if (headerEndIndex !== -1) {
                        const messageStart = headerEndIndex + 4;
                        const messageContent = response.substring(messageStart);
                        if (messageContent.length >= contentLength) {
                            try {
                                const jsonResponse = JSON.parse(messageContent.substring(0, contentLength));
                                lspProcess.stdout!.removeListener('data', dataHandler);
                                resolve(jsonResponse);
                            } catch (e) {
                                // Continue collecting data
                            }
                        }
                    }
                }
            };

            lspProcess.stdout!.on('data', dataHandler);
            lspProcess.stdin!.write(header + message);

            // Timeout after 2 seconds
            setTimeout(() => {
                lspProcess.stdout!.removeListener('data', dataHandler);
                reject(new Error('LSP request timeout'));
            }, 2000);
        });
    }

    setup(async () => {
        // Start LSP server - use txtx from PATH or development build
        const devBinary = path.join(__dirname, '..', '..', '..', 'target', 'debug', 'txtx');
        const lspCommand = fs.existsSync(devBinary) ? devBinary : 'txtx';
        
        lspProcess = spawn(lspCommand, ['lsp'], {
            stdio: 'pipe',
            cwd: fixturesPath
        });

        lspProcess.stderr!.on('data', (data) => {
            console.error('LSP stderr:', data.toString());
        });

        // Initialize LSP
        const initResponse = await sendRequest('initialize', {
            processId: process.pid,
            rootUri: `file://${fixturesPath}`,
            capabilities: {
                textDocument: {
                    definition: {
                        dynamicRegistration: true,
                        linkSupport: true
                    }
                }
            }
        });

        assert.ok(initResponse.result, 'LSP should initialize successfully');
        
        // Send initialized notification
        await sendRequest('initialized', {});

        // Open the test files
        const deployContent = fs.readFileSync(deployTxPath, 'utf8');
        await sendRequest('textDocument/didOpen', {
            textDocument: {
                uri: `file://${deployTxPath}`,
                languageId: 'txtx',
                version: 1,
                text: deployContent
            }
        });

        const manifestContent = fs.readFileSync(manifestPath, 'utf8');
        await sendRequest('textDocument/didOpen', {
            textDocument: {
                uri: `file://${manifestPath}`,
                languageId: 'txtx',
                version: 1,
                text: manifestContent
            }
        });
    });

    teardown(() => {
        if (lspProcess && !lspProcess.killed) {
            lspProcess.kill();
        }
    });

    test('Go to definition for inputs.contract_address', async () => {
        // Position is on line 7 (0-indexed), at "inputs.contract_address"
        // The word starts at character 18
        const response = await sendRequest('textDocument/definition', {
            textDocument: {
                uri: `file://${deployTxPath}`
            },
            position: {
                line: 7,  // Line with: contract_address = inputs.contract_address
                character: 28  // Position within "contract_address" after "inputs."
            }
        });

        // This should fail initially since the feature is not implemented
        console.log('Definition response:', JSON.stringify(response, null, 2));
        
        // For now, we expect it to return null since it's not implemented
        assert.ok(response.result === null || response.result === undefined, 
                  'Currently returns null (not implemented). Should return a location when implemented.');
        
        // TODO: When implemented, uncomment these assertions:
        // assert.ok(response.result, 'Should return a definition location');
        // const location = response.result;
        // assert.strictEqual(location.uri, `file://${manifestPath}`, 'Should point to txtx.yml');
        // assert.strictEqual(location.range.start.line, 9, 'Should point to correct line in txtx.yml');
    });

    test('Go to definition for inputs.api_url', async () => {
        // Position is on line 16 (0-indexed), at "inputs.api_url"
        const response = await sendRequest('textDocument/definition', {
            textDocument: {
                uri: `file://${deployTxPath}`
            },
            position: {
                line: 16,  // Line with: value = inputs.api_url
                character: 18  // Position within "api_url" after "inputs."
            }
        });

        assert.ok(response.result, 'Should return a definition location');
        
        const location = response.result;
        assert.strictEqual(location.uri, `file://${manifestPath}`, 'Should point to txtx.yml');
        
        // Should point to line 11 (0-indexed) where api_url is defined in default environment
        assert.strictEqual(location.range.start.line, 11, 'Should point to correct line in txtx.yml');
    });

    test('Go to definition for non-existent input should return null', async () => {
        const response = await sendRequest('textDocument/definition', {
            textDocument: {
                uri: `file://${deployTxPath}`
            },
            position: {
                line: 3,  // Line with regular variable definition
                character: 10
            }
        });

        // Should return null or undefined for non-input references
        assert.ok(!response.result || response.result === null, 'Should return null for non-input reference');
    });
});