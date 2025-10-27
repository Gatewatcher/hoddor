import {
  FolderOpenOutlined,
  LockOutlined,
  SaveOutlined,
  UnlockOutlined,
} from '@ant-design/icons';
import { Button, Checkbox, Divider, Input, Space, Tag, Typography } from 'antd';

const { Text } = Typography;

interface RAGControlsProps {
  selectedVault: string;
  onVaultChange: (vault: string) => void;
  useRAG: boolean;
  onRAGChange: (useRAG: boolean) => void;
  canUseRAG: boolean;
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
  useRAG,
  onRAGChange,
  canUseRAG,
  isAuthenticated,
  onAuthPassphrase,
  onAuthMFARegister,
  onAuthMFALogin,
  onSaveGraph,
  onLoadGraph,
  isSaving,
  isRestoring,
}: RAGControlsProps) => {
  return (
    <>
      <Space style={{ marginBottom: 16 }} wrap>
        <Text>Vault:</Text>
        <Input
          value={selectedVault}
          onChange={e => onVaultChange(e.target.value)}
          placeholder="Enter vault name (e.g., 'my-vault')"
          style={{ width: 200 }}
        />
        <Checkbox
          checked={useRAG}
          onChange={e => onRAGChange(e.target.checked)}
        >
          Use RAG
        </Checkbox>
        {useRAG && !canUseRAG && (
          <Text type="warning" style={{ fontSize: 12 }}>
            (Embeddings unavailable)
          </Text>
        )}

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
      </Space>
      <Divider style={{ margin: '8px 0' }} />
    </>
  );
};
