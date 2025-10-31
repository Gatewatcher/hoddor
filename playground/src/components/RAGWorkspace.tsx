import { BulbOutlined, RobotOutlined } from '@ant-design/icons';
import {
  Button,
  Card,
  Col,
  Progress,
  Row,
  Select,
  Space,
  Typography,
  message,
} from 'antd';
import { useDispatch, useSelector } from 'react-redux';

import { useServices } from '../contexts/ServicesContext';
import { useChatMessages } from '../hooks/useChatMessages';
import { useGraphAuthentication } from '../hooks/useGraphAuthentication';
import { useGraphPersistence } from '../hooks/useGraphPersistence';
import { useServiceInitialization } from '../hooks/useServiceInitialization';
import { WebLLMService } from '../services';
import { actions } from '../store/app.actions';
import { appSelectors } from '../store/app.selectors';
import { MemoryManager } from './MemoryManager';
import { ChatInput } from './chat/ChatInput';
import { ChatMessages } from './chat/ChatMessages';
import { GraphAuthModal } from './rag/GraphAuthModal';
import { RAGControls } from './rag/RAGControls';

const { Title, Text } = Typography;
const { Option } = Select;

export const RAGWorkspace = () => {
  // Single message API instance
  const [messageApi, contextHolder] = message.useMessage();

  // Custom hooks
  const {
    messages,
    input,
    isLoading,
    setInput,
    sendMessage,
    clearMessages,
    addMessage,
  } = useChatMessages({ enableRAG: true });

  const { isInitializing, initProgress, initialize } =
    useServiceInitialization();

  const {
    isAuthModalOpen,
    authMode,
    handlePassphraseAuth,
    handleMFARegister,
    handleMFAAuth,
    openAuthModal,
    closeAuthModal,
    setAuthMode,
  } = useGraphAuthentication(messageApi);

  const { isSaving, isRestoring, saveGraph, loadGraph } =
    useGraphPersistence(messageApi);

  const selectedVault = useSelector(appSelectors.getSelectedVault);
  const selectedModel = useSelector(appSelectors.getSelectedModel);
  const withRAG = useSelector(appSelectors.getWithRAG);
  const withGraphRAG = useSelector(appSelectors.getWithGraphRAG);
  const servicesReady = useSelector(appSelectors.getServicesReady);
  const identity = useSelector(appSelectors.getIdentity);

  const dispatch = useDispatch();

  const { ragOrchestrator, embeddingService } = useServices();

  const handleInitialize = async () => {
    try {
      await initialize(embeddingsReady => {
        addMessage({
          role: 'assistant',
          content: embeddingsReady
            ? "✅ Hello! I'm ready to help. You can add memories and I'll use them to answer questions!"
            : "⚠️ Hello! I'm ready to help. Embeddings unavailable (CDN issue) - you can chat without RAG for now.",
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

  const isReady = ragOrchestrator?.isReady() ?? false;
  const canUseRAG = embeddingService?.isReady() ?? false;

  return (
    <>
      {contextHolder}
      <div style={{ padding: 16, height: '89vh', overflow: 'auto' }}>
        <Row gutter={16} style={{ height: '100%' }}>
          <Col span={10} style={{ height: '100%' }}>
            {servicesReady && embeddingService ? (
              <MemoryManager />
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
                    Note: If embeddings fail to load (CDN issue), you can still
                    use direct chat.
                  </Text>
                </Space>
              </Card>
            )}
          </Col>

          {/* Right Column: Chat */}
          <Col span={14} style={{ height: '100%' }}>
            <Card
              title={
                <Space>
                  <RobotOutlined />
                  <Title level={4} style={{ margin: 0 }}>
                    RAG Chat
                  </Title>
                </Space>
              }
              style={{
                height: '100%',
                display: 'flex',
                flexDirection: 'column',
              }}
            >
              {!isReady && (
                <Space
                  direction="vertical"
                  style={{ width: '100%', marginBottom: 16 }}
                >
                  <Space>
                    <Text>Model:</Text>
                    <Select
                      value={selectedModel}
                      onChange={model =>
                        dispatch(actions.setSelectedModel(model))
                      }
                      style={{ width: 250 }}
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

              {isReady && (
                <RAGControls
                  selectedVault={selectedVault || ''}
                  onVaultChange={vault => dispatch(actions.selectVault(vault))}
                  withRAG={withRAG}
                  onRAGChange={use => dispatch(actions.setUseRAG(use))}
                  withGraphRAG={withGraphRAG}
                  onGraphRAGChange={use =>
                    dispatch(actions.setUseGraphRAG(use))
                  }
                  canUseRAG={canUseRAG}
                  embeddingService={embeddingService}
                  isAuthenticated={!!identity}
                  onAuthPassphrase={() => openAuthModal('passphrase')}
                  onAuthMFARegister={() => openAuthModal('mfa-register')}
                  onAuthMFALogin={() => openAuthModal('mfa-auth')}
                  onSaveGraph={saveGraph}
                  onLoadGraph={loadGraph}
                  isSaving={isSaving}
                  isRestoring={isRestoring}
                />
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
          </Col>
        </Row>

        <GraphAuthModal
          isOpen={isAuthModalOpen}
          authMode={authMode}
          onClose={closeAuthModal}
          onPassphraseAuth={handlePassphraseAuth}
          onMFARegister={handleMFARegister}
          onMFAAuth={handleMFAAuth}
          onAuthModeChange={setAuthMode}
        />
      </div>
    </>
  );
};
