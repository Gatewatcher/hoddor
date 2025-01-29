import { createAction } from '@reduxjs/toolkit';

import { Identity } from './models/Identity';

enum Types {
  SET_VAULTS = 'app/SET_VAULTS',
  SELECT_VAULT = 'app/SELECT_VAULT',
  SET_NAMESPACES = 'app/SET_NAMESPACES',
  ADD_IDENTITY = 'app/ADD_IDENTITY',
  SET_JSON = 'app/SET_JSON',
  SET_IMAGE = 'app/SET_IMAGE',
  SET_VIDEO = 'app/SET_VIDEO',
  DELETE_VAULT = 'app/DELETE_VAULT',
  FLUSH_IDENTITY = 'app/FLUSH_IDENTITY',
}

export const actions = {
  setVaults: createAction<string[]>(Types.SET_VAULTS),
  selectVault: createAction<string>(Types.SELECT_VAULT),
  setNamespaces: createAction<string[]>(Types.SET_NAMESPACES),
  addIdentity: createAction<Identity>(Types.ADD_IDENTITY),
  setJson: createAction<{}>(Types.SET_JSON),
  setImage: createAction<string>(Types.SET_IMAGE),
  setVideo: createAction<string>(Types.SET_VIDEO),
  deleteVault: createAction(Types.DELETE_VAULT),
  flushIdentity: createAction(Types.FLUSH_IDENTITY),
};
