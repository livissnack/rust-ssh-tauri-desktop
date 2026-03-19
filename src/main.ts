import { createApp } from "vue";
import App from "./App.vue";
import "@fortawesome/fontawesome-free/css/all.min.css";
// 引入基础字重
import "npm:@fontsource/jetbrains-mono/400.css";
// 引入加粗字重（终端渲染强烈建议）
import "npm:@fontsource/jetbrains-mono/700.css";
// 引入斜体（可选，用于代码注释）
import "npm:@fontsource/jetbrains-mono/400-italic.css";

createApp(App).mount("#app");
