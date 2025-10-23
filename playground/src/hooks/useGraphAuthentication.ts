import type { MessageInstance } from 'antd/es/message/interface';
import { useState } from 'react';
import { useDispatch, useSelector } from 'react-redux';

import {
  create_credential,
  get_credential,
  vault_identity_from_passphrase,
} from '../../../hoddor/pkg/hoddor';
import { actions } from '../store/app.actions';
import { appSelectors } from '../store/app.selectors';

type AuthMode = 'passphrase' | 'mfa-register' | 'mfa-auth';

export const useGraphAuthentication = (messageApi: MessageInstance) => {
  const [isAuthModalOpen, setIsAuthModalOpen] = useState(false);
  const [authMode, setAuthMode] = useState<AuthMode>('passphrase');

  const selectedVault = useSelector(appSelectors.getSelectedVault);
  const dispatch = useDispatch();

  const handlePassphraseAuth = async (values: { passphrase: string }) => {
    if (!selectedVault) {
      messageApi.warning('Please select a vault first');
      return;
    }

    try {
      const identityHandle = await vault_identity_from_passphrase(
        values.passphrase,
        selectedVault,
      );
      dispatch(actions.addIdentity(identityHandle.to_json()));
      messageApi.success('Authenticated successfully!');
      setIsAuthModalOpen(false);
    } catch (error) {
      console.error('Passphrase auth failed:', error);
      messageApi.error(`Authentication failed: ${error}`);
    }
  };

  const handleMFARegister = async (values: { username: string }) => {
    if (!selectedVault) {
      messageApi.warning('Please select a vault first');
      return;
    }

    try {
      const identityHandle = await create_credential(
        selectedVault,
        values.username,
      );
      dispatch(actions.addIdentity(identityHandle.to_json()));
      messageApi.success('MFA registered successfully!');
      setIsAuthModalOpen(false);
    } catch (error) {
      console.error('MFA register failed:', error);
      messageApi.error(`MFA registration failed: ${error}`);
    }
  };

  const handleMFAAuth = async (values: { username: string }) => {
    if (!selectedVault) {
      messageApi.warning('Please select a vault first');
      return;
    }

    try {
      const identityHandle = await get_credential(
        selectedVault,
        values.username,
      );
      dispatch(actions.addIdentity(identityHandle.to_json()));
      messageApi.success('Authenticated successfully!');
      setIsAuthModalOpen(false);
    } catch (error) {
      console.error('MFA auth failed:', error);
      messageApi.error(`MFA authentication failed: ${error}`);
    }
  };

  const openAuthModal = (mode: AuthMode) => {
    setAuthMode(mode);
    setIsAuthModalOpen(true);
  };

  const closeAuthModal = () => {
    setIsAuthModalOpen(false);
  };

  return {
    isAuthModalOpen,
    authMode,
    handlePassphraseAuth,
    handleMFARegister,
    handleMFAAuth,
    openAuthModal,
    closeAuthModal,
    setAuthMode,
  };
};
