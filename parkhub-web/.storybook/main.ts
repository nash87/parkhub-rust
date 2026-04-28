// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Storybook 10 config for parkhub-web (React + Astro + Vite).
//
// Run locally:
//     npx storybook dev -p 6006
//
// Build static site:
//     npx storybook build
//
// Stories live next to the components they document (`*.stories.tsx`).
// MDX docs pages are accepted but not required.

import type { StorybookConfig } from '@storybook/react-vite';

const config: StorybookConfig = {
	stories: ['../src/**/*.mdx', '../src/**/*.stories.@(ts|tsx)'],
	addons: ['@storybook/addon-a11y'],
	framework: {
		name: '@storybook/react-vite',
		options: {},
	},
	docs: {
		autodocs: 'tag',
	},
	typescript: {
		check: false,
		reactDocgen: 'react-docgen-typescript',
	},
	core: {
		disableTelemetry: true,
		disableWhatsNewNotifications: true,
	},
};

export default config;
