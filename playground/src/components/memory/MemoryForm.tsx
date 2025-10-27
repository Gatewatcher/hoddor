import { PlusOutlined } from '@ant-design/icons';
import { Button, Input, Space, Typography, message } from 'antd';
import { useState } from 'react';
import { useSelector } from 'react-redux';

import { graph_create_memory_node } from '../../../../hoddor/pkg/hoddor';
import { useServices } from '../../contexts/ServicesContext';
import { appSelectors } from '../../store/app.selectors';
import {
  createMemory,
  encodeContent,
  parseLabels,
} from '../../utils/memoryUtils';

const { TextArea } = Input;
const { Text } = Typography;

interface MemoryFormProps {
  onMemoryAdded: (memory: {
    id: string;
    content: string;
    labels: string[];
    timestamp: Date;
  }) => void;
}

export const MemoryForm = ({ onMemoryAdded }: MemoryFormProps) => {
  const vaultName = useSelector(appSelectors.getSelectedVault);
  const { embeddingService } = useServices();
  const [newMemory, setNewMemory] = useState('');
  const [labels, setLabels] = useState('');
  const [isAdding, setIsAdding] = useState(false);
  const [messageApi, contextHolder] = message.useMessage();

  const handleAddMemory = async () => {
    if (!newMemory.trim()) {
      messageApi.warning('Please enter memory content');
      return;
    }

    if (!vaultName) {
      messageApi.warning('Please select a vault first');
      return;
    }

    if (!embeddingService || !embeddingService.isReady()) {
      messageApi.error('Embedding service not ready');
      return;
    }

    setIsAdding(true);

    try {
      const { embedding } = await embeddingService.embed(newMemory);

      const contentBytes = encodeContent(newMemory);
      const labelList = parseLabels(labels);

      const nodeId = await graph_create_memory_node(
        vaultName,
        contentBytes,
        new Float32Array(embedding),
        labelList,
      );

      const memory = createMemory(nodeId, newMemory, labelList);
      onMemoryAdded(memory);
      setNewMemory('');
      setLabels('');

      messageApi.success('Memory added to graph!');
    } catch (error) {
      console.error('Failed to add memory:', error);
      messageApi.error(`Failed to add memory: ${error}`);
    } finally {
      setIsAdding(false);
    }
  };

  return (
    <>
      {contextHolder}
      <Space direction="vertical" style={{ width: '100%', marginBottom: 16 }}>
        <Text strong>Vault: {vaultName}</Text>
        <TextArea
          value={newMemory}
          onChange={e => setNewMemory(e.target.value)}
          placeholder="Enter a memory to store in the graph (e.g., 'My favorite color is blue')"
          autoSize={{ minRows: 3, maxRows: 6 }}
          disabled={isAdding}
        />
        <Input
          value={labels}
          onChange={e => setLabels(e.target.value)}
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
    </>
  );
};
