/* eslint-disable @typescript-eslint/no-explicit-any */
import { env, pipeline } from '@xenova/transformers';

export interface EmbeddingResult {
  embedding: number[];
  text: string;
  dimensions: number;
}

export class EmbeddingService {
  private model: any | null = null;
  private modelId: string;
  private isInitialized = false;

  constructor(modelId: string = 'Xenova/all-MiniLM-L6-v2') {
    this.modelId = modelId;

    try {
      env.allowLocalModels = false;
      env.useBrowserCache = false;
      env.backends.onnx.wasm.numThreads = 1;

      console.log('📍 Transformers.js configuration:');
      console.log('  - allowLocalModels:', (env as any).allowLocalModels);
      console.log('  - useBrowserCache:', (env as any).useBrowserCache);
      console.log('  - localModelPath:', (env as any).localModelPath);
      console.log('  - cacheDir:', (env as any).cacheDir);
    } catch (e) {
      console.warn('Could not configure Transformers.js env:', e);
    }
  }

  async initialize(): Promise<void> {
    if (this.isInitialized) {
      console.log('Embedding service already initialized');
      return;
    }

    try {
      console.log(`Initializing embedding model: ${this.modelId}`);

      console.log('🧪 Testing direct HuggingFace CDN access...');
      try {
        const testUrl = `https://huggingface.co/${this.modelId}/resolve/main/tokenizer.json`;
        const testResponse = await fetch(testUrl, { cache: 'no-store' });
        const testText = await testResponse.text();
        const isJson = testText.startsWith('{') || testText.startsWith('[');
        console.log(
          `  ✅ Direct fetch ${
            isJson ? 'succeeded' : 'returned HTML'
          }: ${testText.substring(0, 50)}...`,
        );

        if (!isJson) {
          console.warn(
            '⚠️ WARNING: HuggingFace is returning HTML instead of JSON!',
          );
          console.warn(
            '  This might be a network issue, VPN block, or rate limiting.',
          );
        }
      } catch (testError) {
        console.warn('⚠️ Direct CDN test failed:', testError);
      }

      const originalFetch = window.fetch.bind(window);
      let fetchCount = 0;
      window.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
        const url = input?.toString() || '';
        if (
          url.includes('huggingface') ||
          url.includes('Xenova') ||
          url.includes('model')
        ) {
          fetchCount++;
          console.log(`🌐 Fetch #${fetchCount}: ${url}`);
          const response = await originalFetch(input, init);
          const clonedResponse = response.clone();

          clonedResponse
            .text()
            .then(text => {
              const preview = text.substring(0, 50);
              const isHtml = text.startsWith('<!') || text.startsWith('<html');
              console.log(`  ${isHtml ? '❌ HTML' : '✅ DATA'}: ${preview}...`);
            })
            .catch(() => {});

          return response;
        }
        return originalFetch(input, init);
      };

      this.model = await pipeline('feature-extraction', this.modelId, {
        progress_callback: (progress: any) => {
          console.log(
            `Model loading: ${progress.status} - ${progress.name || ''} - ${
              progress.file || ''
            }`,
          );
        },
      });

      window.fetch = originalFetch;

      this.isInitialized = true;
      console.log(
        `✅ Embedding service initialized successfully (${fetchCount} fetches)`,
      );
    } catch (error) {
      console.error('Failed to initialize embedding service:', error);
      throw new Error(`Embedding initialization failed: ${error}`);
    }
  }

  isReady(): boolean {
    return this.isInitialized && this.model !== null;
  }

  async embed(text: string): Promise<EmbeddingResult> {
    if (!this.isReady()) {
      throw new Error(
        'Embedding service not initialized. Call initialize() first.',
      );
    }

    try {
      const output = await this.model!(text, {
        pooling: 'mean',
        normalize: true,
      });

      const embedding = Array.from(output.data) as number[];

      return {
        embedding,
        text,
        dimensions: embedding.length,
      };
    } catch (error) {
      console.error('Embedding generation failed:', error);
      throw new Error(`Failed to generate embedding: ${error}`);
    }
  }

  async embedBatch(texts: string[]): Promise<EmbeddingResult[]> {
    if (!this.isReady()) {
      throw new Error(
        'Embedding service not initialized. Call initialize() first.',
      );
    }

    try {
      const results: EmbeddingResult[] = [];

      for (const text of texts) {
        const result = await this.embed(text);
        results.push(result);
      }

      return results;
    } catch (error) {
      console.error('Batch embedding failed:', error);
      throw new Error(`Failed to generate batch embeddings: ${error}`);
    }
  }

  static cosineSimilarity(a: number[], b: number[]): number {
    if (a.length !== b.length) {
      throw new Error('Embeddings must have the same dimensions');
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

  static getAvailableModels(): string[] {
    return [
      'Xenova/all-MiniLM-L6-v2',
      'Xenova/all-mpnet-base-v2',
      'Xenova/multilingual-e5-small',
    ];
  }

  async reset(): Promise<void> {
    if (this.model) {
      this.model = null;
      this.isInitialized = false;
      console.log('Embedding service reset');
    }
  }
}
