/**
 * 节流函数：在指定时间内只执行第一次调用
 * 适用于：防止按钮连点、防止动画冲突
 */
export function throttle<T extends (...args: any[]) => any>(
    fn: T,
    delay: number
): (...args: Parameters<T>) => void {
    let lastExecTime = 0;

    return function(this: any, ...args: Parameters<T>) {
        const now = Date.now();
        if (now - lastExecTime >= delay) {
            lastExecTime = now;
            fn.apply(this, args);
        }
    };
}

/**
 * 防抖函数：在指定时间内最后一次调用后延迟执行
 * 适用于：搜索输入框请求、窗口 resize 监听
 */
export function debounce<T extends (...args: any[]) => any>(
    fn: T,
    delay: number
): (...args: Parameters<T>) => void {
    let timer: ReturnType<typeof setTimeout> | null = null;

    return function(this: any, ...args: Parameters<T>) {
        if (timer) clearTimeout(timer);
        timer = setTimeout(() => {
            fn.apply(this, args);
        }, delay);
    };
}

export const formatSize = (bytes: number) => {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(i > 0 ? 1 : 0)) + ' ' + sizes[i];
};