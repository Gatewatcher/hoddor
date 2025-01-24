import { DeleteOutlined } from '@ant-design/icons';
import { Button, Table, TableColumnsType } from 'antd';
import { useEffect, useState } from 'react';

import {
  IdentityHandle,
  list_namespaces,
  remove_from_vault,
} from '../../../hoddor/pkg/hoddor';

interface DataType {
  key: React.Key;
  namespace: string;
}

const columns: (
  vaultName: string,
  identity?: IdentityHandle,
) => TableColumnsType<DataType> = (vaultName, identity) => [
  { title: 'Namespace', dataIndex: 'namespace', key: 'namespace' },
  {
    title: 'Action',
    dataIndex: '',
    key: 'x',
    render: value => (
      <Button
        icon={<DeleteOutlined />}
        disabled={!identity}
        onClick={() => {
          if (identity) {
            remove_from_vault(vaultName, identity, value.namespace);
          }
        }}
      />
    ),
  },
];

type VaultProps = { vaultName: string; identity?: IdentityHandle };

export const VaultBody = ({ vaultName, identity }: VaultProps) => {
  const [namespaces, setVaultNamespaces] = useState<string[]>([]);

  const getNamespacesList = async () => {
    const namespaces: string[] = await list_namespaces(vaultName);

    setVaultNamespaces(namespaces);
  };

  useEffect(() => {
    getNamespacesList();
  }, [vaultName]);

  return (
    <Table<DataType>
      columns={columns(vaultName, identity)}
      dataSource={namespaces?.map((namespace, index) => ({
        key: index + namespace,
        namespace,
      }))}
    />
  );
};
