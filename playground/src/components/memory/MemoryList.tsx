import { List, Space, Tag, Typography } from 'antd';

const { Title, Text } = Typography;

interface Memory {
  id: string;
  content: string;
  labels: string[];
  timestamp: Date;
}

interface MemoryListProps {
  memories: Memory[];
}

export const MemoryList = ({ memories }: MemoryListProps) => {
  return (
    <>
      <Title level={5}>Recent Memories ({memories.length})</Title>
      <List
        dataSource={memories}
        renderItem={memory => (
          <List.Item>
            <List.Item.Meta
              title={
                <Space>
                  <Text>{memory.content}</Text>
                  {memory.labels.map(label => (
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
        locale={{ emptyText: 'No memories added yet' }}
      />
    </>
  );
};
