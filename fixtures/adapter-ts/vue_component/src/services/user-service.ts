import type { User } from "../types/user";

export class UserService {
  static getCurrent(): User {
    return { id: "1", name: "Test", email: "test@example.com" };
  }
}
