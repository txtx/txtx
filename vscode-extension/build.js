const esbuild = require('esbuild');

const isProduction = process.env.NODE_ENV === 'production' || process.argv.includes('--production');

esbuild.build({
    entryPoints: ['./src/extension.ts'],
    bundle: true,
    external: ['vscode'],
    platform: 'node',
    target: 'node16',
    outfile: 'out/extension.js',
    format: 'cjs',
    sourcemap: !isProduction,
    minify: isProduction,
    treeShaking: true,
    legalComments: isProduction ? 'none' : 'inline'
}).catch(() => process.exit(1));