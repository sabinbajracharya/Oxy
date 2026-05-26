import { CodeEditor } from '../components/CodeEditor';
import { OutputPane } from '../components/OutputPane';
import { HintToggle } from '../components/HintToggle';
import { OxyRunner } from '../wasm/bridge';
import { findLesson, getPrevNext } from '../tour/data';
import { renderMarkdown } from '../tour/renderer';

const STORAGE_KEY = 'oxy-tour-completed';

function completedLessons(): Set<string> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return new Set(raw ? JSON.parse(raw) : []);
  } catch {
    return new Set();
  }
}

function markCompleted(chapterId: string, lessonId: string): void {
  const set = completedLessons();
  set.add(`${chapterId}/${lessonId}`);
  localStorage.setItem(STORAGE_KEY, JSON.stringify([...set]));
}

function isCompleted(chapterId: string, lessonId: string): boolean {
  return completedLessons().has(`${chapterId}/${lessonId}`);
}

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
      container.innerHTML =
        '<p style="padding:2rem;color:var(--ctp-red);">Lesson not found.</p>';
      return;
    }
    this.runner = OxyRunner.getInstance();

    this.el = document.createElement('div');
    this.el.className = 'tour-view';

    const nav = getPrevNext(this.chapterId, this.lessonId);
    const alreadyDone = isCompleted(this.chapterId, this.lessonId);

    this.el.innerHTML = `
      <div class="tour-left">
        <div class="tour-instructions">
          <div class="tour-breadcrumb">${this.chapterId} / ${lesson.id}</div>
          <h2 class="tour-lesson-title">
            ${lesson.title}
            <span class="tour-lesson-check ${alreadyDone ? 'done' : ''}"></span>
          </h2>
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
        <div class="tour-editor-toolbar">
          <span class="tour-editor-label">Code</span>
          <button class="tour-check-btn" title="Check your solution">Check</button>
        </div>
        <div class="tour-editor-wrap"></div>
        <div class="tour-test-results"></div>
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

    // Check button
    const checkBtn = this.el.querySelector('.tour-check-btn') as HTMLButtonElement;
    checkBtn.addEventListener('click', () => this.runChecks(lesson));

    // Run handler
    editorWrap.addEventListener('editor-run', () => this.runCode());

    // Auto-init WASM and run
    this.runner.init().then((ok) => {
      this.wasmReady = ok;
      if (ok) this.runCode();
    });

    // If already completed, show saved state
    if (alreadyDone) this.showAllPassed();

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

  private runChecks(lesson: { testCode: string }): void {
    const resultsEl = this.el!.querySelector('.tour-test-results') as HTMLElement;
    if (!this.wasmReady || !this.runner) {
      resultsEl.innerHTML = `
        <div class="tour-test-summary tour-test-error">WASM not ready. Try again in a moment.</div>`;
      return;
    }

    const source = this.editor!.getCode() + '\n' + lesson.testCode;
    const results = this.runner.runTests(source);

    if ('error' in results) {
      resultsEl.innerHTML = `
        <div class="tour-test-summary tour-test-fail">
          <span class="tour-test-icon">&#10007;</span>
          ${results.error}
        </div>`;
      return;
    }

    const allPassed = results.every((r) => r.passed);
    const passed = results.filter((r) => r.passed).length;
    const total = results.length;

    if (total === 0) {
      resultsEl.innerHTML = `
        <div class="tour-test-summary tour-test-fail">
          <span class="tour-test-icon">&#10007;</span>
          No tests found for this lesson.
        </div>`;
      return;
    }

    if (allPassed) {
      markCompleted(this.chapterId, this.lessonId);
      this.showAllPassed();
      const checkmark = this.el!.querySelector('.tour-lesson-check') as HTMLElement;
      if (checkmark) checkmark.classList.add('done');
    }

    const statusClass = allPassed ? 'tour-test-pass' : 'tour-test-fail';
    const icon = allPassed ? '&#10003;' : '&#10007;';
    const msg = allPassed
      ? 'All tests passed!'
      : `${passed}/${total} tests passing`;

    let itemsHtml = '';
    for (const r of results) {
      if (!r.passed) {
        const errMsg = r.error ? ` — ${this.escapeHtml(r.error)}` : '';
        itemsHtml += `<div class="tour-test-result-item tour-test-item-fail">
          &#10007; <code>${this.escapeHtml(r.name)}</code>${errMsg}
        </div>`;
      }
    }

    resultsEl.innerHTML = `
      <div class="tour-test-summary ${statusClass}">
        <span class="tour-test-icon">${icon}</span>
        ${msg}
      </div>
      ${itemsHtml}`;
  }

  private showAllPassed(): void {
    const resultsEl = this.el!.querySelector('.tour-test-results') as HTMLElement;
    resultsEl.innerHTML = `
      <div class="tour-test-summary tour-test-pass">
        <span class="tour-test-icon">&#10003;</span>
        All tests passed!
      </div>`;
  }

  private escapeHtml(s: string): string {
    return s
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;')
      .replace(/"/g, '&quot;');
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
