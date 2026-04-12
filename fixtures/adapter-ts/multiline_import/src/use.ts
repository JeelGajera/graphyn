import {
  Session as AuthSession,
} from './model';

export function render(session: AuthSession): string {
  return session.userId + ':' + session.token;
}
