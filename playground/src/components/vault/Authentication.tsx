import { Button, Flex, message } from 'antd';
import { useDispatch, useSelector } from 'react-redux';

import {
  create_credential,
  get_credential,
} from '../../../../hoddor/pkg/hoddor';
import { actions } from './../../store/app.actions';
import { appSelectors } from './../../store/app.selectors';
import { Passphrase } from './Passphrase';

export const Authentication = () => {
  const dispatch = useDispatch();
  const selectedVault = useSelector(appSelectors.getSelectedVault);
  const [messageApi, contextHolder] = message.useMessage();

  if (!selectedVault) {
    return null;
  }

  const handleRegister = async () => {
    const username = prompt('Enter a username');

    if (username) {
      try {
        await create_credential(selectedVault, username);

        const identity = await get_credential(selectedVault, username);

        dispatch(actions.addIdentity(identity.to_json()));

        messageApi.success('You have now an identity.');
      } catch (e) {
        console.log(e);
        messageApi.error('Failed to register you.');
      }
    }
  };

  const handleAuthentication = async () => {
    const username = prompt('Enter a username');

    if (username) {
      try {
        const identity = await get_credential(selectedVault, username);

        dispatch(actions.addIdentity(identity.to_json()));

        messageApi.success('You have now an identity.');
      } catch (e) {
        console.log(e);
        messageApi.error('Failed to authenticate you.');
      }
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
          onClick={handleRegister}
          style={{ marginLeft: 30, marginRight: 15 }}
        >
          MFA Register
        </Button>
        <Button
          color="cyan"
          variant="outlined"
          onClick={() => handleAuthentication()}
        >
          MFA Authenticate
        </Button>
      </Flex>
    </>
  );
};
