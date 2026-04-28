import { renderBlockquote, renderCodeBlock } from './components';

export function render(): string {
    return renderBlockquote() + renderCodeBlock();
}
