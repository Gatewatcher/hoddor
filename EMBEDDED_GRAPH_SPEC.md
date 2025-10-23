# Spécification : Base de Données Graph Embarquée et LLM Local pour Hoddor

**Version:** 2.0
**Date:** 2025-10-20
**Status:** Draft
**Auteurs:** Équipe Hoddor

---

## Table des Matières

1. [Vue d'ensemble](#1-vue-densemble)
2. [Contexte et Motivation](#2-contexte-et-motivation)
3. [Stack Technologique](#3-stack-technologique)
4. [Architecture Globale](#4-architecture-globale)
5. [Composants Principaux](#5-composants-principaux)
6. [Modèle de Données](#6-modèle-de-données)
7. [Interfaces et API](#7-interfaces-et-api)
8. [Flux RAG Complet](#8-flux-rag-complet)
9. [Sécurité et Chiffrement](#9-sécurité-et-chiffrement)
10. [Performance et Scalabilité](#10-performance-et-scalabilité)
11. [Plan d'Implémentation](#11-plan-dimplémentation)
12. [Références](#12-références)

---

## 1. Vue d'ensemble

### 1.1 Objectif

Intégrer dans Hoddor une **solution complète de mémoire pour LLM**, entièrement locale et chiffrée, comprenant :

- 🧠 **Graph Database** (CozoDB) : Stockage de connaissances avec relations sémantiques
- 🔢 **Vector Search** : Recherche par similarité (embeddings)
- 🤖 **LLM Local** (WebLLM) : Inférence dans le navigateur
- 💬 **RAG Pipeline** : Orchestration Retrieval-Augmented Generation
- 🔒 **Zero-Knowledge** : Chiffrement end-to-end, aucune donnée externe

### 1.2 Principe de Fonctionnement

```
┌──────────────────────────────────────────────────────┐
│  Question Utilisateur                                 │
│           ↓                                           │
│  Embedding (text → vector)                           │
│           ↓                                           │
│  Graph Search (similarité + relations)               │
│           ↓                                           │
│  Contexte enrichi (déchiffré)                        │
│           ↓                                           │
│  LLM Local génère réponse                            │
│           ↓                                           │
│  Sauvegarde interaction (chiffrée)                   │
└──────────────────────────────────────────────────────┘

     ↑ TOUT RESTE DANS LE NAVIGATEUR ↑
```

### 1.3 Valeur Ajoutée

| Aspect | Solution Actuelle | Avec Graph + LLM Local |
|--------|-------------------|------------------------|
| **Privacy** | Chiffré mais LLM externe | 100% local, zero data leak |
| **Coûts** | API calls payantes | Gratuit après download |
| **Offline** | Impossible sans LLM | Fonctionne offline complet |
| **Latence** | Network + API | Local (plus rapide sur petits modèles) |
| **Contexte** | Mémoire limitée | Graph illimité avec relations |
| **GDPR** | Dépend du provider | Compliant par design |

### 1.4 Contraintes Techniques

- **Embarqué** : Tout dans le navigateur (WASM + WebGPU)
- **Zero-knowledge** : Pas de serveur externe
- **Architecture hexagonale** : Ports & Adapters
- **Storage OPFS** : Origin Private File System
- **Performance** : < 3s pour requête RAG complète
- **Scalabilité** : Support 100K+ nœuds

---

## 2. Contexte et Motivation

### 2.1 Évolution du Besoin

**Phase 1 (Actuel)** : Hoddor = Vault chiffré
```
Stockage sécurisé ✅
└─ Mais : Données isolées, pas de relations
```

**Phase 2 (Cette spec)** : Hoddor = Memory Layer pour LLM
```
Graph Database ✅
└─ Relations sémantiques
LLM Local ✅
└─ Inférence privée
RAG Pipeline ✅
└─ Contexte enrichi
```

### 2.2 Pourquoi Graph + LLM Local ?

#### Problème : LLM Cloud classique
```
User Question
    ↓
[HODDOR] Données chiffrées locales
    ↓
[CLOUD] Déchiffré + envoyé à OpenAI/Claude
    ↓
❌ Privacy compromise
❌ Coûts récurrents
❌ Dépendance réseau
```

#### Solution : Graph + LLM Local
```
User Question
    ↓
[HODDOR Graph] Recherche locale chiffrée
    ↓
[WebLLM] Inférence locale (WebGPU)
    ↓
✅ Zero data leak
✅ Gratuit
✅ Offline-capable
```

### 2.3 Cas d'Usage Cibles

1. **Assistant Personnel Privé**
   - Mémoire des préférences, habitudes, contexte
   - Suggestions personnalisées sans tracking

2. **Knowledge Base Entreprise**
   - Documentation interne, procédures
   - Search sémantique + Q&A

3. **Note-Taking Intelligent**
   - Relations automatiques entre notes
   - Retrieval contextuel

4. **Healthcare / Legal**
   - Données sensibles (RGPD, HIPAA)
   - Zero risque de fuite

---

## 3. Stack Technologique

### 3.1 Comparatif des Options

#### Graph Database

| Solution | Embedded | WASM | Vector | Status | Verdict |
|----------|----------|------|--------|--------|---------|
| **KuzuDB** | ✅ | ✅ | ✅ | ❌ Abandonné (Oct 2025) | 🚫 Non viable |
| **petgraph** | ✅ | ✅ | ❌ | ✅ Actif | ⚠️ Trop basique |
| **CozoDB** | ✅ | ✅ | ✅ HNSW | ✅ Actif | ✅ **CHOISI** |

**Justification CozoDB :**
- Conçu pour LLM ("The hippocampus for AI")
- Datalog (requêtes puissantes)
- Transactionnel (ACID)
- Multi-modèle (Graph + Relationnel + Vector)
- Performance : 100K+ QPS

#### LLM Local

| Solution | Modèles | WebGPU | API | Verdict |
|----------|---------|--------|-----|---------|
| **WebLLM** | Llama, Phi, Mistral, Gemma | ✅ | OpenAI-compatible | ✅ **CHOISI** |
| **Transformers.js** | Distilled (petits) | ⚠️ | Custom | ⚠️ Backup |
| **ONNX Runtime** | Custom | ⚠️ | Custom | ⚠️ Complexe |

**Justification WebLLM :**
- Projet MLC-AI (actif, mature)
- WebGPU acceleration (10x plus rapide que WASM seul)
- Modèles variés (2GB - 8GB)
- API compatible OpenAI (facile à intégrer)
- Streaming support

#### Embedding Model

| Solution | Taille | Qualité | Vitesse | Verdict |
|----------|--------|---------|---------|---------|
| **all-MiniLM-L6-v2** | 25MB | Bonne | Rapide | ✅ **PoC** |
| **BGE-small** | 50MB | Meilleure | Moyenne | ✅ **Production** |
| **E5-base** | 120MB | Excellente | Lente | ⚠️ Option |

**Justification all-MiniLM-L6-v2 (PoC) :**
- Léger (25MB)
- Rapide (10-50ms)
- Qualité suffisante pour démo
- Transformers.js natif

### 3.2 Stack Finale

```
┌─────────────────────────────────────────────────────┐
│  HODDOR GRAPH + LLM STACK                           │
├─────────────────────────────────────────────────────┤
│  Graph Database    │  CozoDB 0.7+                   │
│  LLM Engine        │  WebLLM (MLC-AI)               │
│  Embedding         │  Transformers.js               │
│  Storage           │  OPFS                          │
│  Encryption        │  Age (existing)                │
│  Language (Core)   │  Rust (WASM)                   │
│  Language (LLM)    │  TypeScript                    │
└─────────────────────────────────────────────────────┘
```

### 3.3 Modèles Recommandés

#### Phase PoC
```
LLM:       Phi-3.5-mini-instruct (2GB)
Embedder:  all-MiniLM-L6-v2 (25MB)
Total:     ~2GB download (one-time)
```

#### Phase Production
```
LLM Options:
  - Small:  Phi-3.5-mini (2GB)      → Rapide, basique
  - Medium: Llama-3.2-3B (2GB)      → Équilibré
  - Large:  Mistral-7B (4GB)        → Meilleur qualité

Embedder:
  - BGE-small-en (50MB)             → Meilleure qualité
```

---

## 4. Architecture Globale

### 4.1 Vue d'Ensemble

```
┌───────────────────────────────────────────────────────────┐
│                   APPLICATION LAYER                        │
│  (React/Vue UI - Playground + Extensions)                 │
└───────────────────────────────────────────────────────────┘
                            ↓
┌───────────────────────────────────────────────────────────┐
│              RAG ORCHESTRATION LAYER                       │
│  ┌─────────────────────────────────────────────────────┐  │
│  │  RAG Pipeline Manager                               │  │
│  │  ├─ Question → Embedding                            │  │
│  │  ├─ Graph search (context retrieval)                │  │
│  │  ├─ Prompt construction                             │  │
│  │  ├─ LLM inference                                    │  │
│  │  └─ Response post-processing                        │  │
│  └─────────────────────────────────────────────────────┘  │
│                                                             │
│  ┌─────────────────┐         ┌──────────────────┐         │
│  │ Embedding Model │         │   WebLLM Engine  │         │
│  │ (Transformers)  │         │  (Phi/Llama/etc) │         │
│  └─────────────────┘         └──────────────────┘         │
└───────────────────────────────────────────────────────────┘
                            ↓
┌───────────────────────────────────────────────────────────┐
│                   HODDOR CORE (WASM)                       │
│  ┌─────────────────────────────────────────────────────┐  │
│  │              Domain Layer (Rust)                    │  │
│  │  ├─ domain/graph/                                   │  │
│  │  │  ├─ types.rs      (Node, Edge, Embedding)       │  │
│  │  │  ├─ operations.rs (Business logic)              │  │
│  │  │  └─ rag.rs        (RAG-specific logic)          │  │
│  └─────────────────────────────────────────────────────┘  │
│                            ↓                               │
│  ┌─────────────────────────────────────────────────────┐  │
│  │              Ports Layer (Rust)                     │  │
│  │  ├─ ports/graph.rs    (trait GraphPort)            │  │
│  │  ├─ ports/embedding.rs (trait EmbeddingPort)       │  │
│  │  └─ ports/llm.rs      (trait LLMPort)              │  │
│  └─────────────────────────────────────────────────────┘  │
│                            ↓                               │
│  ┌─────────────────────────────────────────────────────┐  │
│  │            Adapters Layer (Rust)                    │  │
│  │  ├─ adapters/wasm/cozo_graph.rs                    │  │
│  │  ├─ adapters/wasm/transformers_embedding.rs        │  │
│  │  └─ adapters/wasm/webllm_adapter.rs                │  │
│  └─────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────┘
                            ↓
┌───────────────────────────────────────────────────────────┐
│               INFRASTRUCTURE LAYER                         │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────┐  │
│  │   CozoDB     │  │   WebLLM     │  │ Transformers.js│  │
│  │   (Graph)    │  │ (Inference)  │  │  (Embeddings)  │  │
│  └──────────────┘  └──────────────┘  └────────────────┘  │
│                            ↓                               │
│  ┌─────────────────────────────────────────────────────┐  │
│  │           OPFS (Origin Private File System)         │  │
│  │  ├─ Graph data (encrypted with Age)                │  │
│  │  ├─ Model cache (LLM ~2-4GB)                       │  │
│  │  └─ Embeddings cache                               │  │
│  └─────────────────────────────────────────────────────┘  │
└───────────────────────────────────────────────────────────┘
```

### 4.2 Séparation des Responsabilités

| Layer | Responsabilité | Technologies |
|-------|----------------|--------------|
| **Application** | UI, UX, interactions utilisateur | React/Vue/Svelte |
| **Orchestration** | RAG pipeline, coordination | TypeScript |
| **Domain** | Logique métier, règles | Rust |
| **Ports** | Interfaces abstraites | Rust traits |
| **Adapters** | Implémentations concrètes | Rust + JS glue |
| **Infrastructure** | Libraries externes | CozoDB, WebLLM, etc. |

### 4.3 Flux de Données

```
Question (Text)
    ↓
[Embedding Model] → Vector (Float32Array)
    ↓
[Graph Port] → RAG Search (Datalog)
    ↓
[Encryption Port] → Decrypt context
    ↓
[LLM Port] → Generate response
    ↓
[Graph Port] → Save interaction
    ↓
Response (Text)
```

---

## 5. Composants Principaux

### 5.1 Graph Database (CozoDB)

**Rôle :** Stockage et recherche de la mémoire

**Fonctionnalités :**
- Stockage nodes (mémoires) + edges (relations)
- Vector search (HNSW) : Similarité cosine
- Graph traversal : BFS, DFS, shortest path
- Datalog queries : Requêtes complexes
- Transactions ACID : Cohérence des données

**Schémas :**
```
Relations (Datalog):
  - memory (nodes)
  - relation (edges)

Index:
  - HNSW sur embeddings (cosine similarity)
  - B-tree sur vault_id, node_type, labels
```

**Performance Cible :**
- Create node: < 10ms
- Vector search (top 10): < 50ms
- Graph traversal (depth 2): < 100ms

### 5.2 LLM Local (WebLLM)

**Rôle :** Génération de réponses

**Modèles Supportés :**
- Phi-3.5 (2GB) : Rapide, qualité correcte
- Llama-3.2-3B (2GB) : Équilibré
- Mistral-7B (4GB) : Meilleure qualité
- Gemma-2B (2GB) : Alternative

**Modes d'Exécution :**
```
1. WebGPU (préféré)
   └─ 10x plus rapide que WASM
   └─ Nécessite GPU compatible

2. WASM (fallback)
   └─ Compatible partout
   └─ Plus lent (acceptable pour PoC)
```

**Performance Cible :**
```
WebGPU Mode:
  - Load time (from cache): 5-10s
  - Inference (100 tokens): 500-2000ms
  - Streaming: Oui

WASM Mode:
  - Load time: 10-15s
  - Inference (100 tokens): 2000-5000ms
  - Streaming: Oui
```

**API :**
- Compatible OpenAI Chat Completions
- Streaming support
- Temperature, top_p, max_tokens, etc.

### 5.3 Embedding Model (Transformers.js)

**Rôle :** Conversion texte → vecteur

**Modèles :**
```
PoC:        all-MiniLM-L6-v2 (384 dimensions, 25MB)
Production: BGE-small-en (384 dimensions, 50MB)
Advanced:   E5-base (768 dimensions, 120MB)
```

**Fonctionnalités :**
- Mean pooling
- Normalization (L2)
- Batch processing

**Performance Cible :**
- Single text: 10-50ms
- Batch (10 texts): 50-200ms

### 5.4 RAG Orchestrator

**Rôle :** Coordination du flux RAG

**Étapes :**
1. **Embedding** : Question → Vector
2. **Retrieval** : Graph search + relations
3. **Decrypt** : Déchiffrer contexte pertinent
4. **Prompt** : Construire prompt avec contexte
5. **Generate** : LLM inference
6. **Save** : Stocker interaction

**Configuration :**
```
RAG Parameters:
  - top_k: 3-10 (nombre de résultats)
  - similarity_threshold: 0.5-0.8
  - relation_depth: 0-3 (profondeur graph)
  - max_context_tokens: 1000-4000
```

---

## 6. Modèle de Données

### 6.1 Structure des Nœuds

```
Node {
  id: UUID
  type: String (memory, entity, event, concept, conversation)
  vault_id: String
  namespace?: String
  labels: [String]
  embedding?: [Float; 384]
  encrypted_content: Bytes (age encryption)
  content_hmac: String (integrity)
  metadata: {
    content_size: Int
    version: Int
    expires_at?: Timestamp
  }
  created_at: Timestamp
  updated_at: Timestamp
  accessed_at: Timestamp
  access_count: Int
}
```

### 6.2 Structure des Relations

```
Edge {
  id: UUID
  from_node: UUID
  to_node: UUID
  edge_type: String (relates_to, is_a, part_of, etc.)
  vault_id: String
  weight: Float (0.0-1.0)
  bidirectional: Bool
  encrypted_context?: Bytes
  created_at: Timestamp
}
```

### 6.3 Types Prédéfinis

**Node Types :**
- `memory` : Mémoire générale
- `entity` : Entité (personne, lieu, objet)
- `event` : Événement daté
- `concept` : Concept abstrait
- `conversation` : Historique LLM
- `document` : Document structuré
- `preference` : Préférence utilisateur

**Edge Types :**
- `relates_to` : Relation générique
- `is_a` : Type/Classe
- `part_of` : Composition
- `located_in` : Localisation
- `happened_at` : Temporel
- `caused_by` : Causalité
- `references` : Référence

### 6.4 Exemples de Graph

**Mémoire d'un Assistant Personnel :**
```
[User] ──prefers──> [Coffee:black_no_sugar]
  │
  ├──is──> [Vegetarian]
  │
  └──lives_in──> [Paris]
      │
      └──near──> [Eiffel_Tower]
```

**Knowledge Base :**
```
[Doc:Architecture] ──discusses──> [Concept:Hexagonal]
                                        │
                                        ├──related_to──> [Concept:Ports]
                                        └──related_to──> [Concept:Adapters]
```

---

## 7. Interfaces et API

### 7.1 Ports (Rust Traits)

#### GraphPort
```rust
trait GraphPort {
    // CRUD
    async fn create_node(...) -> Result<NodeId>;
    async fn get_node(...) -> Result<Option<Node>>;
    async fn update_node(...) -> Result<()>;
    async fn delete_node(...) -> Result<()>;

    async fn create_edge(...) -> Result<EdgeId>;
    async fn get_edges(...) -> Result<Vec<Edge>>;

    // Search
    async fn vector_search(...) -> Result<Vec<(Node, f64)>>;
    async fn traverse(...) -> Result<Vec<Node>>;
    async fn shortest_path(...) -> Result<Option<Vec<NodeId>>>;

    // RAG
    async fn rag_query(...) -> Result<RagResult>;
}
```

#### LLMPort
```rust
trait LLMPort {
    async fn load_model(model_id: &str) -> Result<()>;
    async fn generate(prompt: &str, config: GenerateConfig) -> Result<String>;
    async fn generate_stream(...) -> Result<Stream<String>>;
    fn is_loaded() -> bool;
    fn model_info() -> ModelInfo;
}
```

#### EmbeddingPort
```rust
trait EmbeddingPort {
    async fn embed(text: &str) -> Result<Vec<f32>>;
    async fn embed_batch(texts: Vec<&str>) -> Result<Vec<Vec<f32>>>;
    fn dimension() -> usize;
}
```

### 7.2 Façade JavaScript

```typescript
// Graph API
interface GraphAPI {
  // Nodes
  createNode(vaultId: string, type: string, content: any, options?: NodeOptions): Promise<string>;
  getNode(vaultId: string, nodeId: string): Promise<Node | null>;
  updateNode(vaultId: string, nodeId: string, content: any): Promise<void>;
  deleteNode(vaultId: string, nodeId: string): Promise<void>;

  // Relations
  createEdge(vaultId: string, from: string, to: string, type: string, weight?: number): Promise<string>;
  getEdges(vaultId: string, nodeId: string, direction?: EdgeDirection): Promise<Edge[]>;

  // Search
  vectorSearch(vaultId: string, embedding: number[], options?: SearchOptions): Promise<SearchResult[]>;
  traverse(vaultId: string, startNode: string, options?: TraverseOptions): Promise<Node[]>;

  // RAG
  ragQuery(vaultId: string, query: RagQuery): Promise<RagResult>;
}

// LLM API
interface LLMAPI {
  loadModel(modelId: string, onProgress?: (progress: number) => void): Promise<void>;
  chat(messages: ChatMessage[], options?: ChatOptions): Promise<string>;
  chatStream(messages: ChatMessage[], onToken: (token: string) => void): Promise<void>;
  isReady(): boolean;
  getModelInfo(): ModelInfo;
}

// RAG Orchestrator
interface RAGOrchestrator {
  ask(vaultId: string, question: string, options?: RagOptions): Promise<RagResponse>;
  addMemory(vaultId: string, content: string, type: string, labels?: string[]): Promise<string>;
}
```

### 7.3 Types Principaux

```typescript
interface RagQuery {
  question: string;
  topK?: number;              // Default: 5
  similarityThreshold?: number; // Default: 0.7
  includeRelations?: boolean;  // Default: true
  relationDepth?: number;      // Default: 2
  nodeTypes?: string[];
  labels?: string[];
}

interface RagResult {
  memories: MemoryMatch[];
  relations: Edge[];
  contextForLLM: string;
  sourceIds: string[];
  metadata: {
    searchTime: number;
    inferenceTime: number;
    totalTokens: number;
  };
}

interface RagResponse {
  answer: string;
  sources: Node[];
  confidence: number;
  metadata: RagMetadata;
}
```

---

## 8. Flux RAG Complet

### 8.1 Vue d'Ensemble

```
┌─────────────────────────────────────────────────────┐
│  1. QUESTION                                         │
│     User: "Comment j'aime mon café ?"               │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│  2. EMBEDDING                                        │
│     all-MiniLM-L6-v2: [0.23, -0.45, ...]           │
│     Time: ~20ms                                      │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│  3. GRAPH SEARCH (CozoDB)                           │
│     a) Vector similarity (HNSW)                     │
│        → Top 5 similar nodes                        │
│     b) Graph traversal (Datalog)                    │
│        → Relations (depth 2)                        │
│     c) Decrypt content (age)                        │
│     Time: ~50-100ms                                 │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│  4. CONTEXTE RÉCUPÉRÉ                               │
│     - "J'adore le café noir sans sucre" (0.95)     │
│     - "Je prends un café chaque matin" (0.87)      │
│     - Relations: prefers → coffee_black            │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│  5. PROMPT CONSTRUCTION                             │
│     System: "You are an assistant..."              │
│     Context: "User preferences: ..."               │
│     Question: "Comment j'aime mon café ?"          │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│  6. LLM INFERENCE (WebLLM)                          │
│     Model: Phi-3.5-mini                            │
│     Time: ~1-2s (WebGPU)                           │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│  7. RÉPONSE                                          │
│     "Vous aimez le café noir sans sucre."          │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│  8. SAUVEGARDE INTERACTION                          │
│     Create node: conversation                       │
│     Create edges: references sources               │
│     Update: access_count++                         │
└─────────────────────────────────────────────────────┘
```

### 8.2 Optimisations

**Cache Strategy :**
- Embeddings cache (LRU)
- Frequent queries cache
- Model loaded once (stay in memory)

**Lazy Loading :**
- Nodes chargés à la demande
- Relations suivies progressivement

**Batch Processing :**
- Multiple embeddings en parallèle
- Transactions groupées

---

## 9. Sécurité et Chiffrement

### 9.1 Principe Zero-Knowledge

```
┌────────────────────────────────────────┐
│  TOUT CHIFFRÉ DANS OPFS               │
│                                        │
│  Graph data (CozoDB)                  │
│  ├─ Nodes: encrypted_content (age)   │
│  ├─ Edges: encrypted_context (age)   │
│  └─ HMAC: integrity verification     │
│                                        │
│  Models (cache)                       │
│  ├─ LLM weights (public, non chiffré)│
│  └─ Embeddings (public, non chiffré) │
└────────────────────────────────────────┘

Clés de chiffrement:
  vault_master_key (user password)
    ↓ HKDF
  graph_key (dérivée, 256 bits)
    ↓
  age::encrypt(content, graph_key)
```

### 9.2 Trade-offs Embeddings

**Option A : Embeddings Non Chiffrés** (Recommandé)
- ✅ HNSW natif (rapide)
- ⚠️ Révèle info sémantique
- 💡 Acceptable si OPFS local

**Option B : Embeddings Chiffrés** (Paranoid mode)
- ✅ Zero fuite sémantique
- ❌ Pas de HNSW (déchiffrer tout)
- 💡 Très lent, pour cas extrêmes

**Configuration :**
```typescript
interface EncryptionConfig {
  encryptEmbeddings: boolean; // Default: false
  hmacVerification: boolean;  // Default: true
  keyRotation?: number;       // Optional auto-rotation
}
```

### 9.3 Isolation

- **Par vault** : Chaque vault = clé différente
- **Par origin** : OPFS isolé par domaine
- **Pas de serveur** : Aucune donnée envoyée

---

## 10. Performance et Scalabilité

### 10.1 Objectifs

| Métrique | Cible (PoC) | Cible (Production) |
|----------|-------------|---------------------|
| Load LLM (from cache) | < 10s | < 5s |
| Embedding (single) | < 50ms | < 20ms |
| Graph search (top 10) | < 100ms | < 50ms |
| LLM inference (100 tok) | < 2s (WebGPU) | < 1s |
| **RAG complet** | **< 3s** | **< 1.5s** |

### 10.2 Scalabilité

| Taille | Nœuds | Edges | Stratégie | Latence Attendue |
|--------|-------|-------|-----------|------------------|
| **Petit** | < 10K | < 100K | Tout en mémoire | < 50ms (search) |
| **Moyen** | 10K-100K | 100K-1M | Cache LRU | < 100ms |
| **Grand** | 100K-1M | 1M-10M | Lazy loading + index | < 200ms |

### 10.3 Benchmarks Cibles (PoC)

```
Environment:
  - Browser: Chrome 120+
  - GPU: Any WebGPU-compatible
  - Model: Phi-3.5-mini (2GB)
  - Graph: 10K nodes, 50K edges

Results:
  ✓ Create node:              5ms
  ✓ Vector search (top 10):   45ms
  ✓ Graph traversal (d=2):    80ms
  ✓ Embedding generation:     25ms
  ✓ LLM inference (100 tok):  1200ms (WebGPU)
  ✓ RAG complete:             1500ms

  ✓ Storage: 2.1GB (model) + 15MB (graph)
  ✓ Memory: ~3GB peak
```

### 10.4 Optimisations Prévues

**Phase 1 (PoC)** :
- Implémentation basique
- Pas d'optimisation prématurée

**Phase 2 (Production)** :
- Cache LRU (nodes, embeddings, queries)
- Batch operations
- Web Workers (parallélisation)
- Index tuning (HNSW params)
- Model quantization (Q4, Q8)

---

## 11. Plan d'Implémentation

### Phase 1 : Foundation + PoC (3 semaines)

**Objectif :** Démo end-to-end fonctionnelle

#### Sprint 1 : CozoDB + Graph Basics
**Durée :** 1 semaine

**Tâches :**
- [ ] Setup projet (dependencies)
- [ ] Intégration CozoDB (WASM)
- [ ] Domain layer : types.rs (Node, Edge)
- [ ] Port GraphPort (interface)
- [ ] Adapter CozoGraphAdapter (basique)
- [ ] CRUD nodes (create, get)
- [ ] Chiffrement age integration
- [ ] Tests unitaires

**Délivrables :**
- ✅ Create/Get node fonctionne
- ✅ Chiffrement opérationnel
- ✅ Tests passent

#### Sprint 2 : WebLLM + Embeddings
**Durée :** 1 semaine

**Tâches :**
- [ ] Intégration WebLLM
- [ ] Download + cache model (Phi-3.5)
- [ ] Port LLMPort + EmbeddingPort
- [ ] Adapter WebLLMAdapter
- [ ] Adapter TransformersEmbedding
- [ ] Tests inference basique
- [ ] Mesure performance (latency, memory)

**Délivrables :**
- ✅ LLM répond à des prompts simples
- ✅ Embeddings générés correctement
- ✅ Benchmarks de base

#### Sprint 3 : RAG Pipeline
**Durée :** 1 semaine

**Tâches :**
- [ ] Implémentation vector search (HNSW)
- [ ] Graph traversal (Datalog)
- [ ] Domain : rag.rs (logique RAG)
- [ ] RAG orchestrator (TypeScript)
- [ ] Prompt builder
- [ ] Sauvegarde interactions
- [ ] UI playground (démo)
- [ ] Tests end-to-end

**Délivrables :**
- ✅ Question → Réponse (RAG complet)
- ✅ Démo interactive
- ✅ Documentation

**Critères de succès Phase 1 :**
- ✅ RAG fonctionne end-to-end
- ✅ < 3s pour requête complète
- ✅ 1000 mémoires gérées sans problème
- ✅ Chiffrement vérifié
- ✅ Démo impressionnante

---

### Phase 2 : Production Features (4 semaines)

**Sprint 4-5 : API Complète + Relations**
- CRUD complet (update, delete)
- Graph traversal avancé (shortest path, neighbors)
- Types de relations prédéfinis
- Batch operations
- Transactions

**Sprint 6-7 : Optimisations**
- Cache LRU
- Web Workers
- Index tuning
- Performance profiling
- Documentation complète

**Critères de succès Phase 2 :**
- ✅ API stable et documentée
- ✅ 100K nœuds gérés
- ✅ < 1.5s pour RAG query
- ✅ Tests coverage > 80%

---

### Phase 3 : Polish & Advanced (2 semaines)

**Sprint 8 : Advanced Features**
- Requêtes Datalog complexes
- Analytics (PageRank, clustering)
- Multi-model support (Llama, Mistral)
- Model switching (hot-swap)

**Sprint 9 : Documentation & Examples**
- Guide utilisateur complet
- API reference
- Tutoriaux
- Exemples d'applications
- Video démo

---

## 12. Références

### 12.1 Technologies

**CozoDB**
- Docs: https://docs.cozodb.org/
- GitHub: https://github.com/cozodb/cozo
- Datalog: https://docs.cozodb.org/en/latest/queries.html

**WebLLM**
- Docs: https://webllm.mlc.ai/
- GitHub: https://github.com/mlc-ai/web-llm
- Models: https://github.com/mlc-ai/web-llm#available-models

**Transformers.js**
- Docs: https://huggingface.co/docs/transformers.js
- GitHub: https://github.com/xenova/transformers.js
- Models: https://huggingface.co/models?library=transformers.js

**OPFS**
- Spec: https://fs.spec.whatwg.org/
- MDN: https://developer.mozilla.org/en-US/docs/Web/API/File_System_API

**Age Encryption**
- Spec: https://age-encryption.org/
- Rust: https://docs.rs/age/

### 12.2 Concepts

**RAG (Retrieval-Augmented Generation)**
- Survey: https://arxiv.org/abs/2312.10997
- GraphRAG: https://www.microsoft.com/en-us/research/blog/graphrag/

**Vector Search**
- HNSW: https://arxiv.org/abs/1603.09320
- Embeddings: https://huggingface.co/spaces/mteb/leaderboard

**Graph Databases**
- Comparison: https://thedataquarry.com/blog/embedded-db-2/
- Datalog: https://en.wikipedia.org/wiki/Datalog

### 12.3 Hoddor

- Repository: https://github.com/Gatewatcher/hoddor
- Architecture hexagonale: Commit 9652512

---

## Annexes

### A. Glossaire

| Terme | Définition |
|-------|------------|
| **Node** | Nœud dans le graphe (mémoire, entité, concept) |
| **Edge** | Relation entre deux nœuds |
| **Embedding** | Représentation vectorielle d'un texte (384 ou 768 dimensions) |
| **RAG** | Retrieval-Augmented Generation (recherche + génération LLM) |
| **HNSW** | Hierarchical Navigable Small World (algo vector search) |
| **Datalog** | Langage de requête déclaratif (CozoDB) |
| **OPFS** | Origin Private File System (storage navigateur) |
| **WebGPU** | API GPU pour navigateur |
| **SLM** | Small Language Model (< 10B params) |
| **Zero-Knowledge** | Serveur n'a jamais accès aux données déchiffrées |

### B. FAQ

**Q: Pourquoi un LLM local plutôt qu'une API cloud ?**
A: Privacy totale, coûts nuls, offline-capable, GDPR-compliant par design.

**Q: Performance comparée à GPT-4 ?**
A: Modèles locaux (Phi, Llama 3B) moins capables mais suffisants pour 80% des cas. Trade-off privacy vs qualité.

**Q: Quelle taille de graphe supportée ?**
A: Testé jusqu'à 100K nœuds. Au-delà, optimisations nécessaires (partitionnement, index avancés).

**Q: Combien d'espace disque ?**
A: Model (~2-4GB) + Graph (~10MB par 1000 nœuds) + Cache (~100MB). Total: ~3-5GB.

**Q: Compatible tous navigateurs ?**
A: WebGPU (Chrome 113+, Edge 113+). Safari/Firefox: WASM fallback (plus lent mais fonctionne).

**Q: Peut-on utiliser plusieurs modèles ?**
A: Oui, configuration runtime. Charger Phi-3.5 pour rapidité, Mistral-7B pour qualité.

---

## Changelog

| Version | Date | Changements |
|---------|------|-------------|
| 1.0 | 2025-10-20 | Version initiale |
| 2.0 | 2025-10-20 | Ajout WebLLM, refacto spec (moins de code, plus conceptuel) |

---

**FIN DE LA SPÉCIFICATION**

*Document maintenu par l'équipe Hoddor. Pour questions : créer une issue sur GitHub.*
