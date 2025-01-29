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
      let data;

      if (file.type === 'json') {
        data = JSON.parse(await file.text());
      } else if (file.type === 'image/png') {
        data = Array.from(new Uint8Array(await file.arrayBuffer()));
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
