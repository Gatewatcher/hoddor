import { Flex, Typography } from 'antd';
import { useSelector } from 'react-redux';

import { appSelectors } from './../../store/app.selectors';
import { Authentication } from './Authentication';
import { LoggedActions } from './LoggedActions';
import { VaultHeaderTile } from './VaultHeaderTitle';

export const VaultHeader = () => {
  const identity = useSelector(appSelectors.getIdentity);

  return (
    <Flex justify="space-between" style={{ height: 65 }}>
      <VaultHeaderTile />
      <Flex vertical justify="space-between" align="flex-end">
        <Typography.Paragraph style={{ margin: 0 }}>
          {!identity
            ? "We don't find your identity, please enter your passphrase to retreive it:"
            : `Your current identity is: ${identity.public_key}`}
        </Typography.Paragraph>
        {!identity ? <Authentication /> : <LoggedActions />}
      </Flex>
    </Flex>
  );
};
