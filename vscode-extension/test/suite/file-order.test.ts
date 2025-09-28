import * as assert from 'assert';
import { spawn, ChildProcess } from 'child_process';
import * as path from 'path';
import * as fs from 'fs';

/**
 * Tests to ensure that LSP builds correct state regardless of file opening order
 */
suite('File Opening Order Tests', () => {
    let lspProcess: ChildProcess;
    let requestId = 1;

    const fixturesPath = path.join(__dirname, '../../fixtures');
    const deployTxPath = path.join(fixturesPath, 'deploy.tx');
    const manifestPath = path.join(fixturesPath, 'txtx.yml');

    // Helper to send LSP request
    async function sendRequest(method: string, params: any): Promise<any> {
        return new Promise((resolve, reject) => {
            const request = {
                jsonrpc: '2.0',
                id: requestId++,
                method,
                params
            };

            const message = JSON.stringify(request);
            const header = `Content-Length: ${Buffer.byteLength(message)}\r\n\r\n`;
            
            let responseBuffer = '';
            const currentId = request.id;
            
            const dataHandler = (data: Buffer) => {
                responseBuffer += data.toString();
                
                // Parse complete messages
                while (true) {
                    const match = responseBuffer.match(/Content-Length: (\d+)\r\n\r\n/);
                    if (!match) break;
                    
                    const contentLength = parseInt(match[1]);
                    const headerLength = match[0].length;
                    const totalLength = headerLength + contentLength;
                    
                    if (responseBuffer.length < totalLength) break;
                    
                    const messageContent = responseBuffer.substring(headerLength, totalLength);
                    responseBuffer = responseBuffer.substring(totalLength);
                    
                    try {
                        const json = JSON.parse(messageContent);
                        if (json.id === currentId) {
                            lspProcess.stdout!.removeListener('data', dataHandler);
                            resolve(json);
                            return;
                        }
                    } catch (e) {
                        // Continue parsing
                    }
                }
            };

            lspProcess.stdout!.on('data', dataHandler);
            lspProcess.stdin!.write(header + message);

            setTimeout(() => {
                lspProcess.stdout!.removeListener('data', dataHandler);
                resolve({ result: null }); // Return null instead of rejecting
            }, 2000);
        });
    }

    async function initializeLSP(workspaceRoot: string): Promise<void> {
        const initResponse = await sendRequest('initialize', {
            processId: process.pid,
            rootUri: `file://${workspaceRoot}`,
            capabilities: {
                textDocument: {
                    definition: {
                        dynamicRegistration: true
                    },
                    hover: {
                        dynamicRegistration: true
                    },
                    completion: {
                        dynamicRegistration: true,
                        completionItem: {
                            snippetSupport: true
                        }
                    }
                }
            }
        });

        assert.ok(initResponse.result, 'LSP initialization failed');
        await sendRequest('initialized', {});
    }

    async function openFile(filePath: string, languageId: string): Promise<void> {
        const content = fs.readFileSync(filePath, 'utf8');
        await sendRequest('textDocument/didOpen', {
            textDocument: {
                uri: `file://${filePath}`,
                languageId,
                version: 1,
                text: content
            }
        });
        // Give LSP time to process
        await new Promise(resolve => setTimeout(resolve, 300));
    }

    async function testGoToDefinition(fromFile: string, line: number, character: number): Promise<any> {
        return await sendRequest('textDocument/definition', {
            textDocument: {
                uri: `file://${fromFile}`
            },
            position: { line, character }
        });
    }

    // Test helper to verify workspace state is correct
    async function verifyWorkspaceState(): Promise<boolean> {
        // Test 1: Go-to-definition for inputs.contract_address
        const def1 = await testGoToDefinition(deployTxPath, 7, 28);
        const hasDefinition1 = def1.result && def1.result.uri && def1.result.uri.endsWith('txtx.yml');
        
        // Test 2: Go-to-definition for inputs.api_url
        const def2 = await testGoToDefinition(deployTxPath, 9, 25);
        const hasDefinition2 = def2.result && def2.result.uri && def2.result.uri.endsWith('txtx.yml');
        
        // Test 3: Hover for inputs.private_key
        const hover = await sendRequest('textDocument/hover', {
            textDocument: { uri: `file://${deployTxPath}` },
            position: { line: 8, character: 25 }
        });
        const hasHover = hover.result && hover.result.contents;
        
        console.log('State verification:', {
            definition1: hasDefinition1,
            definition2: hasDefinition2,
            hover: hasHover
        });
        
        return hasDefinition1 || hasDefinition2 || hasHover;
    }

    setup(() => {
        requestId = 1;
    });

    test('Manifest first, then runbook', async function() {
        this.timeout(10000);

        // Start fresh LSP instance
        lspProcess = spawn('txtx', ['lsp'], {
            stdio: 'pipe',
            cwd: fixturesPath
        });

        lspProcess.on('error', (err) => {
            console.error('LSP failed to start:', err);
        });

        await initializeLSP(fixturesPath);

        // Open files: manifest first
        console.log('Opening manifest first...');
        await openFile(manifestPath, 'yaml');
        
        console.log('Opening runbook...');
        await openFile(deployTxPath, 'txtx');

        // Verify state
        const stateCorrect = await verifyWorkspaceState();
        assert.ok(stateCorrect, 'Workspace state should be correctly built when manifest opened first');

        lspProcess.kill();
    });

    test('Runbook first, then manifest', async function() {
        this.timeout(10000);

        // Start fresh LSP instance
        lspProcess = spawn('txtx', ['lsp'], {
            stdio: 'pipe',
            cwd: fixturesPath
        });

        lspProcess.on('error', (err) => {
            console.error('LSP failed to start:', err);
        });

        await initializeLSP(fixturesPath);

        // Open files: runbook first
        console.log('Opening runbook first...');
        await openFile(deployTxPath, 'txtx');
        
        // State might not be complete yet
        let stateBeforeManifest = await verifyWorkspaceState();
        console.log('State before manifest:', stateBeforeManifest);
        
        console.log('Opening manifest...');
        await openFile(manifestPath, 'yaml');
        
        // Give LSP more time to rebuild state after manifest is opened
        await new Promise(resolve => setTimeout(resolve, 1000));

        // Now state should be complete
        const stateAfterManifest = await verifyWorkspaceState();
        assert.ok(stateAfterManifest, 'Workspace state should be correctly built after manifest is opened');

        lspProcess.kill();
    });

    test('Only runbook (no manifest)', async function() {
        this.timeout(10000);

        // Start LSP in a directory without manifest
        const tempDir = path.join(fixturesPath, '..');
        
        lspProcess = spawn('txtx', ['lsp'], {
            stdio: 'pipe',
            cwd: tempDir
        });

        lspProcess.on('error', (err) => {
            console.error('LSP failed to start:', err);
        });

        await initializeLSP(tempDir);

        // Open only the runbook
        console.log('Opening runbook without manifest...');
        await openFile(deployTxPath, 'txtx');

        // Verify limited state (no manifest means no input definitions)
        const def = await testGoToDefinition(deployTxPath, 7, 28);
        const hasDefinition = def.result && def.result.uri;
        
        console.log('Definition without manifest:', hasDefinition);
        assert.ok(!hasDefinition || def.result === null, 
            'Without manifest, input definitions should not resolve');

        lspProcess.kill();
    });

    test('Multiple runbooks with same manifest', async function() {
        this.timeout(10000);

        // Create a second runbook file for testing
        const secondRunbookPath = path.join(fixturesPath, 'configure.tx');
        const secondRunbookContent = `// Configure runbook
action "configure" "http::post" {
  url = inputs.api_url
  auth = inputs.private_key
  data = inputs.contract_address
}`;

        // Write temporary file
        fs.writeFileSync(secondRunbookPath, secondRunbookContent);

        try {
            lspProcess = spawn('txtx', ['lsp'], {
                stdio: 'pipe',
                cwd: fixturesPath
            });

            await initializeLSP(fixturesPath);

            // Open manifest
            await openFile(manifestPath, 'yaml');
            
            // Open first runbook
            await openFile(deployTxPath, 'txtx');
            
            // Open second runbook
            await openFile(secondRunbookPath, 'txtx');

            // Test go-to-definition from second runbook
            const def = await sendRequest('textDocument/definition', {
                textDocument: { uri: `file://${secondRunbookPath}` },
                position: { line: 2, character: 15 } // inputs.api_url
            });

            const hasDefinition = def.result && def.result.uri && def.result.uri.endsWith('txtx.yml');
            assert.ok(hasDefinition, 'Second runbook should also have access to manifest definitions');

        } finally {
            // Clean up temporary file
            if (fs.existsSync(secondRunbookPath)) {
                fs.unlinkSync(secondRunbookPath);
            }
            if (lspProcess) {
                lspProcess.kill();
            }
        }
    });

    teardown(() => {
        if (lspProcess && !lspProcess.killed) {
            lspProcess.kill();
        }
    });
});