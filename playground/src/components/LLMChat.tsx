import { RobotOutlined } from '@ant-design/icons';
import { Button, Card, Progress, Select, Space, Typography } from 'antd';
import { useDispatch, useSelector } from 'react-redux';

import { useChatMessages } from '../hooks/useChatMessages';
import { useServiceInitialization } from '../hooks/useServiceInitialization';
import { WebLLMService } from '../services';
import { actions } from '../store/app.actions';
import { appSelectors } from '../store/app.selectors';
import { ChatInput } from './chat/ChatInput';
import { ChatMessages } from './chat/ChatMessages';

const { Title, Text } = Typography;
const { Option } = Select;

export const LLMChat = () => {
  const {
    messages,
    input,
    isLoading,
    setInput,
    sendMessage,
    clearMessages,
    addMessage,
  } = useChatMessages({ enableRAG: false });
  const { isInitializing, initProgress, initialize } =
    useServiceInitialization();

  const selectedModel = useSelector(appSelectors.getSelectedModel);
  const dispatch = useDispatch();

  const handleInitialize = async () => {
    try {
      await initialize(() => {
        addMessage({
          role: 'assistant',
          content: "Hello! I'm ready to help. Ask me anything!",
          timestamp: new Date(),
        });
      });
    } catch (error) {
      addMessage({
        role: 'assistant',
        content: `Failed to initialize: ${error}`,
        timestamp: new Date(),
      });
    }
  };

  const isReady = messages.length > 0;

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
              onChange={model => dispatch(actions.setSelectedModel(model))}
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
        onSend={sendMessage}
        onClear={clearMessages}
        disabled={!isReady}
        loading={isLoading}
      />
    </Card>
  );
};
