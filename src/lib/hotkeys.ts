import { events } from '$lib';

export async function on(combo: string, callback: (event: KeyboardEvent) => void) {
	const comboContainsControlKeys =
		combo.includes('Meta') || combo.includes('Alt') || combo.includes('Ctrl');

	return import('tinykeys').then(({ default: tinykeys }) =>
		tinykeys(window, {
			[combo]: (event) => {
				const target = event.target as HTMLElement;
				const isInput = target.tagName === 'INPUT' || target.tagName === 'TEXTAREA';
				if (isInput && !comboContainsControlKeys) return;

				event.preventDefault();
				event.stopPropagation();

				events.emit('closeCommandPalette');
				callback(event);
			}
		})
	);
}
