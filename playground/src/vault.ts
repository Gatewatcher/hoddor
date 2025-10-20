import { IdentityHandle } from '../../hoddor/pkg/hoddor';

export class VaultWorker {
  private worker: Worker;
  private requestIdCounter = 0;
  private pendingRequests = new Map<number, (value: any) => void>();
  private pendingErrors = new Map<number, (error: any) => void>();

  constructor() {
    this.worker = new Worker(new URL('./worker.ts', import.meta.url), {
      type: 'module',
    });

    this.worker.onmessage = event => {
      const { type, result, error, requestId } = event.data;

      if (requestId !== undefined) {
        if (type === 'success') {
          const resolve = this.pendingRequests.get(requestId);
          if (resolve) resolve(result);
        } else if (type === 'error') {
          const reject = this.pendingErrors.get(requestId);
          if (reject) reject(new Error(error));
        }

        this.pendingRequests.delete(requestId);
        this.pendingErrors.delete(requestId);
      }

      if (event.data?.event === 'vaultUpdate') {
        console.log('Received update from Hoddor using worker!');
        console.log('event.data', event.data);
      }
    };
  }

  private async send(type: string, payload: any): Promise<any> {
    const requestId = this.requestIdCounter++;

    return new Promise((resolve, reject) => {
      this.pendingRequests.set(requestId, resolve);
      this.pendingErrors.set(requestId, reject);

      this.worker.postMessage({ type, payload, requestId });
    });
  }

  async createVault(vaultName: string): Promise<void> {
    await this.send('create_vault', { vaultName });
  }

  async readFromVault(
    vaultName: string,
    identity: IdentityHandle,
    namespace: string,
    expiresInSeconds?: BigInt,
  ): Promise<any> {
    return this.send('read_from_vault', {
      vaultName,
      identity,
      namespace,
      expiresInSeconds,
    });
  }

  async upsertVault(
    vaultName: string,
    identity: IdentityHandle,
    namespace: string,
    data: any,
    expiresInSeconds?: BigInt,
    replaceIfExists?: boolean,
  ): Promise<void> {
    await this.send('upsert_vault', {
      vaultName,
      identity,
      namespace,
      data,
      expiresInSeconds,
      replaceIfExists,
    });
  }

  async removeFromVault(
    vaultName: string,
    identity: IdentityHandle,
    namespace: string,
  ): Promise<void> {
    await this.send('remove_from_vault', { vaultName, identity, namespace });
  }

  async removeVault(vaultName: string): Promise<void> {
    await this.send('remove_vault', { vaultName });
  }

  async listNamespaces(vaultName: string): Promise<string[]> {
    return this.send('list_namespaces', { vaultName });
  }

  async setDebugMode(enabled: boolean): Promise<void> {
    await this.send('set_debug_mode', { enabled });
  }

  async listVaults(): Promise<string[]> {
    return this.send('list_vaults', {});
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
