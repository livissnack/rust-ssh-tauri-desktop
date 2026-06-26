import { createVNode, render } from 'vue';
import ToastComponent from './Toast.vue';

const container = document.createElement('div');
container.className = 'toast-container';
document.body.appendChild(container);

Object.assign(container.style, {
  position: 'fixed',
  right: '20px',
  bottom: '24px',
  display: 'flex',
  flexDirection: 'column-reverse',
  alignItems: 'flex-end',
  gap: '10px',
  zIndex: '9999',
  pointerEvents: 'none',
  maxWidth: 'calc(100vw - 32px)',
});

export const toast = {
  show(message: string, type: 'info' | 'success' | 'warning' | 'error' = 'info', title?: string) {
    const host = document.createElement('div');
    host.className = 'toast-host';

    const vnode = createVNode(ToastComponent, {
      message,
      type,
      title,
      duration: 3200,
      onDismiss: () => {
        render(null, host);
        if (container.contains(host)) container.removeChild(host);
      },
    });

    render(vnode, host);
    container.appendChild(host);
  },
  success: (msg: string, title?: string) => toast.show(msg, 'success', title),
  error: (msg: string, title?: string) => toast.show(msg, 'error', title),
  warning: (msg: string, title?: string) => toast.show(msg, 'warning', title),
  info: (msg: string, title?: string) => toast.show(msg, 'info', title),
};
