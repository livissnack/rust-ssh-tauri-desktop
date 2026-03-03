import { createVNode, render } from 'vue';
import ToastComponent from './Toast.vue';

// 定义容器
const container = document.createElement('div');
container.className = 'toast-container';
document.body.appendChild(container);

// 容器样式：固定在右下角
Object.assign(container.style, {
    position: 'fixed',
    right: '20px',
    bottom: '20px',
    display: 'flex',
    flexDirection: 'column-reverse', // 新的弹窗在上方堆叠
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
                // 动画结束从 DOM 移除
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