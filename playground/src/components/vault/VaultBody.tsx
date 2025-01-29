import { DeleteOutlined, FileOutlined } from '@ant-design/icons';
import JsonView from '@uiw/react-json-view';
import { Image } from 'antd';
import {
  Button,
  Splitter,
  Table,
  TableColumnsType,
  Typography,
  message,
} from 'antd';
import { useEffect } from 'react';
import { useDispatch, useSelector } from 'react-redux';

import {
  IdentityHandle,
  list_namespaces,
  read_from_vault,
  remove_from_vault,
} from '../../../../hoddor/pkg/hoddor';
import { actions } from './../../store/app.actions';
import { appSelectors } from './../../store/app.selectors';

interface DataType {
  key: React.Key;
  namespace: string;
}

const arrayBufferToBase64 = (buffer: Uint8Array): string => {
  let binary = '';
  for (let i = 0; i < buffer.length; i++) {
    binary += String.fromCharCode(buffer[i]);
  }
  return btoa(binary);
};

export const VaultBody = () => {
  const dispatch = useDispatch();

  const identity = useSelector(appSelectors.getIdentity);
  const namespaces = useSelector(appSelectors.getNamespaces);
  const selectedVault = useSelector(appSelectors.getSelectedVault);
  const json = useSelector(appSelectors.getJson);
  const image = useSelector(appSelectors.getImage);
  const [messageApi, contextHolder] = message.useMessage();

  if (!selectedVault) {
    return null;
  }

  const getNamespacesList = async () => {
    const namespaces: string[] = await list_namespaces(selectedVault);

    dispatch(actions.setNamespaces(namespaces));
  };

  const handleReadFile = async (value: { namespace: string }) => {
    if (identity) {
      const data = await read_from_vault(
        selectedVault,
        IdentityHandle.from_json(identity),
        value.namespace,
      );

      try {
        if (Array.isArray(data)) {
          dispatch(actions.setImage(arrayBufferToBase64(new Uint8Array(data))));
        } else {
          dispatch(actions.setJson(Object.fromEntries(data)));
        }
      } catch (e) {
        console.log(e);
        messageApi.error(`Can't read ${value.namespace}.`);
      }
    }
  };

  const handleDelete = async (value: { namespace: string }) => {
    if (identity) {
      await remove_from_vault(
        selectedVault,
        IdentityHandle.from_json(identity),
        value.namespace,
      );

      getNamespacesList();
    }
  };

  useEffect(() => {
    getNamespacesList();
  }, [selectedVault]);

  const columns: TableColumnsType<DataType> = [
    { title: 'Namespace', dataIndex: 'namespace', key: 'namespace' },
    {
      title: 'Action',
      dataIndex: '',
      key: 'x',
      render: value => (
        <>
          <Button
            icon={<FileOutlined />}
            disabled={!identity}
            onClick={() => handleReadFile(value)}
          />
          <Button
            icon={<DeleteOutlined />}
            disabled={!identity}
            onClick={() => handleDelete(value)}
          />
        </>
      ),
    },
  ];

  return (
    <>
      {contextHolder}
      <Splitter
        style={{ boxShadow: '0 0 10px rgba(0, 0, 0, 0.1)', minHeight: '67vh' }}
      >
        <Splitter.Panel defaultSize="70%" min="20%" max="70%">
          <Table<DataType>
            columns={columns}
            dataSource={namespaces?.map((namespace, index) => ({
              key: index + namespace,
              namespace,
            }))}
          />
        </Splitter.Panel>
        <Splitter.Panel>
          {json ? (
            <JsonView value={json} />
          ) : image ? (
            <Image width={200} src={`data:image/jpeg;base64,${image}`} />
          ) : (
            <Typography.Paragraph style={{ margin: 0 }}>
              Nothing to read, select a file.
            </Typography.Paragraph>
          )}
        </Splitter.Panel>
      </Splitter>
    </>
  );
};
