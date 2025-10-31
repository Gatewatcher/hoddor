import {
  ExperimentOutlined,
  FolderOpenOutlined,
  LockOutlined,
  SaveOutlined,
  UnlockOutlined,
} from '@ant-design/icons';
import {
  Button,
  Checkbox,
  Divider,
  Input,
  Space,
  Tag,
  Typography,
  message,
} from 'antd';
import { useState } from 'react';

import { EmbeddingService } from '../../services/embedding';
import { createTestNodes } from '../../utils/createTestNodes';

const { Text } = Typography;

interface RAGControlsProps {
  selectedVault: string;
  onVaultChange: (vault: string) => void;
  withRAG: boolean;
  onRAGChange: (withRAG: boolean) => void;
  withGraphRAG: boolean;
  onGraphRAGChange: (withGraphRAG: boolean) => void;
  canUseRAG: boolean;
  embeddingService: EmbeddingService | null;
  isAuthenticated: boolean;
  onAuthPassphrase: () => void;
  onAuthMFARegister: () => void;
  onAuthMFALogin: () => void;
  onSaveGraph: () => void;
  onLoadGraph: () => void;
  isSaving: boolean;
  isRestoring: boolean;
}

export const RAGControls = ({
  selectedVault,
  onVaultChange,
  withRAG,
  onRAGChange,
  withGraphRAG,
  onGraphRAGChange,
  canUseRAG,
  embeddingService,
  isAuthenticated,
  onAuthPassphrase,
  onAuthMFARegister,
  onAuthMFALogin,
  onSaveGraph,
  onLoadGraph,
  isSaving,
  isRestoring,
}: RAGControlsProps) => {
  const [isCreatingTestNodes, setIsCreatingTestNodes] = useState(false);
  const [messageApi, contextHolder] = message.useMessage();

  const handleCreateTestNodes = async () => {
    if (!selectedVault) {
      messageApi.error('Please select a vault first');
      return;
    }

    if (!embeddingService || !embeddingService.isReady()) {
      messageApi.error(
        'Embedding service not ready. Please wait for initialization.',
      );
      return;
    }

    setIsCreatingTestNodes(true);
    const hideLoading = messageApi.loading(
      'Creating 3 test nodes with real embeddings...',
      0,
    );

    try {
      await createTestNodes(selectedVault, embeddingService);
      hideLoading();
      messageApi.success(`✅ 3 test nodes created and connected!`);
    } catch (error) {
      hideLoading();
      messageApi.error(`Failed to create test nodes: ${error}`);
      console.error(error);
    } finally {
      setIsCreatingTestNodes(false);
    }
  };

  return (
    <>
      {contextHolder}
      <Space style={{ marginBottom: 16 }} wrap>
        <Text>Vault:</Text>
        <Input
          value={selectedVault}
          onChange={e => onVaultChange(e.target.value)}
          placeholder="Enter vault name (e.g., 'my-vault')"
          style={{ width: 200 }}
        />
        <Checkbox
          checked={withRAG}
          onChange={e => onRAGChange(e.target.checked)}
        >
          Use RAG
        </Checkbox>
        {withRAG && !canUseRAG && (
          <Text type="warning" style={{ fontSize: 12 }}>
            (Embeddings unavailable)
          </Text>
        )}
        <Checkbox
          checked={withGraphRAG}
          onChange={e => onGraphRAGChange(e.target.checked)}
          disabled={!withRAG || !canUseRAG}
        >
          Graph RAG
        </Checkbox>

        <Divider type="vertical" />
        {isAuthenticated ? (
          <Tag icon={<UnlockOutlined />} color="success">
            Authenticated
          </Tag>
        ) : (
          <Space.Compact>
            <Button
              icon={<LockOutlined />}
              onClick={onAuthPassphrase}
              size="small"
            >
              Passphrase
            </Button>
            <Button
              icon={<LockOutlined />}
              onClick={onAuthMFARegister}
              size="small"
            >
              MFA Register
            </Button>
            <Button
              icon={<LockOutlined />}
              onClick={onAuthMFALogin}
              size="small"
            >
              MFA Login
            </Button>
          </Space.Compact>
        )}

        <Button
          icon={<SaveOutlined />}
          onClick={onSaveGraph}
          loading={isSaving}
          disabled={!selectedVault || !isAuthenticated}
          type="primary"
        >
          Save Graph
        </Button>
        <Button
          icon={<FolderOpenOutlined />}
          onClick={onLoadGraph}
          loading={isRestoring}
          disabled={!selectedVault || !isAuthenticated}
        >
          Load Graph
        </Button>
        <Button
          icon={<ExperimentOutlined />}
          onClick={handleCreateTestNodes}
          loading={isCreatingTestNodes}
          disabled={!selectedVault}
          type="dashed"
        >
          Add Test Nodes
        </Button>
      </Space>
      <Divider style={{ margin: '8px 0' }} />
    </>
  );
};
