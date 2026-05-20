import { EditorView, keymap } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { rust } from '@codemirror/lang-rust';
import { oneDark } from '@codemirror/theme-one-dark';
import { autocompletion } from '@codemirror/autocomplete';
import { searchKeymap } from '@codemirror/search';
import { lintKeymap } from '@codemirror/lint';

export type ChangeCallback = (code: string) => void;

export class CodeEditor {
  private view: EditorView | null = null;
  private changeCb: ChangeCallback | null = null;

  constructor(
    private parent: HTMLElement,
    onChange?: ChangeCallback,
  ) {
    this.changeCb = onChange ?? null;
  }

  create(initialCode: string): void {
    const updateListener = EditorView.updateListener.of((update) => {
      if (update.docChanged && this.changeCb) {
        this.changeCb(this.view!.state.doc.toString());
      }
    });

    const runKeymap = keymap.of([
      {
        key: 'Ctrl-Enter',
        run: () => {
          this.parent.dispatchEvent(new CustomEvent('editor-run'));
          return true;
        },
      },
      {
        key: 'Mod-Enter',
        run: () => {
          this.parent.dispatchEvent(new CustomEvent('editor-run'));
          return true;
        },
      },
    ]);

    this.view = new EditorView({
      doc: initialCode,
      extensions: [
        history(),
        rust(),
        oneDark,
        autocompletion(),
        updateListener,
        runKeymap,
        keymap.of(defaultKeymap),
        keymap.of(historyKeymap),
        keymap.of(searchKeymap),
        keymap.of(lintKeymap),
        EditorView.lineWrapping,
      ],
      parent: this.parent,
    });
  }

  getCode(): string {
    return this.view?.state.doc.toString() ?? '';
  }

  setCode(code: string): void {
    if (!this.view) return;
    this.view.dispatch({
      changes: { from: 0, to: this.view.state.doc.length, insert: code },
    });
  }

  focus(): void {
    this.view?.focus();
  }

  destroy(): void {
    this.view?.destroy();
    this.view = null;
  }
}
