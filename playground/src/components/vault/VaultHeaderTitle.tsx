import { Button, Flex, Typography, message } from 'antd';
import { useDispatch, useSelector } from 'react-redux';

import { remove_vault } from '../../../../dist/hoddor';
import { actions } from './../../store/app.actions';
import { appSelectors } from './../../store/app.selectors';

export const VaultHeaderTile = () => {
  const dispatch = useDispatch();
  const selectedVault = useSelector(appSelectors.getSelectedVault);
  const [messageApi, contextHolder] = message.useMessage();

  if (!selectedVault) {
    return null;
  }

  const handleDeleteVault = async () => {
    try {
      await remove_vault(selectedVault);
      dispatch(actions.deleteVault());

      messageApi.success('Vault removed successfully');
    } catch (error) {
      messageApi.error(`Failed to remove vault: ${error}`);
    }
  };

  return (
    <>
      {contextHolder}
      <Flex align="center">
        <Typography.Title level={1} style={{ margin: 0, marginRight: 30 }}>
          {selectedVault}
        </Typography.Title>
        <Button color="danger" variant="solid" onClick={handleDeleteVault}>
          Delete vault
        </Button>
      </Flex>
    </>
  );
};
