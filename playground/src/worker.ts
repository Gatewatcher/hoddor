import init, {
  create_vault,
  read_from_vault,
  remove_from_vault,
  upsert_vault,
  list_namespaces,
  create_vault_with_name,
  read_from_vault_with_name,
  remove_from_vault_with_name,
  upsert_vault_with_name,
  list_vaults,
  export_vault,
  import_vault,
  configure_cleanup
} from '../../hoddor/pkg/hoddor.js';

let initialized = false;

const initWasm = async () => {
  if (!initialized) {
    await init();
    initialized = true;
  }
}

self.onmessage = async (message) => {
  const { type, payload } = message.data
  
  try {
    await initWasm();
    
    let result
    switch (type) {
      case 'create_vault':
        await create_vault(payload.password, payload.namespace, payload.data)
        result = { success: true }
        break
      case 'read_from_vault':        
        result = await read_from_vault(payload.password, payload.namespace)
        break
      case 'upsert_vault':
        await upsert_vault(payload.password, payload.namespace, payload.data)
        result = { success: true }
        break
      case 'remove_from_vault':
        await remove_from_vault(payload.password, payload.namespace)
        result = { success: true }
        break
      case 'list_namespaces':
        result = await list_namespaces(payload.password)
        break
      case 'create_vault_with_name':
        await create_vault_with_name(payload.vaultName, payload.password, payload.namespace, payload.data)
        result = { success: true }
        break
      case 'read_from_vault_with_name':        
        result = await read_from_vault_with_name(payload.vaultName, payload.password, payload.namespace)
        break
      case 'upsert_vault_with_name':
        await upsert_vault_with_name(payload.vaultName, payload.password, payload.namespace, payload.data, payload.expiration)
        result = { success: true }
        break
      case 'remove_vault_with_name':
        await remove_from_vault_with_name(payload.vaultName, payload.password, payload.namespace)
        result = { success: true }
        break
      case 'list_vaults':
        result = await list_vaults()
        break
      case 'export_vault':
        result = await export_vault(payload.password)
        break
      case 'import_vault':
        try {
          await import_vault(payload.password, payload.data)
          result = { success: true }
        } catch (e) {
          console.error('Import error:', e);
          throw e;
        }
        break
      case 'configure_cleanup':
        configure_cleanup(payload.intervalSeconds)
        result = { success: true }
        break
    }
    
    self.postMessage({ type: 'success', result })
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    self.postMessage({ 
      type: 'error', 
      error: errorMessage
    });
  }
}

// Needed for TypeScript to recognize this as a module
export type {}
