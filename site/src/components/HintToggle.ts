import { renderMarkdown } from '../tour/renderer';

export class HintToggle {
  static create(hints: string[], index: number): HTMLElement {
    const details = document.createElement('details');
    details.className = 'hint-toggle';
    details.innerHTML = `
      <summary>Hint ${index + 1}</summary>
      <div class="hint-body">${renderMarkdown(hints[index])}</div>
    `;
    return details;
  }
}
