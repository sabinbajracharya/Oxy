import { CodeEditor } from '../components/CodeEditor';
import { OutputPane } from '../components/OutputPane';
import { HintToggle } from '../components/HintToggle';
import { OxyRunner } from '../wasm/bridge';
import { findLesson, getPrevNext } from '../tour/data';
import { renderMarkdown } from '../tour/renderer';

export class TourView {
  private editor: CodeEditor | null = null;
  private output: OutputPane | null = null;
  private runner: OxyRunner | null = null;
  private el: HTMLElement | null = null;
  private wasmReady = false;
  private runTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(
    private chapterId: string,
    private lessonId: string,
  ) {}

  mount(container: HTMLElement): void {
    const lesson = findLesson(this.chapterId, this.lessonId);
    if (!lesson) {
      container.innerHTML = '<p style="padding:2rem;color:var(--ctp-red);">Lesson not found.</p>';
      return;
    }
    this.runner = OxyRunner.getInstance();

    this.el = document.createElement('div');
    this.el.className = 'tour-view';

    const nav = getPrevNext(this.chapterId, this.lessonId);

    this.el.innerHTML = `
      <div class="tour-left">
        <div class="tour-instructions">
          <div class="tour-breadcrumb">${this.chapterId} / ${lesson.id}</div>
          <h2 class="tour-lesson-title">${lesson.title}</h2>
          <div class="tour-content">${renderMarkdown(lesson.instructions)}</div>
          <div class="tour-hints"></div>
        </div>
        <nav class="tour-nav">
          ${nav.prev
            ? `<a href="#/tour/${nav.prev.chapter}/${nav.prev.lesson}" class="tour-nav-btn">Back</a>`
            : '<span class="tour-nav-btn disabled">Back</span>'}
          <span class="tour-nav-sep">&mdash;</span>
          <a href="#/tour/contents" class="tour-nav-btn">Contents</a>
          <span class="tour-nav-sep">&mdash;</span>
          ${nav.next
            ? `<a href="#/tour/${nav.next.chapter}/${nav.next.lesson}" class="tour-nav-btn">Next</a>`
            : '<span class="tour-nav-btn disabled">Next</span>'}
        </nav>
      </div>
      <div class="tour-right">
        <div class="tour-editor-wrap"></div>
        <div class="tour-output-wrap"></div>
      </div>
    `;

    // Hints
    const hintsContainer = this.el.querySelector('.tour-hints') as HTMLElement;
    lesson.hints.forEach((_, i) => {
      hintsContainer.appendChild(HintToggle.create(lesson.hints, i));
    });

    // Editor
    const editorWrap = this.el.querySelector('.tour-editor-wrap') as HTMLElement;
    this.editor = new CodeEditor(editorWrap, () => this.scheduleRun());
    this.editor.create(lesson.initialCode);

    // Output
    const outputWrap = this.el.querySelector('.tour-output-wrap') as HTMLElement;
    this.output = new OutputPane();
    this.output.render(outputWrap);

    // Run handler
    editorWrap.addEventListener('editor-run', () => this.runCode());

    // Auto-init WASM and run
    this.runner.init().then((ok) => {
      this.wasmReady = ok;
      if (ok) this.runCode();
    });

    container.appendChild(this.el);
  }

  private scheduleRun(): void {
    if (this.runTimer) clearTimeout(this.runTimer);
    this.runTimer = setTimeout(() => this.runCode(), 800);
  }

  private async runCode(): Promise<void> {
    if (!this.wasmReady) {
      this.wasmReady = await this.runner!.init();
      if (!this.wasmReady) return;
    }
    const result = this.runner!.run(this.editor!.getCode());
    this.output!.showOutput(result.output, result.error);
  }

  unmount(): void {
    if (this.runTimer) clearTimeout(this.runTimer);
    this.editor?.destroy();
    this.editor = null;
    this.output = null;
    this.runner = null;
    this.el?.remove();
    this.el = null;
  }
}
