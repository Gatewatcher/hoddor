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

    // Configure Transformers.js to use remote CDN
    try {
      // Set remote-only configuration
      // @ts-ignore - Use CDN exclusively
      env.allowLocalModels = false;
      // @ts-ignore - Don't use browser cache for now (to avoid HTML caching issues)
      env.useBrowserCache = false;
      // @ts-ignore - Configure WASM backend
      env.backends.onnx.wasm.numThreads = 1;

      console.log("📍 Transformers.js configuration:");
      console.log("  - allowLocalModels:", (env as any).allowLocalModels);
      console.log("  - useBrowserCache:", (env as any).useBrowserCache);
      console.log("  - localModelPath:", (env as any).localModelPath);
      console.log("  - cacheDir:", (env as any).cacheDir);
    } catch (e) {
      console.warn("Could not configure Transformers.js env:", e);
    }
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

      // Test direct fetch to verify CDN accessibility
      console.log("🧪 Testing direct HuggingFace CDN access...");
      try {
        const testUrl = `https://huggingface.co/${this.modelId}/resolve/main/tokenizer.json`;
        const testResponse = await fetch(testUrl, { cache: 'no-store' });
        const testText = await testResponse.text();
        const isJson = testText.startsWith("{") || testText.startsWith("[");
        console.log(`  ✅ Direct fetch ${isJson ? 'succeeded' : 'returned HTML'}: ${testText.substring(0, 50)}...`);

        if (!isJson) {
          console.warn("⚠️ WARNING: HuggingFace is returning HTML instead of JSON!");
          console.warn("  This might be a network issue, VPN block, or rate limiting.");
        }
      } catch (testError) {
        console.warn("⚠️ Direct CDN test failed:", testError);
      }

      // Install fetch interceptor to see all requests
      const originalFetch = window.fetch.bind(window);
      let fetchCount = 0;
      window.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = input?.toString() || '';
        if (url.includes('huggingface') || url.includes('Xenova') || url.includes('model')) {
          fetchCount++;
          console.log(`🌐 Fetch #${fetchCount}: ${url}`);
          const response = await originalFetch(input, init);
          const clonedResponse = response.clone();

          // Log first few bytes to see if it's HTML or JSON
          clonedResponse.text().then(text => {
            const preview = text.substring(0, 50);
            const isHtml = text.startsWith('<!') || text.startsWith('<html');
            console.log(`  ${isHtml ? '❌ HTML' : '✅ DATA'}: ${preview}...`);
          }).catch(() => {});

          return response;
        }
        return originalFetch(input, init);
      };

      this.model = await pipeline("feature-extraction", this.modelId, {
        progress_callback: (progress: any) => {
          console.log(`Model loading: ${progress.status} - ${progress.name || ''} - ${progress.file || ''}`);
        }
      });

      // Restore original fetch
      window.fetch = originalFetch;

      this.isInitialized = true;
      console.log(`✅ Embedding service initialized successfully (${fetchCount} fetches)`);
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
