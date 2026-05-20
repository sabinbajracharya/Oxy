let wasmModule: {
  run_oxy: (source: string) => string;
  default?: () => Promise<void>;
} | null = null;

let initPromise: Promise<void> | null = null;

export class OxyRunner {
  private static instance: OxyRunner;
  private ready = false;

  static getInstance(): OxyRunner {
    if (!OxyRunner.instance) {
      OxyRunner.instance = new OxyRunner();
    }
    return OxyRunner.instance;
  }

  async init(): Promise<boolean> {
    if (this.ready) return true;
    if (!initPromise) {
      initPromise = this.loadWasm();
    }
    try {
      await initPromise;
      this.ready = true;
    } catch {
      this.ready = false;
    }
    return this.ready;
  }

  private async loadWasm(): Promise<void> {
    const wasmUrl = new URL('./wasm/oxy_wasm.js', window.location.href).href;
    const module = await import(/* @vite-ignore */ wasmUrl);
    wasmModule = module;
    if (module.default) {
      await module.default();
    }
  }

  run(source: string): { output: string; error: boolean } {
    if (!wasmModule) {
      return { output: 'WASM module not loaded. Build the WASM target first.', error: true };
    }
    try {
      const result = wasmModule.run_oxy(source);
      const isError = result.startsWith('Error:');
      return { output: result, error: isError };
    } catch (e) {
      return { output: `Runtime error: ${e}`, error: true };
    }
  }

  isReady(): boolean {
    return this.ready;
  }
}
