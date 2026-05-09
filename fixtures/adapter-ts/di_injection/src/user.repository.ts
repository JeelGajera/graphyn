export class UserRepository {
  async findById(id: string): Promise<User | null> { return null; }
  async findAll(): Promise<User[]> { return []; }
}

type User = { id: string; email: string };
