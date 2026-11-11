// Smoke tests against the compiled package entry (dist/, built by `pretest`).
// They exercise the network-free parts of the public API so `npm test`
// gives real signal in CI (see issue #143).
const lib = require('..');

describe('@midstream/lean-agentic public entry', () => {
  test('exports the documented classes and helpers', () => {
    for (const name of [
      'LeanAgenticClient',
      'AgenticLoop',
      'KnowledgeGraph',
      'StreamProcessor',
      'BatchedStreamProcessor',
      'streamFromAsyncIterator',
    ]) {
      expect(typeof lib[name]).toBe('function');
    }
    expect(lib.EntityType.Value).toBe('Value');
  });
});

describe('KnowledgeGraph', () => {
  test('extracts capitalised words and numeric values', () => {
    const kg = new lib.KnowledgeGraph();
    const entities = kg.extractEntities('Alice paid 42 dollars');
    const names = entities.map((e) => `${e.name}:${e.entityType}`);
    expect(names).toContain('Alice:Unknown');
    expect(names).toContain('42:Value');
    expect(names).not.toContain('paid:Unknown');
  });

  test('update merges duplicates by averaging confidence', () => {
    const kg = new lib.KnowledgeGraph();
    kg.update([{ id: 'a', name: 'A', entityType: 'Concept', attributes: { x: '1' }, confidence: 0.4 }]);
    kg.update([{ id: 'a', name: 'A', entityType: 'Concept', attributes: { y: '2' }, confidence: 0.8 }]);
    expect(kg.entityCount()).toBe(1);
    const [a] = kg.queryEntities('Concept');
    expect(a.confidence).toBeCloseTo(0.6);
    expect(a.attributes).toEqual({ x: '1', y: '2' });
  });

  test('findRelated walks relations up to maxDepth', () => {
    const kg = new lib.KnowledgeGraph();
    kg.addRelation({ id: 'r1', subject: 'a', predicate: 'knows', object: 'b', confidence: 1 });
    kg.addRelation({ id: 'r2', subject: 'b', predicate: 'knows', object: 'c', confidence: 1 });
    kg.addRelation({ id: 'r3', subject: 'c', predicate: 'knows', object: 'd', confidence: 1 });
    expect(kg.relationCount()).toBe(3);
    expect([...kg.findRelated('a', 1)].sort()).toEqual(['b']);
    const twoHops = kg.findRelated('a', 2);
    expect(twoHops.has('b') && twoHops.has('c')).toBe(true);
    expect(twoHops.has('d')).toBe(false);
  });
});

describe('streamFromAsyncIterator', () => {
  test('wraps each item as a timestamped string chunk', async () => {
    async function* source() {
      yield 1;
      yield 'two';
    }
    const chunks = [];
    for await (const chunk of lib.streamFromAsyncIterator(source())) {
      chunks.push(chunk);
    }
    expect(chunks.map((c) => c.content)).toEqual(['1', 'two']);
    for (const c of chunks) expect(typeof c.timestamp).toBe('number');
  });
});
