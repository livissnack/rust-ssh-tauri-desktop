import { ref } from 'vue';

export interface ThemeOption {
    id: string;
    name: string;
    isLight: boolean;
    color: string;
}

export const defaultTheme = ref(localStorage.getItem('app-theme-id') || 'monokai-pro');

export const themeOptions: ThemeOption[] = [
    { id: 'monokai-pro', name: 'Monokai Pro', isLight: false, color: '#ffd866' },
    { id: 'catppuccin', name: 'Catppuccin Macchiato', isLight: false, color: '#8aadf4' },
    { id: 'tokyo-night', name: 'Tokyo Night', isLight: false, color: '#7aa2f7' },
    { id: 'one-hunter', name: 'One Hunter', isLight: false, color: '#4db6ac' },
    { id: 'microsoft-dark', name: 'Microsoft Dark', isLight: false, color: '#2886de' },
    { id: 'github-light', name: 'GitHub Light', isLight: true, color: '#0969da' },
    { id: 'azure-light', name: 'Azure Light', isLight: true, color: '#0078d4' },
    { id: 'rmb-red', name: '炫丽红 (100¥)', isLight: false, color: '#ff4d4f' },
    { id: 'rmb-green', name: '翠绿 (50¥)', isLight: false, color: '#40c0a0' },
    { id: 'rmb-brown', name: '荷花褐 (20¥)', isLight: false, color: '#d4a017' },
    { id: 'rmb-blue', name: '玫瑰蓝 (10¥)', isLight: false, color: '#4a90e2' },
];

export const applyTheme = (theme: string | any) => {
    // 核心修复：确保 themeId 最终是一个纯字符串
    const themeId = typeof theme === 'object' ? theme.id : theme;

    // 安全检查：如果 themeId 还是无效（比如 undefined），直接返回
    if (!themeId || typeof themeId !== 'string' || themeId.includes('[object')) {
        console.error('Invalid themeId provided to applyTheme:', theme);
        return;
    }

    const root = document.documentElement;
    themeOptions.forEach(opt => root.classList.remove(`${opt.id}-theme`));

    // 现在这里绝对不会出现 "[object Object]-theme"
    root.classList.add(`${themeId}-theme`);

    localStorage.setItem('app-theme-id', themeId);
    defaultTheme.value = themeId;
};