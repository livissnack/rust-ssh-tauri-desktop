import { createVNode, render } from 'vue';
import ConfirmComponent from './Confirm.vue';

export const confirm = (message: string, type: 'info' | 'success' | 'warning' | 'error' = 'warning', title?: string): Promise<boolean> => {
    return new Promise((resolve) => {
        const div = document.createElement('div');
        document.body.appendChild(div);

        const vnode = createVNode(ConfirmComponent, {
            message,
            type,
            title,
            // 这里的 onResolve 会被组件内部调用 👈
            onResolve: (result: boolean) => {
                resolve(result);
                render(null, div); // 卸载组件
                document.body.removeChild(div); // 移除 DOM
            }
        });

        render(vnode, div);
    });
};

// 方便调用的快捷方式
confirm.error = (msg: string, title?: string) => confirm(msg, 'error', title);
confirm.warning = (msg: string, title?: string) => confirm(msg, 'warning', title);