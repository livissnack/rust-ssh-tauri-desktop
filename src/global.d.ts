export {};

declare global {
  interface Window {
    __INITIAL_SERVERS__?: unknown[];
    __SESSION_BOOTSTRAP__?: {
      server_id: string;
      session_id: string;
      session_name: string;
      is_local?: boolean;
    };
  }
}
