export class OutputPane {
  private el: HTMLElement;

  constructor() {
    this.el = document.createElement('div');
    this.el.className = 'output-pane';
    this.showPlaceholder();
  }

  showPlaceholder(): void {
    this.el.innerHTML = '<span class="output-placeholder">Run code to see output</span>';
  }

  showOutput(text: string, isError: boolean): void {
    if (!text.trim()) {
      this.el.innerHTML = '<span class="output-placeholder">Program ran with no output</span>';
      return;
    }
    const cls = isError ? 'output-line error' : 'output-line';
    const escaped = text
      .replace(/&/g, '&amp;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
    this.el.innerHTML = `<pre class="${cls}">${escaped}</pre>`;
  }

  showLoading(): void {
    this.el.innerHTML = '<span class="output-placeholder">Running...</span>';
  }

  render(parent: HTMLElement): void {
    parent.appendChild(this.el);
  }

  get element(): HTMLElement {
    return this.el;
  }
}
