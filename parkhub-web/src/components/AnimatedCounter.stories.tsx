// SPDX-License-Identifier: MIT OR Apache-2.0
//
// Storybook story for `AnimatedCounter`. Acts as the canonical example for
// authoring new stories in parkhub-web — keep additions minimal and focused
// on a single component prop surface.

import type { Meta, StoryObj } from '@storybook/react';
import { AnimatedCounter } from './AnimatedCounter';

const meta = {
	title: 'Components/AnimatedCounter',
	component: AnimatedCounter,
	tags: ['autodocs'],
	args: {
		value: 1234,
		duration: 1000,
	},
	argTypes: {
		value: { control: { type: 'number' } },
		duration: { control: { type: 'number', min: 0, max: 5000, step: 100 } },
		format: {
			control: { type: 'inline-radio' },
			options: ['number', 'currency', 'percent'],
		},
		currency: { control: { type: 'text' } },
		maximumFractionDigits: { control: { type: 'number', min: 0, max: 4, step: 1 } },
	},
} satisfies Meta<typeof AnimatedCounter>;

export default meta;

type Story = StoryObj<typeof meta>;

export const Number: Story = {
	args: {
		value: 1234,
		format: 'number',
	},
};

export const Currency: Story = {
	args: {
		value: 4299,
		format: 'currency',
		currency: 'EUR',
		maximumFractionDigits: 2,
	},
};

export const Percent: Story = {
	args: {
		value: 0.876,
		format: 'percent',
		maximumFractionDigits: 1,
	},
};
