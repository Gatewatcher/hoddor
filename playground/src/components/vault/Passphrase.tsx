import { Button, Flex, Form, Input, message } from 'antd';
import { useDispatch, useSelector } from 'react-redux';

import { vault_identity_from_passphrase } from '../../../../dist/hoddor';
import { actions } from './../../store/app.actions';
import { appSelectors } from './../../store/app.selectors';

type FieldType = {
  passphrase?: string;
};

export const Passphrase = () => {
  const dispatch = useDispatch();

  const selectedVault = useSelector(appSelectors.getSelectedVault);
  const [messageApi, contextHolder] = message.useMessage();

  if (!selectedVault) {
    return null;
  }

  const handleSubmit = async (values: { passphrase: string }) => {
    const identity = await vault_identity_from_passphrase(
      values.passphrase,
      selectedVault,
    );
    dispatch(actions.addIdentity(identity.to_json()));

    messageApi.success('You have now an identity.');
  };

  return (
    <>
      {contextHolder}
      <Form
        name="basic"
        labelCol={{ span: 8 }}
        wrapperCol={{ span: 16 }}
        initialValues={{ remember: true }}
        onFinish={handleSubmit}
        autoComplete="off"
      >
        <Form.Item<FieldType>
          style={{ marginBottom: 0, display: 'flex' }}
          label="Passphrase"
          name="passphrase"
          rules={[
            {
              required: true,
              message: 'Please input your passphrase!',
            },
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
    </>
  );
};
