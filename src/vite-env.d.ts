/// <reference types="vite/client" />

declare global {
  interface Window {
    __INITIAL_SERVERS__?: any[];
    __SESSION_BOOTSTRAP__?: {
      server_id: string;
      session_id: string;
      session_name: string;
      is_local?: boolean;
    };
  }
}

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}
