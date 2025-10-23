import {
  BulbOutlined,
  ClearOutlined,
  FolderOpenOutlined,
  LockOutlined,
  RobotOutlined,
  SaveOutlined,
  SendOutlined,
  UnlockOutlined,
} from '@ant-design/icons';
import {
  Button,
  Card,
  Checkbox,
  Col,
  Divider,
  Form,
  Input,
  List,
  Modal,
  Progress,
  Row,
  Select,
  Space,
  Tag,
  Typography,
  message,
} from 'antd';
import React, { useEffect, useRef, useState } from 'react';
import { useSelector } from 'react-redux';
import { useDispatch } from 'react-redux';

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
import { MemoryManager } from './MemoryManager';

const { TextArea } = Input;
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
  const [passphraseForm] = Form.useForm();
  const [mfaForm] = Form.useForm();

  const identity = useSelector(appSelectors.getIdentity);
  const dispatch = useDispatch();
  const [messageApi, contextHolder] = message.useMessage();

  const llmServiceRef = useRef<WebLLMService | null>(null);
  const embeddingServiceRef = useRef<EmbeddingService | null>(null);
  const ragOrchestratorRef = useRef<RAGOrchestrator | null>(null);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
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

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
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
      passphraseForm.resetFields();
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
      mfaForm.resetFields();
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
      mfaForm.resetFields();
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
              <>
                <Space style={{ marginBottom: 16 }} wrap>
                  <Text>Vault:</Text>
                  <Input
                    value={selectedVault}
                    onChange={e => setSelectedVault(e.target.value)}
                    placeholder="Enter vault name (e.g., 'my-vault')"
                    style={{ width: 200 }}
                  />
                  <Checkbox
                    checked={useRAG}
                    onChange={e => setUseRAG(e.target.checked)}
                  >
                    Use RAG
                  </Checkbox>
                  {useRAG && !canUseRAG && (
                    <Text type="warning" style={{ fontSize: 12 }}>
                      (Embeddings unavailable)
                    </Text>
                  )}

                  {/* Authentication Status & Controls */}
                  <Divider type="vertical" />
                  {identity ? (
                    <Tag icon={<UnlockOutlined />} color="success">
                      Authenticated
                    </Tag>
                  ) : (
                    <Space.Compact>
                      <Button
                        icon={<LockOutlined />}
                        onClick={() => openAuthModal('passphrase')}
                        size="small"
                      >
                        Passphrase
                      </Button>
                      <Button
                        icon={<LockOutlined />}
                        onClick={() => openAuthModal('mfa-register')}
                        size="small"
                      >
                        MFA Register
                      </Button>
                      <Button
                        icon={<LockOutlined />}
                        onClick={() => openAuthModal('mfa-auth')}
                        size="small"
                      >
                        MFA Login
                      </Button>
                    </Space.Compact>
                  )}

                  <Button
                    icon={<SaveOutlined />}
                    onClick={handleSaveGraph}
                    loading={isSaving}
                    disabled={!selectedVault || !identity}
                    type="primary"
                  >
                    Save Graph
                  </Button>
                  <Button
                    icon={<FolderOpenOutlined />}
                    onClick={handleLoadGraph}
                    loading={isRestoring}
                    disabled={!selectedVault || !identity}
                  >
                    Load Graph
                  </Button>
                </Space>
                <Divider style={{ margin: '8px 0' }} />
              </>
            )}

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
                      justifyContent:
                        msg.role === 'user' ? 'flex-end' : 'flex-start',
                      border: 'none',
                    }}
                  >
                    <div
                      style={{
                        maxWidth: '70%',
                        padding: '8px 12px',
                        borderRadius: 8,
                        backgroundColor:
                          msg.role === 'user' ? '#1890ff' : '#f0f0f0',
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
                      <div style={{ fontSize: 10, marginTop: 4, opacity: 0.7 }}>
                        {msg.timestamp.toLocaleTimeString()}
                      </div>
                    </div>
                  </List.Item>
                )}
              />
              <div ref={messagesEndRef} />
            </div>

            <Space.Compact style={{ width: '100%' }}>
              <TextArea
                value={input}
                onChange={e => setInput(e.target.value)}
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
              <Button
                icon={<ClearOutlined />}
                onClick={handleClear}
                disabled={!isReady}
              >
                Clear
              </Button>
            </Space.Compact>
          </Card>
        </Col>
      </Row>

      {/* Authentication Modal */}
      <Modal
        title={
          authMode === 'passphrase'
            ? 'Authenticate with Passphrase'
            : authMode === 'mfa-register'
            ? 'Register MFA Credential'
            : 'Authenticate with MFA'
        }
        open={isAuthModalOpen}
        onCancel={() => {
          setIsAuthModalOpen(false);
          passphraseForm.resetFields();
          mfaForm.resetFields();
        }}
        footer={null}
        width={500}
      >
        {authMode === 'passphrase' && (
          <Form
            form={passphraseForm}
            onFinish={handlePassphraseAuth}
            layout="vertical"
          >
            <Form.Item
              label="Passphrase"
              name="passphrase"
              rules={[
                { required: true, message: 'Please enter your passphrase' },
              ]}
            >
              <Input.Password
                placeholder="Enter your passphrase"
                autoComplete="current-password"
              />
            </Form.Item>
            <Form.Item>
              <Space>
                <Button type="primary" htmlType="submit">
                  Authenticate
                </Button>
                <Button onClick={() => setIsAuthModalOpen(false)}>
                  Cancel
                </Button>
              </Space>
            </Form.Item>
            <Divider />
            <Space direction="vertical" style={{ width: '100%' }}>
              <Text type="secondary">Or use a different method:</Text>
              <Space>
                <Button
                  size="small"
                  onClick={() => setAuthMode('mfa-register')}
                >
                  Register MFA
                </Button>
                <Button size="small" onClick={() => setAuthMode('mfa-auth')}>
                  Login with MFA
                </Button>
              </Space>
            </Space>
          </Form>
        )}

        {authMode === 'mfa-register' && (
          <Form form={mfaForm} onFinish={handleMFARegister} layout="vertical">
            <Form.Item
              label="Username"
              name="username"
              rules={[{ required: true, message: 'Please enter a username' }]}
            >
              <Input placeholder="Enter username for credential" />
            </Form.Item>
            <Form.Item>
              <Space>
                <Button type="primary" htmlType="submit">
                  Register Credential
                </Button>
                <Button onClick={() => setIsAuthModalOpen(false)}>
                  Cancel
                </Button>
              </Space>
            </Form.Item>
            <Divider />
            <Space direction="vertical" style={{ width: '100%' }}>
              <Text type="secondary">Or use a different method:</Text>
              <Space>
                <Button size="small" onClick={() => setAuthMode('passphrase')}>
                  Use Passphrase
                </Button>
                <Button size="small" onClick={() => setAuthMode('mfa-auth')}>
                  Login with MFA
                </Button>
              </Space>
            </Space>
          </Form>
        )}

        {authMode === 'mfa-auth' && (
          <Form form={mfaForm} onFinish={handleMFAAuth} layout="vertical">
            <Form.Item
              label="Username"
              name="username"
              rules={[
                { required: true, message: 'Please enter your username' },
              ]}
            >
              <Input placeholder="Enter your username" />
            </Form.Item>
            <Form.Item>
              <Space>
                <Button type="primary" htmlType="submit">
                  Authenticate
                </Button>
                <Button onClick={() => setIsAuthModalOpen(false)}>
                  Cancel
                </Button>
              </Space>
            </Form.Item>
            <Divider />
            <Space direction="vertical" style={{ width: '100%' }}>
              <Text type="secondary">Or use a different method:</Text>
              <Space>
                <Button size="small" onClick={() => setAuthMode('passphrase')}>
                  Use Passphrase
                </Button>
                <Button
                  size="small"
                  onClick={() => setAuthMode('mfa-register')}
                >
                  Register MFA
                </Button>
              </Space>
            </Space>
          </Form>
        )}
      </Modal>
      </div>
    </>
  );
};
