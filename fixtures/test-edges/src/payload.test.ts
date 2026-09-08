import { Payload, encode } from './payload';

// A test file by convention (`*.test.ts`), so every reference it makes into
// non-test code becomes a `tests` edge in addition to the call it already was.
export function testEncode() {
  const p = new Payload();
  return encode(p);
}
