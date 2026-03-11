import {createVNode, render} from 'vue';
import ToastComponent from './Toast.vue';

const container = document.createElement('div');
container.className = 'toast-container';
document.body.appendChild(container);

Object.assign(container.style, {
    position: 'fixed',
    right: '20px',
    bottom: '20px',
    display: 'flex',
    flexDirection: 'column-reverse',
    zIndex: '9999',
    pointerEvents: 'none'
});

export const toast = {
    show(message: string, type: 'info' | 'success' | 'warning' | 'error' = 'info', title?: string) {
        const div = document.createElement('div');
        const vnode = createVNode(ToastComponent, {
            message,
            type,
            title,
            duration: 3000,
            onVnodeUnmounted: () => {
                if (container.contains(div)) container.removeChild(div);
            }
        });

        render(vnode, div);
        container.appendChild(div);
    },
    success: (msg: string, title?: string) => toast.show(msg, 'success', title),
    error: (msg: string, title?: string) => toast.show(msg, 'error', title),
    warning: (msg: string, title?: string) => toast.show(msg, 'warning', title),
};