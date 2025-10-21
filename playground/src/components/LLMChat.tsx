import React, { useState, useEffect, useRef } from "react";
import { Card, Input, Button, Select, Space, Typography, Progress, List } from "antd";
import { SendOutlined, ClearOutlined, RobotOutlined } from "@ant-design/icons";
import { WebLLMService, RAGOrchestrator, EmbeddingService } from "../services";

const { TextArea } = Input;
const { Title, Text } = Typography;
const { Option } = Select;

interface Message {
  role: "user" | "assistant";
  content: string;
  timestamp: Date;
}

export const LLMChat: React.FC = () => {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [isInitializing, setIsInitializing] = useState(false);
  const [initProgress, setInitProgress] = useState(0);
  const [selectedModel, setSelectedModel] = useState("Phi-3.5-mini-instruct-q4f16_1-MLC");

  const llmServiceRef = useRef<WebLLMService | null>(null);
  const embeddingServiceRef = useRef<EmbeddingService | null>(null);
  const ragOrchestratorRef = useRef<RAGOrchestrator | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  const handleInitialize = async () => {
    setIsInitializing(true);
    setInitProgress(0);

    try {
      // Initialize LLM
      const llmService = new WebLLMService(selectedModel);
      await llmService.initialize((report) => {
        setInitProgress(report.progress * 100);
      });
      llmServiceRef.current = llmService;

      // Initialize Embeddings (optional for Phase 1)
      let embeddingService: EmbeddingService | null = null;
      try {
        embeddingService = new EmbeddingService();
        await embeddingService.initialize();
        embeddingServiceRef.current = embeddingService;
        console.log("Embeddings initialized successfully");
      } catch (embError) {
        console.warn("Embedding initialization failed (optional for Phase 1):", embError);
        // Create a mock embedding service that's not ready
        embeddingService = new EmbeddingService();
      }

      // Initialize RAG Orchestrator
      const ragOrchestrator = new RAGOrchestrator(llmService, embeddingService);
      ragOrchestratorRef.current = ragOrchestrator;

      setMessages([
        {
          role: "assistant",
          content: "Hello! I'm ready to help. Ask me anything!",
          timestamp: new Date(),
        },
      ]);
    } catch (error) {
      console.error("Initialization failed:", error);
      setMessages([
        {
          role: "assistant",
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
      role: "user",
      content: input,
      timestamp: new Date(),
    };

    setMessages((prev) => [...prev, userMessage]);
    setInput("");
    setIsLoading(true);

    try {
      // Start streaming response
      const assistantMessage: Message = {
        role: "assistant",
        content: "",
        timestamp: new Date(),
      };

      setMessages((prev) => [...prev, assistantMessage]);

      let fullResponse = "";
      for await (const chunk of ragOrchestratorRef.current.queryStream(input)) {
        fullResponse += chunk;
        setMessages((prev) => {
          const updated = [...prev];
          updated[updated.length - 1] = {
            ...assistantMessage,
            content: fullResponse,
          };
          return updated;
        });
      }
    } catch (error) {
      console.error("Chat failed:", error);
      setMessages((prev) => [
        ...prev,
        {
          role: "assistant",
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

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
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
      style={{ height: "100vh", display: "flex", flexDirection: "column" }}
    >
      {!isReady && (
        <Space direction="vertical" style={{ width: "100%", marginBottom: 16 }}>
          <Space>
            <Text>Select Model:</Text>
            <Select
              value={selectedModel}
              onChange={setSelectedModel}
              style={{ width: 300 }}
              disabled={isInitializing}
            >
              {WebLLMService.getAvailableModels().map((model) => (
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
              strokeColor={{ from: "#108ee9", to: "#87d068" }}
            />
          )}
        </Space>
      )}

      <div
        style={{
          flex: 1,
          overflow: "auto",
          marginBottom: 16,
          padding: 16,
          border: "1px solid #f0f0f0",
          borderRadius: 8,
        }}
      >
        <List
          dataSource={messages}
          renderItem={(msg) => (
            <List.Item
              style={{
                justifyContent: msg.role === "user" ? "flex-end" : "flex-start",
                border: "none",
              }}
            >
              <div
                style={{
                  maxWidth: "70%",
                  padding: "8px 12px",
                  borderRadius: 8,
                  backgroundColor: msg.role === "user" ? "#1890ff" : "#f0f0f0",
                  color: msg.role === "user" ? "white" : "black",
                }}
              >
                <Text
                  style={{
                    color: msg.role === "user" ? "white" : "black",
                    whiteSpace: "pre-wrap",
                  }}
                >
                  {msg.content}
                </Text>
                <div
                  style={{
                    fontSize: 10,
                    marginTop: 4,
                    opacity: 0.7,
                  }}
                >
                  {msg.timestamp.toLocaleTimeString()}
                </div>
              </div>
            </List.Item>
          )}
        />
        <div ref={messagesEndRef} />
      </div>

      <Space.Compact style={{ width: "100%" }}>
        <TextArea
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyPress={handleKeyPress}
          placeholder="Type your message... (Enter to send, Shift+Enter for new line)"
          autoSize={{ minRows: 1, maxRows: 4 }}
          disabled={!isReady || isLoading}
        />
        <Button
          type="primary"
          icon={<SendOutlined />}
          onClick={handleSend}
          loading={isLoading}
          disabled={!isReady || !input.trim()}
        >
          Send
        </Button>
        <Button icon={<ClearOutlined />} onClick={handleClear} disabled={!isReady}>
          Clear
        </Button>
      </Space.Compact>
    </Card>
  );
};
