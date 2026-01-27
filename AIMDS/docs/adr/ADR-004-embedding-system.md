# ADR-004: Embedding System Replacement

**Status**: Proposed
**Date**: 2026-01-27
**Decision Makers**: ML/Security Team

## Context

Current "embedding" is a SHA256 hash spread across 384 dimensions:

```typescript
const hash = createHash('sha256').update(text).digest();
const embedding = new Array(384);
for (let i = 0; i < 384; i++) {
  embedding[i] = hash[i % hash.length] / 255;
}
```

This provides zero semantic understanding - similar texts don't produce similar vectors.

## Decision

Implement real semantic embeddings using sentence-transformers:

### Option A: Local Model (Recommended for Security)
- Use ONNX Runtime for inference
- all-MiniLM-L6-v2 (22M params, 384 dims)
- ~5ms inference on CPU
- No external API calls

### Option B: Remote API
- OpenAI ada-002 embeddings
- Higher quality but latency/cost
- Privacy concerns for security data

### Option C: Hybrid
- Local for initial filtering
- Remote for high-confidence verification

## Chosen: Option A (Local ONNX Model)

**Rationale**:
- Security data stays local
- Predictable latency
- No API costs
- Works offline

## Implementation

### TypeScript (Gateway)

```typescript
import { InferenceSession, Tensor } from 'onnxruntime-node';

class EmbeddingService {
  private session: InferenceSession;
  private tokenizer: Tokenizer;

  async initialize() {
    this.session = await InferenceSession.create('models/all-MiniLM-L6-v2.onnx');
    this.tokenizer = new BertTokenizer('models/vocab.txt');
  }

  async embed(text: string): Promise<number[]> {
    const tokens = this.tokenizer.encode(text);
    const inputTensor = new Tensor('int64', tokens.ids, [1, tokens.length]);
    const attentionTensor = new Tensor('int64', tokens.attention, [1, tokens.length]);

    const results = await this.session.run({
      input_ids: inputTensor,
      attention_mask: attentionTensor
    });

    return Array.from(results.embeddings.data as Float32Array);
  }
}
```

### Rust (Detection Crates)

```rust
use ort::{Session, Value};

pub struct EmbeddingModel {
    session: Session,
    tokenizer: Tokenizer,
}

impl EmbeddingModel {
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let encoding = self.tokenizer.encode(text)?;

        let outputs = self.session.run(vec![
            Value::from_array(encoding.get_ids())?,
            Value::from_array(encoding.get_attention_mask())?,
        ])?;

        Ok(outputs[0].try_extract_tensor()?.view().to_vec())
    }
}
```

## Model Selection

| Model | Dims | Size | Inference | Quality |
|-------|------|------|-----------|---------|
| all-MiniLM-L6-v2 | 384 | 22MB | 5ms | Good |
| all-mpnet-base-v2 | 768 | 420MB | 15ms | Better |
| BGE-small-en | 384 | 33MB | 6ms | Best for retrieval |

**Chosen**: all-MiniLM-L6-v2 for balance of size/quality

## Consequences

**Positive**:
- Real semantic similarity
- Catches paraphrased attacks
- Enables transfer learning

**Negative**:
- Model file size (~22MB)
- ONNX Runtime dependency
- Tokenizer complexity

## Verification

```typescript
// Semantic similarity test
const embed1 = await embed("ignore previous instructions");
const embed2 = await embed("disregard prior directives");
const embed3 = await embed("the weather is nice today");

// Similar meaning should be close
assert(cosineSimilarity(embed1, embed2) > 0.8);
// Unrelated should be distant
assert(cosineSimilarity(embed1, embed3) < 0.3);
```
