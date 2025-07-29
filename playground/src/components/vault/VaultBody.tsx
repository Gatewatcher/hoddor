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
import ReactMarkdown from 'react-markdown';
import { useDispatch, useSelector } from 'react-redux';

import {
  arrayBufferToBase64,
  getMimeTypeFromExtension,
} from '../../utils/file.utils';
import { VaultWorker } from '../../vault';
import { actions } from './../../store/app.actions';
import { appSelectors } from './../../store/app.selectors';

interface DataType {
  key: React.Key;
  namespace: string;
}

const vaultWorker = new VaultWorker();

export const VaultBody = () => {
  const dispatch = useDispatch();

  const identity = useSelector(appSelectors.getIdentity);
  const namespaces = useSelector(appSelectors.getNamespaces);
  const selectedVault = useSelector(appSelectors.getSelectedVault);
  const json = useSelector(appSelectors.getJson);
  const image = useSelector(appSelectors.getImage);
  const video = useSelector(appSelectors.getVideo);
  const markdown = useSelector(appSelectors.getMarkdown);
  const text = useSelector(appSelectors.getText);
  const audio = useSelector(appSelectors.getAudio);
  const [messageApi, contextHolder] = message.useMessage();

  if (!selectedVault) {
    return null;
  }

  const getNamespacesList = async () => {
    const namespaces: string[] =
      await vaultWorker.listNamespaces(selectedVault);

    dispatch(actions.setNamespaces(namespaces));
  };

  const handleReadFile = async (value: { namespace: string }) => {
    if (!identity) {
      messageApi.error('No identity.');
      return;
    }
    try {
      const data = await vaultWorker.readFromVault(
        selectedVault,
        identity,
        value.namespace,
      );

      if (!data) {
        messageApi.error('No data received from vault');
        return;
      }

      let processedData;
      if (data instanceof Map) {
        processedData = Array.from(data.values());
      } else {
        processedData = data;
      }

      const uint8Array = new Uint8Array(processedData);
      const mimeType = getMimeTypeFromExtension(value.namespace);
      console.log('Detected MIME type:', mimeType);

      if (mimeType === 'application/json') {
        try {
          dispatch(actions.setJson(Object.fromEntries(data)));
        } catch (e) {
          console.error('Failed to parse JSON:', e);
          messageApi.error('Failed to parse JSON data');
        }
      } else if (mimeType.startsWith('image/')) {
        dispatch(actions.setImage(arrayBufferToBase64(uint8Array)));
      } else if (mimeType.startsWith('video/')) {
        const blob = new Blob([uint8Array], { type: mimeType });
        dispatch(actions.setVideo(URL.createObjectURL(blob)));
      } else if (mimeType === 'text/markdown') {
        const content = new TextDecoder().decode(uint8Array);
        dispatch(actions.setMarkdown(content));
      } else if (mimeType.startsWith('text/')) {
        const textContent = new TextDecoder().decode(uint8Array);
        dispatch(actions.setText(textContent));
      } else if (mimeType.startsWith('audio/')) {
        try {
          const blob = new Blob([uint8Array], { type: mimeType });
          console.log('Audio blob size:', blob.size);
          const audioUrl = URL.createObjectURL(blob);
          dispatch(actions.setAudio(audioUrl));
        } catch (audioError) {
          console.error('Audio processing error:', audioError);
          messageApi.error(`Failed to process audio file: ${audioError}`);
        }
      } else {
        messageApi.error(`Unsupported file type: ${mimeType}`);
      }
    } catch (e) {
      console.error('Error processing file:', e);
      messageApi.error(`Can't read ${value.namespace}.`);
    }
  };

  const handleDelete = async (value: { namespace: string }) => {
    if (identity) {
      await vaultWorker.removeFromVault(
        selectedVault,
        identity,
        value.namespace,
      );

      getNamespacesList();
    }
  };

  useEffect(() => {
    getNamespacesList();

    return () => {
      if (audio) {
        URL.revokeObjectURL(audio);
      }
      if (video) {
        URL.revokeObjectURL(video);
      }
    };
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
        <Splitter.Panel style={{ padding: '20px' }}>
          {json ? (
            <JsonView value={json} />
          ) : image ? (
            <Image width="100%" src={`data:image/jpeg;base64,${image}`} />
          ) : video ? (
            <video id="video" width="100%" controls src={video}></video>
          ) : markdown ? (
            <ReactMarkdown>{markdown}</ReactMarkdown>
          ) : text ? (
            <Typography.Paragraph style={{ whiteSpace: 'pre-wrap' }}>
              {text}
            </Typography.Paragraph>
          ) : audio ? (
            <audio
              controls
              src={audio}
              style={{ width: '100%' }}
              onError={e => {
                const audioElement = e.currentTarget;
                messageApi.error(
                  `Error playing audio file: ${
                    audioElement.error?.message || 'Unknown error'
                  }`,
                );
              }}
              preload="metadata"
            />
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
