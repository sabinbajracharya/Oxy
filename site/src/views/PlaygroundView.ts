import { CodeEditor } from '../components/CodeEditor';
import { OutputPane } from '../components/OutputPane';
import { OxyRunner } from '../wasm/bridge';

const DEFAULT_CODE = `fn fib(n: i64) -> i64 {
    if n <= 1 { return n; }
    fib(n - 1) + fib(n - 2)
}

fn main() {
    for i in 0..10 {
        let result = fib(i);
        println!("fib({}) = {}", i, result);
    }

    // Try changing the code above!
    let nums = [1, 2, 3, 4, 5];
    let doubled = nums.iter().map(|x| x * 2).collect::<Vec<_>>();
    println!("doubled: {}", doubled);
}
`;

function decodeShare(hash: string): string | null {
  const idx = hash.indexOf(';code=');
  if (idx === -1) return null;
  try {
    return atob(hash.slice(idx + 6));
  } catch {
    return null;
  }
}

export class PlaygroundView {
  private editor: CodeEditor | null = null;
  private output: OutputPane | null = null;
  private runner: OxyRunner | null = null;
  private el: HTMLElement | null = null;
  private wasmReady = false;

  mount(container: HTMLElement): void {
    this.runner = OxyRunner.getInstance();
    this.el = document.createElement('div');
    this.el.className = 'playground-view';

    const hash = window.location.hash;
    const shared = decodeShare(hash);
    const initialCode = shared ?? DEFAULT_CODE;

    this.el.innerHTML = `
      <div class="playground-toolbar">
        <span class="pg-file-label">main.ox</span>
        <div class="pg-toolbar-actions">
          <button class="btn btn-primary btn-run" title="Run (Ctrl+Enter)">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor"><polygon points="5,3 19,12 5,21"/></svg>
            Run
          </button>
          <button class="btn btn-secondary btn-share" title="Share">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 12v8a2 2 0 002 2h12a2 2 0 002-2v-8"/><polyline points="16 6 12 2 8 6"/><line x1="12" y1="2" x2="12" y2="15"/></svg>
            Share
          </button>
        </div>
      </div>
      <div class="playground-main">
        <div class="pg-editor-wrap"></div>
        <div class="pg-output-wrap"></div>
      </div>
    `;

    const editorWrap = this.el.querySelector('.pg-editor-wrap') as HTMLElement;
    const outputWrap = this.el.querySelector('.pg-output-wrap') as HTMLElement;

    this.output = new OutputPane();
    this.output.render(outputWrap);

    this.editor = new CodeEditor(editorWrap, () => {
      // on change — no auto-run in playground (user clicks Run)
    });
    this.editor.create(initialCode);

    // Run handler
    const runBtn = this.el.querySelector('.btn-run') as HTMLButtonElement;
    const run = async () => {
      if (!this.wasmReady) {
        this.output!.showLoading();
        this.wasmReady = await this.runner!.init();
        if (!this.wasmReady) {
          this.output!.showOutput('Failed to load WASM. Ensure wasm-pack build has been run.', true);
          return;
        }
      }
      this.output!.showLoading();
      const result = this.runner!.run(this.editor!.getCode());
      this.output!.showOutput(result.output, result.error);
    };

    runBtn.addEventListener('click', run);
    editorWrap.addEventListener('editor-run', run);

    // Share handler
    const shareBtn = this.el.querySelector('.btn-share') as HTMLButtonElement;
    shareBtn.addEventListener('click', () => {
      const code = this.editor!.getCode();
      const encoded = btoa(code);
      const url = `${window.location.origin}${window.location.pathname}#/playground;code=${encoded}`;
      navigator.clipboard.writeText(url).then(() => {
        shareBtn.textContent = 'Copied!';
        setTimeout(() => {
          shareBtn.innerHTML = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 12v8a2 2 0 002 2h12a2 2 0 002-2v-8"/><polyline points="16 6 12 2 8 6"/><line x1="12" y1="2" x2="12" y2="15"/></svg> Share`;
        }, 2000);
      }).catch(() => {
        shareBtn.textContent = 'Error';
        setTimeout(() => {
          shareBtn.innerHTML = `<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 12v8a2 2 0 002 2h12a2 2 0 002-2v-8"/><polyline points="16 6 12 2 8 6"/><line x1="12" y1="2" x2="12" y2="15"/></svg> Share`;
        }, 2000);
      });
    });

    // Auto-init WASM
    this.runner.init().then((ok) => { this.wasmReady = ok; });

    container.appendChild(this.el);
  }

  unmount(): void {
    this.editor?.destroy();
    this.editor = null;
    this.output = null;
    this.runner = null;
    this.el?.remove();
    this.el = null;
  }
}
