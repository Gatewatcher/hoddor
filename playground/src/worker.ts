import init, {
  IdentityHandle,
  configure_cleanup,
  create_vault,
  export_vault,
  import_vault,
  list_namespaces,
  list_vaults,
  read_from_vault,
  remove_from_vault,
  upsert_vault,
} from '../../dist/hoddor.js';

let initialized = false;

const initWasm = async () => {
  if (!initialized) {
    await init();
    initialized = true;
  }
};

self.onmessage = async message => {
  const { type, payload, requestId } = message.data;

  try {
    await initWasm();

    let result;

    switch (type) {
      case 'create_vault':
        await create_vault(payload.vaultName);
        result = { success: true };
        break;
      case 'read_from_vault':
        result = await read_from_vault(
          payload.vaultName,
          IdentityHandle.from_json(payload.identity),
          payload.namespace,
        );
        break;
      case 'upsert_vault':
        await upsert_vault(
          payload.vaultName,
          IdentityHandle.from_json(payload.identity),
          payload.namespace,
          payload.data,
          payload.expiresInSeconds,
          payload.replaceIfExists,
        );
        result = { success: true };
        break;
      case 'remove_from_vault':
        await remove_from_vault(
          payload.vaultName,
          IdentityHandle.from_json(payload.identity),
          payload.namespace,
        );
        result = { success: true };
        break;
      case 'list_namespaces':
        result = await list_namespaces(payload.vaultName);
        break;
      case 'list_vaults':
        result = await list_vaults();
        break;
      case 'export_vault':
        result = await export_vault(payload.vaultName);
        break;
      case 'import_vault':
        result = await import_vault(payload.vaultName, payload.data);
        break;
      case 'configure_cleanup':
        configure_cleanup(payload.intervalSeconds);
        result = { success: true };
        break;
      default:
        throw new Error(`Unknown message type: ${type}`);
    }

    self.postMessage({ type: 'success', result, requestId });
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    self.postMessage({
      type: 'error',
      error: errorMessage,
      requestId,
    });
  }
};

// Needed for TypeScript to recognize this as a module
export type {};
