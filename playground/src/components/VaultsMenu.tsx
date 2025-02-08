import { Menu } from 'antd';
import { useEffect } from 'react';
import { useDispatch, useSelector } from 'react-redux';

import { actions } from './../store/app.actions';
import { appSelectors } from './../store/app.selectors';
import { VaultWorker } from '../vault';

const vaultWorker = new VaultWorker();

export const VaultsMenu = () => {
  const dispatch = useDispatch();
  const vaults = useSelector(appSelectors.getVaults);

  const getVaultsList = async () => {
    dispatch(actions.setVaults(await vaultWorker.listVaults()));
  };

  useEffect(() => {
    if (!vaults.length) {
      getVaultsList();
    }
  }, [vaults]);

  return (
    !!vaults.length && (
      <Menu
        items={vaults.map((vault, index) => ({
          key: vault + index,
          label: vault,
          onClick: () => dispatch(actions.selectVault(vault)),
        }))}
      />
    )
  );
};
