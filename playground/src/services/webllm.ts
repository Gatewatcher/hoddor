import * as webllm from '@mlc-ai/web-llm';

export interface ChatMessage {
  role: 'system' | 'user' | 'assistant';
  content: string;
}

export interface ChatOptions {
  temperature?: number;
  top_p?: number;
  max_tokens?: number;
}

export class WebLLMService {
  private engine: webllm.MLCEngine | null = null;
  private modelId: string;
  private isInitialized = false;

  constructor(modelId: string = 'Phi-3.5-mini-instruct-q4f16_1-MLC') {
    this.modelId = modelId;
  }

  async initialize(
    onProgress?: (progress: webllm.InitProgressReport) => void,
  ): Promise<void> {
    if (this.isInitialized) {
      console.log('WebLLM already initialized');
      return;
    }

    try {
      console.log(`Initializing WebLLM with model: ${this.modelId}`);

      this.engine = await webllm.CreateMLCEngine(this.modelId, {
        initProgressCallback: onProgress,
      });

      this.isInitialized = true;
      console.log('WebLLM initialized successfully');
    } catch (error) {
      console.error('Failed to initialize WebLLM:', error);
      throw new Error(`WebLLM initialization failed: ${error}`);
    }
  }

  isReady(): boolean {
    return this.isInitialized && this.engine !== null;
  }

  async chat(
    messages: ChatMessage[],
    options: ChatOptions = {},
  ): Promise<string> {
    if (!this.isReady()) {
      throw new Error(
        'WebLLM service not initialized. Call initialize() first.',
      );
    }

    try {
      const response = await this.engine!.chat.completions.create({
        messages,
        temperature: options.temperature ?? 0.7,
        top_p: options.top_p ?? 0.95,
        max_tokens: options.max_tokens ?? 512,
      });

      return response.choices[0]?.message?.content || '';
    } catch (error) {
      console.error('Chat completion failed:', error);
      throw new Error(`Chat failed: ${error}`);
    }
  }

  async *chatStream(
    messages: ChatMessage[],
    options: ChatOptions = {},
  ): AsyncGenerator<string, void, unknown> {
    if (!this.isReady()) {
      throw new Error(
        'WebLLM service not initialized. Call initialize() first.',
      );
    }

    try {
      const stream = await this.engine!.chat.completions.create({
        messages,
        temperature: options.temperature ?? 0.7,
        top_p: options.top_p ?? 0.95,
        max_tokens: options.max_tokens ?? 512,
        stream: true,
      });

      for await (const chunk of stream) {
        const content = chunk.choices[0]?.delta?.content;
        if (content) {
          yield content;
        }
      }
    } catch (error) {
      console.error('Streaming chat failed:', error);
      throw new Error(`Chat stream failed: ${error}`);
    }
  }

  static getAvailableModels(): string[] {
    return [
      'Phi-3.5-mini-instruct-q4f16_1-MLC',
      'Llama-3.2-3B-Instruct-q4f16_1-MLC',
      'Qwen2.5-3B-Instruct-q4f16_1-MLC',
    ];
  }

  async reset(): Promise<void> {
    if (this.engine) {
      this.engine = null;
      this.isInitialized = false;
      console.log('WebLLM engine reset');
    }
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  async getRuntimeStats(): Promise<any | null> {
    if (!this.isReady()) {
      return null;
    }
    return null;
  }
}
