import { Button, Divider, Form, Input, Modal, Space, Typography } from 'antd';

const { Text } = Typography;

interface GraphAuthModalProps {
  isOpen: boolean;
  authMode: 'passphrase' | 'mfa-register' | 'mfa-auth';
  onClose: () => void;
  onPassphraseAuth: (values: { passphrase: string }) => void;
  onMFARegister: (values: { username: string }) => void;
  onMFAAuth: (values: { username: string }) => void;
  onAuthModeChange: (mode: 'passphrase' | 'mfa-register' | 'mfa-auth') => void;
}

export const GraphAuthModal = ({
  isOpen,
  authMode,
  onClose,
  onPassphraseAuth,
  onMFARegister,
  onMFAAuth,
  onAuthModeChange,
}: GraphAuthModalProps) => {
  const [passphraseForm] = Form.useForm();
  const [mfaForm] = Form.useForm();

  const handleClose = () => {
    passphraseForm.resetFields();
    mfaForm.resetFields();
    onClose();
  };

  return (
    <Modal
      title={
        authMode === 'passphrase'
          ? 'Authenticate with Passphrase'
          : authMode === 'mfa-register'
          ? 'Register MFA Credential'
          : 'Authenticate with MFA'
      }
      open={isOpen}
      onCancel={handleClose}
      footer={null}
      width={500}
    >
      {authMode === 'passphrase' && (
        <Form
          form={passphraseForm}
          onFinish={onPassphraseAuth}
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
              <Button onClick={handleClose}>Cancel</Button>
            </Space>
          </Form.Item>
          <Divider />
          <Space direction="vertical" style={{ width: '100%' }}>
            <Text type="secondary">Or use a different method:</Text>
            <Space>
              <Button
                size="small"
                onClick={() => onAuthModeChange('mfa-register')}
              >
                Register MFA
              </Button>
              <Button size="small" onClick={() => onAuthModeChange('mfa-auth')}>
                Login with MFA
              </Button>
            </Space>
          </Space>
        </Form>
      )}

      {authMode === 'mfa-register' && (
        <Form form={mfaForm} onFinish={onMFARegister} layout="vertical">
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
              <Button onClick={handleClose}>Cancel</Button>
            </Space>
          </Form.Item>
          <Divider />
          <Space direction="vertical" style={{ width: '100%' }}>
            <Text type="secondary">Or use a different method:</Text>
            <Space>
              <Button
                size="small"
                onClick={() => onAuthModeChange('passphrase')}
              >
                Use Passphrase
              </Button>
              <Button size="small" onClick={() => onAuthModeChange('mfa-auth')}>
                Login with MFA
              </Button>
            </Space>
          </Space>
        </Form>
      )}

      {authMode === 'mfa-auth' && (
        <Form form={mfaForm} onFinish={onMFAAuth} layout="vertical">
          <Form.Item
            label="Username"
            name="username"
            rules={[{ required: true, message: 'Please enter your username' }]}
          >
            <Input placeholder="Enter your username" />
          </Form.Item>
          <Form.Item>
            <Space>
              <Button type="primary" htmlType="submit">
                Authenticate
              </Button>
              <Button onClick={handleClose}>Cancel</Button>
            </Space>
          </Form.Item>
          <Divider />
          <Space direction="vertical" style={{ width: '100%' }}>
            <Text type="secondary">Or use a different method:</Text>
            <Space>
              <Button
                size="small"
                onClick={() => onAuthModeChange('passphrase')}
              >
                Use Passphrase
              </Button>
              <Button
                size="small"
                onClick={() => onAuthModeChange('mfa-register')}
              >
                Register MFA
              </Button>
            </Space>
          </Space>
        </Form>
      )}
    </Modal>
  );
};
