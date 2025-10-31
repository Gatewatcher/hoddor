import { GraphNodeResult, GraphNodeWithNeighborsResult } from 'types/graph';

import {
  graph_vector_search,
  graph_vector_search_with_neighbors,
} from '../../../hoddor/pkg/hoddor';
import { EmbeddingService } from './embedding';
import { ChatMessage, WebLLMService } from './webllm';

export interface RAGContext {
  content: string;
  relevance: number;
  nodeId?: string;
  isNeighbor?: boolean;
}

export interface RAGQueryOptions {
  maxContextItems?: number;
  searchQuality?: number;
  temperature?: number;
  useStreaming?: boolean;
  vaultName?: string;
  useGraphRAG?: boolean;
  neighborEdgeTypes?: string[];
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
    const searchQuality = options.searchQuality ?? 100;

    const extractContent = (
      node: GraphNodeWithNeighborsResult | GraphNodeResult,
    ): string => {
      if (node.content && node.content.length > 0) {
        return node.content;
      }
      return `[${node.labels.join(', ')}]`;
    };

    if (options.useGraphRAG) {
      const results: GraphNodeWithNeighborsResult[] =
        await graph_vector_search_with_neighbors(
          options.vaultName,
          new Float32Array(embedding),
          limit,
          searchQuality,
        );

      const contexts: RAGContext[] = [];
      results.forEach((result: GraphNodeWithNeighborsResult) => {
        const content = extractContent(result);

        contexts.push({
          content,
          relevance: result.similarity,
          nodeId: result.id,
          isNeighbor: false,
        });

        result.neighbors.forEach((neighbor: GraphNodeResult) => {
          contexts.push({
            content: extractContent(neighbor),
            relevance: result.similarity * 0.8,
            nodeId: neighbor.id,
            isNeighbor: true,
          });
        });
      });

      return contexts;
    }

    const results: GraphNodeResult[] = await graph_vector_search(
      options.vaultName,
      new Float32Array(embedding),
      limit,
      searchQuality,
    );

    return results.map(result => {
      const content = extractContent(result);

      return {
        content,
        relevance: result.similarity,
        nodeId: result.id,
        isNeighbor: false,
      };
    }) as RAGContext[];
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

Please answer the question using the context above.`;
  }

  setSystemPrompt(prompt: string): void {
    this.systemPrompt = prompt;
  }

  isReady(): boolean {
    return this.llmService.isReady();
  }
}
