import { LogoutOutlined, UploadOutlined } from '@ant-design/icons';
import { Button, Flex, Upload, message } from 'antd';
import { RcFile } from 'antd/es/upload';
import { useDispatch, useSelector } from 'react-redux';

import {
  IdentityHandle,
  list_namespaces,
  upsert_vault,
} from '../../../../hoddor/pkg/hoddor';
import { actions } from './../../store/app.actions';
import { appSelectors } from './../../store/app.selectors';

const readChunk = (blob: Blob, reader: FileReader): Promise<Uint8Array> => {
  return new Promise((resolve, reject) => {
    reader.onload = () => resolve(new Uint8Array(reader.result as ArrayBuffer));
    reader.onerror = reject;
    reader.readAsArrayBuffer(blob);
  });
};

export const LoggedActions = () => {
  const dispatch = useDispatch();
  const identity = useSelector(appSelectors.getIdentity);
  const selectedVault = useSelector(appSelectors.getSelectedVault);
  const [messageApi, contextHolder] = message.useMessage();

  if (!selectedVault) {
    return null;
  }

  const getNamespacesList = async () => {
    const namespaces: string[] = await list_namespaces(selectedVault);

    dispatch(actions.setNamespaces(namespaces));
  };

  const handleLogout = () => {
    dispatch(actions.flushIdentity());

    messageApi.success('You are now logged out.');
  };

  const uploadAction = async (file: RcFile) => {
    if (identity) {
      let data: unknown = file;

      if (file.type === 'application/json') {
        data = JSON.parse(await file.text());
      } else if (file.type.includes('image/')) {
        data = Array.from(new Uint8Array(await file.arrayBuffer()));
      } else if (file.type.includes('video/')) {
        const reader = new FileReader();
        data = Array.from(await readChunk(file, reader));
      }

      await upsert_vault(
        selectedVault,
        IdentityHandle.from_json(identity),
        file.name,
        data,
        undefined,
        false,
      ).finally(messageApi.success(`${file.name} file uploaded successfully.`));

      getNamespacesList();
    }
    return 'done';
  };

  return (
    <>
      {contextHolder}
      <Flex>
        <Button
          type="primary"
          icon={<LogoutOutlined />}
          onClick={handleLogout}
          style={{ marginRight: 25 }}
        >
          Logout
        </Button>
        <Upload action={uploadAction} name="file" fileList={[]}>
          <Button icon={<UploadOutlined />}>Click to Upload</Button>
        </Upload>
      </Flex>
    </>
  );
};
