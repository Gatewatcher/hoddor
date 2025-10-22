import React, { useState, useEffect, useRef } from "react";
import {
  Row,
  Col,
  Card,
  Input,
  Button,
  Select,
  Space,
  Typography,
  Progress,
  List,
  Checkbox,
  Divider,
} from "antd";
import { SendOutlined, ClearOutlined, RobotOutlined, BulbOutlined } from "@ant-design/icons";
import { WebLLMService, RAGOrchestrator, EmbeddingService } from "../services";
import { MemoryManager } from "./MemoryManager";

const { TextArea } = Input;
const { Title, Text } = Typography;
const { Option } = Select;

interface Message {
  role: "user" | "assistant";
  content: string;
  timestamp: Date;
}

export const RAGWorkspace: React.FC = () => {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const [isInitializing, setIsInitializing] = useState(false);
  const [initProgress, setInitProgress] = useState(0);
  const [selectedModel, setSelectedModel] = useState("Phi-3.5-mini-instruct-q4f16_1-MLC");
  const [selectedVault, setSelectedVault] = useState<string>("");
  const [useRAG, setUseRAG] = useState(true);
  const [servicesReady, setServicesReady] = useState(false);

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

      // Initialize Embeddings
      let embeddingService: EmbeddingService | null = null;
      try {
        embeddingService = new EmbeddingService();
        await embeddingService.initialize();
        embeddingServiceRef.current = embeddingService;
        console.log("Embeddings initialized successfully");
      } catch (embError) {
        console.warn("Embedding initialization failed:", embError);
        embeddingService = new EmbeddingService();
        embeddingServiceRef.current = embeddingService;
      }

      // Initialize RAG Orchestrator
      const ragOrchestrator = new RAGOrchestrator(llmService, embeddingService);
      ragOrchestratorRef.current = ragOrchestrator;

      // Force re-render to update MemoryManager
      setServicesReady(true);

      const embeddingsReady = embeddingServiceRef.current?.isReady();

      setMessages([
        {
          role: "assistant",
          content: embeddingsReady
            ? "✅ Hello! I'm ready to help. You can add memories and I'll use them to answer questions!"
            : "⚠️ Hello! I'm ready to help. Embeddings unavailable (CDN issue) - you can chat without RAG for now.",
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
      const assistantMessage: Message = {
        role: "assistant",
        content: "",
        timestamp: new Date(),
      };

      setMessages((prev) => [...prev, assistantMessage]);

      // Pass vault name if RAG is enabled and vault is selected
      const options = useRAG && selectedVault ? { vaultName: selectedVault } : {};

      let fullResponse = "";
      for await (const chunk of ragOrchestratorRef.current.queryStream(input, options)) {
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
  const canUseRAG = embeddingServiceRef.current?.isReady() ?? false;

  return (
    <div style={{ padding: 16, height: "100vh", overflow: "auto" }}>
      <Row gutter={16} style={{ height: "100%" }}>
        {/* Left Column: Memory Manager */}
        <Col span={10} style={{ height: "100%" }}>
          {servicesReady && embeddingServiceRef.current ? (
            <MemoryManager
              vaultName={selectedVault}
              embeddingService={embeddingServiceRef.current}
              onMemoryAdded={() => {
                console.log("Memory added!");
              }}
            />
          ) : (
            <Card
              title={
                <Space>
                  <BulbOutlined />
                  <Title level={4} style={{ margin: 0 }}>
                    Memory Manager
                  </Title>
                </Space>
              }
            >
              <Space direction="vertical">
                <Text type="secondary">
                  Please initialize the services in the chat panel first →
                </Text>
                <Text type="secondary" style={{ fontSize: 12 }}>
                  Note: If embeddings fail to load (CDN issue), you can still use direct chat.
                </Text>
              </Space>
            </Card>
          )}
        </Col>

        {/* Right Column: Chat */}
        <Col span={14} style={{ height: "100%" }}>
          <Card
            title={
              <Space>
                <RobotOutlined />
                <Title level={4} style={{ margin: 0 }}>
                  RAG Chat
                </Title>
              </Space>
            }
            style={{ height: "100%", display: "flex", flexDirection: "column" }}
          >
            {!isReady && (
              <Space direction="vertical" style={{ width: "100%", marginBottom: 16 }}>
                <Space>
                  <Text>Model:</Text>
                  <Select
                    value={selectedModel}
                    onChange={setSelectedModel}
                    style={{ width: 250 }}
                    disabled={isInitializing}
                  >
                    {WebLLMService.getAvailableModels().map((model) => (
                      <Option key={model} value={model}>
                        {model}
                      </Option>
                    ))}
                  </Select>
                  <Button type="primary" onClick={handleInitialize} loading={isInitializing}>
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

            {isReady && (
              <>
                <Space style={{ marginBottom: 16 }}>
                  <Text>Vault:</Text>
                  <Input
                    value={selectedVault}
                    onChange={(e) => setSelectedVault(e.target.value)}
                    placeholder="Enter vault name (e.g., 'my-vault')"
                    style={{ width: 200 }}
                  />
                  <Checkbox checked={useRAG} onChange={(e) => setUseRAG(e.target.checked)}>
                    Use RAG
                  </Checkbox>
                  {useRAG && !canUseRAG && (
                    <Text type="warning" style={{ fontSize: 12 }}>
                      (Embeddings unavailable)
                    </Text>
                  )}
                </Space>
                <Divider style={{ margin: "8px 0" }} />
              </>
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
                      <div style={{ fontSize: 10, marginTop: 4, opacity: 0.7 }}>
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
                placeholder="Ask a question... (Enter to send)"
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
        </Col>
      </Row>
    </div>
  );
};
