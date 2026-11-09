/**
 * Child process for the crash test in tests/unit/rvf-write-batching.test.ts.
 * argv: <path> <flushed> <unflushed> <flushIntervalMs> <waitMs>
 * Writes `flushed` documents and flushes, writes `unflushed` more, waits
 * `waitMs`, prints "ready" and then idles until the parent SIGKILLs it.
 */
import { RvfStore } from '../../src/agentdb/rvf-store';
import { defaultRvfLoader } from '../../src/agentdb/store-factory';

const [path, flushed, unflushed, interval, waitMs] = process.argv.slice(2);
const DIM = 8;
const vec = (i: number) => Array.from({ length: DIM }, (_, j) => Math.sin(i * 7 + j) + 0.01);

async function main(): Promise<void> {
  const store = await RvfStore.open(await defaultRvfLoader(), {
    path,
    dimension: DIM,
    flushIntervalMs: Number(interval),
    maxBatch: 100000,
  });
  const write = (i: number) =>
    store.insert({ collection: 'incidents', document: { id: `d${i}`, embedding: vec(i), timestamp: i } });
  const a = Number(flushed);
  const b = Number(unflushed);
  for (let i = 0; i < a; i++) await write(i);
  await store.flush();
  for (let i = a; i < a + b; i++) await write(i);
  await new Promise(r => setTimeout(r, Number(waitMs)));
  process.stdout.write('ready\n');
  setInterval(() => undefined, 1000);
}

main().catch(e => {
  process.stderr.write(String(e?.stack ?? e));
  process.exit(1);
});
