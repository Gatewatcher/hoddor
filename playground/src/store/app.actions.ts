import { createAction } from '@reduxjs/toolkit';

import { IdentityHandle } from '../../../hoddor/pkg/hoddor';

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
  // RAG/LLM actions
  SET_SELECTED_MODEL = 'app/SET_SELECTED_MODEL',
  SET_USE_RAG = 'app/SET_USE_RAG',
  SET_USE_GRAPH_RAG = 'app/SET_USE_GRAPH_RAG',
  SET_SERVICES_READY = 'app/SET_SERVICES_READY',
  TRIGGER_MEMORY_REFRESH = 'app/TRIGGER_MEMORY_REFRESH',
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
  // RAG/LLM actions
  setSelectedModel: createAction<string>(Types.SET_SELECTED_MODEL),
  setUseRAG: createAction<boolean>(Types.SET_USE_RAG),
  setUseGraphRAG: createAction<boolean>(Types.SET_USE_GRAPH_RAG),
  setServicesReady: createAction<boolean>(Types.SET_SERVICES_READY),
  triggerMemoryRefresh: createAction(Types.TRIGGER_MEMORY_REFRESH),
};
