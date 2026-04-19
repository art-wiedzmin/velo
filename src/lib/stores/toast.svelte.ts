export class ToastStore {
  message = $state<string | null>(null);
  visible = $state(false);
  private timer: number | null = null;

  show(msg: string, durationMs = 2200): void {
    this.message = msg;
    this.visible = true;
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = setTimeout(() => {
      this.visible = false;
    }, durationMs) as unknown as number;
  }
}

export const toast = new ToastStore();
