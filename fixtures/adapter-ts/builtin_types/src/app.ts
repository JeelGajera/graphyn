interface Config {
    host: string;
    port: number;
}

function greet(name: string): string {
    return name.toUpperCase();
}

const items: Array<number> = [1, 2, 3];
const len = items.length;

type PartialConfig = Partial<Config>;

export function processConfig(cfg: Config): string {
    return cfg.host;
}
