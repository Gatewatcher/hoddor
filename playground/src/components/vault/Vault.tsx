import { Divider } from 'antd';

import { VaultBody } from './VaultBody';
import { VaultHeader } from './VaultHeader';

export const Vault = () => {
  return (
    <>
      <VaultHeader />
      <Divider />
      <VaultBody />
    </>
  );
};
