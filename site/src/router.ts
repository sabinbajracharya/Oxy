import type { View, Route, ViewFactory } from './types';

export class Router {
  private routes: Route[] = [];
  private currentView: View | null = null;
  private container: HTMLElement;

  constructor(container: HTMLElement) {
    this.container = container;
    window.addEventListener('hashchange', () => this.route());
  }

  add(pattern: RegExp, factory: ViewFactory): this {
    this.routes.push({ pattern, factory });
    return this;
  }

  start(): void {
    if (!window.location.hash) {
      window.location.hash = '#/';
      return;
    }
    this.route();
  }

  navigate(hash: string): void {
    window.location.hash = hash;
  }

  private async route(): Promise<void> {
    const raw = window.location.hash.slice(1) || '/';
    // strip leading / so #/playground matches ^playground$
    const normalized = raw.replace(/^\//, '');
    const hash = normalized.split(';')[0];

    for (const route of this.routes) {
      const match = hash.match(route.pattern);
      if (match) {
        this.mountView(await route.factory(match));
        return;
      }
    }
    this.show404();
  }

  private mountView(view: View): void {
    if (this.currentView?.unmount) {
      this.currentView.unmount();
    }
    this.container.innerHTML = '';
    this.currentView = view;
    view.mount(this.container);
  }

  private show404(): void {
    this.container.innerHTML = `
      <div style="display:flex;align-items:center;justify-content:center;min-height:60vh;flex-direction:column;gap:1rem;">
        <h1 style="font-size:4rem;font-weight:800;">404</h1>
        <p style="color:var(--text-muted);">Page not found</p>
        <a href="#/" style="color:var(--accent);">Go home</a>
      </div>`;
  }
}
