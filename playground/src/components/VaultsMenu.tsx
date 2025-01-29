import { Menu } from 'antd';
import { useEffect } from 'react';
import { useDispatch, useSelector } from 'react-redux';

import { list_vaults } from '../../../hoddor/pkg/hoddor';
import { actions } from './../store/app.actions';
import { appSelectors } from './../store/app.selectors';

export const VaultsMenu = () => {
  const dispatch = useDispatch();
  const vaults = useSelector(appSelectors.getVaults);

  const getVaultsList = async () => {
    dispatch(actions.setVaults(await list_vaults()));
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
