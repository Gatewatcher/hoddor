import React, { useState, useEffect } from "react";
import { Card, Input, Button, Space, Typography, List, Tag, message } from "antd";
import { PlusOutlined, BulbOutlined, ReloadOutlined } from "@ant-design/icons";
import { EmbeddingService } from "../services";
import { graph_create_memory_node, graph_list_memory_nodes } from "../../../hoddor/pkg/hoddor";

const { TextArea } = Input;
const { Title, Text } = Typography;

interface Memory {
  id: string;
  content: string;
  labels: string[];
  timestamp: Date;
}

interface MemoryManagerProps {
  vaultName?: string;
  embeddingService: EmbeddingService | null;
  onMemoryAdded?: () => void;
  refreshTrigger?: number; // Change this to trigger reload from graph
}

export const MemoryManager: React.FC<MemoryManagerProps> = ({
  vaultName,
  embeddingService,
  onMemoryAdded,
  refreshTrigger,
}) => {
  const [memories, setMemories] = useState<Memory[]>([]);
  const [newMemory, setNewMemory] = useState("");
  const [labels, setLabels] = useState("");
  const [isAdding, setIsAdding] = useState(false);
  const [isLoading, setIsLoading] = useState(false);

  // Load memories from graph when vault changes or refresh is triggered
  useEffect(() => {
    const loadMemories = async () => {
      if (!vaultName) {
        setMemories([]);
        return;
      }

      setIsLoading(true);
      try {
        console.log("📂 Loading memories from graph for vault:", vaultName);
        const nodes = await graph_list_memory_nodes(vaultName, 100);

        const decoder = new TextDecoder();
        const loadedMemories: Memory[] = nodes.map((node: any) => {
          let content = "";
          try {
            if (node.encrypted_content && node.encrypted_content.length > 0) {
              content = decoder.decode(new Uint8Array(node.encrypted_content));
            }
          } catch (error) {
            console.warn(`Failed to decode memory ${node.id}:`, error);
            content = "[Unable to decode content]";
          }

          return {
            id: node.id,
            content,
            labels: node.labels || [],
            timestamp: new Date(), // We don't store timestamps in graph yet
          };
        });

        console.log(`✅ Loaded ${loadedMemories.length} memories from graph`);
        setMemories(loadedMemories);
      } catch (error) {
        console.error("Failed to load memories:", error);
        // Don't show error message - might just be empty graph
      } finally {
        setIsLoading(false);
      }
    };

    loadMemories();
  }, [vaultName, refreshTrigger]);

  const handleAddMemory = async () => {
    if (!newMemory.trim()) {
      message.warning("Please enter memory content");
      return;
    }

    if (!vaultName) {
      message.warning("Please select a vault first");
      return;
    }

    if (!embeddingService || !embeddingService.isReady()) {
      message.error("Embedding service not ready");
      return;
    }

    setIsAdding(true);

    try {
      // Generate embedding
      const { embedding } = await embeddingService.embed(newMemory);

      // For now, we'll use simple encryption (just encode to bytes)
      // TODO: Integrate with Age encryption from vault identity
      const encoder = new TextEncoder();
      const contentBytes = encoder.encode(newMemory);

      // Simple HMAC placeholder (should use proper crypto)
      const hmac = await crypto.subtle.digest("SHA-256", contentBytes);
      const hmacHex = Array.from(new Uint8Array(hmac))
        .map((b) => b.toString(16).padStart(2, "0"))
        .join("");

      // Parse labels
      const labelList = labels
        .split(",")
        .map((l) => l.trim())
        .filter((l) => l.length > 0);

      // Create memory node in graph
      const nodeId = await graph_create_memory_node(
        vaultName,
        contentBytes,
        hmacHex,
        new Float32Array(embedding),
        labelList
      );

      // Add to local list
      const memory: Memory = {
        id: nodeId,
        content: newMemory,
        labels: labelList,
        timestamp: new Date(),
      };

      setMemories([memory, ...memories]);
      setNewMemory("");
      setLabels("");

      message.success("Memory added to graph!");
      onMemoryAdded?.();
    } catch (error) {
      console.error("Failed to add memory:", error);
      message.error(`Failed to add memory: ${error}`);
    } finally {
      setIsAdding(false);
    }
  };

  return (
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
      {!vaultName && (
        <Text type="warning">
          Please select a vault to start adding memories
        </Text>
      )}

      {vaultName && (
        <>
          <Space direction="vertical" style={{ width: "100%", marginBottom: 16 }}>
            <Text strong>Vault: {vaultName}</Text>
            <TextArea
              value={newMemory}
              onChange={(e) => setNewMemory(e.target.value)}
              placeholder="Enter a memory to store in the graph (e.g., 'My favorite color is blue')"
              autoSize={{ minRows: 3, maxRows: 6 }}
              disabled={isAdding}
            />
            <Input
              value={labels}
              onChange={(e) => setLabels(e.target.value)}
              placeholder="Labels (comma-separated, e.g., 'personal, preferences')"
              disabled={isAdding}
            />
            <Button
              type="primary"
              icon={<PlusOutlined />}
              onClick={handleAddMemory}
              loading={isAdding}
              disabled={!embeddingService || !embeddingService.isReady()}
            >
              Add Memory to Graph
            </Button>
            {(!embeddingService || !embeddingService.isReady()) && (
              <Text type="warning" style={{ fontSize: 12 }}>
                ⚠️ Embeddings unavailable (CDN issue). RAG features disabled.
                <br />
                You can still use the LLM for direct chat without memory context.
              </Text>
            )}
          </Space>

          <Title level={5}>Recent Memories ({memories.length})</Title>
          <List
            dataSource={memories}
            renderItem={(memory) => (
              <List.Item>
                <List.Item.Meta
                  title={
                    <Space>
                      <Text>{memory.content}</Text>
                      {memory.labels.map((label) => (
                        <Tag key={label} color="blue">
                          {label}
                        </Tag>
                      ))}
                    </Space>
                  }
                  description={
                    <Space direction="vertical" size={0}>
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        ID: {memory.id}
                      </Text>
                      <Text type="secondary" style={{ fontSize: 12 }}>
                        Added: {memory.timestamp.toLocaleString()}
                      </Text>
                    </Space>
                  }
                />
              </List.Item>
            )}
            locale={{ emptyText: "No memories added yet" }}
          />
        </>
      )}
    </Card>
  );
};
