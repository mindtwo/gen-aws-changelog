import { rolldown } from 'rolldown';
import { rmSync } from 'node:fs';

// Clean dist directory before build
try {
    rmSync('dist', { recursive: true, force: true });
} catch (err) {
    // Ignore if dist doesn't exist
}

const bundle = await rolldown({
    input: 'cli/main.mjs',
    external: [
        // Node.js built-in modules
        /^node:/,
    ],
    platform: 'node',
    treeshake: true,
});

await bundle.write({
    file: 'dist/index.mjs',
    format: 'esm',
    banner: '#!/usr/bin/env node',
    sourcemap: false,
    inlineDynamicImports: true,
});

console.log('✓ Build complete');
