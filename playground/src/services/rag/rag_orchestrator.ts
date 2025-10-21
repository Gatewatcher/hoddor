import { WebLLMService, ChatMessage } from "../llm/webllm_service";
import { EmbeddingService } from "../embeddings/embedding_service";

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
   * Query with RAG pattern (will be extended in Phase 2 with graph integration)
   */
  async query(
    question: string,
    options: RAGQueryOptions = {}
  ): Promise<string> {
    // Phase 1: Direct query without RAG context
    // Phase 2: Will add vector search and context retrieval

    const messages: ChatMessage[] = [
      { role: "system", content: this.systemPrompt },
      { role: "user", content: question },
    ];

    return await this.llmService.chat(messages, {
      temperature: options.temperature ?? 0.7,
      max_tokens: 512,
    });
  }

  /**
   * Stream query response
   */
  async *queryStream(
    question: string,
    options: RAGQueryOptions = {}
  ): AsyncGenerator<string, void, unknown> {
    const messages: ChatMessage[] = [
      { role: "system", content: this.systemPrompt },
      { role: "user", content: question },
    ];

    yield* this.llmService.chatStream(messages, {
      temperature: options.temperature ?? 0.7,
      max_tokens: 512,
    });
  }

  // Private methods for Phase 2 implementation
  // These will be used when graph integration is added

  // async findRelevantContext(query: string, options: RAGQueryOptions): Promise<RAGContext[]>
  // buildPromptWithContext(question: string, contexts: RAGContext[]): string

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
