import { pipeline, env } from "@xenova/transformers";

export interface EmbeddingResult {
  embedding: number[];
  text: string;
  dimensions: number;
}

export class EmbeddingService {
  private model: any | null = null;
  private modelId: string;
  private isInitialized = false;

  constructor(modelId: string = "Xenova/all-MiniLM-L6-v2") {
    this.modelId = modelId;

    // Configure Transformers.js to use HuggingFace CDN
    env.allowLocalModels = false;
    env.useBrowserCache = true;
  }

  /**
   * Initialize the embedding model
   */
  async initialize(): Promise<void> {
    if (this.isInitialized) {
      console.log("Embedding service already initialized");
      return;
    }

    try {
      console.log(`Initializing embedding model: ${this.modelId}`);

      this.model = await pipeline("feature-extraction", this.modelId, {
        progress_callback: (progress: any) => {
          console.log(`Model loading: ${progress.status} - ${progress.name || ''}`);
        }
      });

      this.isInitialized = true;
      console.log("Embedding service initialized successfully");
    } catch (error) {
      console.error("Failed to initialize embedding service:", error);
      throw new Error(`Embedding initialization failed: ${error}`);
    }
  }

  /**
   * Check if the service is ready
   */
  isReady(): boolean {
    return this.isInitialized && this.model !== null;
  }

  /**
   * Generate embeddings for a single text
   */
  async embed(text: string): Promise<EmbeddingResult> {
    if (!this.isReady()) {
      throw new Error("Embedding service not initialized. Call initialize() first.");
    }

    try {
      const output = await this.model!(text, {
        pooling: "mean",
        normalize: true,
      });

      // Convert tensor to array
      const embedding = Array.from(output.data) as number[];

      return {
        embedding,
        text,
        dimensions: embedding.length,
      };
    } catch (error) {
      console.error("Embedding generation failed:", error);
      throw new Error(`Failed to generate embedding: ${error}`);
    }
  }

  /**
   * Generate embeddings for multiple texts
   */
  async embedBatch(texts: string[]): Promise<EmbeddingResult[]> {
    if (!this.isReady()) {
      throw new Error("Embedding service not initialized. Call initialize() first.");
    }

    try {
      const results: EmbeddingResult[] = [];

      // Process in batches to avoid memory issues
      for (const text of texts) {
        const result = await this.embed(text);
        results.push(result);
      }

      return results;
    } catch (error) {
      console.error("Batch embedding failed:", error);
      throw new Error(`Failed to generate batch embeddings: ${error}`);
    }
  }

  /**
   * Calculate cosine similarity between two embeddings
   */
  static cosineSimilarity(a: number[], b: number[]): number {
    if (a.length !== b.length) {
      throw new Error("Embeddings must have the same dimensions");
    }

    let dotProduct = 0;
    let normA = 0;
    let normB = 0;

    for (let i = 0; i < a.length; i++) {
      dotProduct += a[i] * b[i];
      normA += a[i] * a[i];
      normB += b[i] * b[i];
    }

    return dotProduct / (Math.sqrt(normA) * Math.sqrt(normB));
  }

  /**
   * Get available embedding models
   */
  static getAvailableModels(): string[] {
    return [
      "Xenova/all-MiniLM-L6-v2",       // 384 dims, 80MB, fast
      "Xenova/all-mpnet-base-v2",      // 768 dims, 420MB, accurate
      "Xenova/multilingual-e5-small",  // 384 dims, 118MB, multilingual
    ];
  }

  /**
   * Reset the service
   */
  async reset(): Promise<void> {
    if (this.model) {
      this.model = null;
      this.isInitialized = false;
      console.log("Embedding service reset");
    }
  }
}
