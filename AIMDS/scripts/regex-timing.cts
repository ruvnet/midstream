/**
 * Regex safety timing harness for the pattern packs.
 *
 * Runs every compiled pattern (normal + compact form) against ~100 KB
 * adversarial inputs and reports the slowest pattern/input pairs, then times
 * full InjectionDetector.detect() (normalise + variants + decode + re-scan)
 * on the same inputs. Threshold: 50 ms per pattern per input.
 *
 *   npx tsx scripts/regex-timing.cts [--json]
 */
import { performance } from 'node:perf_hooks';
import { loadPacks, compilePack, createInjectionDetector, normalizeText, textVariants } from '../src/detection';

const KB = 1024;
const SIZE = 100 * KB;
const rep = (s: string) => s.repeat(Math.ceil(SIZE / s.length)).slice(0, SIZE);

const inputs: Record<string, string> = {
  'a*100k': rep('a'),
  'DAN-no-mode': rep('DAN '),
  'mode-no-DAN': rep('mode '),
  'ignore-near-miss': rep('ignore all previous instruction '),
  'use-the-tool-near-miss': rep('use the fetch_url '),
  'assistant-near-miss': rep('assistant must '),
  'open-brackets': rep('[['),
  'open-braces': rep('{{'),
  'lt-lt': rep('<<'),
  'url-long-b64-no-verb': 'https://x.invalid/?d=' + rep('QUJDRA'),
  'verb-url-no-query': rep('open https://x.invalid/path/segment '),
  'verb-url-short-values': rep('open https://x.invalid/?a=1&b=2&c=3 '),
  'markdown-images': rep('![x](https://x.invalid/i.png?d=abcdefghijklmnop) '),
  'slack-mentions': rep('<@U0123ABC> '),
  'slack-channel': rep('<!channel> assistant '),
  'urlenc-100k': rep('%41'),
  'hex-100k': rep('0123456789abcdef'),
  'base64-100k': rep('SWdub3JlIGFsbCBw'),
  'spaces-100k': rep('ignore \t \n'),
  'single-letters': rep('i g n o r e '),
  'dotted-letters': rep('i.g.n.o.r.e.'),
  'system-colons': rep('system :'),
  'post-the-near-miss': rep('post the conversation history summary '),
  'decode-near-miss': rep('decode this and '),
  'translate-near-miss': rep('translate and '),
  'from-admin-near-miss': rep('from admin <@U1> '),
  'zero-width-100k': rep('i​g​n'),
  'cyrillic-100k': rep('игноре '),
  'random-printable': (() => {
    let s = '';
    let x = 12345;
    while (s.length < SIZE) {
      x = (x * 1103515245 + 12345) & 0x7fffffff;
      s += String.fromCharCode(32 + (x % 95));
    }
    return s;
  })(),
};

interface Row { pattern: string; form: 'normal' | 'compact'; input: string; ms: number }
const rows: Row[] = [];
// Time each pattern against every text variant the engine would actually scan
// (base / separators / leet / compact), picking the regex form the engine picks.
const packs = loadPacks();
const compiled = [...packs.values()].flatMap((p) => compilePack(p));
for (const [name, text] of Object.entries(inputs)) {
  for (const v of textVariants(normalizeText(text))) {
    for (const c of compiled) {
      const re = v.compactForm ? c.compactRe : c.re;
      if (!re) continue;
      const t0 = performance.now();
      re.exec(v.text);
      rows.push({ pattern: c.spec.id, form: v.compactForm ? 'compact' : 'normal', input: `${name}/${v.name}`, ms: performance.now() - t0 });
    }
  }
}
rows.sort((a, b) => b.ms - a.ms);

const detector = createInjectionDetector();
const detectRows: { input: string; ms: number; threats: number; decoded: number }[] = [];
for (const [name, text] of Object.entries(inputs)) {
  const t0 = performance.now();
  const r = detector.detect(text);
  detectRows.push({ input: name, ms: performance.now() - t0, threats: r.threats.length, decoded: r.scanned.decoded });
}
detectRows.sort((a, b) => b.ms - a.ms);

const over = rows.filter((r) => r.ms > 50);
if (process.argv.includes('--json')) {
  console.log(JSON.stringify({ size: SIZE, inputs: Object.keys(inputs).length, patterns: rows.length, over50ms: over, slowest: rows.slice(0, 15), detect: detectRows }, null, 2));
} else {
  console.log(`inputs=${Object.keys(inputs).length} x ${SIZE / KB} KB, pattern/input pairs=${rows.length}`);
  console.log(`pairs over 50 ms: ${over.length}`);
  for (const r of over) console.log(`  OVER ${r.pattern} (${r.form}) on ${r.input}: ${r.ms.toFixed(1)} ms`);
  console.log('slowest 15 pairs:');
  for (const r of rows.slice(0, 15)) console.log(`  ${r.pattern.padEnd(14)} ${r.form.padEnd(7)} ${r.input.padEnd(36)} ${r.ms.toFixed(2)} ms`);
  console.log('full detect() per input (slowest first):');
  for (const r of detectRows) console.log(`  ${r.input.padEnd(24)} ${r.ms.toFixed(1)} ms  threats=${r.threats} decoded=${r.decoded}`);
}
process.exitCode = over.length ? 1 : 0;
