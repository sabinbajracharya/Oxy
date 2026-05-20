export class HomeView {
  private el: HTMLElement | null = null;

  mount(container: HTMLElement): void {
    this.el = document.createElement('div');
    this.el.className = 'home-view';
    this.el.innerHTML = `
      <div class="home-bg">
        <div class="orb orb-1"></div>
        <div class="orb orb-2"></div>
        <div class="orb orb-3"></div>
      </div>
      <section class="hero">
        <div class="hero-badge">v0.3.0</div>
        <h1 class="hero-title">
          <span class="hero-title-line">A compiled language</span>
          <span class="hero-title-line">with <span class="gradient-text">Rust-like syntax</span></span>
          <span class="hero-title-line">without the borrow checker</span>
        </h1>
        <p class="hero-subtitle">
          Clean, expressive, and fast. Oxy compiles to bytecode and runs on a stack-based VM.
        </p>
        <div class="hero-actions">
          <a href="#/playground" class="btn btn-primary btn-lg">Try the Playground</a>
          <a href="#/tour/basics/hello-world" class="btn btn-secondary btn-lg">Start the Tour</a>
        </div>
      </section>

      <section class="code-compare">
        <div class="code-card">
          <div class="code-card-header">
            <span class="code-dot red"></span>
            <span class="code-dot yellow"></span>
            <span class="code-dot green"></span>
            <span class="code-label">main.ox</span>
          </div>
          <pre><code><span class="tok-kw">fn</span> <span class="tok-fn">fib</span>(n: i64) -> i64 {
    <span class="tok-kw">if</span> n <= 1 { <span class="tok-kw">return</span> n; }
    <span class="tok-fn">fib</span>(n - 1) + <span class="tok-fn">fib</span>(n - 2)
}

<span class="tok-kw">fn</span> <span class="tok-fn">main</span>() {
    <span class="tok-kw">for</span> i <span class="tok-kw">in</span> 0..10 {
        <span class="tok-macro">println!</span>(<span class="tok-str">"fib({}) = {}"</span>, i, <span class="tok-fn">fib</span>(i));
    }
}</code></pre>
        </div>
        <div class="code-compare-divider">
          <span class="compare-label">No borrows, no lifetimes</span>
        </div>
        <div class="code-card dim">
          <div class="code-card-header">
            <span class="code-dot red"></span>
            <span class="code-dot yellow"></span>
            <span class="code-dot green"></span>
            <span class="code-label">main.rs — what you avoid</span>
          </div>
          <pre><code><span class="tok-kw">fn</span> <span class="tok-fn">fib</span>(n: i64) -> i64 {
    <span class="tok-kw">if</span> n <= 1 { <span class="tok-kw">return</span> n; }
    <span class="tok-fn">fib</span>(n - 1) + <span class="tok-fn">fib</span>(n - 2)
}

<span class="tok-kw">fn</span> <span class="tok-fn">main</span>() {
    <span class="tok-comment">// no &amp;i, no .iter(), no .collect()</span>
    <span class="tok-comment">// no lifetime annotations</span>
    <span class="tok-comment">// no borrowck headaches</span>
    <span class="tok-kw">for</span> i <span class="tok-kw">in</span> 0..10 {
        <span class="tok-macro">println!</span>(<span class="tok-str">"fib({}) = {}"</span>, i, <span class="tok-fn">fib</span>(i));
    }
}</code></pre>
        </div>
      </section>

      <section class="features">
        <h2 class="section-title">Everything you need</h2>
        <div class="feature-grid">
          <div class="feature-card">
            <div class="feature-icon">&#9733;</div>
            <h3>Familiar Syntax</h3>
            <p>Rust-inspired syntax with <code>fn</code>, <code>let</code>, <code>struct</code>, <code>enum</code>, <code>match</code>, generics, and traits.</p>
          </div>
          <div class="feature-card">
            <div class="feature-icon">⚡</div>
            <h3>No Borrow Checker</h3>
            <p>Write Rust-like code without ownership tracking. Values are freely passed and returned.</p>
          </div>
          <div class="feature-card">
            <div class="feature-icon">&#9874;</div>
            <h3>Bytecode VM</h3>
            <p>Compiles to compact bytecode. Stack-based virtual machine with tracing and disassembly tools.</p>
          </div>
          <div class="feature-card">
            <div class="feature-icon">&#9731;</div>
            <h3>Rich Standard Library</h3>
            <p>Collections, JSON, HTTP client, filesystem, regex, math, random, and more built in.</p>
          </div>
          <div class="feature-card">
            <div class="feature-icon">&#9728;</div>
            <h3>WebAssembly</h3>
            <p>Run Oxy in the browser via WASM. This playground and tour are powered by it.</p>
          </div>
          <div class="feature-card">
            <div class="feature-icon">&#9998;</div>
            <h3>VS Code Extension</h3>
            <p>Syntax highlighting, completions, hover info, go-to-definition, and diagnostics.</p>
          </div>
        </div>
      </section>

      <section class="cta-section">
        <h2>Ready to try it?</h2>
        <p>Open the playground and start writing Oxy code right in your browser.</p>
        <a href="#/playground" class="btn btn-primary btn-lg">Open Playground</a>
      </section>
    `;
    container.appendChild(this.el);
  }

  unmount(): void {
    this.el?.remove();
    this.el = null;
  }
}
