export class Payload {
  userId: string;
  email: string;
}

export function encode(payload: Payload): string {
  return payload.userId;
}
