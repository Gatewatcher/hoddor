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
import React, { useRef, useState } from 'react';
import { useDispatch, useSelector } from 'react-redux';

import {
  create_credential,
  get_credential,
  graph_backup_vault,
  graph_restore_vault,
  vault_identity_from_passphrase,
} from '../../../hoddor/pkg/hoddor';
import { EmbeddingService, RAGOrchestrator, WebLLMService } from '../services';
import { actions } from '../store/app.actions';
import { appSelectors } from '../store/app.selectors';
import { ChatInput } from './chat/ChatInput';
import { ChatMessages } from './chat/ChatMessages';
import { MemoryManager } from './MemoryManager';
import { GraphAuthModal } from './rag/GraphAuthModal';
import { RAGControls } from './rag/RAGControls';

const { Title, Text } = Typography;
const { Option } = Select;

interface Message {
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
}

export const RAGWorkspace: React.FC = () => {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [isInitializing, setIsInitializing] = useState(false);
  const [initProgress, setInitProgress] = useState(0);
  const [selectedModel, setSelectedModel] = useState(
    'Phi-3.5-mini-instruct-q4f16_1-MLC',
  );
  const [selectedVault, setSelectedVault] = useState<string>('');
  const [useRAG, setUseRAG] = useState(true);
  const [servicesReady, setServicesReady] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [isRestoring, setIsRestoring] = useState(false);
  const [memoryRefreshTrigger, setMemoryRefreshTrigger] = useState(0);

  // Authentication state
  const [isAuthModalOpen, setIsAuthModalOpen] = useState(false);
  const [authMode, setAuthMode] = useState<
    'passphrase' | 'mfa-register' | 'mfa-auth'
  >('passphrase');

  const identity = useSelector(appSelectors.getIdentity);
  const dispatch = useDispatch();
  const [messageApi, contextHolder] = message.useMessage();

  const llmServiceRef = useRef<WebLLMService | null>(null);
  const embeddingServiceRef = useRef<EmbeddingService | null>(null);
  const ragOrchestratorRef = useRef<RAGOrchestrator | null>(null);

  const handleInitialize = async () => {
    setIsInitializing(true);
    setInitProgress(0);

    try {
      // Initialize LLM
      const llmService = new WebLLMService(selectedModel);
      await llmService.initialize(report => {
        setInitProgress(report.progress * 100);
      });
      llmServiceRef.current = llmService;

      // Initialize Embeddings
      let embeddingService: EmbeddingService | null = null;
      try {
        embeddingService = new EmbeddingService();
        await embeddingService.initialize();
        embeddingServiceRef.current = embeddingService;
      } catch (embError) {
        console.error('Embedding initialization failed:', embError);
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
          role: 'assistant',
          content: embeddingsReady
            ? "✅ Hello! I'm ready to help. You can add memories and I'll use them to answer questions!"
            : "⚠️ Hello! I'm ready to help. Embeddings unavailable (CDN issue) - you can chat without RAG for now.",
          timestamp: new Date(),
        },
      ]);
    } catch (error) {
      console.error('Initialization failed:', error);
      setMessages([
        {
          role: 'assistant',
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
      role: 'user',
      content: input,
      timestamp: new Date(),
    };

    setMessages(prev => [...prev, userMessage]);
    setInput('');
    setIsLoading(true);

    try {
      const assistantMessage: Message = {
        role: 'assistant',
        content: '',
        timestamp: new Date(),
      };

      setMessages(prev => [...prev, assistantMessage]);

      // Pass vault name if RAG is enabled and vault is selected
      const options =
        useRAG && selectedVault ? { vaultName: selectedVault } : {};

      let fullResponse = '';
      for await (const chunk of ragOrchestratorRef.current.queryStream(
        input,
        options,
      )) {
        fullResponse += chunk;
        setMessages(prev => {
          const updated = [...prev];
          updated[updated.length - 1] = {
            ...assistantMessage,
            content: fullResponse,
          };
          return updated;
        });
      }
    } catch (error) {
      console.error('Chat failed:', error);
      setMessages(prev => [
        ...prev,
        {
          role: 'assistant',
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

  const handleSaveGraph = async () => {
    if (!selectedVault) {
      messageApi.warning('Please select a vault first');
      return;
    }

    if (!identity) {
      messageApi.error('Please authenticate first (Passphrase or MFA)');
      return;
    }

    if (!identity.public_key || !identity.private_key) {
      messageApi.error('Identity is incomplete - please authenticate again');
      return;
    }

    setIsSaving(true);
    try {
      await graph_backup_vault(
        selectedVault,
        identity.public_key(),
        identity.private_key(),
      );
      messageApi.success(`Graph saved to OPFS for vault: ${selectedVault}`);
    } catch (error) {
      console.error('Failed to save graph:', error);
      messageApi.error(`Failed to save graph: ${error}`);
    } finally {
      setIsSaving(false);
    }
  };

  const handleLoadGraph = async () => {
    if (!selectedVault) {
      messageApi.warning('Please select a vault first');
      return;
    }

    if (!identity) {
      messageApi.error('Please authenticate first (Passphrase or MFA)');
      return;
    }

    if (!identity.public_key || !identity.private_key) {
      messageApi.error('Identity is incomplete - please authenticate again');
      return;
    }

    setIsRestoring(true);
    try {
      const found = await graph_restore_vault(
        selectedVault,
        identity.public_key(),
        identity.private_key(),
      );
      if (found) {
        messageApi.success(`Graph loaded from OPFS for vault: ${selectedVault}`);
        setMemoryRefreshTrigger(prev => prev + 1);
      } else {
        messageApi.info('No saved graph found (this is the first time)');
      }
    } catch (error) {
      console.error('Failed to load graph:', error);
      messageApi.error(`Failed to load graph: ${error}`);
    } finally {
      setIsRestoring(false);
    }
  };

  const handlePassphraseAuth = async (values: { passphrase: string }) => {
    if (!selectedVault) {
      messageApi.warning('Please select a vault first');
      return;
    }

    try {
      const identityHandle = await vault_identity_from_passphrase(
        values.passphrase,
        selectedVault,
      );
      dispatch(actions.addIdentity(identityHandle.to_json()));
      messageApi.success('Authenticated successfully!');
      setIsAuthModalOpen(false);
    } catch (error) {
      console.error('Passphrase auth failed:', error);
      messageApi.error(`Authentication failed: ${error}`);
    }
  };

  const handleMFARegister = async (values: { username: string }) => {
    if (!selectedVault) {
      messageApi.warning('Please select a vault first');
      return;
    }

    try {
      const identityHandle = await create_credential(
        selectedVault,
        values.username,
      );
      dispatch(actions.addIdentity(identityHandle.to_json()));
      messageApi.success('MFA registered successfully!');
      setIsAuthModalOpen(false);
    } catch (error) {
      console.error('MFA register failed:', error);
      messageApi.error(`MFA registration failed: ${error}`);
    }
  };

  const handleMFAAuth = async (values: { username: string }) => {
    if (!selectedVault) {
      messageApi.warning('Please select a vault first');
      return;
    }

    try {
      const identityHandle = await get_credential(
        selectedVault,
        values.username,
      );
      dispatch(actions.addIdentity(identityHandle.to_json()));
      messageApi.success('Authenticated successfully!');
      setIsAuthModalOpen(false);
    } catch (error) {
      console.error('MFA auth failed:', error);
      messageApi.error(`MFA authentication failed: ${error}`);
    }
  };

  const openAuthModal = (mode: 'passphrase' | 'mfa-register' | 'mfa-auth') => {
    setAuthMode(mode);
    setIsAuthModalOpen(true);
  };

  const isReady = ragOrchestratorRef.current?.isReady() ?? false;
  const canUseRAG = embeddingServiceRef.current?.isReady() ?? false;

  return (
    <>
      {contextHolder}
      <div style={{ padding: 16, height: '89vh', overflow: 'auto' }}>
        <Row gutter={16} style={{ height: '100%' }}>
        <Col span={10} style={{ height: '100%' }}>
          {servicesReady && embeddingServiceRef.current ? (
            <MemoryManager
              vaultName={selectedVault}
              embeddingService={embeddingServiceRef.current}
              refreshTrigger={memoryRefreshTrigger}
              onMemoryAdded={() => {}}
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
            style={{ height: '100%', display: 'flex', flexDirection: 'column' }}
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
                    onChange={setSelectedModel}
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
                selectedVault={selectedVault}
                onVaultChange={setSelectedVault}
                useRAG={useRAG}
                onRAGChange={setUseRAG}
                canUseRAG={canUseRAG}
                isAuthenticated={!!identity}
                onAuthPassphrase={() => openAuthModal('passphrase')}
                onAuthMFARegister={() => openAuthModal('mfa-register')}
                onAuthMFALogin={() => openAuthModal('mfa-auth')}
                onSaveGraph={handleSaveGraph}
                onLoadGraph={handleLoadGraph}
                isSaving={isSaving}
                isRestoring={isRestoring}
              />
            )}

            <ChatMessages messages={messages} />

            <ChatInput
              value={input}
              onChange={setInput}
              onSend={handleSend}
              onClear={handleClear}
              disabled={!isReady}
              loading={isLoading}
            />
          </Card>
        </Col>
      </Row>

      <GraphAuthModal
        isOpen={isAuthModalOpen}
        authMode={authMode}
        onClose={() => setIsAuthModalOpen(false)}
        onPassphraseAuth={handlePassphraseAuth}
        onMFARegister={handleMFARegister}
        onMFAAuth={handleMFAAuth}
        onAuthModeChange={setAuthMode}
      />
      </div>
    </>
  );
};
