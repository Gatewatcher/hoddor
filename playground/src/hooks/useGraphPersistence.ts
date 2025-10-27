import type { MessageInstance } from 'antd/es/message/interface';
import { useState } from 'react';
import { useDispatch, useSelector } from 'react-redux';

import {
  graph_backup_vault,
  graph_restore_vault,
} from '../../../hoddor/pkg/hoddor';
import { actions } from '../store/app.actions';
import { appSelectors } from '../store/app.selectors';

export const useGraphPersistence = (messageApi: MessageInstance) => {
  const [isSaving, setIsSaving] = useState(false);
  const [isRestoring, setIsRestoring] = useState(false);

  const selectedVault = useSelector(appSelectors.getSelectedVault);
  const identity = useSelector(appSelectors.getIdentity);
  const dispatch = useDispatch();

  const saveGraph = async () => {
    if (!selectedVault) {
      messageApi.warning('Please select a vault first');
      return;
    }

    if (!identity) {
      messageApi.error('Please authenticate first (Passphrase or MFA)');
      return;
    }

    if (!identity.public_key || !identity.private_key) {
      messageApi.error('Identity is incomplete - please authenticate again');
      return;
    }

    setIsSaving(true);
    try {
      await graph_backup_vault(
        selectedVault,
        identity.public_key,
        identity.private_key,
      );
      messageApi.success(`Graph saved to OPFS for vault: ${selectedVault}`);
    } catch (error) {
      console.error('Failed to save graph:', error);
      messageApi.error(`Failed to save graph: ${error}`);
    } finally {
      setIsSaving(false);
    }
  };

  const loadGraph = async () => {
    if (!selectedVault) {
      messageApi.warning('Please select a vault first');
      return;
    }

    if (!identity) {
      messageApi.error('Please authenticate first (Passphrase or MFA)');
      return;
    }

    if (!identity.public_key || !identity.private_key) {
      messageApi.error('Identity is incomplete - please authenticate again');
      return;
    }

    setIsRestoring(true);
    try {
      const found = await graph_restore_vault(
        selectedVault,
        identity.public_key,
        identity.private_key,
      );
      if (found) {
        messageApi.success(
          `Graph loaded from OPFS for vault: ${selectedVault}`,
        );
        dispatch(actions.triggerMemoryRefresh());
      } else {
        messageApi.info('No saved graph found (this is the first time)');
      }
    } catch (error) {
      console.error('Failed to load graph:', error);
      messageApi.error(`Failed to load graph: ${error}`);
    } finally {
      setIsRestoring(false);
    }
  };

  return {
    isSaving,
    isRestoring,
    saveGraph,
    loadGraph,
  };
};
