import { chromium } from '@playwright/test';
const url = process.argv[2];
const browser = await chromium.launch();
const ctx = await browser.newContext({ viewport: { width: 1600, height: 900 } });
const page = await ctx.newPage();
const cdp = await ctx.newCDPSession(page);
await cdp.send('Network.enable');
await cdp.send('Network.emulateNetworkConditions', { offline: false, latency: 80, downloadThroughput: 1_000_000 / 8, uploadThroughput: 500_000 / 8 });
const t0 = Date.now(); const samples = [];
await page.goto(url, { waitUntil: 'commit' });
for (let i = 0; i < 80; i++) {
  const r = await page.evaluate(() => {
    const card = document.querySelector('main [class*="grid overflow-hidden rounded-lg"]');
    const side = document.querySelector('main > div.w-\\[250px\\]');
    const c = card ? card.getBoundingClientRect() : null; const s = side ? side.getBoundingClientRect() : null;
    return `card=${c ? Math.round(c.x) + '+' + Math.round(c.width) : '-'} sidebar=${s ? Math.round(s.x) + ',' + Math.round(s.y) : '-'}`;
  }).catch(() => 'nav');
  samples.push([Date.now() - t0, r]); await page.waitForTimeout(50);
}
const uniq = []; for (const s of samples) if (!uniq.length || uniq[uniq.length - 1][1] !== s[1]) uniq.push(s);
console.log(JSON.stringify(uniq));
await browser.close();
