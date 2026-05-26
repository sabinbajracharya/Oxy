import { CHAPTERS } from '../tour/data';

const STORAGE_KEY = 'oxy-tour-completed';

function completedLessons(): Set<string> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return new Set(raw ? JSON.parse(raw) : []);
  } catch {
    return new Set();
  }
}

export class TourContentsView {
  private el: HTMLElement | null = null;

  mount(container: HTMLElement): void {
    this.el = document.createElement('div');
    this.el.className = 'tour-contents';

    const done = completedLessons();

    let html = '<h1>Table of Contents</h1><div class="toc-grid">';
    for (const ch of CHAPTERS) {
      const completed = ch.lessons.filter((le) => done.has(`${ch.id}/${le.id}`)).length;
      html += `<div class="toc-chapter-card">
        <h3>
          ${ch.title}
          ${completed > 0 ? `<span class="toc-chapter-progress">${completed}/${ch.lessons.length}</span>` : ''}
        </h3>
        <ol>`;
      for (const le of ch.lessons) {
        const isDone = done.has(`${ch.id}/${le.id}`);
        html += `<li>
          <a href="#/tour/${ch.id}/${le.id}">${le.title}</a>
          ${isDone ? '<span class="toc-check done">&#10003;</span>' : ''}
        </li>`;
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
