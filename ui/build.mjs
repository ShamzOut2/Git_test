// Bundles the two entry points and copies static files into dist/.
import * as esbuild from 'esbuild';
import { cpSync, mkdirSync, rmSync } from 'node:fs';

const watch = process.argv.includes('--watch');
rmSync('dist', { recursive: true, force: true });
mkdirSync('dist/assets', { recursive: true });
cpSync('static', 'dist', { recursive: true });

const options = {
  entryPoints: { overlay: 'src/overlay.ts', control: 'src/control.ts' },
  outdir: 'dist/assets',
  bundle: true,
  format: 'esm',
  target: 'es2022',
  minify: !watch,
  sourcemap: watch ? 'inline' : false,
  logLevel: 'info',
};

if (watch) {
  const ctx = await esbuild.context(options);
  await ctx.watch();
} else {
  await esbuild.build(options);
}
