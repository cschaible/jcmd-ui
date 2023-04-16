import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/kit/vite';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// Consult https://kit.svelte.dev/docs/integrations#preprocessors
	// for more information about preprocessors
	preprocess: vitePreprocess(),

	kit: {
		// adapter-auto only supports some environments, see https://kit.svelte.dev/docs/adapter-auto for a list.
		// If your environment is not supported or you settled on a specific environment, switch out the adapter.
		// See https://kit.svelte.dev/docs/adapters for more information about adapters.
		adapter: adapter()
	},
	// Disable warning: visible, non-interactive elements with an on:click event must be accompanied by an
	// on:keydown, on:keyup, or on:keypress event
	// More details can be found here:
	// https://stackoverflow.com/questions/74974066/
	// visible-non-interactive-elements-with-an-onclick-event-must-be-accompanied-by
	onwarn: (warning, handler) => {
		if (warning.code === 'a11y-click-events-have-key-events') return
		handler(warning)
	}
};

export default config;
