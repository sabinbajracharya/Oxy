export interface View {
  mount(container: HTMLElement): void;
  unmount(): void;
}

export type ViewFactory = (params: RegExpMatchArray) => View | Promise<View>;

export interface Route {
  pattern: RegExp;
  factory: ViewFactory;
}
