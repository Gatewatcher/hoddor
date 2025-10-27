import { BulbOutlined } from '@ant-design/icons';
import { Card, Space, Typography } from 'antd';
import { useSelector } from 'react-redux';

import { useMemories } from '../hooks/useMemories';
import { appSelectors } from '../store/app.selectors';
import { MemoryForm } from './memory/MemoryForm';
import { MemoryList } from './memory/MemoryList';

const { Title, Text } = Typography;

export const MemoryManager = () => {
  const vaultName = useSelector(appSelectors.getSelectedVault);
  const { memories, addMemory } = useMemories();

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
          <MemoryForm onMemoryAdded={addMemory} />
          <MemoryList memories={memories} />
        </>
      )}
    </Card>
  );
};
