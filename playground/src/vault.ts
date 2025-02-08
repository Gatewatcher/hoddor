import { IdentityHandle } from '../../hoddor/pkg/hoddor';

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

  async createVault(vaultName: string): Promise<void> {
    await this.send('create_vault', { vaultName })
  }

  async readFromVault(vaultName: string, identity: IdentityHandle, namespace: string, expiresInSeconds?: BigInt): Promise<any> {
    return this.send('read_from_vault', { vaultName, identity, namespace, expiresInSeconds });
  }

  async upsertVault(vaultName: string, identity: IdentityHandle, namespace: string, data: any, expiresInSeconds?: BigInt, replaceIfExists?: boolean): Promise<void> {
    await this.send('upsert_vault', { vaultName, identity, namespace, data, expiresInSeconds, replaceIfExists })
  }

  async removeFromVault(vaultName: string, identity: IdentityHandle, namespace: string): Promise<void> {
    await this.send('remove_from_vault', { vaultName, identity, namespace })
  }

  async removeVault(vaultName: string): Promise<void> {
    await this.send('remove_vault', { vaultName })
  }

  async listNamespaces(vaultName: string): Promise<string[]> {
    return this.send('list_namespaces', { vaultName })
  }

  async setDebugMode(enabled: boolean): Promise<void> {
    await this.send('set_debug_mode', { enabled })
  }

  async listVaults(): Promise<string[]> {
    return this.send('list_vaults', {})
  }

  async exportVault(vaultName: string): Promise<Uint8Array> {
    const response = await this.send('export_vault', { vaultName });
    return response;
  }

  async importVault(vaultName: string, data: Uint8Array): Promise<void> {
    await this.send('import_vault', { vaultName, data });
  }

  async configureCleanup(intervalSeconds: number): Promise<void> {
    await this.send('configure_cleanup', { intervalSeconds });
  }
}
