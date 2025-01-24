import { UploadOutlined } from '@ant-design/icons';
import { Button, Flex, Form, Input, Typography, Upload, message } from 'antd';
import { RcFile } from 'antd/es/upload';

import {
  IdentityHandle,
  upsert_vault,
  vault_identity_from_passphrase,
} from '../../../hoddor/pkg/hoddor';

type FieldType = {
  passphrase?: string;
};

type VaultProps = {
  vaultName: string;
  identity?: IdentityHandle;
  setIdentity: React.Dispatch<React.SetStateAction<IdentityHandle | undefined>>;
};

export const VaultHeader = ({
  vaultName,
  identity,
  setIdentity,
}: VaultProps) => {
  const uploadAction = async (file: RcFile) => {
    const namespace = prompt('Please input your namespace!');

    if (identity && namespace) {
      upsert_vault(vaultName, identity, namespace, file, undefined, false)
        .then(message.success(`${namespace} file uploaded successfully.`))
        .catch(message.error(`${namespace} file upload failed.`));
    }
    return 'done';
  };

  return (
    <Flex justify="space-between" style={{ height: 65 }}>
      <Typography.Title level={1} style={{ margin: 0 }}>
        {vaultName}
      </Typography.Title>
      <Flex vertical justify="space-between" align="flex-end">
        <Typography.Paragraph style={{ margin: 0 }}>
          {!identity
            ? "We don't find your identity, please enter your passphrase to retreive it:"
            : `Your current identity is: ${identity.public_key}`}
        </Typography.Paragraph>
        {!identity ? (
          <Form
            name="basic"
            labelCol={{ span: 8 }}
            wrapperCol={{ span: 16 }}
            initialValues={{ remember: true }}
            onFinish={async values => {
              const identity = await vault_identity_from_passphrase(
                values.passphrase,
                vaultName,
              );
              setIdentity(identity);
            }}
            autoComplete="off"
          >
            <Form.Item<FieldType>
              style={{ marginBottom: 0, display: 'flex' }}
              label="Passphrase"
              name="passphrase"
              rules={[
                { required: true, message: 'Please input your passphrase!' },
              ]}
            >
              <Flex>
                <Input style={{ marginRight: 10 }} />
                <Button type="primary" htmlType="submit">
                  Submit
                </Button>
              </Flex>
            </Form.Item>
          </Form>
        ) : (
          <Upload action={uploadAction} name="file">
            <Button icon={<UploadOutlined />}>Click to Upload</Button>
          </Upload>
        )}
      </Flex>
    </Flex>
  );
};
