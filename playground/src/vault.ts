export class VaultWorker {
  private worker: Worker

  constructor() {
    this.worker = new Worker(
      new URL('./worker.ts', import.meta.url),
      { type: 'module' }
    )
  }

  private async send(type: string, payload: any): Promise<any> {
    return new Promise((resolve, reject) => {
      const handler = (message: MessageEvent) => {
        this.worker.removeEventListener('message', handler)
        if (message.data.type === 'error') {
          reject(new Error(message.data.error));
        } else {
          resolve(message.data.result)
        }
      }
      
      this.worker.addEventListener('message', handler)
      this.worker.postMessage({ type, payload })
    })
  }

  async createVault(password: string, namespace: string, data: any): Promise<void> {
    await this.send('create_vault', { password, namespace, data })
  }

  async readFromVault(password: string, namespace: string): Promise<any> {
    return this.send('read_from_vault', {password, namespace});
  }

  async upsertVault(password: string, namespace: string, data: any): Promise<void> {
    await this.send('upsert_vault', { password, namespace, data })
  }

  async removeFromVault(password: string, namespace: string): Promise<void> {
    await this.send('remove_from_vault', { password, namespace })
  }

  async listNamespaces(password: string): Promise<string[]> {
    return this.send('list_namespaces', {password})
  }

  async setDebugMode(enabled: boolean): Promise<void> {
    await this.send('set_debug_mode', { enabled })
  }

  async createVaultWithName(vaultName: string, password: string, namespace: string, data: any): Promise<void> {
    await this.send('create_vault_with_name', { vaultName, password, namespace, data })
  }

  async readFromVaultWithName(vaultName: string, password: string, namespace: string): Promise<any> {
    return this.send('read_from_vault_with_name', { vaultName, password, namespace })
  }

  async upsertVaultWithName(vaultName: string, password: string, namespace: string, data: any, expiration?: any): Promise<void> {
    await this.send('upsert_vault_with_name', { vaultName, password, namespace, data, expiration })
  }

  async removeVaultWithName(vaultName: string, password: string, namespace: string): Promise<void> {
    await this.send('remove_vault_with_name', { vaultName, password, namespace })
  }

  async listVaults(): Promise<string[]> {
    return this.send('list_vaults', {})
  }

  async listNamespacesWithName(vaultName: string, password: string): Promise<string[]> {
    return this.send('list_namespaces_with_name', { vaultName, password })
  }

  async exportVault(password: string): Promise<Uint8Array> {
    const response = await this.send('export_vault', { password });
    return response;
  }

  async importVault(password: string, data: Uint8Array): Promise<void> {
    await this.send('import_vault', { password, data });
  }

  async configureCleanup(intervalSeconds: number): Promise<void> {
    await this.send('configure_cleanup', { intervalSeconds });
  }
}
