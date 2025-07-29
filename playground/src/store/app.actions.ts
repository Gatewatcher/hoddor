import { createAction } from '@reduxjs/toolkit';

import { IdentityHandle } from '../../../dist/hoddor';

enum Types {
  SET_VAULTS = 'app/SET_VAULTS',
  SELECT_VAULT = 'app/SELECT_VAULT',
  SET_NAMESPACES = 'app/SET_NAMESPACES',
  ADD_IDENTITY = 'app/ADD_IDENTITY',
  SET_JSON = 'app/SET_JSON',
  SET_IMAGE = 'app/SET_IMAGE',
  SET_VIDEO = 'app/SET_VIDEO',
  SET_MARKDOWN = 'app/SET_MARKDOWN',
  SET_TEXT = 'app/SET_TEXT',
  SET_AUDIO = 'app/SET_AUDIO',
  DELETE_VAULT = 'app/DELETE_VAULT',
  FLUSH_IDENTITY = 'app/FLUSH_IDENTITY',
}

export const actions = {
  setVaults: createAction<string[]>(Types.SET_VAULTS),
  selectVault: createAction<string>(Types.SELECT_VAULT),
  setNamespaces: createAction<string[]>(Types.SET_NAMESPACES),
  addIdentity: createAction<IdentityHandle>(Types.ADD_IDENTITY),
  setJson: createAction<{}>(Types.SET_JSON),
  setImage: createAction<string>(Types.SET_IMAGE),
  setVideo: createAction<string>(Types.SET_VIDEO),
  setMarkdown: createAction<string | null>(Types.SET_MARKDOWN),
  setText: createAction<string | null>(Types.SET_TEXT),
  setAudio: createAction<string | null>(Types.SET_AUDIO),
  deleteVault: createAction(Types.DELETE_VAULT),
  flushIdentity: createAction(Types.FLUSH_IDENTITY),
};
