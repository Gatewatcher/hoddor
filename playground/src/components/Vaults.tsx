import { Breadcrumb, Divider, Layout, Typography } from 'antd';
import { useSelector } from 'react-redux';

import { appSelectors } from './../store/app.selectors.ts';
import './../styles.css';
import { VaultsActions } from './VaultsActions.tsx';
import { VaultsMenu } from './VaultsMenu.tsx';
import { Vault } from './vault/Vault.tsx';

export const Vaults = () => {
  const selectedVault = useSelector(appSelectors.getSelectedVault);

  return (
    <Layout style={{ height: '100%' }} hasSider>
      <Layout.Sider
        width={250}
        style={{
          backgroundColor: '#FFFFFF',
          padding: 25,
        }}
      >
        <img src="./assets/hoddor_logo.png" style={{ width: 200 }} />
        <Divider />
        <VaultsActions />
        <VaultsMenu />
      </Layout.Sider>
      <Layout style={{ padding: '0 48px' }}>
        <Breadcrumb style={{ margin: '16px 0' }}>
          <Breadcrumb.Item>Vaults</Breadcrumb.Item>
          <Breadcrumb.Item>{selectedVault || '-'}</Breadcrumb.Item>
        </Breadcrumb>
        <Layout style={{ overflowY: 'scroll', height: '100%' }}>
          <Layout.Content
            style={{
              background: '#FFFFFF',
              minHeight: '79vh',
              padding: 24,
              borderRadius: 8,
              overflowY: 'scroll',
            }}
          >
            {selectedVault ? (
              <Vault />
            ) : (
              <Typography.Title level={5} style={{ margin: 0 }}>
                No vault selected, please select or create one!
              </Typography.Title>
            )}
          </Layout.Content>
        </Layout>
        <Layout.Footer style={{ textAlign: 'center' }}>
          Hoddor - Cryptographic Browser Vault ©{new Date().getFullYear()}{' '}
          Created by Gatewatcher
        </Layout.Footer>
      </Layout>
    </Layout>
  );
};
