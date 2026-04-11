import { Session } from './types';

export function render(session: Session): string {
  return `${session.userId}-${session.token}`;
}
