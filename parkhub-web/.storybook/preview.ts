// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Storybook 9 preview wiring for parkhub-web. Pulls in the Tailwind v4 sheet
// + i18n bootstrap so stories render with the same styling pipeline as the
// real app.

import type { Preview } from '@storybook/react';
import '../src/styles/global.css';
import '../src/i18n';

const preview: Preview = {
	parameters: {
		controls: {
			matchers: {
				color: /(background|color)$/i,
				date: /Date$/i,
			},
		},
		a11y: {
			// axe-core config — fail loudly on serious violations during story
			// runs but keep moderate/minor issues at warn-level so authoring
			// stays fluid.
			config: {
				rules: [
					{ id: 'color-contrast', enabled: true },
					{ id: 'landmark-one-main', enabled: false },
				],
			},
		},
		backgrounds: {
			default: 'app',
			values: [
				{ name: 'app', value: '#0b0d12' },
				{ name: 'light', value: '#ffffff' },
			],
		},
	},
	tags: ['autodocs'],
};

export default preview;
