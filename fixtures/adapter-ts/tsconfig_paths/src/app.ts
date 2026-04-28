import { helper } from '@utils/helpers';
import { AppConfig } from '@/config';

export function main(): string {
    const cfg: AppConfig = { name: 'test' };
    return helper() + cfg.name;
}
