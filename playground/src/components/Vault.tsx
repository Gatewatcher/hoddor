import { Divider } from 'antd';

import { IdentityHandle } from '../../../hoddor/pkg/hoddor';
import { VaultBody } from './VaultBody';
import { VaultHeader } from './VaultHeader';

type VaultProps = {
  vaultName: string;
  identity?: IdentityHandle;
  setIdentity: React.Dispatch<React.SetStateAction<IdentityHandle | undefined>>;
};

export const Vault = ({ vaultName, identity, setIdentity }: VaultProps) => {
  return (
    <>
      <VaultHeader
        vaultName={vaultName}
        identity={identity}
        setIdentity={setIdentity}
      />
      <Divider />
      <VaultBody vaultName={vaultName} identity={identity} />
    </>
  );
};
