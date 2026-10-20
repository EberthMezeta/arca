import { create } from "zustand";

export type ToastTone = "success" | "info" | "error";

export interface ToastAction {
  actionLabel: string;
  runAction: () => void;
}

export interface ToastNotification {
  toastId: number;
  message: string;
  tone: ToastTone;
  durationMilliseconds: number;
  toastAction: ToastAction | null;
  /** Si existe, se muestra una barra que se vacía (p. ej. borrado del portapapeles). */
  countdownSeconds: number | null;
}

export type ToastRequest = Pick<ToastNotification, "message"> & Partial<Omit<ToastNotification, "toastId" | "message">>;

interface ToastState {
  visibleToasts: ToastNotification[];
  showToast: (toastRequest: ToastRequest) => void;
  dismissToast: (toastId: number) => void;
}

const DEFAULT_TOAST_DURATION_MILLISECONDS = 4000;
const MAXIMUM_VISIBLE_TOASTS = 3;
let nextToastId = 1;

export const useToastStore = create<ToastState>((setState, getState) => ({
  visibleToasts: [],

  showToast: (toastRequest) => {
    const toastNotification: ToastNotification = {
      toastId: nextToastId++,
      tone: "info",
      durationMilliseconds: DEFAULT_TOAST_DURATION_MILLISECONDS,
      toastAction: null,
      countdownSeconds: null,
      ...toastRequest,
    };
    setState((currentState) => ({
      visibleToasts: [...currentState.visibleToasts, toastNotification].slice(-MAXIMUM_VISIBLE_TOASTS),
    }));
    window.setTimeout(() => getState().dismissToast(toastNotification.toastId), toastNotification.durationMilliseconds);
  },

  dismissToast: (toastId) =>
    setState((currentState) => ({
      visibleToasts: currentState.visibleToasts.filter((visibleToast) => visibleToast.toastId !== toastId),
    })),
}));
