import { CHAPTERS } from '../tour/data';

export class TourContentsView {
  private el: HTMLElement | null = null;

  mount(container: HTMLElement): void {
    this.el = document.createElement('div');
    this.el.className = 'tour-contents';

    let html = '<h1>Table of Contents</h1><div class="toc-grid">';
    for (const ch of CHAPTERS) {
      html += `<div class="toc-chapter-card">
        <h3>${ch.title}</h3>
        <ol>`;
      for (const le of ch.lessons) {
        html += `<li><a href="#/tour/${ch.id}/${le.id}">${le.title}</a></li>`;
      }
      html += '</ol></div>';
    }
    html += '</div>';
    this.el.innerHTML = html;
    container.appendChild(this.el);
  }

  unmount(): void {
    this.el?.remove();
    this.el = null;
  }
}
