/**
 * Unit tests for LSP client functionality
 * These tests don't require VSCode to be running
 */

import * as assert from 'assert';
import * as net from 'net';
import * as child_process from 'child_process';
import { EventEmitter } from 'events';

// Mock LSP message helpers
function createLspMessage(content: any): string {
    const jsonStr = JSON.stringify(content);
    const contentLength = Buffer.byteLength(jsonStr, 'utf8');
    return `Content-Length: ${contentLength}\r\n\r\n${jsonStr}`;
}

function parseLspMessage(data: string): any {
    const headerEnd = data.indexOf('\r\n\r\n');
    if (headerEnd === -1) return null;
    
    const headers = data.substring(0, headerEnd);
    const contentStart = headerEnd + 4;
    
    const contentLengthMatch = headers.match(/Content-Length: (\d+)/);
    if (!contentLengthMatch) return null;
    
    const contentLength = parseInt(contentLengthMatch[1]);
    const content = data.substring(contentStart, contentStart + contentLength);
    
    try {
        return JSON.parse(content);
    } catch {
        return null;
    }
}

class MockLspServer extends EventEmitter {
    private server: net.Server | null = null;
    private connections: Set<net.Socket> = new Set();
    
    async start(port: number = 0): Promise<number> {
        return new Promise((resolve, reject) => {
            this.server = net.createServer((socket) => {
                this.connections.add(socket);
                let buffer = '';
                
                socket.on('data', (data) => {
                    buffer += data.toString();
                    const message = parseLspMessage(buffer);
                    
                    if (message) {
                        this.handleMessage(socket, message);
                        buffer = ''; // Reset buffer after processing
                    }
                });
                
                socket.on('close', () => {
                    this.connections.delete(socket);
                });
            });
            
            this.server.listen(port, '127.0.0.1', () => {
                const address = this.server!.address() as net.AddressInfo;
                resolve(address.port);
            });
            
            this.server.on('error', reject);
        });
    }
    
    private handleMessage(socket: net.Socket, message: any) {
        this.emit('message', message);
        
        // Handle initialize request
        if (message.method === 'initialize') {
            const response = {
                jsonrpc: '2.0',
                id: message.id,
                result: {
                    capabilities: {
                        definitionProvider: true,
                        hoverProvider: true,
                        completionProvider: {
                            triggerCharacters: ['.']
                        }
                    }
                }
            };
            socket.write(createLspMessage(response));
        }
        
        // Handle textDocument/definition request
        if (message.method === 'textDocument/definition') {
            const response = {
                jsonrpc: '2.0',
                id: message.id,
                result: {
                    uri: 'file:///test/txtx.yml',
                    range: {
                        start: { line: 10, character: 4 },
                        end: { line: 10, character: 20 }
                    }
                }
            };
            socket.write(createLspMessage(response));
        }
    }
    
    async stop(): Promise<void> {
        for (const socket of this.connections) {
            socket.end();
        }
        
        return new Promise((resolve) => {
            if (this.server) {
                this.server.close(() => resolve());
            } else {
                resolve();
            }
        });
    }
}

class SimpleLspClient {
    private socket: net.Socket | null = null;
    private requestId: number = 1;
    private responseHandlers: Map<number, (response: any) => void> = new Map();
    
    async connect(port: number): Promise<void> {
        return new Promise((resolve, reject) => {
            this.socket = net.createConnection({ port, host: '127.0.0.1' }, () => {
                resolve();
            });
            
            let buffer = '';
            this.socket.on('data', (data) => {
                buffer += data.toString();
                const message = parseLspMessage(buffer);
                
                if (message && message.id) {
                    const handler = this.responseHandlers.get(message.id);
                    if (handler) {
                        handler(message);
                        this.responseHandlers.delete(message.id);
                    }
                    buffer = '';
                }
            });
            
            this.socket.on('error', reject);
        });
    }
    
    async request(method: string, params: any): Promise<any> {
        return new Promise((resolve, reject) => {
            const id = this.requestId++;
            const request = {
                jsonrpc: '2.0',
                id,
                method,
                params
            };
            
            this.responseHandlers.set(id, (response) => {
                if (response.error) {
                    reject(response.error);
                } else {
                    resolve(response.result);
                }
            });
            
            this.socket!.write(createLspMessage(request));
            
            // Timeout after 5 seconds
            setTimeout(() => {
                if (this.responseHandlers.has(id)) {
                    this.responseHandlers.delete(id);
                    reject(new Error('Request timeout'));
                }
            }, 5000);
        });
    }
    
    disconnect(): void {
        if (this.socket) {
            this.socket.end();
        }
    }
}

suite('LSP Client Unit Tests', () => {
    let server: MockLspServer;
    let client: SimpleLspClient;
    let port: number;
    
    suiteSetup(async () => {
        server = new MockLspServer();
        port = await server.start();
    });
    
    suiteTeardown(async () => {
        await server.stop();
    });
    
    setup(async () => {
        client = new SimpleLspClient();
        await client.connect(port);
    });
    
    teardown(() => {
        client.disconnect();
    });
    
    test('Should initialize LSP connection', async () => {
        const result = await client.request('initialize', {
            processId: process.pid,
            rootUri: 'file:///test',
            capabilities: {}
        });
        
        assert.ok(result.capabilities);
        assert.ok(result.capabilities.definitionProvider);
        assert.ok(result.capabilities.hoverProvider);
    });
    
    test('Should get definition location', async () => {
        const result = await client.request('textDocument/definition', {
            textDocument: { uri: 'file:///test/deploy.tx' },
            position: { line: 5, character: 28 }
        });
        
        assert.equal(result.uri, 'file:///test/txtx.yml');
        assert.equal(result.range.start.line, 10);
        assert.equal(result.range.start.character, 4);
    });
    
    test('Should handle multiple concurrent requests', async () => {
        const promises = [];
        
        for (let i = 0; i < 5; i++) {
            promises.push(client.request('initialize', {
                processId: process.pid,
                rootUri: 'file:///test',
                capabilities: {}
            }));
        }
        
        const results = await Promise.all(promises);
        
        assert.equal(results.length, 5);
        results.forEach(result => {
            assert.ok(result.capabilities);
        });
    });
});

export { MockLspServer, SimpleLspClient, createLspMessage, parseLspMessage };