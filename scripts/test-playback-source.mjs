import assert from 'node:assert/strict';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
import { chromium } from 'playwright-core';
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const output = join(root, 'test-results', 'playback-source', new Date().toISOString().replace(/[:.]/g, '-'));
await mkdir(output, { recursive: true });
const report = { passed: [], externalRequests: [] };
const vite = await createServer({ root, configFile: join(root, 'vite.config.ts'), server: { host: '127.0.0.1', port: 0, strictPort: false, hmr: false } });
let browser;
try {
  await vite.listen();
  const origin = vite.resolvedUrls.local[0];
  browser = await chromium.launch({ executablePath: process.env.BROWSER_EXECUTABLE || undefined, headless: true });
  const page = await browser.newPage();
  await page.route('**/*', async route => {
    const url = new URL(route.request().url());
    if (url.origin !== new URL(origin).origin) { report.externalRequests.push(url.origin); return route.abort(); }
    if (url.pathname === '/__playback_contract__') return route.fulfill({ contentType: 'text/html', body: await vite.transformIndexHtml('/__playback_contract__','<!doctype html><title>Local playback contract</title>') });
    return route.continue();
  });
  await page.goto(`${origin}__playback_contract__`);
  const result = await page.evaluate(async () => {
    const raw = 'https://media.fixture.invalid/list.m3u8?auth_key=synthetic';
    const source = '0123456789abcdef0123456789abcdef';
    let payload = { video_url: raw, source_id: source };
    window.__TAURI_INTERNALS__ = { invoke: async command => command === 'api_play' ? payload : undefined };
    const { api, selfTest } = await import('/src/api.ts');
    const first = await api.play(123,1);
    const wrapped = new URL(first.video_url);
    const fetched = [];
    const key = 'https://media.fixture.invalid/media.key?auth_key=synthetic';
    const keyProxy = `${wrapped.origin}/seg?u=${encodeURIComponent(key)}&s=${source}`;
    window.fetch = async url => {
      fetched.push(String(url));
      return new URL(url).pathname === '/hls'
        ? new Response(`#EXTM3U\n#EXT-X-KEY:METHOD=AES-128,URI="${keyProxy}"\n`,{ status: 200 })
        : new Response(new Uint8Array(16),{ status: 200 });
    };
    await selfTest();
    payload = { video_url: raw };
    const legacy = await api.play(123,1);
    return { first, path: wrapped.pathname, upstream: wrapped.searchParams.get('u'), source: wrapped.searchParams.get('s'), fetched, legacy, raw, key };
  });
  assert.equal(result.path,'/hls');
  assert.equal(result.upstream,result.raw);
  assert.equal(result.source,'0123456789abcdef0123456789abcdef');
  report.passed.push('api.play carries source_id through the local playlist URL');
  assert.equal(result.fetched.length,3);
  assert.equal(result.fetched[0],result.first.video_url);
  report.passed.push('selfTest reuses local playlist URLs without double wrapping');
  for (const keyRequest of result.fetched.slice(1)) {
    const key = new URL(keyRequest);
    assert.equal(key.searchParams.get('u'),result.key);
    assert.equal(key.searchParams.get('s'),result.source);
  }
  report.passed.push('both diagnostic key paths preserve source_id separately from the upstream query');
  assert.equal(result.legacy.video_url,result.raw);
  report.passed.push('existing fixtures without source_id retain the original source contract');
  const lifecycle = await browser.newPage();
  await lifecycle.route('**/*', route => {
    const url = new URL(route.request().url());
    if (url.origin !== new URL(origin).origin) { report.externalRequests.push(url.origin); return route.abort(); }
    if (['/hls','/seg'].includes(url.pathname)) return route.fulfill({ contentType: 'application/vnd.apple.mpegurl', body: '#EXTM3U\n#EXT-X-TARGETDURATION:1\n#EXT-X-ENDLIST\n' });
    return route.continue();
  });
  await lifecycle.goto(`${origin}tests/player-seek.html?inactive=1&diagnostics=1&proxy=${encodeURIComponent(new URL(origin).origin)}`);
  await lifecycle.waitForFunction(() => typeof window.playerSeekFixture?.setActive === 'function');
  await lifecycle.evaluate(async () => {
    const { api } = await import('/src/api.ts');
    const original = api.play;
    window.sourceRequests = 0;
    api.play = async (...args) => { window.sourceRequests++; return original(...args); };
  });
  await lifecycle.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  assert.equal(await lifecycle.evaluate(() => window.sourceRequests),0);
  await lifecycle.evaluate(() => window.playerSeekFixture.setActive(true));
  await lifecycle.waitForFunction(() => window.sourceRequests === 1);
  await lifecycle.evaluate(() => window.playerSeekFixture.setActive(false));
  await lifecycle.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  assert.equal(await lifecycle.evaluate(() => window.sourceRequests),1);
  await lifecycle.evaluate(() => window.playerSeekFixture.setActive(true));
  await lifecycle.waitForFunction(() => window.sourceRequests === 2);
  report.passed.push('Feed defers source requests while inactive and refreshes the same episode on reactivation');
  assert.deepEqual(report.externalRequests,[]);
  console.log(JSON.stringify(report,null,2));
} finally {
  await writeFile(join(output, 'report.json'),JSON.stringify(report,null,2));
  await browser?.close(); await vite.close();
  console.log('Artifacts: ' + output);
}
