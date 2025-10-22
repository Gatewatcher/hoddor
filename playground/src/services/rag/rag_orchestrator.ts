import { WebLLMService, ChatMessage } from "../llm/webllm_service";
import { EmbeddingService } from "../embeddings/embedding_service";
import { graph_vector_search } from "../../../../hoddor/pkg/hoddor";

export interface RAGContext {
  content: string;
  relevance: number;
  nodeId?: string;
}

export interface RAGQueryOptions {
  maxContextItems?: number;
  minRelevance?: number;
  temperature?: number;
  useStreaming?: boolean;
  vaultName?: string; // For Phase 2 graph integration
}

export class RAGOrchestrator {
  private llmService: WebLLMService;
  private embeddingService: EmbeddingService;
  private systemPrompt: string;

  constructor(
    llmService: WebLLMService,
    embeddingService: EmbeddingService,
    systemPrompt?: string
  ) {
    this.llmService = llmService;
    this.embeddingService = embeddingService;
    this.systemPrompt =
      systemPrompt ||
      `You are a helpful AI assistant with access to the user's knowledge graph.
When answering questions, use the provided context from the knowledge base.
Always cite which parts of the context you used to answer.`;
  }

  /**
   * Query with RAG pattern (Phase 2: with graph integration)
   */
  async query(
    question: string,
    options: RAGQueryOptions = {}
  ): Promise<string> {
    // Phase 2: Try to find relevant context from graph
    let contexts: RAGContext[] = [];

    if (options.vaultName && this.embeddingService.isReady()) {
      try {
        contexts = await this.findRelevantContext(question, options);
      } catch (error) {
        console.warn("Failed to fetch graph context:", error);
        // Continue without context
      }
    }

    // Build prompt with context if available
    const userPrompt = contexts.length > 0
      ? this.buildPromptWithContext(question, contexts)
      : question;

    const messages: ChatMessage[] = [
      { role: "system", content: this.systemPrompt },
      { role: "user", content: userPrompt },
    ];

    return await this.llmService.chat(messages, {
      temperature: options.temperature ?? 0.7,
      max_tokens: 512,
    });
  }

  /**
   * Stream query response (Phase 2: with graph integration)
   */
  async *queryStream(
    question: string,
    options: RAGQueryOptions = {}
  ): AsyncGenerator<string, void, unknown> {
    // Phase 2: Try to find relevant context from graph
    let contexts: RAGContext[] = [];

    if (options.vaultName && this.embeddingService.isReady()) {
      try {
        contexts = await this.findRelevantContext(question, options);
      } catch (error) {
        console.warn("Failed to fetch graph context:", error);
        // Continue without context
      }
    }

    // Build prompt with context if available
    const userPrompt = contexts.length > 0
      ? this.buildPromptWithContext(question, contexts)
      : question;

    const messages: ChatMessage[] = [
      { role: "system", content: this.systemPrompt },
      { role: "user", content: userPrompt },
    ];

    yield* this.llmService.chatStream(messages, {
      temperature: options.temperature ?? 0.7,
      max_tokens: 512,
    });
  }

  /**
   * Find relevant context from graph using vector search
   */
  private async findRelevantContext(
    query: string,
    options: RAGQueryOptions
  ): Promise<RAGContext[]> {
    if (!this.embeddingService.isReady() || !options.vaultName) {
      return [];
    }

    // Generate embedding for query
    const { embedding } = await this.embeddingService.embed(query);

    // Search graph for similar nodes
    const limit = options.maxContextItems ?? 5;
    const minSimilarity = options.minRelevance ?? 0.5;

    const results = await graph_vector_search(
      options.vaultName,
      new Float32Array(embedding),
      limit,
      minSimilarity
    );

    console.log(`🔍 RAG found ${results.length} relevant memories for: "${query}"`);

    // Convert results to RAGContext
    // Decrypt content for RAG (currently just base64 encoded, not encrypted)
    const decoder = new TextDecoder();
    return results.map((result: any, idx: number) => {
      let content = "";

      try {
        // Decrypt/decode the content
        if (result.encrypted_content && result.encrypted_content.length > 0) {
          content = decoder.decode(new Uint8Array(result.encrypted_content));
        } else {
          content = `[${result.labels.join(", ")}]`;
        }
      } catch (error) {
        console.warn(`Failed to decode memory ${result.id}:`, error);
        content = `[${result.labels.join(", ")}]`;
      }

      console.log(`  [${idx + 1}] (${result.similarity.toFixed(2)}): ${content.substring(0, 60)}...`);

      return {
        content,
        relevance: result.similarity,
        nodeId: result.id,
      };
    });
  }

  /**
   * Build prompt with context citations
   */
  private buildPromptWithContext(
    question: string,
    contexts: RAGContext[]
  ): string {
    if (contexts.length === 0) {
      return question;
    }

    const contextText = contexts
      .map((ctx, idx) => `[${idx + 1}] (relevance: ${ctx.relevance.toFixed(2)}) ${ctx.content}`)
      .join("\n");

    return `Context from knowledge base:
${contextText}

Question: ${question}

Please answer the question using the context above. Cite the context numbers [1], [2], etc. when relevant.`;
  }

  /**
   * Update system prompt
   */
  setSystemPrompt(prompt: string): void {
    this.systemPrompt = prompt;
  }

  /**
   * Check if services are ready
   * Phase 1: Only requires LLM to be ready
   * Phase 2: Will also check embedding service when RAG is fully integrated
   */
  isReady(): boolean {
    return this.llmService.isReady();
  }
}
