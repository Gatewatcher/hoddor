import { ImportOutlined } from '@ant-design/icons';
import {
  Breadcrumb,
  Button,
  Divider,
  Layout,
  Menu,
  Typography,
  Upload,
  theme,
} from 'antd';
import { ItemType, MenuItemType } from 'antd/es/menu/interface.js';
import { RcFile } from 'antd/es/upload/interface.js';
import { useEffect, useState } from 'react';

import {
  IdentityHandle,
  create_vault,
  list_vaults,
} from '../../../hoddor/pkg/hoddor.js';
import './../styles.css';
import { VaultWorker } from './../vault.ts';
import { Vault } from './Vault.tsx';

const vaultWorker = new VaultWorker();

const buildVaultItem = async (
  element: any,
  index: number,
  callback: React.Dispatch<React.SetStateAction<string>>,
) =>
  ({
    key: element + index,
    label: element,
    onClick: () => callback(element),
  }) as ItemType<MenuItemType>;

export const Vaults = () => {
  const {
    token: { colorBgContainer, borderRadiusLG },
  } = theme.useToken();
  const [vaults, setVaults] = useState<ItemType<MenuItemType>[]>([]);
  const [selectedVault, setSelectedVault] = useState('');
  const [identity, setIdentity] = useState<IdentityHandle>();

  const getVaultsList = async () => {
    const vaultsList: string[] = await list_vaults();
    const vaults = await Promise.all(
      vaultsList.map((element, index) =>
        buildVaultItem(element, index, setSelectedVault),
      ),
    );

    setVaults(vaults);
  };

  const uploadAction = async (file: RcFile) => {
    vaultWorker.importVault(
      'default',
      new Uint8Array(await file.arrayBuffer()),
    );
    return 'done';
  };

  useEffect(() => {
    if (!vaults.length) {
      getVaultsList();
    }
  }, []);

  return (
    <Layout style={{ height: '100%' }} hasSider>
      <Layout.Sider
        width={250}
        style={{
          background: '#1E1E1E',
          padding: 25,
        }}
      >
        <img src="./assets/hoddor_logo.png" style={{ width: 200 }} />
        <Divider />
        <Upload action={uploadAction} name="file" fileList={[]}>
          <Button block>Import Vault</Button>
        </Upload>
        <Button
          type="primary"
          block
          onClick={async () => {
            const vaultName = prompt('Vault name:');
            if (vaultName) {
              await create_vault(vaultName);
              await getVaultsList();
            }
          }}
        >
          Create vault
        </Button>
        <Divider />
        {vaults.length && (
          <Menu
            theme="dark"
            mode="inline"
            style={{ background: '#1E1E1E', color: '#FFFFFF' }}
            items={vaults}
          />
        )}
      </Layout.Sider>
      <Layout style={{ padding: '0 48px' }}>
        <Breadcrumb style={{ margin: '16px 0' }}>
          <Breadcrumb.Item>Vaults</Breadcrumb.Item>
          <Breadcrumb.Item>{selectedVault || '-'}</Breadcrumb.Item>
        </Breadcrumb>
        <Layout style={{ overflowY: 'scroll', height: '100%' }}>
          <Layout.Content
            style={{
              background: colorBgContainer,
              minHeight: '100vh',
              padding: 24,
              borderRadius: borderRadiusLG,
            }}
          >
            {selectedVault ? (
              <Vault
                vaultName={selectedVault}
                identity={identity}
                setIdentity={setIdentity}
              />
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
