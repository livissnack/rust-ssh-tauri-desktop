export const LOCAL_SERVER_ID = '__local__';

export type SessionStatus = 'idle' | 'connecting' | 'connected' | 'failed' | 'disconnected';

export type OpenSession = {
  id: string;
  serverId: string;
  name: string;
  kind?: 'ssh' | 'local';
};

export function isLocalSession(
  session: Pick<OpenSession, 'serverId' | 'kind'> | null | undefined
) {
  if (!session) return false;
  return session.serverId === LOCAL_SERVER_ID || session.kind === 'local';
}

export function isSessionConnected(status: SessionStatus | undefined) {
  return status === 'connected';
}