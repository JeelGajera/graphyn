import { Account as DeepAccount } from '../../models/account';

export function mapAccount(input: DeepAccount): string {
  return input.id;
}
