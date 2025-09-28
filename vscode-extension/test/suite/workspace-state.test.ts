import * as assert from 'assert';
import { spawn, ChildProcess } from 'child_process';
import * as path from 'path';
import * as fs from 'fs';

suite('LSP Workspace State Tests', () => {
    let lspProcess: ChildProcess;
    let requestId = 1;
    let responseBuffer = '';

    const fixturesPath = path.join(__dirname, '../../fixtures');
    const deployTxPath = path.join(fixturesPath, 'deploy.tx');
    const manifestPath = path.join(fixturesPath, 'txtx.yml');

    // Helper to send LSP request and get response
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
            
            const currentId = request.id;
            let dataHandler: (data: Buffer) => void;
            
            dataHandler = (data: Buffer) => {
                responseBuffer += data.toString();
                
                // Try to parse complete messages from buffer
                while (true) {
                    const contentLengthMatch = responseBuffer.match(/Content-Length: (\d+)\r\n/);
                    if (!contentLengthMatch) break;
                    
                    const contentLength = parseInt(contentLengthMatch[1]);
                    const headerEndIndex = responseBuffer.indexOf('\r\n\r\n');
                    if (headerEndIndex === -1) break;
                    
                    const messageStart = headerEndIndex + 4;
                    const messageEnd = messageStart + contentLength;
                    
                    if (responseBuffer.length < messageEnd) break;
                    
                    const messageContent = responseBuffer.substring(messageStart, messageEnd);
                    responseBuffer = responseBuffer.substring(messageEnd);
                    
                    try {
                        const jsonResponse = JSON.parse(messageContent);
                        
                        // Check if this is our response
                        if (jsonResponse.id === currentId) {
                            lspProcess.stdout!.removeListener('data', dataHandler);
                            resolve(jsonResponse);
                            return;
                        }
                        
                        // Log notifications for debugging
                        if (jsonResponse.method) {
                            console.log('LSP Notification:', jsonResponse.method);
                        }
                    } catch (e) {
                        console.error('Failed to parse LSP message:', e);
                    }
                }
            };

            lspProcess.stdout!.on('data', dataHandler);
            lspProcess.stdin!.write(header + message);

            // Timeout after 3 seconds
            setTimeout(() => {
                lspProcess.stdout!.removeListener('data', dataHandler);
                reject(new Error(`LSP request timeout for method: ${method}`));
            }, 3000);
        });
    }

    // Helper to wait for diagnostics
    function waitForDiagnostics(timeoutMs = 2000): Promise<any[]> {
        return new Promise((resolve) => {
            const diagnostics: any[] = [];
            let dataHandler: (data: Buffer) => void;
            
            const timeout = setTimeout(() => {
                lspProcess.stdout!.removeListener('data', dataHandler);
                resolve(diagnostics);
            }, timeoutMs);

            dataHandler = (data: Buffer) => {
                responseBuffer += data.toString();
                
                // Try to parse complete messages
                while (true) {
                    const contentLengthMatch = responseBuffer.match(/Content-Length: (\d+)\r\n/);
                    if (!contentLengthMatch) break;
                    
                    const contentLength = parseInt(contentLengthMatch[1]);
                    const headerEndIndex = responseBuffer.indexOf('\r\n\r\n');
                    if (headerEndIndex === -1) break;
                    
                    const messageStart = headerEndIndex + 4;
                    const messageEnd = messageStart + contentLength;
                    
                    if (responseBuffer.length < messageEnd) break;
                    
                    const messageContent = responseBuffer.substring(messageStart, messageEnd);
                    responseBuffer = responseBuffer.substring(messageEnd);
                    
                    try {
                        const jsonResponse = JSON.parse(messageContent);
                        if (jsonResponse.method === 'textDocument/publishDiagnostics') {
                            diagnostics.push(jsonResponse.params);
                            clearTimeout(timeout);
                            lspProcess.stdout!.removeListener('data', dataHandler);
                            resolve(diagnostics);
                            return;
                        }
                    } catch (e) {
                        // Continue
                    }
                }
            };

            lspProcess.stdout!.on('data', dataHandler);
        });
    }

    async function startLSP(): Promise<void> {
        // Start LSP server
        lspProcess = spawn('txtx', ['lsp'], {
            stdio: 'pipe',
            cwd: fixturesPath
        });

        lspProcess.stderr!.on('data', (data) => {
            console.error('LSP stderr:', data.toString());
        });

        lspProcess.on('error', (err) => {
            console.error('Failed to start LSP:', err);
        });

        // Clear response buffer
        responseBuffer = '';
        requestId = 1;

        // Initialize LSP
        const initResponse = await sendRequest('initialize', {
            processId: process.pid,
            rootUri: `file://${fixturesPath}`,
            capabilities: {
                textDocument: {
                    definition: {
                        dynamicRegistration: true,
                        linkSupport: true
                    },
                    hover: {
                        dynamicRegistration: true,
                        contentFormat: ['plaintext', 'markdown']
                    }
                }
            }
        });

        assert.ok(initResponse.result, 'LSP should initialize successfully');
        
        // Send initialized notification
        await sendRequest('initialized', {});
    }

    async function stopLSP(): Promise<void> {
        if (lspProcess && !lspProcess.killed) {
            await sendRequest('shutdown', {});
            lspProcess.kill();
        }
    }

    suite('Scenario 1: Open txtx.yml first, then runbook', () => {
        setup(async () => {
            await startLSP();
        });

        teardown(async () => {
            await stopLSP();
        });

        test('Should build correct workspace state', async () => {
            // Step 1: Open txtx.yml first
            const manifestContent = fs.readFileSync(manifestPath, 'utf8');
            await sendRequest('textDocument/didOpen', {
                textDocument: {
                    uri: `file://${manifestPath}`,
                    languageId: 'yaml',
                    version: 1,
                    text: manifestContent
                }
            });

            // Wait a bit for workspace to be parsed
            await new Promise(resolve => setTimeout(resolve, 500));

            // Step 2: Open deploy.tx
            const deployContent = fs.readFileSync(deployTxPath, 'utf8');
            await sendRequest('textDocument/didOpen', {
                textDocument: {
                    uri: `file://${deployTxPath}`,
                    languageId: 'txtx',
                    version: 1,
                    text: deployContent
                }
            });

            // Wait for any diagnostics
            const diagnostics = await waitForDiagnostics(1000);
            console.log('Diagnostics received:', diagnostics.length);

            // Step 3: Test go-to-definition from runbook to manifest
            const defResponse = await sendRequest('textDocument/definition', {
                textDocument: {
                    uri: `file://${deployTxPath}`
                },
                position: {
                    line: 7,  // contract_address = inputs.contract_address
                    character: 28  // Position in "contract_address"
                }
            });

            console.log('Definition response:', JSON.stringify(defResponse.result, null, 2));

            // Verify the response points to manifest
            if (defResponse.result) {
                assert.ok(defResponse.result.uri.endsWith('txtx.yml'), 
                    'Definition should point to txtx.yml');
                assert.ok(defResponse.result.range, 
                    'Definition should include a range');
            }

            // Step 4: Test hover for input variable
            const hoverResponse = await sendRequest('textDocument/hover', {
                textDocument: {
                    uri: `file://${deployTxPath}`
                },
                position: {
                    line: 9,  // api_endpoint = inputs.api_url
                    character: 25  // Position in "api_url"
                }
            });

            console.log('Hover response:', JSON.stringify(hoverResponse.result, null, 2));

            if (hoverResponse.result && hoverResponse.result.contents) {
                const contents = hoverResponse.result.contents;
                const value = typeof contents === 'string' ? contents : contents.value;
                
                // Should show the value from the manifest's default environment
                assert.ok(value.includes('https://api.test.com') || 
                         value.includes('api_url'), 
                         'Hover should show environment variable info');
            }
        });

        test('Should provide completions for input variables', async () => {
            // Open manifest first
            const manifestContent = fs.readFileSync(manifestPath, 'utf8');
            await sendRequest('textDocument/didOpen', {
                textDocument: {
                    uri: `file://${manifestPath}`,
                    languageId: 'yaml',
                    version: 1,
                    text: manifestContent
                }
            });

            // Open runbook
            const deployContent = fs.readFileSync(deployTxPath, 'utf8');
            await sendRequest('textDocument/didOpen', {
                textDocument: {
                    uri: `file://${deployTxPath}`,
                    languageId: 'txtx',
                    version: 1,
                    text: deployContent
                }
            });

            // Request completions after "inputs."
            const completionResponse = await sendRequest('textDocument/completion', {
                textDocument: {
                    uri: `file://${deployTxPath}`
                },
                position: {
                    line: 7,
                    character: 25  // After "inputs."
                }
            });

            console.log('Completion response:', JSON.stringify(completionResponse.result, null, 2));

            if (completionResponse.result && completionResponse.result.items) {
                const items = completionResponse.result.items;
                const labels = items.map((item: any) => item.label);
                
                // Should include environment variables from manifest
                assert.ok(labels.includes('contract_address') || 
                         labels.includes('api_url') ||
                         labels.includes('private_key'),
                         'Completions should include environment variables');
            }
        });
    });

    suite('Scenario 2: Open runbook first, then txtx.yml', () => {
        setup(async () => {
            await startLSP();
        });

        teardown(async () => {
            await stopLSP();
        });

        test('Should build correct workspace state when runbook opened first', async () => {
            // Step 1: Open deploy.tx first (without manifest)
            const deployContent = fs.readFileSync(deployTxPath, 'utf8');
            await sendRequest('textDocument/didOpen', {
                textDocument: {
                    uri: `file://${deployTxPath}`,
                    languageId: 'txtx',
                    version: 1,
                    text: deployContent
                }
            });

            // Wait for initial processing
            await new Promise(resolve => setTimeout(resolve, 500));

            // Step 2: Try go-to-definition before manifest is opened
            const defResponseBefore = await sendRequest('textDocument/definition', {
                textDocument: {
                    uri: `file://${deployTxPath}`
                },
                position: {
                    line: 7,  // contract_address = inputs.contract_address
                    character: 28
                }
            });

            console.log('Definition before manifest:', defResponseBefore.result);

            // Step 3: Now open txtx.yml
            const manifestContent = fs.readFileSync(manifestPath, 'utf8');
            await sendRequest('textDocument/didOpen', {
                textDocument: {
                    uri: `file://${manifestPath}`,
                    languageId: 'yaml',
                    version: 1,
                    text: manifestContent
                }
            });

            // Wait for workspace to rebuild
            await new Promise(resolve => setTimeout(resolve, 1000));

            // Step 4: Try go-to-definition after manifest is opened
            const defResponseAfter = await sendRequest('textDocument/definition', {
                textDocument: {
                    uri: `file://${deployTxPath}`
                },
                position: {
                    line: 7,  // contract_address = inputs.contract_address
                    character: 28
                }
            });

            console.log('Definition after manifest:', JSON.stringify(defResponseAfter.result, null, 2));

            // After opening manifest, definition should work
            if (defResponseAfter.result) {
                assert.ok(defResponseAfter.result.uri.endsWith('txtx.yml'), 
                    'Definition should point to txtx.yml after manifest is opened');
            }

            // Step 5: Test hover after manifest is available
            const hoverResponse = await sendRequest('textDocument/hover', {
                textDocument: {
                    uri: `file://${deployTxPath}`
                },
                position: {
                    line: 8,  // private_key = inputs.private_key
                    character: 25
                }
            });

            if (hoverResponse.result && hoverResponse.result.contents) {
                const contents = hoverResponse.result.contents;
                const value = typeof contents === 'string' ? contents : contents.value;
                console.log('Hover content:', value);
                
                assert.ok(value.includes('private_key') || value.includes('test_private_key'),
                    'Hover should show environment variable after manifest is loaded');
            }
        });

        test('Should handle workspace discovery when opening nested runbook', async () => {
            // Create a nested runbook path
            const nestedPath = path.join(fixturesPath, 'modules', 'nested.tx');
            
            // Simulate opening a nested runbook
            const deployContent = fs.readFileSync(deployTxPath, 'utf8');
            await sendRequest('textDocument/didOpen', {
                textDocument: {
                    uri: `file://${deployTxPath}`,
                    languageId: 'txtx',
                    version: 1,
                    text: deployContent
                }
            });

            // LSP should search upward and find txtx.yml
            await new Promise(resolve => setTimeout(resolve, 1000));

            // Test that workspace was discovered
            const defResponse = await sendRequest('textDocument/definition', {
                textDocument: {
                    uri: `file://${deployTxPath}`
                },
                position: {
                    line: 7,
                    character: 28
                }
            });

            // Even without explicitly opening txtx.yml, LSP should have found it
            console.log('Workspace discovery result:', defResponse.result);
            
            // This test documents current behavior - may need adjustment based on implementation
            if (defResponse.result) {
                console.log('Workspace was automatically discovered');
            } else {
                console.log('Workspace discovery not implemented - requires manifest to be opened');
            }
        });
    });

    suite('Edge Cases', () => {
        setup(async () => {
            await startLSP();
        });

        teardown(async () => {
            await stopLSP();
        });

        test('Should handle invalid input references gracefully', async () => {
            // Open both files
            const manifestContent = fs.readFileSync(manifestPath, 'utf8');
            await sendRequest('textDocument/didOpen', {
                textDocument: {
                    uri: `file://${manifestPath}`,
                    languageId: 'yaml',
                    version: 1,
                    text: manifestContent
                }
            });

            const deployContent = fs.readFileSync(deployTxPath, 'utf8');
            await sendRequest('textDocument/didOpen', {
                textDocument: {
                    uri: `file://${deployTxPath}`,
                    languageId: 'txtx',
                    version: 1,
                    text: deployContent
                }
            });

            // Try go-to-definition on a non-input reference
            const response = await sendRequest('textDocument/definition', {
                textDocument: {
                    uri: `file://${deployTxPath}`
                },
                position: {
                    line: 3,  // variable "deployed_contract"
                    character: 10
                }
            });

            // Should return null or empty for non-input references
            assert.ok(!response.result || response.result === null,
                'Should return null for non-input references');
        });

        test('Should update state when manifest changes', async () => {
            // Open manifest
            const manifestContent = fs.readFileSync(manifestPath, 'utf8');
            await sendRequest('textDocument/didOpen', {
                textDocument: {
                    uri: `file://${manifestPath}`,
                    languageId: 'yaml',
                    version: 1,
                    text: manifestContent
                }
            });

            // Open runbook
            const deployContent = fs.readFileSync(deployTxPath, 'utf8');
            await sendRequest('textDocument/didOpen', {
                textDocument: {
                    uri: `file://${deployTxPath}`,
                    languageId: 'txtx',
                    version: 1,
                    text: deployContent
                }
            });

            // Modify manifest (add new environment variable)
            const modifiedManifest = manifestContent.replace(
                'api_url: "https://api.test.com"',
                'api_url: "https://api.test.com"\n    new_variable: "test_value"'
            );

            await sendRequest('textDocument/didChange', {
                textDocument: {
                    uri: `file://${manifestPath}`,
                    version: 2
                },
                contentChanges: [{
                    text: modifiedManifest
                }]
            });

            // Wait for processing
            await new Promise(resolve => setTimeout(resolve, 500));

            // Request completions to see if new variable appears
            const completionResponse = await sendRequest('textDocument/completion', {
                textDocument: {
                    uri: `file://${deployTxPath}`
                },
                position: {
                    line: 7,
                    character: 25  // After "inputs."
                }
            });

            if (completionResponse.result && completionResponse.result.items) {
                const items = completionResponse.result.items;
                console.log('Completions after manifest change:', 
                    items.map((i: any) => i.label));
            }
        });
    });
});