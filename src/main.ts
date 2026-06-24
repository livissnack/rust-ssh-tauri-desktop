import { createApp } from "vue";
import App from "./App.vue";
import Tooltip from "./components/Tooltip.vue";
import NumberInput from "./components/NumberInput.vue";
import AppSelect from "./components/AppSelect.vue";
import "@fortawesome/fontawesome-free/css/all.min.css";
createApp(App)
  .component('Tooltip', Tooltip)
  .component('NumberInput', NumberInput)
  .component('AppSelect', AppSelect)
  .mount("#app");
