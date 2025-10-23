import { graph_vector_search } from '../../../hoddor/pkg/hoddor';
import { EmbeddingService } from './embedding';
import { ChatMessage, WebLLMService } from './webllm';

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
  vaultName?: string;
}

export class RAGOrchestrator {
  private llmService: WebLLMService;
  private embeddingService: EmbeddingService;
  private systemPrompt: string;

  constructor(
    llmService: WebLLMService,
    embeddingService: EmbeddingService,
    systemPrompt?: string,
  ) {
    this.llmService = llmService;
    this.embeddingService = embeddingService;
    this.systemPrompt =
      systemPrompt ||
      `You are a helpful AI assistant with access to the user's knowledge graph.
When answering questions, use the provided context from the knowledge base.
Always cite which parts of the context you used to answer.`;
  }

  async query(
    question: string,
    options: RAGQueryOptions = {},
  ): Promise<string> {
    let contexts: RAGContext[] = [];

    if (options.vaultName && this.embeddingService.isReady()) {
      try {
        contexts = await this.findRelevantContext(question, options);
      } catch (error) {
        console.warn('Failed to fetch graph context:', error);
      }
    }

    const userPrompt =
      contexts.length > 0
        ? this.buildPromptWithContext(question, contexts)
        : question;

    const messages: ChatMessage[] = [
      { role: 'system', content: this.systemPrompt },
      { role: 'user', content: userPrompt },
    ];

    return await this.llmService.chat(messages, {
      temperature: options.temperature ?? 0.7,
      max_tokens: 512,
    });
  }

  async *queryStream(
    question: string,
    options: RAGQueryOptions = {},
  ): AsyncGenerator<string, void, unknown> {
    let contexts: RAGContext[] = [];

    if (options.vaultName && this.embeddingService.isReady()) {
      try {
        contexts = await this.findRelevantContext(question, options);
      } catch (error) {
        console.warn('Failed to fetch graph context:', error);
      }
    }

    const userPrompt =
      contexts.length > 0
        ? this.buildPromptWithContext(question, contexts)
        : question;

    const messages: ChatMessage[] = [
      { role: 'system', content: this.systemPrompt },
      { role: 'user', content: userPrompt },
    ];

    yield* this.llmService.chatStream(messages, {
      temperature: options.temperature ?? 0.7,
      max_tokens: 512,
    });
  }

  private async findRelevantContext(
    query: string,
    options: RAGQueryOptions,
  ): Promise<RAGContext[]> {
    if (!this.embeddingService.isReady() || !options.vaultName) {
      return [];
    }

    const { embedding } = await this.embeddingService.embed(query);

    const limit = options.maxContextItems ?? 5;
    const minSimilarity = options.minRelevance ?? 0.5;

    const results = await graph_vector_search(
      options.vaultName,
      new Float32Array(embedding),
      limit,
      minSimilarity,
    );

    console.log(
      `🔍 RAG found ${results.length} relevant memories for: "${query}"`,
    );

    const decoder = new TextDecoder();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    return results.map((result: any, idx: number) => {
      let content = '';

      try {
        if (result.encrypted_content && result.encrypted_content.length > 0) {
          content = decoder.decode(new Uint8Array(result.encrypted_content));
        } else {
          content = `[${result.labels.join(', ')}]`;
        }
      } catch (error) {
        console.warn(`Failed to decode memory ${result.id}:`, error);
        content = `[${result.labels.join(', ')}]`;
      }

      console.log(
        `  [${idx + 1}] (${result.similarity.toFixed(2)}): ${content.substring(
          0,
          60,
        )}...`,
      );

      return {
        content,
        relevance: result.similarity,
        nodeId: result.id,
      };
    });
  }

  private buildPromptWithContext(
    question: string,
    contexts: RAGContext[],
  ): string {
    if (contexts.length === 0) {
      return question;
    }

    const contextText = contexts
      .map(
        (ctx, idx) =>
          `[${idx + 1}] (relevance: ${ctx.relevance.toFixed(2)}) ${
            ctx.content
          }`,
      )
      .join('\n');

    return `Context from knowledge base:
${contextText}

Question: ${question}

Please answer the question using the context above. Cite the context numbers [1], [2], etc. when relevant.`;
  }

  setSystemPrompt(prompt: string): void {
    this.systemPrompt = prompt;
  }

  isReady(): boolean {
    return this.llmService.isReady();
  }
}
