const esbuild = require('esbuild');

esbuild.build({
  entryPoints: ['src/index.ts'],
  bundle: true,
  platform: 'node',
  target: 'node18',
  outfile: 'dist/aimds.bundle.js',
  // Externalize packages with native bindings or wasm that cannot be bundled
  external: [
    'lean-agentic',
    'agentdb',
    '@xenova/transformers',
    'onnxruntime-node',
    'sharp',
  ],
  sourcemap: true,
  minify: false, // Keep readable for debugging
}).catch(() => process.exit(1));
