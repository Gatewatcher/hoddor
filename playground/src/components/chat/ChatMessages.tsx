import { List, Typography } from 'antd';
import { useEffect, useRef } from 'react';

const { Text } = Typography;

interface Message {
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
}

interface ChatMessagesProps {
  messages: Message[];
}

export const ChatMessages = ({ messages }: ChatMessagesProps) => {
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  return (
    <div
      style={{
        flex: 1,
        overflow: 'auto',
        marginBottom: 16,
        padding: 16,
        border: '1px solid #f0f0f0',
        borderRadius: 8,
      }}
    >
      <List
        dataSource={messages}
        renderItem={msg => (
          <List.Item
            style={{
              justifyContent: msg.role === 'user' ? 'flex-end' : 'flex-start',
              border: 'none',
            }}
          >
            <div
              style={{
                maxWidth: '70%',
                padding: '8px 12px',
                borderRadius: 8,
                backgroundColor: msg.role === 'user' ? '#1890ff' : '#f0f0f0',
                color: msg.role === 'user' ? 'white' : 'black',
              }}
            >
              <Text
                style={{
                  color: msg.role === 'user' ? 'white' : 'black',
                  whiteSpace: 'pre-wrap',
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
  );
};
