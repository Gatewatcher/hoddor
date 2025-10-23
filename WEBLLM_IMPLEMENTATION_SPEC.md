# Spécification d'Implémentation : Intégration WebLLM dans Hoddor

**Version:** 1.0
**Date:** 2025-01-22
**Status:** Ready for Implementation
**Contexte:** Suite à l'implémentation de SimpleGraphAdapter

---

## 📋 Table des Matières

1. [Vue d'ensemble](#1-vue-densemble)
2. [Architecture Technique](#2-architecture-technique)
3. [Composants à Implémenter](#3-composants-à-implémenter)
4. [Interfaces et API](#4-interfaces-et-api)
5. [Flux d'Intégration](#5-flux-dintégration)
6. [Plan d'Implémentation](#6-plan-dimplémentation)
7. [Tests et Validation](#7-tests-et-validation)
8. [Considérations de Performance](#8-considérations-de-performance)

---

## 1. Vue d'ensemble

### 1.1 Objectif

Intégrer WebLLM dans Hoddor pour permettre l'inférence LLM locale dans le navigateur, en s'appuyant sur le `SimpleGraphAdapter` existant pour fournir du contexte via RAG.

### 1.2 État Actuel

✅ **Déjà Implémenté:**
- SimpleGraphAdapter (in-memory graph avec HashMap)
- GraphPersistence (backup/restore chiffré avec Age)
- GraphPort trait (interface pour graph operations)
- OPFS storage (OpfsStorage)
- Architecture hexagonale (ports & adapters)

❌ **À Implémenter:**
- WebLLM adapter (LLM inference)
- Embedding adapter (text → vector)
- RAG orchestrator (coordination)
- JavaScript/TypeScript façade pour UI

### 1.3 Principe d'Intégration

```
┌─────────────────────────────────────────┐
│   UI Layer (React/Vue/Vanilla JS)       │
│   ├─ Chat interface                     │
│   └─ Model selector                     │
└─────────────────────────────────────────┘
              ↓ JavaScript
┌─────────────────────────────────────────┐
│   TypeScript Orchestration Layer        │
│   ├─ RAGOrchestrator                    │
│   ├─ WebLLMAdapter (JS binding)         │
│   └─ EmbeddingService                   │
└─────────────────────────────────────────┘
              ↓ wasm-bindgen
┌─────────────────────────────────────────┐
│   Hoddor WASM Core (Rust)               │
│   ├─ SimpleGraphAdapter                 │
│   ├─ GraphPersistence                   │
│   └─ Domain logic                       │
└─────────────────────────────────────────┘
```

**Décision architecturale clé:** WebLLM reste en JavaScript pur (ne pas WASM-wrapp), car il est optimisé pour WebGPU/WASM internalement.

---

## 2. Architecture Technique

### 2.1 Stack Complète

| Layer | Technologies | Fichiers |
|-------|--------------|----------|
| **UI** | React/Vue | `src/playground/` |
| **Orchestration** | TypeScript | `src/services/rag/` |
| **WebLLM** | JavaScript (@mlc-ai/web-llm) | `src/services/llm/` |
| **Embeddings** | Transformers.js | `src/services/embeddings/` |
| **Graph** | Rust (WASM) | `src/adapters/wasm/simple_graph.rs` |
| **Storage** | OPFS + Age | `src/adapters/wasm/opfs_storage.rs` |

### 2.2 Diagramme de Composants

```
┌──────────────────────────────────────────────────────────┐
│                   TYPESCRIPT LAYER                        │
│  ┌────────────────────────────────────────────────────┐  │
│  │              RAGOrchestrator                       │  │
│  │  ┌──────────────────────────────────────────────┐ │  │
│  │  │  async ask(question: string): Promise<...>   │ │  │
│  │  │  1. Generate embedding (question)            │ │  │
│  │  │  2. Search graph (via WASM)                  │ │  │
│  │  │  3. Decrypt context (via WASM)               │ │  │
│  │  │  4. Build prompt                             │ │  │
│  │  │  5. Call LLM                                 │ │  │
│  │  │  6. Save interaction (via WASM)              │ │  │
│  │  └──────────────────────────────────────────────┘ │  │
│  └────────────────────────────────────────────────────┘  │
│                                                           │
│  ┌─────────────────┐   ┌──────────────────────────────┐ │
│  │ WebLLMService   │   │   EmbeddingService           │ │
│  │ - loadModel()   │   │   - embed(text)              │ │
│  │ - chat()        │   │   - dimension: 384           │ │
│  │ - stream()      │   │   (Transformers.js wrapper)  │ │
│  └─────────────────┘   └──────────────────────────────┘ │
│         ↓                          ↓                     │
│  ┌─────────────────┐   ┌──────────────────────────────┐ │
│  │ @mlc-ai/web-llm│   │   @xenova/transformers       │ │
│  │ (NPM package)   │   │   (NPM package)              │ │
│  └─────────────────┘   └──────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
                          ↓ wasm-bindgen
┌──────────────────────────────────────────────────────────┐
│                     RUST WASM LAYER                       │
│  ┌────────────────────────────────────────────────────┐  │
│  │            HoddorCore (wasm-bindgen)               │  │
│  │  ┌──────────────────────────────────────────────┐ │  │
│  │  │  #[wasm_bindgen]                             │ │  │
│  │  │  pub fn vector_search(embedding: Vec<f32>)   │ │  │
│  │  │  pub fn get_neighbors(node_id: &str)         │ │  │
│  │  │  pub fn create_node(...)                     │ │  │
│  │  └──────────────────────────────────────────────┘ │  │
│  └────────────────────────────────────────────────────┘  │
│                          ↓                                │
│  ┌────────────────────────────────────────────────────┐  │
│  │         SimpleGraphAdapter                         │  │
│  │  (HashMap in-memory graph)                         │  │
│  └────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────┘
```

### 2.3 Communication Rust ↔ JavaScript

**Rust expose via wasm-bindgen:**
```rust
#[wasm_bindgen]
pub struct HoddorCore {
    graph: SimpleGraphAdapter,
    // ...
}

#[wasm_bindgen]
impl HoddorCore {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { ... }

    pub async fn vector_search(
        &self,
        vault_id: String,
        embedding: Vec<f32>,
        top_k: usize,
    ) -> Result<JsValue, JsValue> { ... }

    pub async fn create_memory(
        &self,
        vault_id: String,
        content: String,
        embedding: Vec<f32>,
    ) -> Result<String, JsValue> { ... }
}
```

**TypeScript utilise:**
```typescript
import init, { HoddorCore } from './pkg/hoddor';

await init();
const core = new HoddorCore();

// Vector search
const results = await core.vector_search(
  "my_vault",
  embedding, // Float32Array
  5
);
```

---

## 3. Composants à Implémenter

### 3.1 Composant 1: WebLLMService (TypeScript)

**Fichier:** `src/services/llm/webllm_service.ts`

**Responsabilités:**
- Charger et gérer les modèles WebLLM
- Fournir interface chat compatible OpenAI
- Gérer le streaming de tokens
- Cache du modèle en mémoire

**API Publique:**
```typescript
export class WebLLMService {
  // Initialization
  async init(modelId: string, onProgress?: (progress: number) => void): Promise<void>;

  // Chat completion
  async chat(messages: ChatMessage[], options?: ChatOptions): Promise<string>;

  // Streaming chat
  async chatStream(
    messages: ChatMessage[],
    onToken: (token: string) => void,
    options?: ChatOptions
  ): Promise<void>;

  // Model management
  isReady(): boolean;
  getModelInfo(): ModelInfo;
  async unload(): Promise<void>;

  // Supported models
  static getAvailableModels(): ModelDescriptor[];
}

interface ChatMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

interface ChatOptions {
  temperature?: number;     // 0.0 - 2.0, default 0.7
  max_tokens?: number;      // default 512
  top_p?: number;           // default 0.9
  frequency_penalty?: number;
  presence_penalty?: number;
}
```

**Implémentation:**
```typescript
import * as webllm from "@mlc-ai/web-llm";

export class WebLLMService {
  private engine: webllm.MLCEngine | null = null;
  private currentModel: string | null = null;

  async init(modelId: string, onProgress?: (progress: number) => void): Promise<void> {
    // Create engine with progress callback
    const engineConfig: webllm.EngineConfig = {
      initProgressCallback: (report) => {
        onProgress?.(report.progress);
      },
    };

    this.engine = await webllm.CreateMLCEngine(modelId, engineConfig);
    this.currentModel = modelId;
  }

  async chat(messages: ChatMessage[], options?: ChatOptions): Promise<string> {
    if (!this.engine) throw new Error("Model not loaded");

    const completion = await this.engine.chat.completions.create({
      messages,
      temperature: options?.temperature ?? 0.7,
      max_tokens: options?.max_tokens ?? 512,
      top_p: options?.top_p ?? 0.9,
      stream: false,
    });

    return completion.choices[0].message.content;
  }

  async chatStream(
    messages: ChatMessage[],
    onToken: (token: string) => void,
    options?: ChatOptions
  ): Promise<void> {
    if (!this.engine) throw new Error("Model not loaded");

    const chunks = await this.engine.chat.completions.create({
      messages,
      temperature: options?.temperature ?? 0.7,
      max_tokens: options?.max_tokens ?? 512,
      stream: true,
    });

    for await (const chunk of chunks) {
      const delta = chunk.choices[0]?.delta?.content;
      if (delta) onToken(delta);
    }
  }

  isReady(): boolean {
    return this.engine !== null;
  }

  getModelInfo(): ModelInfo {
    if (!this.engine || !this.currentModel) {
      throw new Error("No model loaded");
    }
    return {
      modelId: this.currentModel,
      // Add more info from engine if available
    };
  }

  static getAvailableModels(): ModelDescriptor[] {
    return [
      {
        id: "Phi-3.5-mini-instruct-q4f16_1-MLC",
        name: "Phi-3.5 Mini (2GB)",
        size: "2GB",
        description: "Fast, suitable for most tasks",
      },
      {
        id: "Llama-3.2-3B-Instruct-q4f16_1-MLC",
        name: "Llama 3.2 3B (2GB)",
        size: "2GB",
        description: "Balanced quality and speed",
      },
      {
        id: "Mistral-7B-Instruct-v0.3-q4f16_1-MLC",
        name: "Mistral 7B (4GB)",
        size: "4GB",
        description: "Best quality, slower",
      },
    ];
  }
}
```

**Modèles Recommandés (Phase 1):**
- **PoC:** `Phi-3.5-mini-instruct-q4f16_1-MLC` (2GB, rapide)
- **Production:** Configurable par utilisateur

---

### 3.2 Composant 2: EmbeddingService (TypeScript)

**Fichier:** `src/services/embeddings/embedding_service.ts`

**Responsabilités:**
- Générer embeddings via Transformers.js
- Cache des embeddings fréquents
- Support batch processing

**API Publique:**
```typescript
export class EmbeddingService {
  async init(modelId?: string): Promise<void>;
  async embed(text: string): Promise<number[]>;
  async embedBatch(texts: string[]): Promise<number[][]>;
  getDimension(): number;
  clearCache(): void;
}
```

**Implémentation:**
```typescript
import { pipeline, env } from '@xenova/transformers';

// Configure to use local cache (OPFS or IndexedDB)
env.allowLocalModels = true;
env.allowRemoteModels = true;

export class EmbeddingService {
  private extractor: any = null;
  private readonly dimension = 384; // all-MiniLM-L6-v2
  private cache = new Map<string, number[]>();
  private readonly maxCacheSize = 1000;

  async init(modelId: string = 'Xenova/all-MiniLM-L6-v2'): Promise<void> {
    // Load model (cached automatically by Transformers.js)
    this.extractor = await pipeline('feature-extraction', modelId);
  }

  async embed(text: string): Promise<number[]> {
    // Check cache
    if (this.cache.has(text)) {
      return this.cache.get(text)!;
    }

    if (!this.extractor) {
      throw new Error("Embedding model not initialized");
    }

    // Generate embedding
    const output = await this.extractor(text, {
      pooling: 'mean',
      normalize: true,
    });

    // Extract Float32Array and convert to number[]
    const embedding = Array.from(output.data);

    // Update cache (LRU-like)
    if (this.cache.size >= this.maxCacheSize) {
      const firstKey = this.cache.keys().next().value;
      this.cache.delete(firstKey);
    }
    this.cache.set(text, embedding);

    return embedding;
  }

  async embedBatch(texts: string[]): Promise<number[][]> {
    // Process in parallel (Transformers.js handles batching internally)
    return Promise.all(texts.map(text => this.embed(text)));
  }

  getDimension(): number {
    return this.dimension;
  }

  clearCache(): void {
    this.cache.clear();
  }
}
```

---

### 3.3 Composant 3: RAGOrchestrator (TypeScript)

**Fichier:** `src/services/rag/rag_orchestrator.ts`

**Responsabilités:**
- Orchestrer le flux RAG complet
- Coordonner graph, embeddings, et LLM
- Construire prompts contextuels
- Gérer le streaming de réponses

**API Publique:**
```typescript
export class RAGOrchestrator {
  constructor(
    private core: HoddorCore,
    private llm: WebLLMService,
    private embeddings: EmbeddingService
  ) {}

  async ask(
    vaultId: string,
    question: string,
    options?: RAGOptions
  ): Promise<RAGResponse>;

  async askStream(
    vaultId: string,
    question: string,
    onToken: (token: string) => void,
    options?: RAGOptions
  ): Promise<RAGResponse>;

  async addMemory(
    vaultId: string,
    content: string,
    type?: string,
    labels?: string[]
  ): Promise<string>;
}

interface RAGOptions {
  topK?: number;                   // Default: 5
  similarityThreshold?: number;     // Default: 0.7
  includeRelations?: boolean;       // Default: true
  maxContextTokens?: number;        // Default: 2000
  systemPrompt?: string;
}

interface RAGResponse {
  answer: string;
  sources: SourceNode[];
  metadata: {
    searchTime: number;
    inferenceTime: number;
    tokensUsed: number;
  };
}
```

**Implémentation:**
```typescript
export class RAGOrchestrator {
  constructor(
    private core: HoddorCore,
    private llm: WebLLMService,
    private embeddings: EmbeddingService
  ) {}

  async ask(
    vaultId: string,
    question: string,
    options?: RAGOptions
  ): Promise<RAGResponse> {
    const startTime = performance.now();

    // 1. Generate question embedding
    const questionEmbedding = await this.embeddings.embed(question);

    // 2. Search graph for relevant context
    const searchStart = performance.now();
    const searchResults = await this.core.vector_search(
      vaultId,
      questionEmbedding,
      options?.topK ?? 5
    );
    const searchTime = performance.now() - searchStart;

    // Parse results from WASM (JsValue → TypeScript objects)
    const results = JSON.parse(searchResults as string) as SearchResult[];

    // Filter by similarity threshold
    const relevantResults = results.filter(
      r => r.similarity >= (options?.similarityThreshold ?? 0.7)
    );

    // 3. Get relations if requested
    let relations: Edge[] = [];
    if (options?.includeRelations) {
      // For each result, get neighbors
      for (const result of relevantResults) {
        const neighbors = await this.core.get_neighbors(
          vaultId,
          result.node.id
        );
        // Merge relations...
      }
    }

    // 4. Build context for LLM
    const context = this.buildContext(relevantResults, relations, options?.maxContextTokens);

    // 5. Build prompt
    const messages = this.buildPrompt(question, context, options?.systemPrompt);

    // 6. Call LLM
    const inferenceStart = performance.now();
    const answer = await this.llm.chat(messages);
    const inferenceTime = performance.now() - inferenceStart;

    // 7. Save interaction to graph
    await this.saveInteraction(vaultId, question, answer, relevantResults);

    return {
      answer,
      sources: relevantResults.map(r => r.node),
      metadata: {
        searchTime,
        inferenceTime,
        tokensUsed: this.estimateTokens(answer), // Rough estimate
      },
    };
  }

  private buildContext(
    results: SearchResult[],
    relations: Edge[],
    maxTokens?: number
  ): string {
    let context = "# Relevant Information\n\n";

    for (const result of results) {
      context += `## Memory (similarity: ${result.similarity.toFixed(2)})\n`;
      context += `${result.node.content}\n\n`;

      // Truncate if too long
      if (maxTokens && this.estimateTokens(context) > maxTokens) {
        break;
      }
    }

    if (relations.length > 0) {
      context += "## Related Concepts\n";
      // Add relations info...
    }

    return context;
  }

  private buildPrompt(
    question: string,
    context: string,
    systemPrompt?: string
  ): ChatMessage[] {
    return [
      {
        role: 'system',
        content: systemPrompt ??
          "You are a helpful assistant. Use the provided context to answer questions accurately.",
      },
      {
        role: 'user',
        content: `Context:\n${context}\n\nQuestion: ${question}`,
      },
    ];
  }

  private async saveInteraction(
    vaultId: string,
    question: string,
    answer: string,
    sources: SearchResult[]
  ): Promise<void> {
    // Create conversation node
    const conversationData = JSON.stringify({
      question,
      answer,
      sourceIds: sources.map(s => s.node.id),
      timestamp: Date.now(),
    });

    const embedding = await this.embeddings.embed(question);

    await this.core.create_memory(
      vaultId,
      conversationData,
      embedding,
      ['conversation'],
      'conversation'
    );

    // Create edges to source nodes
    // ... (create_edge calls)
  }

  private estimateTokens(text: string): number {
    // Rough estimate: 1 token ≈ 4 characters
    return Math.ceil(text.length / 4);
  }

  async addMemory(
    vaultId: string,
    content: string,
    type: string = 'memory',
    labels: string[] = []
  ): Promise<string> {
    // Generate embedding
    const embedding = await this.embeddings.embed(content);

    // Save to graph (via WASM)
    const nodeId = await this.core.create_memory(
      vaultId,
      content,
      embedding,
      labels,
      type
    );

    return nodeId;
  }
}
```

---

### 3.4 Composant 4: WASM Bindings (Rust)

**Fichier:** `src/lib.rs` (ajouts)

**Nouvelles fonctions wasm-bindgen:**

```rust
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};

#[wasm_bindgen]
pub struct HoddorCore {
    graph: SimpleGraphAdapter,
    storage: OpfsStorage,
    // persistence: GraphPersistence<...>,
}

#[wasm_bindgen]
impl HoddorCore {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        // Initialize with SimpleGraphAdapter
        Self {
            graph: SimpleGraphAdapter::new(),
            storage: OpfsStorage::new(),
        }
    }

    /// Search nodes by vector similarity
    #[wasm_bindgen]
    pub async fn vector_search(
        &self,
        vault_id: String,
        embedding: Vec<f32>,
        top_k: usize,
    ) -> Result<JsValue, JsValue> {
        // 1. Get all nodes for the vault
        let all_nodes = self.graph.list_nodes_by_type(&vault_id, "memory", None).await
            .map_err(|e| JsValue::from_str(&format!("Search failed: {}", e)))?;

        // 2. Calculate cosine similarity for each node
        let mut results: Vec<(GraphNode, f64)> = all_nodes
            .into_iter()
            .filter_map(|node| {
                node.embedding.as_ref().map(|node_emb| {
                    let similarity = cosine_similarity(&embedding, node_emb);
                    (node, similarity)
                })
            })
            .collect();

        // 3. Sort by similarity (descending)
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 4. Take top K
        results.truncate(top_k);

        // 5. Convert to JSON
        let search_results: Vec<SearchResultJson> = results
            .into_iter()
            .map(|(node, similarity)| SearchResultJson {
                node: NodeJson::from_node(&node),
                similarity,
            })
            .collect();

        let json = serde_json::to_string(&search_results)
            .map_err(|e| JsValue::from_str(&format!("Serialization failed: {}", e)))?;

        Ok(JsValue::from_str(&json))
    }

    /// Create a memory node with embedding
    #[wasm_bindgen]
    pub async fn create_memory(
        &self,
        vault_id: String,
        content: String,
        embedding: Vec<f32>,
        labels: Vec<String>,
        node_type: String,
    ) -> Result<String, JsValue> {
        // Encrypt content
        let encrypted_content = content.as_bytes().to_vec(); // TODO: actual encryption

        // Generate HMAC
        let content_hmac = "placeholder_hmac".to_string(); // TODO: actual HMAC

        // Create node
        let node_id = self.graph
            .create_node(
                &vault_id,
                &node_type,
                encrypted_content,
                content_hmac,
                labels,
                Some(embedding),
                None,
            )
            .await
            .map_err(|e| JsValue::from_str(&format!("Create node failed: {}", e)))?;

        Ok(node_id.as_str().to_string())
    }

    /// Get neighbors of a node
    #[wasm_bindgen]
    pub async fn get_neighbors(
        &self,
        vault_id: String,
        node_id: String,
    ) -> Result<JsValue, JsValue> {
        let node_id_parsed = NodeId::from_string(&node_id)
            .map_err(|e| JsValue::from_str(&format!("Invalid node ID: {}", e)))?;

        let neighbors = self.graph
            .get_neighbors(&vault_id, &node_id_parsed, None)
            .await
            .map_err(|e| JsValue::from_str(&format!("Get neighbors failed: {}", e)))?;

        let nodes_json: Vec<NodeJson> = neighbors
            .into_iter()
            .map(|n| NodeJson::from_node(&n))
            .collect();

        let json = serde_json::to_string(&nodes_json)
            .map_err(|e| JsValue::from_str(&format!("Serialization failed: {}", e)))?;

        Ok(JsValue::from_str(&json))
    }
}

// Helper: cosine similarity
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    (dot_product / (magnitude_a * magnitude_b)) as f64
}

// JSON serialization structs
#[derive(Serialize, Deserialize)]
struct SearchResultJson {
    node: NodeJson,
    similarity: f64,
}

#[derive(Serialize, Deserialize)]
struct NodeJson {
    id: String,
    content: String, // Decrypted
    node_type: String,
    labels: Vec<String>,
    created_at: u64,
}

impl NodeJson {
    fn from_node(node: &GraphNode) -> Self {
        // Decrypt content here
        let content = String::from_utf8_lossy(&node.encrypted_content).to_string();

        Self {
            id: node.id.as_str().to_string(),
            content,
            node_type: node.node_type.clone(),
            labels: node.labels.clone(),
            created_at: node.created_at,
        }
    }
}
```

---

## 4. Interfaces et API

### 4.1 API Complète Exposée au Frontend

```typescript
// Main entry point
export class HoddorRAG {
  private core: HoddorCore;
  private llm: WebLLMService;
  private embeddings: EmbeddingService;
  private rag: RAGOrchestrator;

  static async init(options?: InitOptions): Promise<HoddorRAG> {
    // 1. Initialize WASM
    await initWasm();
    const core = new HoddorCore();

    // 2. Initialize LLM
    const llm = new WebLLMService();
    await llm.init(
      options?.modelId ?? "Phi-3.5-mini-instruct-q4f16_1-MLC",
      options?.onModelProgress
    );

    // 3. Initialize Embeddings
    const embeddings = new EmbeddingService();
    await embeddings.init();

    // 4. Create orchestrator
    const rag = new RAGOrchestrator(core, llm, embeddings);

    return new HoddorRAG(core, llm, embeddings, rag);
  }

  // Chat interface
  async ask(vaultId: string, question: string, options?: RAGOptions): Promise<RAGResponse> {
    return this.rag.ask(vaultId, question, options);
  }

  async askStream(
    vaultId: string,
    question: string,
    onToken: (token: string) => void,
    options?: RAGOptions
  ): Promise<RAGResponse> {
    return this.rag.askStream(vaultId, question, onToken, options);
  }

  // Memory management
  async addMemory(
    vaultId: string,
    content: string,
    type?: string,
    labels?: string[]
  ): Promise<string> {
    return this.rag.addMemory(vaultId, content, type, labels);
  }

  // Model management
  async switchModel(modelId: string, onProgress?: (progress: number) => void): Promise<void> {
    await this.llm.unload();
    await this.llm.init(modelId, onProgress);
  }

  getAvailableModels(): ModelDescriptor[] {
    return WebLLMService.getAvailableModels();
  }

  isReady(): boolean {
    return this.llm.isReady();
  }
}
```

### 4.2 Exemple d'Utilisation

```typescript
// Initialize
const hoddor = await HoddorRAG.init({
  modelId: "Phi-3.5-mini-instruct-q4f16_1-MLC",
  onModelProgress: (progress) => {
    console.log(`Loading model: ${Math.round(progress * 100)}%`);
  },
});

// Add some memories
await hoddor.addMemory(
  "my_vault",
  "I love black coffee with no sugar",
  "preference",
  ["coffee", "preferences"]
);

await hoddor.addMemory(
  "my_vault",
  "I drink coffee every morning at 8am",
  "habit",
  ["coffee", "routine"]
);

// Ask a question (with streaming)
const response = await hoddor.askStream(
  "my_vault",
  "How do I like my coffee?",
  (token) => {
    process.stdout.write(token); // Stream tokens
  }
);

console.log("\n\nSources:", response.sources);
console.log("Metadata:", response.metadata);
```

---

## 5. Flux d'Intégration

### 5.1 Flux RAG Détaillé

```
┌─────────────────────────────────────────────────────────┐
│ USER: "How do I like my coffee?"                        │
└─────────────────────────────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────────┐
│ [1] EmbeddingService.embed(question)                    │
│     → Transformers.js (all-MiniLM-L6-v2)                │
│     → [0.23, -0.45, 0.12, ...] (384 dims)               │
│     ⏱ ~20ms                                              │
└─────────────────────────────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────────┐
│ [2] HoddorCore.vector_search(embedding, top_k=5)        │
│     → Rust WASM                                          │
│     → SimpleGraphAdapter.list_nodes()                    │
│     → cosine_similarity() for each node                 │
│     → Sort & return top 5                                │
│     ⏱ ~50ms                                              │
└─────────────────────────────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────────┐
│ [3] Results (from WASM → JS):                           │
│     [                                                    │
│       {                                                  │
│         node: {                                          │
│           id: "uuid-1",                                  │
│           content: "I love black coffee no sugar",      │
│           similarity: 0.92                              │
│         }                                                │
│       },                                                 │
│       {                                                  │
│         node: {                                          │
│           id: "uuid-2",                                  │
│           content: "I drink coffee every morning",      │
│           similarity: 0.85                              │
│         }                                                │
│       }                                                  │
│     ]                                                    │
└─────────────────────────────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────────┐
│ [4] RAGOrchestrator.buildContext()                      │
│     → Combine results into context string               │
│     → "# Relevant Information\n\n                       │
│        ## Memory 1 (0.92)\n                             │
│        I love black coffee with no sugar\n\n            │
│        ## Memory 2 (0.85)\n                             │
│        I drink coffee every morning"                    │
└─────────────────────────────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────────┐
│ [5] RAGOrchestrator.buildPrompt()                       │
│     → messages = [                                       │
│         {                                                │
│           role: "system",                                │
│           content: "You are a helpful assistant..."     │
│         },                                               │
│         {                                                │
│           role: "user",                                  │
│           content: "Context:\n[...]\n\nQuestion: ..."   │
│         }                                                │
│       ]                                                  │
└─────────────────────────────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────────┐
│ [6] WebLLMService.chat(messages)                        │
│     → @mlc-ai/web-llm                                   │
│     → Model: Phi-3.5-mini (WebGPU)                      │
│     → Generate response                                 │
│     ⏱ ~1-2s                                              │
└─────────────────────────────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────────┐
│ [7] RESPONSE:                                            │
│     "Based on your preferences, you like your coffee    │
│      black with no sugar. You typically drink it in     │
│      the morning around 8am."                           │
└─────────────────────────────────────────────────────────┘
                       ↓
┌─────────────────────────────────────────────────────────┐
│ [8] RAGOrchestrator.saveInteraction()                   │
│     → Create conversation node                          │
│     → Create edges to source nodes                      │
│     → Store in graph for future context                │
└─────────────────────────────────────────────────────────┘
```

---

## 6. Plan d'Implémentation

### 6.1 Phase 1: Foundation (Semaine 1)

**Objectif:** WebLLM fonctionne en standalone (sans graph)

**Tâches:**
- [ ] Setup npm dependencies (`@mlc-ai/web-llm`, `@xenova/transformers`)
- [ ] Implémenter `WebLLMService` (load model, chat basique)
- [ ] Implémenter `EmbeddingService` (embed basique)
- [ ] Tests unitaires services
- [ ] UI simple : textarea + button → LLM response
- [ ] Mesurer performance (load time, inference time)

**Délivrables:**
- ✅ LLM répond à des questions simples
- ✅ Embeddings générés correctement
- ✅ Benchmark de base documenté

**Critères de succès:**
- Model charge en < 10s (from cache)
- Inference < 2s pour 100 tokens (WebGPU)
- UI responsive

---

### 6.2 Phase 2: WASM Integration (Semaine 2)

**Objectif:** Graph + LLM communiquent

**Tâches:**
- [ ] Ajouter wasm-bindgen bindings (`vector_search`, `create_memory`)
- [ ] Implémenter `cosine_similarity` en Rust
- [ ] Tester WASM ↔ JS communication
- [ ] Implémenter `RAGOrchestrator` (v1 basique)
- [ ] Tests d'intégration Rust ↔ TypeScript
- [ ] UI : Afficher sources utilisées

**Délivrables:**
- ✅ Graph search retourne résultats pertinents
- ✅ RAG pipeline connecté end-to-end
- ✅ Tests passent

**Critères de succès:**
- Vector search < 100ms (10K nodes)
- Sources affichées correctement dans UI
- Pas de memory leaks (vérifier avec DevTools)

---

### 6.3 Phase 3: RAG Complete (Semaine 3)

**Objectif:** RAG fonctionnel avec contexte enrichi

**Tâches:**
- [ ] Implémenter `get_neighbors` (relations)
- [ ] Améliorer prompt building (contexte structuré)
- [ ] Implémenter streaming (chatStream)
- [ ] Sauvegarder conversations dans graph
- [ ] Créer edges vers sources
- [ ] Optimiser: cache embeddings
- [ ] UI: Streaming de tokens, afficher graph des sources
- [ ] Documentation API complète

**Délivrables:**
- ✅ RAG complet avec relations
- ✅ Streaming fonctionne
- ✅ Conversations sauvegardées
- ✅ Démo impressive

**Critères de succès:**
- RAG query < 3s total
- Context includes relations (neighbors)
- UI affiche sources + graph
- Documentation complète

---

### 6.4 Phase 4: Polish & Optimization (Semaine 4)

**Tâches:**
- [ ] Optimiser vector search (caching, indexing)
- [ ] Implémenter LRU cache (embeddings, queries)
- [ ] Model switching UI
- [ ] Progress indicators (model loading)
- [ ] Error handling robuste
- [ ] Tests E2E complets
- [ ] Performance benchmarks
- [ ] Documentation utilisateur

**Délivrables:**
- ✅ Production-ready
- ✅ Multi-model support
- ✅ Optimisé (< 1.5s RAG query)
- ✅ Docs complètes

---

## 7. Tests et Validation

### 7.1 Tests Unitaires (Rust)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.01);

        let c = vec![1.0, 0.0, 0.0];
        let d = vec![0.0, 1.0, 0.0];
        let sim2 = cosine_similarity(&c, &d);
        assert!((sim2 - 0.0).abs() < 0.01);
    }

    #[wasm_bindgen_test]
    async fn test_vector_search_wasm() {
        let core = HoddorCore::new();

        // Create test nodes with embeddings
        let emb1 = vec![1.0, 0.0, 0.0];
        core.create_memory(
            "test_vault".to_string(),
            "content1".to_string(),
            emb1.clone(),
            vec![],
            "memory".to_string(),
        ).await.unwrap();

        // Search
        let results = core.vector_search(
            "test_vault".to_string(),
            emb1,
            5,
        ).await.unwrap();

        // Verify
        let parsed: Vec<SearchResultJson> = serde_json::from_str(&results.as_string().unwrap()).unwrap();
        assert_eq!(parsed.len(), 1);
        assert!(parsed[0].similarity > 0.99);
    }
}
```

### 7.2 Tests d'Intégration (TypeScript)

```typescript
describe('RAGOrchestrator', () => {
  let hoddor: HoddorRAG;

  beforeAll(async () => {
    hoddor = await HoddorRAG.init({
      modelId: "Phi-3.5-mini-instruct-q4f16_1-MLC",
    });
  });

  test('should add memory and retrieve it', async () => {
    const nodeId = await hoddor.addMemory(
      'test_vault',
      'I love TypeScript',
      'preference'
    );

    expect(nodeId).toBeTruthy();

    const response = await hoddor.ask(
      'test_vault',
      'What programming language do I like?'
    );

    expect(response.answer).toContain('TypeScript');
    expect(response.sources.length).toBeGreaterThan(0);
  });

  test('should stream tokens', async () => {
    const tokens: string[] = [];
    await hoddor.askStream(
      'test_vault',
      'Tell me about TypeScript',
      (token) => tokens.push(token)
    );

    expect(tokens.length).toBeGreaterThan(0);
  });
});
```

### 7.3 Tests End-to-End (Playwright)

```typescript
test('RAG workflow', async ({ page }) => {
  await page.goto('http://localhost:3000');

  // Wait for model to load
  await page.waitForSelector('.model-ready');

  // Add memory
  await page.fill('#memory-input', 'I am a software engineer');
  await page.click('#add-memory');

  // Ask question
  await page.fill('#question-input', 'What is my profession?');
  await page.click('#ask-button');

  // Check response
  const answer = await page.waitForSelector('.answer');
  expect(await answer.textContent()).toContain('software engineer');

  // Check sources displayed
  const sources = await page.$$('.source-node');
  expect(sources.length).toBeGreaterThan(0);
});
```

---

## 8. Considérations de Performance

### 8.1 Targets de Performance

| Métrique | Phase 1 (PoC) | Phase 4 (Optimisé) |
|----------|---------------|---------------------|
| **Model Loading** | < 10s | < 5s |
| **Embedding (single)** | < 50ms | < 20ms |
| **Vector Search (10K nodes)** | < 100ms | < 50ms |
| **LLM Inference (100 tok)** | < 2s | < 1s |
| **RAG Complete** | < 3s | < 1.5s |
| **Memory Usage** | < 4GB | < 3GB |

### 8.2 Optimisations Prévues

**Phase 1-2 (Baseline):**
- Implémentation simple
- Pas d'optimisation prématurée

**Phase 3-4 (Optimizations):**
- LRU cache pour embeddings
- Batch embedding generation
- Web Worker pour isolation
- IndexedDB cache pour models
- Quantized models (Q4 vs Q8)

### 8.3 Memory Management

```typescript
// Example: Proper cleanup
class ResourceManager {
  async cleanup() {
    // Unload LLM
    await this.llm.unload();

    // Clear caches
    this.embeddings.clearCache();

    // Clear WASM memory (if needed)
    this.core.free();
  }
}
```

---

## 9. Annexes

### 9.1 Dépendances NPM

```json
{
  "dependencies": {
    "@mlc-ai/web-llm": "^0.2.79",
    "@xenova/transformers": "^2.17.0"
  },
  "devDependencies": {
    "wasm-pack": "^0.12.0",
    "@types/node": "^20.0.0",
    "vitest": "^1.0.0",
    "playwright": "^1.40.0"
  }
}
```

### 9.2 Configuration Rust

```toml
[dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
js-sys = "0.3"

[lib]
crate-type = ["cdylib", "rlib"]
```

### 9.3 Modèles WebLLM Recommandés

| Model ID | Size | Speed | Quality | Use Case |
|----------|------|-------|---------|----------|
| `Phi-3.5-mini-instruct-q4f16_1-MLC` | 2GB | Fast | Good | PoC, Dev |
| `Llama-3.2-3B-Instruct-q4f16_1-MLC` | 2GB | Medium | Better | Production |
| `Mistral-7B-Instruct-v0.3-q4f16_1-MLC` | 4GB | Slow | Best | Quality-focused |

**Recommandation:** Commencer avec Phi-3.5, offrir Llama 3.2 en production.

---

## 10. Next Steps

**Immédiat (après validation de cette spec):**
1. ✅ Validation de l'architecture par l'équipe
2. Setup du projet TypeScript (`src/services/`)
3. Installation des dépendances NPM
4. Implémentation Phase 1 (Week 1)

**Questions Ouvertes:**
- Quel UI framework utiliser ? (React, Vue, Vanilla ?)
- Faut-il un worker séparé pour WebLLM ?
- Quelle stratégie de cache pour les modèles ?

---

**FIN DE LA SPÉCIFICATION D'IMPLÉMENTATION**

*Document prêt pour implémentation. Pour questions : créer une issue sur GitHub.*
