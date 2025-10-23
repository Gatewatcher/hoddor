import { BulbOutlined } from '@ant-design/icons';
import { Card, Space, Typography } from 'antd';
import React, { useEffect, useState } from 'react';
import { useSelector } from 'react-redux';

import { graph_list_memory_nodes } from '../../../hoddor/pkg/hoddor';
import { appSelectors } from '../store/app.selectors';
import { MemoryForm } from './memory/MemoryForm';
import { MemoryList } from './memory/MemoryList';

const { Title, Text } = Typography;

interface Memory {
  id: string;
  content: string;
  labels: string[];
  timestamp: Date;
}

export const MemoryManager: React.FC = () => {
  const vaultName = useSelector(appSelectors.getSelectedVault);
  const refreshTrigger = useSelector(appSelectors.getMemoryRefreshTrigger);
  const [memories, setMemories] = useState<Memory[]>([]);
  const [, setIsLoading] = useState(false);

  // Load memories from graph when vault changes or refresh is triggered
  useEffect(() => {
    const loadMemories = async () => {
      if (!vaultName) {
        setMemories([]);
        return;
      }

      setIsLoading(true);
      try {
        const nodes = await graph_list_memory_nodes(vaultName, 100);

        const decoder = new TextDecoder();
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        const loadedMemories: Memory[] = nodes.map((node: any) => {
          let content = '';
          try {
            if (node.encrypted_content && node.encrypted_content.length > 0) {
              content = decoder.decode(new Uint8Array(node.encrypted_content));
            }
          } catch (error) {
            console.error('Failed to decode memory:', error);
            content = '[Unable to decode content]';
          }

          return {
            id: node.id,
            content,
            labels: node.labels || [],
            timestamp: new Date(),
          };
        });

        setMemories(loadedMemories);
      } catch (error) {
        console.error('Failed to load memories:', error);
      } finally {
        setIsLoading(false);
      }
    };

    loadMemories();
  }, [vaultName, refreshTrigger]);

  const handleMemoryAdded = (memory: Memory) => {
    setMemories([memory, ...memories]);
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
          <MemoryForm onMemoryAdded={handleMemoryAdded} />
          <MemoryList memories={memories} />
        </>
      )}
    </Card>
  );
};
