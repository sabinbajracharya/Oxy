export class Footer {
  private el: HTMLElement;

  constructor() {
    this.el = document.createElement('footer');
    this.el.className = 'site-footer';
    this.el.innerHTML = `
      <div class="footer-content">
        <p>Built with Rust + WebAssembly</p>
        <p class="footer-license">MIT License</p>
      </div>
    `;
  }

  render(parent: HTMLElement): void {
    parent.append(this.el);
  }
}
