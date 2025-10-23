import { RobotOutlined } from '@ant-design/icons';
import { Button, Card, Progress, Select, Space, Typography } from 'antd';
import React, { useRef, useState } from 'react';

import { EmbeddingService, RAGOrchestrator, WebLLMService } from '../services';
import { ChatInput } from './chat/ChatInput';
import { ChatMessages } from './chat/ChatMessages';

const { Title, Text } = Typography;
const { Option } = Select;

interface Message {
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
}

export const LLMChat: React.FC = () => {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isInitializing, setIsInitializing] = useState(false);
  const [initProgress, setInitProgress] = useState(0);
  const [selectedModel, setSelectedModel] = useState(
    'Phi-3.5-mini-instruct-q4f16_1-MLC',
  );

  const llmServiceRef = useRef<WebLLMService | null>(null);
  const embeddingServiceRef = useRef<EmbeddingService | null>(null);
  const ragOrchestratorRef = useRef<RAGOrchestrator | null>(null);

  const handleInitialize = async () => {
    setIsInitializing(true);
    setInitProgress(0);

    try {
      const llmService = new WebLLMService(selectedModel);
      await llmService.initialize(report => {
        setInitProgress(report.progress * 100);
      });
      llmServiceRef.current = llmService;

      let embeddingService: EmbeddingService | null = null;
      try {
        embeddingService = new EmbeddingService();
        await embeddingService.initialize();
        embeddingServiceRef.current = embeddingService;
      } catch (embError) {
        console.error('Embedding initialization failed:', embError);
        embeddingService = new EmbeddingService();
      }

      const ragOrchestrator = new RAGOrchestrator(llmService, embeddingService);
      ragOrchestratorRef.current = ragOrchestrator;

      setMessages([
        {
          role: 'assistant',
          content: "Hello! I'm ready to help. Ask me anything!",
          timestamp: new Date(),
        },
      ]);
    } catch (error) {
      console.error('Initialization failed:', error);
      setMessages([
        {
          role: 'assistant',
          content: `Failed to initialize: ${error}`,
          timestamp: new Date(),
        },
      ]);
    } finally {
      setIsInitializing(false);
      setInitProgress(0);
    }
  };

  const handleSend = async () => {
    if (!input.trim() || !ragOrchestratorRef.current) return;

    const userMessage: Message = {
      role: 'user',
      content: input,
      timestamp: new Date(),
    };

    setMessages(prev => [...prev, userMessage]);
    setInput('');
    setIsLoading(true);

    try {
      const assistantMessage: Message = {
        role: 'assistant',
        content: '',
        timestamp: new Date(),
      };

      setMessages(prev => [...prev, assistantMessage]);

      let fullResponse = '';
      for await (const chunk of ragOrchestratorRef.current.queryStream(input)) {
        fullResponse += chunk;
        setMessages(prev => {
          const updated = [...prev];
          updated[updated.length - 1] = {
            ...assistantMessage,
            content: fullResponse,
          };
          return updated;
        });
      }
    } catch (error) {
      console.error('Chat failed:', error);
      setMessages(prev => [
        ...prev,
        {
          role: 'assistant',
          content: `Error: ${error}`,
          timestamp: new Date(),
        },
      ]);
    } finally {
      setIsLoading(false);
    }
  };

  const handleClear = () => {
    setMessages([]);
  };

  const isReady = ragOrchestratorRef.current?.isReady() ?? false;

  return (
    <Card
      title={
        <Space>
          <RobotOutlined />
          <Title level={4} style={{ margin: 0 }}>
            WebLLM Chat
          </Title>
        </Space>
      }
      style={{ height: '100vh', display: 'flex', flexDirection: 'column' }}
    >
      {!isReady && (
        <Space direction="vertical" style={{ width: '100%', marginBottom: 16 }}>
          <Space>
            <Text>Select Model:</Text>
            <Select
              value={selectedModel}
              onChange={setSelectedModel}
              style={{ width: 300 }}
              disabled={isInitializing}
            >
              {WebLLMService.getAvailableModels().map(model => (
                <Option key={model} value={model}>
                  {model}
                </Option>
              ))}
            </Select>
            <Button
              type="primary"
              onClick={handleInitialize}
              loading={isInitializing}
            >
              Initialize
            </Button>
          </Space>
          {isInitializing && (
            <Progress
              percent={Math.round(initProgress)}
              status="active"
              strokeColor={{ from: '#108ee9', to: '#87d068' }}
            />
          )}
        </Space>
      )}

      <ChatMessages messages={messages} />

      <ChatInput
        value={input}
        onChange={setInput}
        onSend={handleSend}
        onClear={handleClear}
        disabled={!isReady}
        loading={isLoading}
      />
    </Card>
  );
};
