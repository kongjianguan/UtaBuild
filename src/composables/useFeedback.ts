import { reactive } from 'vue';
import type { ConfirmDialogOptions, ToastAction, ToastState, ToastTone } from '../types';

const toast = reactive<ToastState>({
  id: 0,
  visible: false,
  message: '',
  tone: 'error',
  action: undefined,
});

let toastTimer: ReturnType<typeof setTimeout> | null = null;

export function hideToast(): void {
  toast.visible = false;
  toast.action = undefined;
  if (toastTimer) {
    clearTimeout(toastTimer);
    toastTimer = null;
  }
}

export function showToast(msg: string, tone: ToastTone = 'error', action?: ToastAction): void {
  if (toastTimer) {
    clearTimeout(toastTimer);
    toastTimer = null;
  }

  toast.id += 1;
  toast.message = msg;
  toast.tone = tone;
  toast.action = action;
  toast.visible = true;

  if (tone !== 'error') {
    toastTimer = setTimeout(() => {
      hideToast();
    }, 4200);
  }
}

export const showError = (msg: string, action?: ToastAction) => showToast(msg, 'error', action);
export const showSuccess = (msg: string) => showToast(msg, 'success');
export const showInfo = (msg: string) => showToast(msg, 'info');

interface ConfirmState {
  open: boolean;
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  danger: boolean;
}

const confirmState = reactive<ConfirmState>({
  open: false,
  title: '',
  message: '',
  confirmLabel: '確認',
  cancelLabel: 'キャンセル',
  danger: false,
});

let pendingResolver: ((confirmed: boolean) => void) | null = null;

export function confirmDialog(options: ConfirmDialogOptions): Promise<boolean> {
  if (pendingResolver) {
    return Promise.resolve(false);
  }

  confirmState.title = options.title;
  confirmState.message = options.message;
  confirmState.confirmLabel = options.confirmLabel || '確認';
  confirmState.cancelLabel = options.cancelLabel || 'キャンセル';
  confirmState.danger = options.danger === true;
  confirmState.open = true;

  return new Promise<boolean>((resolve) => {
    pendingResolver = resolve;
  });
}

export function resolveConfirm(confirmed: boolean): void {
  if (!pendingResolver) {
    confirmState.open = false;
    return;
  }
  const resolver = pendingResolver;
  pendingResolver = null;
  confirmState.open = false;
  resolver(confirmed);
}

export function useFeedback() {
  return {
    toast,
    confirmState,
    showToast,
    hideToast,
    showError,
    showSuccess,
    showInfo,
    confirmDialog,
    resolveConfirm,
  };
}
