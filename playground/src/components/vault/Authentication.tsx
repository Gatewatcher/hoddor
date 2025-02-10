import { Button, Flex, Form, Input, Modal, message } from 'antd';
import { useState } from 'react';
import { useDispatch, useSelector } from 'react-redux';

import {
  create_credential,
  get_credential,
} from '../../../../hoddor/pkg/hoddor';
import { actions } from './../../store/app.actions';
import { appSelectors } from './../../store/app.selectors';
import { Passphrase } from './Passphrase';

type FieldType = {
  username?: string;
};

export const Authentication = () => {
  const dispatch = useDispatch();
  const selectedVault = useSelector(appSelectors.getSelectedVault);
  const [messageApi, contextHolder] = message.useMessage();
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [mfaType, setMfaType] = useState<'register' | 'authenticate'>();

  if (!selectedVault) {
    return null;
  }

  const handleRegister = async (username: string) => {
    try {
      const identity = await create_credential(selectedVault, username);

      dispatch(actions.addIdentity(identity));

      messageApi.success('You have now an identity.');
    } catch (error) {
      messageApi.error('Failed to register you.');
    }
  };

  const handleAuthentication = async (username: string) => {
    try {
      const identity = await get_credential(selectedVault, username);

      dispatch(actions.addIdentity(identity.to_json()));

      messageApi.success('You have now an identity.');
    } catch (error) {
      messageApi.error('Failed to authenticate you.');
    }
  };

  return (
    <>
      {contextHolder}
      <Flex>
        <Passphrase />
        <Button
          color="cyan"
          variant="solid"
          onClick={() => {
            setIsModalOpen(true);
            setMfaType('register');
          }}
          style={{ marginLeft: 30, marginRight: 15 }}
        >
          MFA Register
        </Button>
        <Button
          color="cyan"
          variant="outlined"
          onClick={() => {
            setIsModalOpen(true);
            setMfaType('authenticate');
          }}
        >
          MFA Authenticate
        </Button>
        <Modal
          title={
            mfaType === 'authenticate'
              ? 'MFA Authentication'
              : 'MFA Registration'
          }
          open={isModalOpen}
          onCancel={() => setIsModalOpen(false)}
          footer={(_, { CancelBtn }) => (
            <>
              <CancelBtn />
              <Button form="mfa" htmlType="submit" type="primary">
                Submit
              </Button>
            </>
          )}
          okButtonProps={{ htmlType: 'submit' }}
        >
          <Form
            id="mfa"
            onFinish={
              mfaType === 'authenticate'
                ? value => handleAuthentication(value.username)
                : value => handleRegister(value.username)
            }
            layout="vertical"
          >
            <Form.Item<FieldType>
              label="Username"
              name="username"
              rules={[
                {
                  required: true,
                  message: 'Please input your username!',
                },
              ]}
            >
              <Input style={{ width: '100%' }} />
            </Form.Item>
          </Form>
        </Modal>
      </Flex>
    </>
  );
};
