export const LOCAL_SERVER_ID = '__local__';

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
