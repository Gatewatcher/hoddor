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
import { getMimeTypeFromExtension } from '../../utils/file.utils';

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
      try {
        const arrayBuffer = await file.arrayBuffer();
        const uint8Array = new Uint8Array(arrayBuffer);
        const mimeType = getMimeTypeFromExtension(file.name);
        let data: unknown;

        if (mimeType === 'application/json') {
          const text = await file.text();
          data = JSON.parse(text);
        } else if (mimeType.startsWith('text/') || mimeType === 'text/markdown') {
          data = Array.from(new TextEncoder().encode(await file.text()));
        } else {
          data = Array.from(uint8Array);
        }

        await upsert_vault(
          selectedVault,
          IdentityHandle.from_json(identity),
          file.name,
          data,
          undefined,
          false,
        );

        messageApi.success(`${file.name} file uploaded successfully.`);
        getNamespacesList();
      } catch (error) {
        console.error('Upload failed:', error);
        messageApi.error(`Failed to upload ${file.name}: ${error}`);
      }
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
        <Upload 
          action={uploadAction} 
          name="file" 
          fileList={[]}
          accept=".json,.txt,.md,.markdown,.mp3,.wav,.ogg,.m4a,.aac,.mp4,.mov,.jpg,.jpeg,.png,.gif,.webp"
        >
          <Button icon={<UploadOutlined />}>Click to Upload</Button>
        </Upload>
      </Flex>
    </>
  );
};
