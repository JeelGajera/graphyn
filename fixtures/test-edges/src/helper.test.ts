import { testEncode } from './payload.test';

// A test referring to another test. This must NOT produce a `tests` edge:
// a helper exercising a helper is not coverage of the code under test, and
// counting it would make every test appear to cover the whole suite.
export function testViaHelper() {
  return testEncode();
}
